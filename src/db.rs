use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use crate::types::{AlertDirection, Holding, PriceAlert};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(password: &str) -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)
            .context("Failed to open database")?;

        conn.pragma_update(None, "key", password)?;

        // Test that the key works by querying the schema
        let ok = conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            });

        if ok.is_err() {
            anyhow::bail!("Wrong password or corrupted database");
        }

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS favourites (
                coin_id TEXT PRIMARY KEY
            );
            CREATE TABLE IF NOT EXISTS holdings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                coin_id TEXT NOT NULL UNIQUE,
                amount REAL NOT NULL DEFAULT 0.0
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS price_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                coin_id TEXT NOT NULL,
                target_price REAL NOT NULL,
                direction TEXT NOT NULL DEFAULT 'above',
                triggered INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        // Migration: add buy_price column if it doesn't exist
        let _ = conn.execute_batch("ALTER TABLE holdings ADD COLUMN buy_price REAL");

        Ok(Self { conn })
    }

    fn db_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("bags");
        path.push("bags.db");
        path
    }

    // -- Favourites --

    pub fn is_favourite(&self, coin_id: &str) -> bool {
        self.conn
            .query_row(
                "SELECT 1 FROM favourites WHERE coin_id = ?1",
                [coin_id],
                |_| Ok(()),
            )
            .is_ok()
    }

    pub fn toggle_favourite(&self, coin_id: &str) -> Result<bool> {
        if self.is_favourite(coin_id) {
            self.conn
                .execute("DELETE FROM favourites WHERE coin_id = ?1", [coin_id])?;
            Ok(false)
        } else {
            self.conn
                .execute("INSERT INTO favourites (coin_id) VALUES (?1)", [coin_id])?;
            Ok(true)
        }
    }

    pub fn get_favourites(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT coin_id FROM favourites")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // -- Holdings --

    pub fn set_holding(&self, coin_id: &str, amount: f64, buy_price: Option<f64>) -> Result<()> {
        if amount <= 0.0 {
            self.conn
                .execute("DELETE FROM holdings WHERE coin_id = ?1", [coin_id])?;
        } else {
            self.conn.execute(
                "INSERT INTO holdings (coin_id, amount, buy_price) VALUES (?1, ?2, ?3)
                 ON CONFLICT(coin_id) DO UPDATE SET amount = excluded.amount,
                 buy_price = COALESCE(excluded.buy_price, holdings.buy_price)",
                rusqlite::params![coin_id, amount, buy_price],
            )?;
        }
        Ok(())
    }

    pub fn set_buy_price(&self, coin_id: &str, price: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE holdings SET buy_price = ?1 WHERE coin_id = ?2",
            rusqlite::params![price, coin_id],
        )?;
        Ok(())
    }

    // -- Settings --

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            self.conn
                .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        } else {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
        }
        Ok(())
    }

    pub fn get_holdings(&self) -> Result<Vec<Holding>> {
        let mut stmt = self
            .conn
            .prepare("SELECT coin_id, amount, buy_price FROM holdings WHERE amount > 0")?;
        let rows = stmt.query_map([], |row| {
            Ok(Holding {
                coin_id: row.get(0)?,
                amount: row.get(1)?,
                buy_price: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // -- Price Alerts --

    pub fn add_alert(&self, coin_id: &str, target_price: f64, direction: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO price_alerts (coin_id, target_price, direction) VALUES (?1, ?2, ?3)",
            rusqlite::params![coin_id, target_price, direction],
        )?;
        Ok(())
    }

    pub fn get_alerts(&self) -> Result<Vec<PriceAlert>> {
        let mut stmt = self
            .conn
            .prepare("SELECT coin_id, target_price, direction, triggered FROM price_alerts")?;
        let rows = stmt.query_map([], |row| {
            let dir_str: String = row.get(2)?;
            let direction = if dir_str == "below" {
                AlertDirection::Below
            } else {
                AlertDirection::Above
            };
            Ok(PriceAlert {
                coin_id: row.get(0)?,
                target_price: row.get(1)?,
                direction,
                triggered: row.get::<_, i32>(3)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn mark_alert_triggered(&self, coin_id: &str, target_price: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE price_alerts SET triggered = 1 WHERE coin_id = ?1 AND target_price = ?2",
            rusqlite::params![coin_id, target_price],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_alert(&self, coin_id: &str, target_price: f64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM price_alerts WHERE coin_id = ?1 AND target_price = ?2",
            rusqlite::params![coin_id, target_price],
        )?;
        Ok(())
    }

}
