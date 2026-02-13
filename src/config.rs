use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_refresh")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_refresh() -> u64 {
    60
}

fn default_currency() -> String {
    "usd".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh(),
            currency: default_currency(),
            theme: default_theme(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let mut cfg: Config = serde_yaml::from_str(&contents)?;
            if cfg.refresh_interval_secs < 30 {
                cfg.refresh_interval_secs = 30;
            }
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        fs::write(&path, yaml)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("bags");
        path.push("config.yaml");
        path
    }
}
