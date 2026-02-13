use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table,
    },
    Frame,
};

use crate::app::App;
use crate::theme::THEME_NAMES;
use crate::types::*;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Fill background
    let bg_block = Block::default().style(Style::default().bg(app.theme.bg));
    f.render_widget(bg_block, f.area());

    match app.input_mode {
        InputMode::Password | InputMode::PasswordConfirm => {
            draw_lock_screen(f, app);
            return;
        }
        _ => {}
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // top bar
            Constraint::Min(5),   // main
            Constraint::Length(1), // bottom bar
        ])
        .split(f.area());

    draw_top_bar(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_bottom_bar(f, app, chunks[2]);

    if app.popup_open {
        draw_popup(f, app);
    }

    if app.input_mode == InputMode::EditingAmount {
        draw_input_popup(f, app);
    }

    if app.input_mode == InputMode::Settings {
        draw_settings(f, app);
    }

    if app.input_mode == InputMode::SearchCoin || app.input_mode == InputMode::SearchResults {
        draw_search(f, app);
    }
}

// -- Lock screen --

fn draw_lock_screen(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();

    // Center a box
    let box_w = 44_u16.min(area.width.saturating_sub(4));
    let has_error = app.password_error.is_some();
    let box_h = if has_error { 6 } else { 5_u16 };
    let x = (area.width.saturating_sub(box_w)) / 2;
    let y = (area.height.saturating_sub(box_h)) / 2;
    let popup = Rect::new(x, y, box_w, box_h);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" [bags] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut constraints = vec![
        Constraint::Length(1), // prompt label
        Constraint::Length(1), // input field
    ];
    if has_error {
        constraints.push(Constraint::Length(1)); // error
    }
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let prompt = if app.input_mode == InputMode::PasswordConfirm {
        "  Confirm password:"
    } else if app.is_new_db {
        "  Set a password:"
    } else {
        "  Enter password:"
    };

    let prompt_p = Paragraph::new(prompt)
        .style(Style::default().fg(t.dim));
    f.render_widget(prompt_p, chunks[0]);

    let dots = "\u{2022}".repeat(app.password_buf.len());
    let input_line = format!("  {}_", dots);
    let input_p = Paragraph::new(input_line)
        .style(Style::default().fg(t.fg));
    f.render_widget(input_p, chunks[1]);

    if let Some(ref err) = app.password_error {
        let err_p = Paragraph::new(format!("  {}", err))
            .style(Style::default().fg(t.error));
        f.render_widget(err_p, chunks[2]);
    }
}

// -- Top bar --

fn draw_top_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let refresh_info = if app.loading {
        "loading...".to_string()
    } else if app.last_refresh_display.is_empty() {
        String::new()
    } else {
        app.last_refresh_display.clone()
    };

    let tabs_list = [Tab::Markets, Tab::Favourites, Tab::Portfolio];
    let mut spans: Vec<Span> = Vec::new();

    spans.push(Span::styled(
        " [bags] ",
        Style::default().fg(t.title).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled("\u{2502} ", Style::default().fg(t.dim)));

    for (i, tab) in tabs_list.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" \u{b7} ", Style::default().fg(t.dim)));
        }
        if tab.index() == app.tab.index() {
            spans.push(Span::styled(
                tab.label(),
                Style::default().fg(t.title).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                tab.label(),
                Style::default().fg(t.dim),
            ));
        }
    }

    if !refresh_info.is_empty() {
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        let pad = (area.width as usize).saturating_sub(used + refresh_info.len() + 1);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        spans.push(Span::styled(
            refresh_info,
            Style::default().fg(t.dim),
        ));
    }

    let bar = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(t.border)));
    f.render_widget(bar, area);
}

// -- Main table --

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    let t = &app.theme;

    if app.loading && app.coins.is_empty() {
        let loading = Paragraph::new("  Fetching market data...")
            .style(Style::default().fg(t.dim));
        f.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = app.error {
        if app.coins.is_empty() {
            let msg = Paragraph::new(format!("  Error: {}", err))
                .style(Style::default().fg(t.error));
            f.render_widget(msg, area);
            return;
        }
    }

    let table_height = area.height.saturating_sub(2) as usize;
    app.page_height = table_height.max(1);

    let visible = app.visible_coins();

    if visible.is_empty() {
        let msg = match app.tab {
            Tab::Favourites => "  No favourites yet. Press 'f' to favourite a coin.",
            Tab::Portfolio => "  No holdings. Press 'a' to add a holding.",
            _ => "  No data.",
        };
        let p = Paragraph::new(msg).style(Style::default().fg(t.dim));
        f.render_widget(p, area);
        return;
    }

    let is_portfolio = app.tab == Tab::Portfolio;

    let header_cells = {
        let mut h = vec!["#", "Name", "Ticker", "Price", "1h%", "24h%", "7d%", "Volume", "MCap"];
        if is_portfolio {
            h.push("Qty");
            h.push("Value");
        }
        h
    };

    let header = Row::new(
        header_cells
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(t.dim))),
    )
    .height(1);

    let bg = t.bg;
    let fg = t.fg;
    let positive = t.positive;
    let negative = t.negative;
    let dim = t.dim;
    let accent = t.accent;
    let highlight_bg = t.highlight_bg;
    let highlight_fg = t.highlight_fg;

    let rows: Vec<Row> = visible
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(app.page_height)
        .map(|(i, (_, coin))| {
            let rank = coin.market_cap_rank.map(|r| r.to_string()).unwrap_or_default();
            let name = coin.name.clone();
            let symbol = coin.symbol.to_uppercase();
            let price = format_price(coin.current_price);
            let h1 = format_pct(coin.price_change_percentage_1h_in_currency);
            let h24 = format_pct(coin.price_change_percentage_24h_in_currency);
            let d7 = format_pct(coin.price_change_percentage_7d_in_currency);
            let vol = format_large(coin.total_volume);
            let mcap = format_large(coin.market_cap);

            let mut cells = vec![
                Cell::from(rank).style(Style::default().fg(dim)),
                Cell::from(name).style(Style::default().fg(fg)),
                Cell::from(symbol).style(Style::default().fg(accent)),
                Cell::from(price).style(Style::default().fg(fg)),
                pct_cell(coin.price_change_percentage_1h_in_currency, &h1, positive, negative, dim),
                pct_cell(coin.price_change_percentage_24h_in_currency, &h24, positive, negative, dim),
                pct_cell(coin.price_change_percentage_7d_in_currency, &d7, positive, negative, dim),
                Cell::from(vol).style(Style::default().fg(dim)),
                Cell::from(mcap).style(Style::default().fg(dim)),
            ];

            if is_portfolio {
                let amt = app.holding_for(&coin.id);
                let val = amt * coin.current_price;
                cells.push(Cell::from(format_amount(amt)).style(Style::default().fg(fg)));
                cells.push(Cell::from(format_price(val)).style(Style::default().fg(accent)));
            }

            let style = if i == app.selected {
                Style::default().bg(highlight_bg).fg(highlight_fg)
            } else {
                Style::default().bg(bg)
            };

            Row::new(cells).style(style)
        })
        .collect();

    let widths = if is_portfolio {
        vec![
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
        ]
    } else {
        vec![
            Constraint::Length(4),
            Constraint::Min(14),
            Constraint::Length(6),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ]
    };

    let mut block = Block::default().borders(Borders::NONE);

    if is_portfolio {
        let total = app.total_portfolio_value();
        block = block.title(Line::from(vec![
            Span::styled(" Total: ", Style::default().fg(t.dim)),
            Span::styled(
                format!("${} ", format_price(total)),
                Style::default().fg(t.title).add_modifier(Modifier::BOLD),
            ),
        ])).title_alignment(ratatui::layout::Alignment::Right);
    }

    let table = Table::new(rows, &widths)
        .header(header)
        .block(block)
        .column_spacing(1);

    f.render_widget(table, area);
}

// -- Bottom bar --

fn draw_bottom_bar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let hints = if app.popup_open {
        " Esc close | h/l cycle view "
    } else if app.input_mode == InputMode::Settings {
        " j/k navigate | Enter edit | s save | Esc cancel "
    } else {
        match app.tab {
            Tab::Markets => " j/k scroll | Tab switch | Enter details | f fav | a hold | c add coin | S settings | q quit ",
            Tab::Favourites => " j/k scroll | Tab switch | Enter details | f unfav | a hold | c add coin | S settings | q quit ",
            Tab::Portfolio => " j/k scroll | Tab switch | Enter details | a edit | d remove | c add coin | S settings | q quit ",
        }
    };

    let mut spans = vec![Span::styled(hints, Style::default().fg(t.dim))];

    if let Some(ref err) = app.error {
        spans.push(Span::styled(
            format!(" | {}", err),
            Style::default().fg(t.error),
        ));
    }

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

// -- Chart popup --

fn draw_popup(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let coin = match app.selected_coin() {
        Some(c) => c,
        None => return,
    };

    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let title = format!(" {} ({}) - {} ", coin.name, coin.symbol.to_uppercase(), app.chart_view.label());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cache_key = (coin.id.clone(), app.chart_view.days());
    if let Some(history) = app.chart_cache.get(&cache_key) {
        if history.prices.is_empty() {
            let msg = Paragraph::new("  No price data available.")
                .style(Style::default().fg(t.dim));
            f.render_widget(msg, inner);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(inner);

        let first = history.prices.first().copied().unwrap_or(0.0);
        let last = history.prices.last().copied().unwrap_or(0.0);
        let min = history.prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = history.prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let change_pct = if first > 0.0 { ((last - first) / first) * 100.0 } else { 0.0 };

        let change_color = if change_pct >= 0.0 { t.positive } else { t.negative };
        let sign = if change_pct >= 0.0 { "+" } else { "" };

        let stats = Paragraph::new(Line::from(vec![
            Span::styled(format!(" Price: {} ", format_price(last)), Style::default().fg(t.fg)),
            Span::styled(
                format!(" {}{:.2}% ", sign, change_pct),
                Style::default().fg(change_color),
            ),
            Span::styled(
                format!(" Lo: {}  Hi: {} ", format_price(min), format_price(max)),
                Style::default().fg(t.dim),
            ),
        ]));
        f.render_widget(stats, chunks[0]);

        let spark_width = chunks[1].width as usize;
        let spark_height = chunks[1].height as usize;
        let resolution = spark_height.max(1) as f64 * 8.0; // braille chars give 8 vertical levels per row

        // Downsample prices to match terminal width
        let sampled = downsample(&history.prices, spark_width);

        let spark_data: Vec<u64> = {
            let min_p = sampled.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_p = sampled.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max_p - min_p;
            if range == 0.0 {
                vec![(resolution / 2.0) as u64; sampled.len()]
            } else {
                sampled
                    .iter()
                    .map(|p| ((p - min_p) / range * resolution) as u64)
                    .collect()
            }
        };

        let spark_color = if change_pct >= 0.0 { t.positive } else { t.negative };

        let sparkline = Sparkline::default()
            .data(&spark_data)
            .style(Style::default().fg(spark_color));
        f.render_widget(sparkline, chunks[1]);
    } else if app.loading_chart {
        let msg = Paragraph::new("  Loading chart data...")
            .style(Style::default().fg(t.dim));
        f.render_widget(msg, inner);
    } else {
        let msg = Paragraph::new("  No data. Press h/l to reload.")
            .style(Style::default().fg(t.dim));
        f.render_widget(msg, inner);
    }
}

// -- Amount input popup --

fn draw_input_popup(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let coin = match app.selected_coin() {
        Some(c) => c,
        None => return,
    };

    let area = centered_rect(40, 5, f.area());
    let area = Rect {
        height: area.height.max(5),
        ..area
    };
    f.render_widget(Clear, area);

    let title = format!(" {} amount ", coin.symbol.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.input_accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = if app.input_buf.is_empty() {
        "Enter amount (0 to remove): _"
    } else {
        &app.input_buf
    };

    let input = Paragraph::new(format!(" {}_", text.trim_end_matches('_')))
        .style(Style::default().fg(t.fg));
    f.render_widget(input, inner);
}

// -- Settings dialog --

fn draw_settings(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();
    let box_w = 60_u16.min(area.width.saturating_sub(4));
    let box_h = 20_u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(box_w)) / 2;
    let y = (area.height.saturating_sub(box_h)) / 2;
    let popup = Rect::new(x, y, box_w, box_h);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // [0] blank
            Constraint::Length(1), // [1] currency label
            Constraint::Length(1), // [2] currency value
            Constraint::Length(1), // [3] blank
            Constraint::Length(1), // [4] theme label
            Constraint::Length(1), // [5] theme value
            Constraint::Length(1), // [6] blank
            Constraint::Length(1), // [7] coingecko label
            Constraint::Length(1), // [8] coingecko value
            Constraint::Length(1), // [9] blank
            Constraint::Length(1), // [10] cmc label
            Constraint::Length(1), // [11] cmc value
            Constraint::Length(1), // [12] blank
            Constraint::Length(1), // [13] hint
            Constraint::Min(0),
        ])
        .split(inner);

    // -- Currency field --
    {
        let is_selected = app.settings_field == SettingsField::Currency;
        let label_style = if is_selected {
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.dim)
        };
        let marker = if is_selected { "\u{25b8} " } else { "  " };
        let label = Paragraph::new(format!("{}Currency", marker)).style(label_style);
        f.render_widget(label, chunks[1]);

        let cur = CURRENCIES[app.settings_currency_idx];
        let sym = currency_symbol(cur);
        let val_spans = if is_selected {
            Line::from(vec![
                Span::styled("    \u{25c2} ", Style::default().fg(t.dim)),
                Span::styled(
                    format!("{} ({})", cur.to_uppercase(), sym),
                    Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" \u{25b8}", Style::default().fg(t.dim)),
            ])
        } else {
            Line::from(Span::styled(
                format!("    {} ({})", cur.to_uppercase(), sym),
                Style::default().fg(t.accent),
            ))
        };
        f.render_widget(Paragraph::new(val_spans), chunks[2]);
    }

    // -- Theme field --
    {
        let is_selected = app.settings_field == SettingsField::Theme;
        let label_style = if is_selected {
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.dim)
        };
        let marker = if is_selected { "\u{25b8} " } else { "  " };
        let label = Paragraph::new(format!("{}Theme", marker)).style(label_style);
        f.render_widget(label, chunks[4]);

        let theme_name = THEME_NAMES[app.settings_theme_idx];
        let val_spans = if is_selected {
            Line::from(vec![
                Span::styled("    \u{25c2} ", Style::default().fg(t.dim)),
                Span::styled(
                    theme_name,
                    Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" \u{25b8}", Style::default().fg(t.dim)),
            ])
        } else {
            Line::from(Span::styled(
                format!("    {}", theme_name),
                Style::default().fg(t.accent),
            ))
        };
        f.render_widget(Paragraph::new(val_spans), chunks[5]);
    }

    // -- API key fields --
    let api_fields: [(SettingsField, &String, usize, usize); 2] = [
        (SettingsField::CoingeckoApiKey, &app.settings_coingecko_key, 7, 8),
        (SettingsField::CoinmarketcapApiKey, &app.settings_cmc_key, 10, 11),
    ];

    for (field, value, label_row, value_row) in &api_fields {
        let is_selected = app.settings_field == *field;
        let is_editing = is_selected && app.settings_editing;

        let label_style = if is_selected {
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.dim)
        };

        let marker = if is_selected { "\u{25b8} " } else { "  " };
        let label = Paragraph::new(format!("{}{}", marker, field.label()))
            .style(label_style);
        f.render_widget(label, chunks[*label_row]);

        let display_val = if value.is_empty() {
            if is_editing { "_".to_string() } else { "(not set)".to_string() }
        } else {
            mask_key(value)
        };

        let val_text = if is_editing {
            let full = if value.is_empty() {
                String::new()
            } else {
                value.to_string()
            };
            format!("    {}_", full)
        } else {
            format!("    {}", display_val)
        };

        let val_style = if is_editing {
            Style::default().fg(t.input_accent)
        } else if value.is_empty() {
            Style::default().fg(t.dim)
        } else {
            Style::default().fg(t.accent)
        };

        let val_p = Paragraph::new(val_text).style(val_style);
        f.render_widget(val_p, chunks[*value_row]);
    }

    let hint = if app.settings_editing {
        "  Enter/Esc finish editing"
    } else if app.settings_field == SettingsField::Currency || app.settings_field == SettingsField::Theme {
        "  h/l change | s save & close | Esc cancel"
    } else {
        "  Enter edit | s save & close | Esc cancel"
    };
    let hint_p = Paragraph::new(hint)
        .style(Style::default().fg(t.dim));
    f.render_widget(hint_p, chunks[13]);
}

fn draw_search(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = f.area();
    let box_w = 50_u16.min(area.width.saturating_sub(4));
    let box_h = if app.input_mode == InputMode::SearchResults {
        (4 + app.search_results.len() as u16 + 2).min(area.height.saturating_sub(2))
    } else {
        8_u16.min(area.height.saturating_sub(2))
    };
    let x = (area.width.saturating_sub(box_w)) / 2;
    let y = (area.height.saturating_sub(box_h)) / 2;
    let popup = Rect::new(x, y, box_w, box_h);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Add coin ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    if app.input_mode == InputMode::SearchCoin {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // blank
                Constraint::Length(1), // label
                Constraint::Length(1), // input
                Constraint::Length(1), // blank
                Constraint::Min(1),   // error/hint
            ])
            .split(inner);

        let label = Paragraph::new("  Search by name or ticker:")
            .style(Style::default().fg(t.dim));
        f.render_widget(label, chunks[1]);

        let input_text = if app.search_query.is_empty() {
            "  _".to_string()
        } else {
            format!("  {}_", app.search_query)
        };
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(t.fg));
        f.render_widget(input, chunks[2]);

        if app.search_loading {
            let msg = Paragraph::new("  Searching...")
                .style(Style::default().fg(t.dim));
            f.render_widget(msg, chunks[4]);
        } else if let Some(ref err) = app.search_error {
            let msg = Paragraph::new(format!("  {}", err))
                .style(Style::default().fg(t.error));
            f.render_widget(msg, chunks[4]);
        } else {
            let hint = Paragraph::new("  Enter search | Esc cancel")
                .style(Style::default().fg(t.dim));
            f.render_widget(hint, chunks[4]);
        }
    } else {
        // SearchResults
        let mut constraints = vec![
            Constraint::Length(1), // blank
        ];
        for _ in &app.search_results {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(1)); // blank
        constraints.push(Constraint::Min(1));    // hint

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, result) in app.search_results.iter().enumerate() {
            let row_idx = 1 + i;
            if row_idx >= chunks.len().saturating_sub(2) {
                break;
            }

            let is_sel = i == app.search_selected;
            let marker = if is_sel { "\u{25b8} " } else { "  " };
            let rank = result
                .market_cap_rank
                .map(|r| format!("#{}", r))
                .unwrap_or_default();

            let line = Line::from(vec![
                Span::styled(
                    marker,
                    if is_sel {
                        Style::default().fg(t.fg)
                    } else {
                        Style::default().fg(t.dim)
                    },
                ),
                Span::styled(
                    &result.name,
                    if is_sel {
                        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(t.fg)
                    },
                ),
                Span::styled(
                    format!(" ({}) ", result.symbol.to_uppercase()),
                    Style::default().fg(t.dim),
                ),
                Span::styled(rank, Style::default().fg(t.dim)),
            ]);

            let style = if is_sel {
                Style::default().bg(t.highlight_bg)
            } else {
                Style::default()
            };

            let p = Paragraph::new(line).style(style);
            f.render_widget(p, chunks[row_idx]);
        }

        let hint_idx = chunks.len().saturating_sub(1);
        let hint = Paragraph::new("  j/k select | Enter add | Esc back")
            .style(Style::default().fg(t.dim));
        f.render_widget(hint, chunks[hint_idx]);
    }
}

fn mask_key(key: &str) -> String {
    if key.len() <= 6 {
        "\u{2022}".repeat(key.len())
    } else {
        let visible = &key[..3];
        let hidden = "\u{2022}".repeat(key.len() - 3);
        format!("{}{}", visible, hidden)
    }
}

// -- Helpers --

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let x = (r.width.saturating_sub(popup_width)) / 2;
    let y = (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(r.x + x, r.y + y, popup_width, popup_height)
}

fn format_price(v: f64) -> String {
    if v >= 1.0 {
        let s = format!("{:.2}", v);
        add_commas(&s)
    } else if v >= 0.01 {
        format!("{:.4}", v)
    } else if v > 0.0 {
        format!("{:.6}", v)
    } else {
        "0.00".to_string()
    }
}

fn format_large(v: f64) -> String {
    if v >= 1_000_000_000_000.0 {
        format!("{:.1}T", v / 1_000_000_000_000.0)
    } else if v >= 1_000_000_000.0 {
        format!("{:.1}B", v / 1_000_000_000.0)
    } else if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        format!("{:.0}", v)
    }
}

fn format_pct(v: Option<f64>) -> String {
    match v {
        Some(p) => {
            let sign = if p >= 0.0 { "+" } else { "" };
            format!("{}{:.1}%", sign, p)
        }
        None => "--".to_string(),
    }
}

fn format_amount(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else if v >= 1.0 {
        let s = format!("{:.4}", v);
        let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
        add_commas(&s)
    } else {
        format!("{:.6}", v)
    }
}

fn pct_cell(v: Option<f64>, formatted: &str, positive: Color, negative: Color, dim: Color) -> Cell<'static> {
    let color = match v {
        Some(p) if p > 0.0 => positive,
        Some(p) if p < 0.0 => negative,
        _ => dim,
    };
    Cell::from(formatted.to_string()).style(Style::default().fg(color))
}

fn downsample(data: &[f64], target_len: usize) -> Vec<f64> {
    if target_len == 0 || data.is_empty() {
        return vec![];
    }
    if data.len() <= target_len {
        return data.to_vec();
    }
    let mut result = Vec::with_capacity(target_len);
    let bucket_size = data.len() as f64 / target_len as f64;
    for i in 0..target_len {
        let start = (i as f64 * bucket_size) as usize;
        let end = (((i + 1) as f64 * bucket_size) as usize).min(data.len());
        if start >= end {
            if let Some(&last) = result.last() {
                result.push(last);
            }
            continue;
        }
        // Use min-max-close to preserve peaks and valleys
        let slice = &data[start..end];
        let min = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let close = slice[slice.len() - 1];
        // Pick whichever extreme is further from the previous point to preserve shape
        if let Some(&prev) = result.last() {
            let d_min = f64::abs(min - prev);
            let d_max = f64::abs(max - prev);
            if d_min > d_max {
                result.push(min);
            } else {
                result.push(max);
            }
        } else {
            result.push(close);
        }
    }
    result
}

fn add_commas(s: &str) -> String {
    let parts: Vec<&str> = s.split('.').collect();
    let int_part = parts[0];
    let mut result = String::new();
    for (i, c) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 && c != '-' {
            result.push(',');
        }
        result.push(c);
    }
    let int_formatted: String = result.chars().rev().collect();
    if parts.len() > 1 {
        format!("{}.{}", int_formatted, parts[1])
    } else {
        int_formatted
    }
}
