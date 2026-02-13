mod api;
mod app;
mod config;
mod db;
mod theme;
mod types;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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

    run_main_loop(terminal, app, client).await
}

async fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mut client: CoinGeckoClient,
) -> Result<()> {
    let tick_rate = Duration::from_millis(250);

    loop {
        let refresh_dur = Duration::from_secs(app.config.refresh_interval_secs);
        app.update_refresh_display();

        terminal.draw(|f| ui::draw(f, &mut *app))?;

        // Auto-refresh
        if let Some(last) = app.last_refresh {
            if last.elapsed() >= refresh_dur {
                app.refresh_market_data(&client).await;
                app.refresh_db_state().await;
                app.clamp_selection();
            }
        }

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.quit = true;
                }

                match app.input_mode {
                    InputMode::EditingAmount => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.input_buf.clear();
                        }
                        KeyCode::Enter => {
                            if let Ok(amount) = app.input_buf.trim().parse::<f64>() {
                                if let Some(coin) = app.selected_coin() {
                                    let coin_id = coin.id.clone();
                                    if let Some(ref db) = app.db {
                                        let db = db.lock().await;
                                        let _ = db.set_holding(&coin_id, amount);
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
                        KeyCode::Esc => app.quit = true,
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
                                    let _ = db.set_holding(&coin_id, 0.0);
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
                if app.settings_field == SettingsField::Currency {
                    app.cycle_currency(false);
                } else if app.settings_field == SettingsField::Theme {
                    app.cycle_theme(false);
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if app.settings_field == SettingsField::Currency {
                    app.cycle_currency(true);
                } else if app.settings_field == SettingsField::Theme {
                    app.cycle_theme(true);
                }
            }
            KeyCode::Char('s') => {
                // Save settings
                let new_currency = CURRENCIES[app.settings_currency_idx].to_string();
                let new_theme_name = theme::THEME_NAMES[app.settings_theme_idx].to_string();
                let currency_changed = new_currency != app.config.currency;

                if let Some(ref db) = app.db {
                    let db = db.lock().await;
                    let _ = db.set_setting("coingecko_api_key", &app.settings_coingecko_key);
                    let _ = db.set_setting("cmc_api_key", &app.settings_cmc_key);
                    let _ = db.set_setting("currency", &new_currency);
                }
                app.coingecko_api_key = app.settings_coingecko_key.clone();
                app.cmc_api_key = app.settings_cmc_key.clone();
                app.config.currency = new_currency;
                app.config.theme = new_theme_name.clone();
                app.theme = theme::by_name(&new_theme_name);
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
