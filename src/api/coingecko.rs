use anyhow::{Context, Result};
use reqwest::Client;
use crate::types::{Coin, GlobalMarketStats, PriceHistory, SearchResult};

const BASE_URL: &str = "https://api.coingecko.com/api/v3";
const PRO_BASE_URL: &str = "https://pro-api.coingecko.com/api/v3";

pub struct CoinGeckoClient {
    client: Client,
    currency: String,
    api_key: String,
}

impl CoinGeckoClient {
    pub fn new(currency: &str, api_key: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (compatible; desktop-app)")
                .build()
                .unwrap(),
            currency: currency.to_lowercase(),
            api_key: api_key.to_string(),
        }
    }

    fn base_url(&self) -> &str {
        if self.api_key.is_empty() {
            BASE_URL
        } else {
            PRO_BASE_URL
        }
    }

    fn apply_key(&self, url: &str) -> String {
        if self.api_key.is_empty() {
            url.to_string()
        } else {
            let sep = if url.contains('?') { "&" } else { "?" };
            format!("{}{}x_cg_pro_api_key={}", url, sep, self.api_key)
        }
    }

    pub async fn fetch_markets(&self, limit: u32) -> Result<Vec<Coin>> {
        let url = format!(
            "{}/coins/markets?vs_currency={}&order=market_cap_desc&per_page={}&page=1&sparkline=false&price_change_percentage=1h,24h,7d",
            self.base_url(), self.currency, limit
        );
        let url = self.apply_key(&url);

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach CoinGecko API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error {}: {}", status, body);
        }

        let text = resp.text().await.context("Failed to read response body")?;
        let coins: Vec<Coin> = match serde_json::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                anyhow::bail!(
                    "Failed to parse market data: {} | response: {}",
                    e,
                    &text[..text.len().min(300)]
                );
            }
        };
        Ok(coins)
    }

    pub async fn fetch_price_history(&self, coin_id: &str, days: u32) -> Result<PriceHistory> {
        let url = format!(
            "{}/coins/{}/market_chart?vs_currency={}&days={}",
            self.base_url(), coin_id, self.currency, days
        );
        let url = self.apply_key(&url);

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach CoinGecko API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error {}: {}", status, body);
        }

        let data: serde_json::Value = resp.json().await?;
        let price_points = data["prices"]
            .as_array()
            .context("Missing prices array")?;

        let mut prices = Vec::with_capacity(price_points.len());

        for point in price_points {
            let arr = point.as_array().context("Invalid price point")?;
            if arr.len() >= 2 {
                prices.push(arr[1].as_f64().unwrap_or(0.0));
            }
        }

        Ok(PriceHistory { prices })
    }

    pub async fn search_coins(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!(
            "{}/search?query={}",
            self.base_url(),
            query
        );
        let url = self.apply_key(&url);

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach CoinGecko API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error {}: {}", status, body);
        }

        let data: serde_json::Value = resp.json().await?;
        let coins = data["coins"]
            .as_array()
            .context("Missing coins array")?;

        let results: Vec<SearchResult> = coins
            .iter()
            .take(10)
            .filter_map(|c| {
                Some(SearchResult {
                    id: c["id"].as_str()?.to_string(),
                    name: c["name"].as_str()?.to_string(),
                    symbol: c["symbol"].as_str()?.to_string(),
                    market_cap_rank: c["market_cap_rank"].as_u64().map(|r| r as u32),
                })
            })
            .collect();

        Ok(results)
    }

    pub async fn fetch_coin_market(&self, coin_id: &str) -> Result<Option<Coin>> {
        let url = format!(
            "{}/coins/markets?vs_currency={}&ids={}&sparkline=false&price_change_percentage=1h,24h,7d",
            self.base_url(), self.currency, coin_id
        );
        let url = self.apply_key(&url);

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach CoinGecko API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error {}: {}", status, body);
        }

        let coins: Vec<Coin> = resp.json().await.context("Failed to parse coin data")?;
        Ok(coins.into_iter().next())
    }

    pub async fn fetch_global(&self) -> Result<GlobalMarketStats> {
        let url = format!("{}/global", self.base_url());
        let url = self.apply_key(&url);

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach CoinGecko API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error {}: {}", status, body);
        }

        let data: serde_json::Value = resp.json().await?;
        let d = &data["data"];

        let total_market_cap = d["total_market_cap"]["usd"]
            .as_f64()
            .unwrap_or(0.0);
        let btc_dominance = d["market_cap_percentage"]["btc"]
            .as_f64()
            .unwrap_or(0.0);

        Ok(GlobalMarketStats {
            total_market_cap_usd: total_market_cap,
            btc_dominance,
            fear_greed_index: None,
            fear_greed_label: None,
        })
    }

    pub async fn fetch_fear_greed(&self) -> Result<(u32, String)> {
        let resp = self
            .client
            .get("https://api.alternative.me/fng/")
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to reach Fear & Greed API")?;

        let data: serde_json::Value = resp.json().await?;
        let entry = &data["data"][0];
        let value = entry["value"]
            .as_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let label = entry["value_classification"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        Ok((value, label))
    }
}
