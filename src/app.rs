use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::CoinGeckoClient;
use crate::config::Config;
use crate::db::Db;
use crate::notifications;
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
    // Global stats
    pub global_stats: Option<GlobalMarketStats>,
    // Alerts
    pub alerts: Vec<PriceAlert>,
    pub alert_flash: Option<(String, std::time::Instant)>,
    pub alert_input_buf: String,
    pub alert_direction: AlertDirection,
    // Sort
    pub sort_column: Option<SortColumn>,
    pub sort_direction: SortDirection,
    pub sort_picking: bool,
    // Filter
    pub filter_query: String,
    // Notifications
    pub notification_method: NotificationMethod,
    pub ntfy_topic: String,
    pub settings_notification_idx: usize,
    pub settings_ntfy_topic: String,
    // Error timing
    pub error_time: Option<std::time::Instant>,
    // Buy price editing
    pub buy_price_buf: String,
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
            global_stats: None,
            alerts: Vec::new(),
            alert_flash: None,
            alert_input_buf: String::new(),
            alert_direction: AlertDirection::Above,
            sort_column: None,
            sort_direction: SortDirection::Asc,
            sort_picking: false,
            filter_query: String::new(),
            notification_method: NotificationMethod::None,
            ntfy_topic: String::new(),
            settings_notification_idx: 0,
            settings_ntfy_topic: String::new(),
            error_time: None,
            buy_price_buf: String::new(),
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
        self.notification_method = notification_method_from_str(
            &db_lock.get_setting("notification_method").unwrap_or_default(),
        );
        self.ntfy_topic = db_lock
            .get_setting("ntfy_topic")
            .unwrap_or_default();
        self.settings_notification_idx = NOTIFICATION_METHODS
            .iter()
            .position(|m| *m == notification_method_label(self.notification_method))
            .unwrap_or(0);
        self.alerts = db_lock.get_alerts().unwrap_or_default();
    }

    pub fn visible_coins(&self) -> Vec<(usize, &Coin)> {
        let mut items: Vec<(usize, &Coin)> = match self.tab {
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
        };

        // Apply filter
        if !self.filter_query.is_empty() {
            let q = self.filter_query.to_lowercase();
            items.retain(|(_, c)| {
                c.name.to_lowercase().contains(&q)
                    || c.symbol.to_lowercase().contains(&q)
            });
        }

        // Apply sort
        if let Some(col) = self.sort_column {
            let dir = self.sort_direction;
            items.sort_by(|(_, a), (_, b)| {
                let cmp = match col {
                    SortColumn::Rank => a.market_cap_rank.cmp(&b.market_cap_rank),
                    SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    SortColumn::Price => a.current_price.partial_cmp(&b.current_price).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::Change1h => a.price_change_percentage_1h_in_currency.partial_cmp(&b.price_change_percentage_1h_in_currency).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::Change24h => a.price_change_percentage_24h_in_currency.partial_cmp(&b.price_change_percentage_24h_in_currency).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::Change7d => a.price_change_percentage_7d_in_currency.partial_cmp(&b.price_change_percentage_7d_in_currency).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::Volume => a.total_volume.partial_cmp(&b.total_volume).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::MarketCap => a.market_cap.partial_cmp(&b.market_cap).unwrap_or(std::cmp::Ordering::Equal),
                };
                match dir {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }

        items
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
        // Auto-clear errors after 10s
        if let Some(err_time) = self.error_time {
            if err_time.elapsed().as_secs() >= 10 {
                self.error = None;
                self.error_time = None;
            }
        }
        // Clear alert flash after 2s
        if let Some((_, flash_time)) = &self.alert_flash {
            if flash_time.elapsed().as_secs() >= 2 {
                self.alert_flash = None;
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

    pub fn current_settings_value_mut(&mut self) -> &mut String {
        match self.settings_field {
            SettingsField::CoingeckoApiKey => &mut self.settings_coingecko_key,
            SettingsField::CoinmarketcapApiKey => &mut self.settings_cmc_key,
            SettingsField::NtfyTopic => &mut self.settings_ntfy_topic,
            SettingsField::Currency | SettingsField::Theme | SettingsField::Notifications => &mut self.settings_coingecko_key, // unused for cycle fields
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
        // Truncate display to 80 chars
        let display = if msg.len() > 80 {
            format!("{}...", &msg[..77])
        } else {
            msg
        };
        self.error = Some(display);
        self.error_time = Some(std::time::Instant::now());
    }

    pub fn check_alerts(&mut self) {
        for alert in &mut self.alerts {
            if alert.triggered {
                continue;
            }
            if let Some(coin) = self.coins.iter().find(|c| c.id == alert.coin_id) {
                let triggered = match alert.direction {
                    AlertDirection::Above => coin.current_price >= alert.target_price,
                    AlertDirection::Below => coin.current_price <= alert.target_price,
                };
                if triggered {
                    alert.triggered = true;
                    // Terminal bell
                    print!("\x07");
                    // Flash
                    self.alert_flash = Some((alert.coin_id.clone(), std::time::Instant::now()));
                    // Send notification
                    notifications::send_alert(
                        self.notification_method,
                        &self.ntfy_topic,
                        &coin.name,
                        alert.target_price,
                        coin.current_price,
                        alert.direction,
                    );
                    // Mark in DB
                    if let Some(ref db) = self.db {
                        if let Ok(db) = db.try_lock() {
                            let _ = db.mark_alert_triggered(&alert.coin_id, alert.target_price);
                        }
                    }
                }
            }
        }
    }

    pub async fn refresh_alerts(&mut self) {
        if let Some(ref db) = self.db {
            let db = db.lock().await;
            self.alerts = db.get_alerts().unwrap_or_default();
        }
    }

    pub async fn refresh_global_stats(&mut self, client: &CoinGeckoClient) {
        match client.fetch_global().await {
            Ok(mut stats) => {
                // Try to get fear & greed index
                if let Ok((index, label)) = client.fetch_fear_greed().await {
                    stats.fear_greed_index = Some(index);
                    stats.fear_greed_label = Some(label);
                }
                self.global_stats = Some(stats);
            }
            Err(_) => {
                // Silently ignore - global stats are non-critical
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
        self.settings_notification_idx = NOTIFICATION_METHODS
            .iter()
            .position(|m| *m == notification_method_label(self.notification_method))
            .unwrap_or(0);
        self.settings_ntfy_topic = self.ntfy_topic.clone();
        self.settings_field = SettingsField::Currency;
        self.settings_editing = false;
        self.input_mode = InputMode::Settings;
    }

    pub fn cycle_notification(&mut self, forward: bool) {
        let len = NOTIFICATION_METHODS.len();
        if forward {
            self.settings_notification_idx = (self.settings_notification_idx + 1) % len;
        } else {
            self.settings_notification_idx = (self.settings_notification_idx + len - 1) % len;
        }
    }

    pub fn buy_price_for(&self, coin_id: &str) -> Option<f64> {
        self.holdings
            .iter()
            .find(|h| h.coin_id == coin_id)
            .and_then(|h| h.buy_price)
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
