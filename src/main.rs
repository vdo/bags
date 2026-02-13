mod api;
mod app;
mod config;
mod db;
mod notifications;
mod theme;
mod types;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use api::CoinGeckoClient;
use app::App;
use config::Config;
use db::Db;
use types::*;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    let db_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("bags")
        .join("bags.db");

    let is_new = !db_path.exists();

    let mut app = App::new(config.clone(), is_new);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(ref e) = result {
        let msg = format!("Fatal: {}", e);
        app::log_error(&msg);
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(250);

    // Password entry loop
    loop {
        terminal.draw(|f| ui::draw(f, &mut *app))?;

        if app.quit {
            return Ok(());
        }

        if app.unlocked {
            break;
        }

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(());
                }

                match app.input_mode {
                    InputMode::Password => match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Enter => {
                            if app.password_buf.is_empty() {
                                app.password_error = Some("Password cannot be empty".into());
                            } else if app.is_new_db {
                                app.password_first = app.password_buf.clone();
                                app.password_buf.clear();
                                app.password_error = None;
                                app.input_mode = InputMode::PasswordConfirm;
                            } else {
                                match Db::open(&app.password_buf) {
                                    Ok(db) => {
                                        app.load_api_keys_from_db(&db);
                                        app.unlock(db);
                                    }
                                    Err(_) => {
                                        app.password_error = Some("Wrong password".into());
                                        app.password_buf.clear();
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.password_buf.pop();
                        }
                        KeyCode::Char(c) => {
                            app.password_buf.push(c);
                        }
                        _ => {}
                    },
                    InputMode::PasswordConfirm => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Password;
                            app.password_buf.clear();
                            app.password_first.clear();
                            app.password_error = None;
                        }
                        KeyCode::Enter => {
                            if app.password_buf == app.password_first {
                                match Db::open(&app.password_buf) {
                                    Ok(db) => {
                                        app.unlock(db);
                                    }
                                    Err(e) => {
                                        let msg = format!("DB error: {}", e);
                                        app::log_error(&msg);
                                        app.password_error = Some(msg);
                                        app.password_buf.clear();
                                        app.password_first.clear();
                                        app.input_mode = InputMode::Password;
                                    }
                                }
                            } else {
                                app.password_error = Some("Passwords do not match".into());
                                app.password_buf.clear();
                                app.password_first.clear();
                                app.input_mode = InputMode::Password;
                            }
                        }
                        KeyCode::Backspace => {
                            app.password_buf.pop();
                        }
                        KeyCode::Char(c) => {
                            app.password_buf.push(c);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    // Now unlocked -- create client and fetch data
    let client = CoinGeckoClient::new(&app.config.currency, &app.coingecko_api_key);
    app.refresh_db_state().await;
    app.refresh_market_data(&client).await;
    app.refresh_alerts().await;
    app.check_alerts();

    // Fetch global stats in background (non-blocking)
    app.refresh_global_stats(&client).await;

    run_main_loop(terminal, app, client).await
}

async fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mut client: CoinGeckoClient,
) -> Result<()> {
    let tick_rate = Duration::from_millis(250);

    // Track layout info for mouse clicks
    let mut top_bar_height: u16 = 3;
    let mut bottom_bar_y: u16 = 0;

    loop {
        let refresh_dur = Duration::from_secs(app.config.refresh_interval_secs);
        app.update_refresh_display();

        terminal.draw(|f| {
            let area = f.area();
            top_bar_height = 3;
            bottom_bar_y = area.height.saturating_sub(1);
            ui::draw(f, &mut *app);
        })?;

        // Auto-refresh
        if let Some(last) = app.last_refresh {
            if last.elapsed() >= refresh_dur {
                app.refresh_market_data(&client).await;
                app.refresh_db_state().await;
                app.clamp_selection();
                app.check_alerts();
                app.refresh_global_stats(&client).await;
            }
        }

        if event::poll(tick_rate)? {
            let ev = event::read()?;

            // Handle mouse events
            if let Event::Mouse(mouse) = ev {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if app.input_mode == InputMode::Normal && !app.popup_open && !app.sort_picking {
                            let row = mouse.row;
                            let col = mouse.column;

                            // Click on top bar tabs
                            if row < top_bar_height {
                                // Rough tab click detection based on position
                                let tabs_start = 10_u16; // after "[bags] | "
                                if col >= tabs_start {
                                    let rel = col - tabs_start;
                                    if rel < 7 { // "Markets"
                                        app.tab = Tab::Markets;
                                        app.selected = 0;
                                        app.clamp_selection();
                                    } else if rel < 22 { // " . Favourites"
                                        app.tab = Tab::Favourites;
                                        app.selected = 0;
                                        app.clamp_selection();
                                    } else if rel < 36 { // " . Portfolio"
                                        app.tab = Tab::Portfolio;
                                        app.selected = 0;
                                        app.clamp_selection();
                                    }
                                }
                            }
                            // Click on table rows (below header, above bottom bar)
                            else if row >= top_bar_height + 1 && row < bottom_bar_y {
                                let table_row = (row - top_bar_height - 1) as usize;
                                let target = app.scroll_offset + table_row;
                                let visible_len = app.visible_coins().len();
                                if target < visible_len {
                                    app.selected = target;
                                    app.adjust_scroll();
                                }
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if app.input_mode == InputMode::Normal && !app.popup_open {
                            let len = app.visible_coins().len();
                            if len > 0 {
                                app.selected = (app.selected + 3).min(len - 1);
                            }
                            app.adjust_scroll();
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if app.input_mode == InputMode::Normal && !app.popup_open {
                            app.selected = app.selected.saturating_sub(3);
                            app.adjust_scroll();
                        }
                    }
                    _ => {}
                }
                continue;
            }

            if let Event::Key(key) = ev {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.quit = true;
                }

                match app.input_mode {
                    InputMode::Filtering => match key.code {
                        KeyCode::Esc => {
                            app.filter_query.clear();
                            app.input_mode = InputMode::Normal;
                            app.clamp_selection();
                        }
                        KeyCode::Enter => {
                            app.input_mode = InputMode::Normal;
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Backspace => {
                            app.filter_query.pop();
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Char(c) => {
                            app.filter_query.push(c);
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        _ => {}
                    },
                    InputMode::EditingAlert => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.alert_input_buf.clear();
                        }
                        KeyCode::Tab => {
                            app.alert_direction = match app.alert_direction {
                                AlertDirection::Above => AlertDirection::Below,
                                AlertDirection::Below => AlertDirection::Above,
                            };
                        }
                        KeyCode::Enter => {
                            if let Ok(price) = app.alert_input_buf.trim().parse::<f64>() {
                                if let Some(coin) = app.selected_coin() {
                                    let coin_id = coin.id.clone();
                                    let dir_str = match app.alert_direction {
                                        AlertDirection::Above => "above",
                                        AlertDirection::Below => "below",
                                    };
                                    if let Some(ref db) = app.db {
                                        let db = db.lock().await;
                                        let _ = db.add_alert(&coin_id, price, dir_str);
                                    }
                                    app.refresh_alerts().await;
                                }
                            }
                            app.input_mode = InputMode::Normal;
                            app.alert_input_buf.clear();
                        }
                        KeyCode::Backspace => {
                            app.alert_input_buf.pop();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                            app.alert_input_buf.push(c);
                        }
                        _ => {}
                    },
                    InputMode::EditingBuyPrice => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.buy_price_buf.clear();
                        }
                        KeyCode::Enter => {
                            if let Ok(price) = app.buy_price_buf.trim().parse::<f64>() {
                                if let Some(coin) = app.selected_coin() {
                                    let coin_id = coin.id.clone();
                                    if let Some(ref db) = app.db {
                                        let db = db.lock().await;
                                        let _ = db.set_buy_price(&coin_id, price);
                                    }
                                    app.refresh_db_state().await;
                                }
                            }
                            app.input_mode = InputMode::Normal;
                            app.buy_price_buf.clear();
                        }
                        KeyCode::Backspace => {
                            app.buy_price_buf.pop();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                            app.buy_price_buf.push(c);
                        }
                        _ => {}
                    },
                    InputMode::EditingAmount => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input_buf.clear();
                        }
                        KeyCode::Enter => {
                            if let Ok(amount) = app.input_buf.trim().parse::<f64>() {
                                if let Some(coin) = app.selected_coin() {
                                    let coin_id = coin.id.clone();
                                    let current_price = coin.current_price;
                                    // Auto-record buy price if this is a new holding
                                    let existing = app.holding_for(&coin_id);
                                    let buy_price = if existing <= 0.0 && amount > 0.0 {
                                        Some(current_price)
                                    } else {
                                        None
                                    };
                                    if let Some(ref db) = app.db {
                                        let db = db.lock().await;
                                        let _ = db.set_holding(&coin_id, amount, buy_price);
                                    }
                                    app.refresh_db_state().await;
                                }
                            }
                            app.input_mode = InputMode::Normal;
                            app.input_buf.clear();
                        }
                        KeyCode::Backspace => {
                            app.input_buf.pop();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                            app.input_buf.push(c);
                        }
                        _ => {}
                    },
                    InputMode::Settings => {
                        handle_settings_key(app, key.code, &mut client).await;
                    }
                    InputMode::SearchCoin => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.search_query.clear();
                            app.search_error = None;
                        }
                        KeyCode::Enter => {
                            if !app.search_query.is_empty() {
                                app.search_loading = true;
                                app.search_error = None;
                                let query = app.search_query.clone();
                                match client.search_coins(&query).await {
                                    Ok(results) => {
                                        if results.is_empty() {
                                            app.search_error = Some("No results found".into());
                                        } else {
                                            app.search_results = results;
                                            app.search_selected = 0;
                                            app.input_mode = InputMode::SearchResults;
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("Search: {}", e);
                                        app::log_error(&msg);
                                        app.search_error = Some(msg);
                                    }
                                }
                                app.search_loading = false;
                            }
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                        }
                        _ => {}
                    },
                    InputMode::SearchResults => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::SearchCoin;
                            app.search_results.clear();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            let len = app.search_results.len();
                            if len > 0 {
                                app.search_selected = (app.search_selected + 1).min(len - 1);
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.search_selected = app.search_selected.saturating_sub(1);
                        }
                        KeyCode::Enter => {
                            if let Some(result) = app.search_results.get(app.search_selected) {
                                let coin_id = result.id.clone();
                                // Add to favourites
                                if let Some(ref db) = app.db {
                                    let db = db.lock().await;
                                    if !db.is_favourite(&coin_id) {
                                        let _ = db.toggle_favourite(&coin_id);
                                    }
                                }
                                // Fetch coin data if not already in list
                                if !app.coins.iter().any(|c| c.id == coin_id) {
                                    if let Ok(Some(coin)) = client.fetch_coin_market(&coin_id).await {
                                        app.coins.push(coin);
                                    }
                                }
                                app.refresh_db_state().await;
                                app.input_mode = InputMode::Normal;
                                app.search_query.clear();
                                app.search_results.clear();
                                // Switch to favourites tab
                                app.tab = Tab::Favourites;
                                app.selected = 0;
                                app.clamp_selection();
                            }
                        }
                        _ => {}
                    },
                    InputMode::Normal if app.sort_picking => {
                        app.sort_picking = false;
                        match key.code {
                            KeyCode::Char('r') | KeyCode::Char('#') => {
                                toggle_sort(app, SortColumn::Rank);
                            }
                            KeyCode::Char('n') => {
                                toggle_sort(app, SortColumn::Name);
                            }
                            KeyCode::Char('p') => {
                                toggle_sort(app, SortColumn::Price);
                            }
                            KeyCode::Char('1') => {
                                toggle_sort(app, SortColumn::Change1h);
                            }
                            KeyCode::Char('2') => {
                                toggle_sort(app, SortColumn::Change24h);
                            }
                            KeyCode::Char('7') => {
                                toggle_sort(app, SortColumn::Change7d);
                            }
                            KeyCode::Char('v') => {
                                toggle_sort(app, SortColumn::Volume);
                            }
                            KeyCode::Char('m') => {
                                toggle_sort(app, SortColumn::MarketCap);
                            }
                            KeyCode::Esc => {
                                app.sort_column = None;
                            }
                            _ => {}
                        }
                        app.selected = 0;
                        app.clamp_selection();
                    }
                    InputMode::Normal if app.popup_open => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.popup_open = false;
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            app.chart_view = app.chart_view.next();
                            fetch_chart_if_needed(app, &client).await;
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            app.chart_view = app.chart_view.prev();
                            fetch_chart_if_needed(app, &client).await;
                        }
                        _ => {}
                    },
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => app.quit = true,
                        KeyCode::Esc => {
                            if !app.filter_query.is_empty() {
                                app.filter_query.clear();
                                app.selected = 0;
                                app.clamp_selection();
                            } else {
                                app.quit = true;
                            }
                        }
                        KeyCode::Char('/') => {
                            app.filter_query.clear();
                            app.input_mode = InputMode::Filtering;
                        }
                        KeyCode::Char('s') => {
                            app.sort_picking = true;
                        }
                        KeyCode::Char('A') => {
                            if app.selected_coin().is_some() {
                                app.alert_input_buf.clear();
                                app.alert_direction = AlertDirection::Above;
                                app.input_mode = InputMode::EditingAlert;
                            }
                        }
                        KeyCode::Char('b') => {
                            if let Some(coin) = app.selected_coin() {
                                let coin_id = coin.id.clone();
                                if app.holding_for(&coin_id) > 0.0 {
                                    app.buy_price_buf = app.buy_price_for(&coin_id)
                                        .map(|p| format!("{}", p))
                                        .unwrap_or_default();
                                    app.input_mode = InputMode::EditingBuyPrice;
                                }
                            }
                        }
                        KeyCode::Tab => {
                            app.tab = app.tab.next();
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Char('1') => {
                            app.tab = Tab::Markets;
                            app.selected = 0;
                        }
                        KeyCode::Char('2') => {
                            app.tab = Tab::Favourites;
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Char('3') => {
                            app.tab = Tab::Portfolio;
                            app.selected = 0;
                            app.clamp_selection();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            let len = app.visible_coins().len();
                            if len > 0 {
                                app.selected = (app.selected + 1).min(len - 1);
                            }
                            app.adjust_scroll();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.selected = app.selected.saturating_sub(1);
                            app.adjust_scroll();
                        }
                        KeyCode::PageDown | KeyCode::Char('d')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                || key.code == KeyCode::PageDown =>
                        {
                            let len = app.visible_coins().len();
                            if len > 0 {
                                app.selected =
                                    (app.selected + app.page_height).min(len - 1);
                            }
                            app.adjust_scroll();
                        }
                        KeyCode::PageUp | KeyCode::Char('u')
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                || key.code == KeyCode::PageUp =>
                        {
                            app.selected =
                                app.selected.saturating_sub(app.page_height);
                            app.adjust_scroll();
                        }
                        KeyCode::Char('g') => {
                            app.selected = 0;
                            app.adjust_scroll();
                        }
                        KeyCode::Char('G') => {
                            let len = app.visible_coins().len();
                            if len > 0 {
                                app.selected = len - 1;
                            }
                            app.adjust_scroll();
                        }
                        KeyCode::Enter => {
                            if app.selected_coin().is_some() {
                                app.popup_open = true;
                                app.chart_view = ChartView::Day1;
                                fetch_chart_if_needed(app, &client).await;
                            }
                        }
                        KeyCode::Char('f') => {
                            if let Some(coin) = app.selected_coin() {
                                let coin_id = coin.id.clone();
                                if let Some(ref db) = app.db {
                                    let db = db.lock().await;
                                    let _ = db.toggle_favourite(&coin_id);
                                }
                                app.refresh_db_state().await;
                                app.clamp_selection();
                            }
                        }
                        KeyCode::Char('a') => {
                            if app.selected_coin().is_some() {
                                let coin = app.selected_coin().unwrap();
                                let current = app.holding_for(&coin.id);
                                app.input_buf = if current > 0.0 {
                                    format!("{}", current)
                                } else {
                                    String::new()
                                };
                                app.input_mode = InputMode::EditingAmount;
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(coin) = app.selected_coin() {
                                let coin_id = coin.id.clone();
                                if let Some(ref db) = app.db {
                                    let db = db.lock().await;
                                    let _ = db.set_holding(&coin_id, 0.0, None);
                                }
                                app.refresh_db_state().await;
                                app.clamp_selection();
                            }
                        }
                        KeyCode::Char('r') => {
                            app.loading = true;
                            app.refresh_market_data(&client).await;
                            app.refresh_db_state().await;
                            app.clamp_selection();
                            app.check_alerts();
                            app.refresh_global_stats(&client).await;
                        }
                        KeyCode::Char('S') => {
                            app.open_settings();
                        }
                        KeyCode::Char('c') => {
                            app.search_query.clear();
                            app.search_results.clear();
                            app.search_error = None;
                            app.search_selected = 0;
                            app.input_mode = InputMode::SearchCoin;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}

fn toggle_sort(app: &mut App, col: SortColumn) {
    if app.sort_column == Some(col) {
        match app.sort_direction {
            SortDirection::Asc => app.sort_direction = SortDirection::Desc,
            SortDirection::Desc => {
                app.sort_column = None;
                app.sort_direction = SortDirection::Asc;
            }
        }
    } else {
        app.sort_column = Some(col);
        app.sort_direction = SortDirection::Asc;
    }
}

async fn handle_settings_key(app: &mut App, key: KeyCode, client: &mut CoinGeckoClient) {
    if app.settings_editing {
        match key {
            KeyCode::Esc => {
                app.settings_editing = false;
            }
            KeyCode::Enter => {
                app.settings_editing = false;
            }
            KeyCode::Backspace => {
                if app.settings_field.is_text_field() {
                    app.current_settings_value_mut().pop();
                }
            }
            KeyCode::Char(c) => {
                if app.settings_field.is_text_field() {
                    app.current_settings_value_mut().push(c);
                }
            }
            _ => {}
        }
    } else {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Revert theme to saved value
                app.theme = theme::by_name(&app.config.theme);
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
                app.settings_field = app.settings_field.next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.settings_field = app.settings_field.prev();
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if app.settings_field.is_text_field() {
                    app.settings_editing = true;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                match app.settings_field {
                    SettingsField::Currency => app.cycle_currency(false),
                    SettingsField::Theme => app.cycle_theme(false),
                    SettingsField::Notifications => app.cycle_notification(false),
                    _ => {}
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                match app.settings_field {
                    SettingsField::Currency => app.cycle_currency(true),
                    SettingsField::Theme => app.cycle_theme(true),
                    SettingsField::Notifications => app.cycle_notification(true),
                    _ => {}
                }
            }
            KeyCode::Char('s') => {
                // Save settings
                let new_currency = CURRENCIES[app.settings_currency_idx].to_string();
                let new_theme_name = theme::THEME_NAMES[app.settings_theme_idx].to_string();
                let currency_changed = new_currency != app.config.currency;
                let new_notif = NOTIFICATION_METHODS[app.settings_notification_idx];

                if let Some(ref db) = app.db {
                    let db = db.lock().await;
                    let _ = db.set_setting("coingecko_api_key", &app.settings_coingecko_key);
                    let _ = db.set_setting("cmc_api_key", &app.settings_cmc_key);
                    let _ = db.set_setting("currency", &new_currency);
                    let _ = db.set_setting("notification_method", new_notif);
                    let _ = db.set_setting("ntfy_topic", &app.settings_ntfy_topic);
                }
                app.coingecko_api_key = app.settings_coingecko_key.clone();
                app.cmc_api_key = app.settings_cmc_key.clone();
                app.config.currency = new_currency;
                app.config.theme = new_theme_name.clone();
                app.theme = theme::by_name(&new_theme_name);
                app.notification_method = notification_method_from_str(new_notif);
                app.ntfy_topic = app.settings_ntfy_topic.clone();
                let _ = app.config.save();

                // Recreate client with new key/currency
                *client = CoinGeckoClient::new(&app.config.currency, &app.coingecko_api_key);
                app.chart_cache.clear();

                if currency_changed {
                    app.loading = true;
                    app.refresh_market_data(client).await;
                }

                app.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }
}

async fn fetch_chart_if_needed(app: &mut App, client: &CoinGeckoClient) {
    let (coin_id, days) = match app.selected_coin() {
        Some(c) => (c.id.clone(), app.chart_view.days()),
        None => return,
    };
    let key = (coin_id.clone(), days);
    if app.chart_cache.contains_key(&key) {
        return;
    }

    app.loading_chart = true;

    match client.fetch_price_history(&coin_id, days).await {
        Ok(history) => {
            app.chart_cache.insert(key, history);
        }
        Err(e) => {
            app.set_error(format!("Chart: {}", e));
        }
    }
    app.loading_chart = false;
}
