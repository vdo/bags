use serde::{Deserialize, Deserializer, Serialize};

fn f64_or_zero<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    Option::<f64>::deserialize(d).map(|v| v.unwrap_or(0.0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default, deserialize_with = "f64_or_zero")]
    pub current_price: f64,
    #[serde(default, deserialize_with = "f64_or_zero")]
    pub market_cap: f64,
    #[serde(default, deserialize_with = "f64_or_zero")]
    pub total_volume: f64,
    pub price_change_percentage_1h_in_currency: Option<f64>,
    pub price_change_percentage_24h_in_currency: Option<f64>,
    pub price_change_percentage_7d_in_currency: Option<f64>,
    pub market_cap_rank: Option<u32>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub circulating_supply: Option<f64>,
    pub max_supply: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Holding {
    pub coin_id: String,
    pub amount: f64,
    pub buy_price: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GlobalMarketStats {
    pub total_market_cap_usd: f64,
    pub btc_dominance: f64,
    pub fear_greed_index: Option<u32>,
    pub fear_greed_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PriceAlert {
    pub coin_id: String,
    pub target_price: f64,
    pub direction: AlertDirection,
    pub triggered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertDirection {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Rank,
    Name,
    Price,
    Change1h,
    Change24h,
    Change7d,
    Volume,
    MarketCap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationMethod {
    None,
    Desktop,
    Ntfy,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Markets,
    Favourites,
    Portfolio,
}

impl Tab {
    pub fn index(self) -> usize {
        match self {
            Tab::Markets => 0,
            Tab::Favourites => 1,
            Tab::Portfolio => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Tab::Markets,
            1 => Tab::Favourites,
            2 => Tab::Portfolio,
            _ => Tab::Markets,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tab::Markets => "Markets",
            Tab::Favourites => "Favourites",
            Tab::Portfolio => "Portfolio",
        }
    }

    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % 3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartView {
    Day1,
    Day7,
    Day30,
}

impl ChartView {
    pub fn label(self) -> &'static str {
        match self {
            ChartView::Day1 => "1D",
            ChartView::Day7 => "7D",
            ChartView::Day30 => "30D",
        }
    }

    pub fn days(self) -> u32 {
        match self {
            ChartView::Day1 => 1,
            ChartView::Day7 => 7,
            ChartView::Day30 => 30,
        }
    }

    pub fn next(self) -> Self {
        match self {
            ChartView::Day1 => ChartView::Day7,
            ChartView::Day7 => ChartView::Day30,
            ChartView::Day30 => ChartView::Day1,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ChartView::Day1 => ChartView::Day30,
            ChartView::Day7 => ChartView::Day1,
            ChartView::Day30 => ChartView::Day7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub prices: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingAmount,
    Password,
    PasswordConfirm,
    Settings,
    SearchCoin,
    SearchResults,
    Filtering,
    EditingAlert,
    EditingBuyPrice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Currency,
    Theme,
    CoingeckoApiKey,
    CoinmarketcapApiKey,
    Notifications,
    NtfyTopic,
}

impl SettingsField {
    pub fn label(self) -> &'static str {
        match self {
            SettingsField::Currency => "Currency",
            SettingsField::Theme => "Theme",
            SettingsField::CoingeckoApiKey => "CoinGecko API Key",
            SettingsField::CoinmarketcapApiKey => "CoinMarketCap API Key",
            SettingsField::Notifications => "Notifications",
            SettingsField::NtfyTopic => "Ntfy Topic",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SettingsField::Currency => SettingsField::Theme,
            SettingsField::Theme => SettingsField::CoingeckoApiKey,
            SettingsField::CoingeckoApiKey => SettingsField::CoinmarketcapApiKey,
            SettingsField::CoinmarketcapApiKey => SettingsField::Notifications,
            SettingsField::Notifications => SettingsField::NtfyTopic,
            SettingsField::NtfyTopic => SettingsField::Currency,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsField::Currency => SettingsField::NtfyTopic,
            SettingsField::Theme => SettingsField::Currency,
            SettingsField::CoingeckoApiKey => SettingsField::Theme,
            SettingsField::CoinmarketcapApiKey => SettingsField::CoingeckoApiKey,
            SettingsField::Notifications => SettingsField::CoinmarketcapApiKey,
            SettingsField::NtfyTopic => SettingsField::Notifications,
        }
    }

    pub fn is_text_field(self) -> bool {
        matches!(self, SettingsField::CoingeckoApiKey | SettingsField::CoinmarketcapApiKey | SettingsField::NtfyTopic)
    }

    pub fn is_cycle_field(self) -> bool {
        matches!(self, SettingsField::Currency | SettingsField::Theme | SettingsField::Notifications)
    }
}

pub const CURRENCIES: &[&str] = &[
    "usd", "eur", "gbp", "jpy", "aud", "cad", "chf", "cny", "krw", "inr", "brl", "btc", "eth",
];

pub const NOTIFICATION_METHODS: &[&str] = &["none", "desktop", "ntfy", "both"];

pub fn notification_method_from_str(s: &str) -> NotificationMethod {
    match s {
        "desktop" => NotificationMethod::Desktop,
        "ntfy" => NotificationMethod::Ntfy,
        "both" => NotificationMethod::Both,
        _ => NotificationMethod::None,
    }
}

pub fn notification_method_label(m: NotificationMethod) -> &'static str {
    match m {
        NotificationMethod::None => "none",
        NotificationMethod::Desktop => "desktop",
        NotificationMethod::Ntfy => "ntfy",
        NotificationMethod::Both => "both",
    }
}

pub fn currency_symbol(code: &str) -> &'static str {
    match code {
        "usd" => "$",
        "eur" => "\u{20ac}",
        "gbp" => "\u{a3}",
        "jpy" | "cny" | "krw" => "\u{a5}",
        "aud" | "cad" => "$",
        "chf" => "Fr",
        "inr" => "\u{20b9}",
        "brl" => "R$",
        "btc" => "\u{20bf}",
        "eth" => "\u{39e}",
        _ => "$",
    }
}
