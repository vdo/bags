use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::CoinGeckoClient;
use crate::config::Config;
use crate::db::Db;
use crate::theme::{self, Theme, THEME_NAMES};
use crate::types::*;

pub struct App {
    pub tab: Tab,
    pub coins: Vec<Coin>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub page_height: usize,
    pub popup_open: bool,
    pub chart_view: ChartView,
    pub chart_cache: HashMap<(String, u32), PriceHistory>,
    pub loading_chart: bool,
    pub input_mode: InputMode,
    pub input_buf: String,
    pub db: Option<Arc<Mutex<Db>>>,
    pub favourites: Vec<String>,
    pub holdings: Vec<Holding>,
    pub last_refresh: Option<std::time::Instant>,
    pub last_refresh_display: String,
    pub error: Option<String>,
    pub loading: bool,
    pub config: Config,
    pub theme: Theme,
    pub quit: bool,
    // Lock screen
    pub password_buf: String,
    pub password_first: String,
    pub password_error: Option<String>,
    pub is_new_db: bool,
    pub unlocked: bool,
    // Settings
    pub settings_field: SettingsField,
    pub settings_coingecko_key: String,
    pub settings_cmc_key: String,
    pub settings_currency_idx: usize,
    pub settings_theme_idx: usize,
    pub settings_editing: bool,
    pub coingecko_api_key: String,
    pub cmc_api_key: String,
    // Search
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_selected: usize,
    pub search_loading: bool,
    pub search_error: Option<String>,
}

impl App {
    pub fn new(config: Config, is_new_db: bool) -> Self {
        let loaded_theme = theme::by_name(&config.theme);
        let theme_idx = THEME_NAMES
            .iter()
            .position(|t| *t == config.theme)
            .unwrap_or(0);
        Self {
            tab: Tab::Markets,
            coins: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            page_height: 20,
            popup_open: false,
            chart_view: ChartView::Day1,
            chart_cache: HashMap::new(),
            loading_chart: false,
            input_mode: InputMode::Password,
            input_buf: String::new(),
            db: None,
            favourites: Vec::new(),
            holdings: Vec::new(),
            last_refresh: None,
            last_refresh_display: String::new(),
            error: None,
            loading: true,
            config,
            theme: loaded_theme,
            quit: false,
            password_buf: String::new(),
            password_first: String::new(),
            password_error: None,
            is_new_db,
            unlocked: false,
            settings_field: SettingsField::Currency,
            settings_coingecko_key: String::new(),
            settings_cmc_key: String::new(),
            settings_currency_idx: 0,
            settings_theme_idx: theme_idx,
            settings_editing: false,
            coingecko_api_key: String::new(),
            cmc_api_key: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_loading: false,
            search_error: None,
        }
    }

    pub fn unlock(&mut self, db: Db) {
        let db = Arc::new(Mutex::new(db));
        self.db = Some(db);
        self.unlocked = true;
        self.input_mode = InputMode::Normal;
        self.password_buf.clear();
        self.password_first.clear();
    }

    pub fn load_api_keys_from_db(&mut self, db_lock: &Db) {
        self.coingecko_api_key = db_lock
            .get_setting("coingecko_api_key")
            .unwrap_or_default();
        self.cmc_api_key = db_lock
            .get_setting("cmc_api_key")
            .unwrap_or_default();
    }

    pub fn visible_coins(&self) -> Vec<(usize, &Coin)> {
        match self.tab {
            Tab::Markets => self.coins.iter().enumerate().collect(),
            Tab::Favourites => self
                .coins
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    self.favourites.contains(&c.id)
                        || self.holdings.iter().any(|h| h.coin_id == c.id && h.amount > 0.0)
                })
                .collect(),
            Tab::Portfolio => self
                .coins
                .iter()
                .enumerate()
                .filter(|(_, c)| self.holdings.iter().any(|h| h.coin_id == c.id && h.amount > 0.0))
                .collect(),
        }
    }

    pub fn selected_coin(&self) -> Option<&Coin> {
        let visible = self.visible_coins();
        visible.get(self.selected).map(|(_, c)| *c)
    }

    pub fn clamp_selection(&mut self) {
        let len = self.visible_coins().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
        self.adjust_scroll();
    }

    pub fn adjust_scroll(&mut self) {
        if self.page_height == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.page_height {
            self.scroll_offset = self.selected - self.page_height + 1;
        }
    }

    pub fn total_portfolio_value(&self) -> f64 {
        self.holdings
            .iter()
            .filter_map(|h| {
                self.coins
                    .iter()
                    .find(|c| c.id == h.coin_id)
                    .map(|c| c.current_price * h.amount)
            })
            .sum()
    }

    pub fn holding_for(&self, coin_id: &str) -> f64 {
        self.holdings
            .iter()
            .find(|h| h.coin_id == coin_id)
            .map(|h| h.amount)
            .unwrap_or(0.0)
    }

    pub fn update_refresh_display(&mut self) {
        if let Some(inst) = self.last_refresh {
            let secs = inst.elapsed().as_secs();
            if secs < 60 {
                self.last_refresh_display = format!("{}s ago", secs);
            } else {
                self.last_refresh_display = format!("{}m ago", secs / 60);
            }
        }
    }

    pub async fn refresh_db_state(&mut self) {
        if let Some(ref db) = self.db {
            let db = db.lock().await;
            self.favourites = db.get_favourites().unwrap_or_default();
            self.holdings = db.get_holdings().unwrap_or_default();
        }
    }

    pub async fn refresh_market_data(&mut self, client: &CoinGeckoClient) {
        match client.fetch_markets(50).await {
            Ok(coins) => {
                self.coins = coins;
                self.last_refresh = Some(std::time::Instant::now());
                self.error = None;
                self.loading = false;
            }
            Err(e) => {
                self.set_error(format!("API: {}", e));
                self.loading = false;
            }
        }
    }

    pub fn open_settings(&mut self) {
        self.settings_coingecko_key = self.coingecko_api_key.clone();
        self.settings_cmc_key = self.cmc_api_key.clone();
        self.settings_currency_idx = CURRENCIES
            .iter()
            .position(|c| *c == self.config.currency)
            .unwrap_or(0);
        self.settings_theme_idx = THEME_NAMES
            .iter()
            .position(|t| *t == self.config.theme)
            .unwrap_or(0);
        self.settings_field = SettingsField::Currency;
        self.settings_editing = false;
        self.input_mode = InputMode::Settings;
    }

    pub fn current_settings_value_mut(&mut self) -> &mut String {
        match self.settings_field {
            SettingsField::CoingeckoApiKey => &mut self.settings_coingecko_key,
            SettingsField::CoinmarketcapApiKey => &mut self.settings_cmc_key,
            SettingsField::Currency | SettingsField::Theme => &mut self.settings_coingecko_key, // unused for cycle fields
        }
    }

    pub fn cycle_currency(&mut self, forward: bool) {
        let len = CURRENCIES.len();
        if forward {
            self.settings_currency_idx = (self.settings_currency_idx + 1) % len;
        } else {
            self.settings_currency_idx = (self.settings_currency_idx + len - 1) % len;
        }
    }

    pub fn cycle_theme(&mut self, forward: bool) {
        let len = THEME_NAMES.len();
        if forward {
            self.settings_theme_idx = (self.settings_theme_idx + 1) % len;
        } else {
            self.settings_theme_idx = (self.settings_theme_idx + len - 1) % len;
        }
        // Live preview
        self.theme = theme::by_name(THEME_NAMES[self.settings_theme_idx]);
    }

    pub fn set_error(&mut self, msg: String) {
        log_error(&msg);
        self.error = Some(msg);
    }
}

fn log_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("bags");
    path.push("errors.log");
    path
}

pub fn log_error(msg: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "[{}] {}", now, msg);
    }
}
