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

    if app.input_mode == InputMode::EditingAlert {
        draw_alert_popup(f, app);
    }

    if app.input_mode == InputMode::EditingBuyPrice {
        draw_buyprice_popup(f, app);
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
        .title(" bags ")
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

    let tabs_list = [Tab::Markets, Tab::Favourites, Tab::Portfolio];
    let mut spans: Vec<Span> = Vec::new();

    spans.push(Span::styled(
        " bags ",
        Style::default().fg(t.title).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled("\u{2689} ", Style::default().fg(t.dim)));

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

    // Global market stats
    if let Some(ref stats) = app.global_stats {
        spans.push(Span::styled(" \u{2502} ", Style::default().fg(t.dim)));
        spans.push(Span::styled(
            format!("MCap(USD):{} ", format_large(stats.total_market_cap_usd)),
            Style::default().fg(t.dim),
        ));
        spans.push(Span::styled(
            format!("BTC Dom:{:.1}% ", stats.btc_dominance),
            Style::default().fg(t.accent),
        ));
        if let (Some(idx), Some(ref label)) = (stats.fear_greed_index, &stats.fear_greed_label) {
            let fg_color = if idx >= 60 { t.positive } else if idx <= 40 { t.negative } else { t.dim };
            spans.push(Span::styled(
                format!("F&G:{} {} ", idx, label),
                Style::default().fg(fg_color),
            ));
        }
    }

    // Right-align refresh info
    let refresh_info = if app.loading {
        "loading...".to_string()
    } else if app.last_refresh_display.is_empty() {
        String::new()
    } else {
        app.last_refresh_display.clone()
    };

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

fn sort_indicator(app: &App, col: SortColumn) -> &'static str {
    if app.sort_column == Some(col) {
        match app.sort_direction {
            SortDirection::Asc => " \u{25b4}",
            SortDirection::Desc => " \u{25be}",
        }
    } else {
        ""
    }
}

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
            _ => if !app.filter_query.is_empty() {
                "  No matches for filter."
            } else {
                "  No data."
            },
        };
        let p = Paragraph::new(msg).style(Style::default().fg(t.dim));
        f.render_widget(p, area);
        return;
    }

    let is_portfolio = app.tab == Tab::Portfolio;

    let header_cells = {
        let mut h = vec![
            format!("#{}", sort_indicator(app, SortColumn::Rank)),
            format!("Name{}", sort_indicator(app, SortColumn::Name)),
            "Ticker".to_string(),
            format!("Price{}", sort_indicator(app, SortColumn::Price)),
            format!("1h%{}", sort_indicator(app, SortColumn::Change1h)),
            format!("24h%{}", sort_indicator(app, SortColumn::Change24h)),
            format!("7d%{}", sort_indicator(app, SortColumn::Change7d)),
            "24h Hi".to_string(),
            "24h Lo".to_string(),
            format!("Volume{}", sort_indicator(app, SortColumn::Volume)),
            format!("MCap{}", sort_indicator(app, SortColumn::MarketCap)),
        ];
        if is_portfolio {
            h.push("Qty".to_string());
            h.push("Value".to_string());
            h.push("P&L".to_string());
            h.push("P&L%".to_string());
        }
        h
    };

    let header = Row::new(
        header_cells
            .iter()
            .map(|h| Cell::from(h.as_str()).style(Style::default().fg(t.dim))),
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

    // Check for alert flash
    let flash_coin_id = app.alert_flash.as_ref().map(|(id, _)| id.clone());

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
            let hi24 = coin.high_24h.map(|v| format_price(v)).unwrap_or_else(|| "--".into());
            let lo24 = coin.low_24h.map(|v| format_price(v)).unwrap_or_else(|| "--".into());
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
                Cell::from(hi24).style(Style::default().fg(dim)),
                Cell::from(lo24).style(Style::default().fg(dim)),
                Cell::from(vol).style(Style::default().fg(dim)),
                Cell::from(mcap).style(Style::default().fg(dim)),
            ];

            if is_portfolio {
                let amt = app.holding_for(&coin.id);
                let val = amt * coin.current_price;
                cells.push(Cell::from(format_amount(amt)).style(Style::default().fg(fg)));
                cells.push(Cell::from(format_price(val)).style(Style::default().fg(accent)));

                // P&L
                if let Some(buy) = app.buy_price_for(&coin.id) {
                    let pnl = (coin.current_price - buy) * amt;
                    let pnl_pct = if buy > 0.0 { ((coin.current_price - buy) / buy) * 100.0 } else { 0.0 };
                    let pnl_color = if pnl >= 0.0 { positive } else { negative };
                    let sign = if pnl >= 0.0 { "+" } else { "" };
                    cells.push(Cell::from(format!("{}{}", sign, format_price(pnl.abs()))).style(
                        Style::default().fg(pnl_color),
                    ));
                    cells.push(Cell::from(format!("{}{:.1}%", sign, pnl_pct)).style(
                        Style::default().fg(pnl_color),
                    ));
                } else {
                    cells.push(Cell::from("--").style(Style::default().fg(dim)));
                    cells.push(Cell::from("--").style(Style::default().fg(dim)));
                }
            }

            let is_flashing = flash_coin_id.as_deref() == Some(&coin.id);
            let style = if i == app.selected {
                Style::default().bg(highlight_bg).fg(highlight_fg)
            } else if is_flashing {
                Style::default().bg(t.accent).fg(t.bg)
            } else {
                Style::default().bg(bg)
            };

            Row::new(cells).style(style)
        })
        .collect();

    let widths = if is_portfolio {
        vec![
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
        ]
    } else {
        vec![
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(6),
            Constraint::Length(11),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(9),
        ]
    };

    let mut block = Block::default().borders(Borders::NONE);

    if is_portfolio {
        let total = app.total_portfolio_value();
        block = block.title(Line::from(vec![
            Span::styled(" Total: ", Style::default().fg(t.dim)),
            Span::styled(
                format!("{}{} ", currency_symbol(&app.config.currency), format_price(total)),
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

    // Filter bar mode
    if app.input_mode == InputMode::Filtering {
        let n = app.visible_coins().len();
        let text = format!(" / {}_  ({} results)", app.filter_query, n);
        let bar = Paragraph::new(text).style(Style::default().fg(t.input_accent));
        f.render_widget(bar, area);
        return;
    }

    // Sort picking mode
    if app.sort_picking {
        let bar = Paragraph::new(" Sort: r)ank  n)ame  p)rice  1)h  2)4h  7)d  v)ol  m)cap  Esc)clear ")
            .style(Style::default().fg(t.input_accent));
        f.render_widget(bar, area);
        return;
    }

    let hints = if app.popup_open {
        " Esc close | h/l cycle view "
    } else if app.input_mode == InputMode::Settings {
        " j/k navigate | Enter edit | s save | Esc cancel "
    } else {
        match app.tab {
            Tab::Markets => " j/k \u{2195} | Tab \u{21c6} | Enter detail | f fav | a hold | / filter | s sort | A alert | c add | S set | q quit ",
            Tab::Favourites => " j/k \u{2195} | Tab \u{21c6} | Enter detail | f unfav | a hold | / filter | s sort | A alert | c add | S set | q quit ",
            Tab::Portfolio => " j/k \u{2195} | Tab \u{21c6} | Enter detail | a edit | d rm | b buy$ | / filter | s sort | A alert | c add | S set | q quit ",
        }
    };

    let mut spans = vec![Span::styled(hints, Style::default().fg(t.dim))];

    // Show active filter indicator
    if !app.filter_query.is_empty() && app.input_mode != InputMode::Filtering {
        spans.push(Span::styled(
            format!(" [/{}]", app.filter_query),
            Style::default().fg(t.accent),
        ));
    }

    if let Some(ref err) = app.error {
        spans.push(Span::styled(
            format!(" \u{2502} {}", err),
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

    let area = centered_rect(75, 65, f.area());
    f.render_widget(Clear, area);

    let title = format!(" {} ({}) - {} ", coin.name, coin.symbol.to_uppercase(), app.chart_view.label());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cache_key = (coin.id.clone(), app.chart_view.days());

    // Build info section with supply + alerts
    let mut info_lines: Vec<Line> = Vec::new();

    // Supply info line
    let circ = coin.circulating_supply.map(|v| format_large(v)).unwrap_or_else(|| "--".into());
    let max_s = coin.max_supply.map(|v| format_large(v)).unwrap_or_else(|| "\u{221e}".into());
    info_lines.push(Line::from(vec![
        Span::styled(format!(" Supply: {} / {} ", circ, max_s), Style::default().fg(t.dim)),
    ]));

    // Active alerts for this coin
    let coin_alerts: Vec<&PriceAlert> = app.alerts.iter().filter(|a| a.coin_id == coin.id && !a.triggered).collect();
    if !coin_alerts.is_empty() {
        let mut alert_spans = vec![Span::styled(" Alerts: ", Style::default().fg(t.dim))];
        for (i, alert) in coin_alerts.iter().enumerate() {
            if i > 0 {
                alert_spans.push(Span::styled(", ", Style::default().fg(t.dim)));
            }
            let dir = match alert.direction {
                AlertDirection::Above => "\u{25b2}",
                AlertDirection::Below => "\u{25bc}",
            };
            alert_spans.push(Span::styled(
                format!("{}{}", dir, format_price(alert.target_price)),
                Style::default().fg(t.accent),
            ));
        }
        info_lines.push(Line::from(alert_spans));
    }

    let info_height = info_lines.len() as u16;

    if let Some(history) = app.chart_cache.get(&cache_key) {
        if history.prices.is_empty() {
            let msg = Paragraph::new("  No price data available.")
                .style(Style::default().fg(t.dim));
            f.render_widget(msg, inner);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),           // stats
                Constraint::Min(3),              // chart
                Constraint::Length(info_height),  // supply + alerts
            ])
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
        let resolution = spark_height.max(1) as f64 * 8.0;

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

        // Info section
        let info_p = Paragraph::new(info_lines);
        f.render_widget(info_p, chunks[2]);
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

// -- Alert popup --

fn draw_alert_popup(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let coin = match app.selected_coin() {
        Some(c) => c,
        None => return,
    };

    let area = centered_rect(45, 5, f.area());
    let area = Rect { height: area.height.max(7), ..area };
    f.render_widget(Clear, area);

    let title = format!(" {} alert ", coin.symbol.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.input_accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // current price
            Constraint::Length(1), // direction
            Constraint::Length(1), // input
            Constraint::Length(1), // hint
            Constraint::Min(0),
        ])
        .split(inner);

    let price_line = format!(" Current: {}", format_price(coin.current_price));
    f.render_widget(
        Paragraph::new(price_line).style(Style::default().fg(t.dim)),
        chunks[0],
    );

    let dir_label = match app.alert_direction {
        AlertDirection::Above => "\u{25b2} Above",
        AlertDirection::Below => "\u{25bc} Below",
    };
    f.render_widget(
        Paragraph::new(format!(" Direction: {} (Tab to toggle)", dir_label))
            .style(Style::default().fg(t.accent)),
        chunks[1],
    );

    let input_text = if app.alert_input_buf.is_empty() {
        " Target: _".to_string()
    } else {
        format!(" Target: {}_", app.alert_input_buf)
    };
    f.render_widget(
        Paragraph::new(input_text).style(Style::default().fg(t.fg)),
        chunks[2],
    );

    f.render_widget(
        Paragraph::new(" Enter save | Esc cancel").style(Style::default().fg(t.dim)),
        chunks[3],
    );
}

// -- Buy price popup --

fn draw_buyprice_popup(f: &mut Frame, app: &App) {
    let t = &app.theme;
    let coin = match app.selected_coin() {
        Some(c) => c,
        None => return,
    };

    let area = centered_rect(40, 5, f.area());
    let area = Rect { height: area.height.max(5), ..area };
    f.render_widget(Clear, area);

    let title = format!(" {} buy price ", coin.symbol.to_uppercase());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.input_accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = if app.buy_price_buf.is_empty() {
        "Enter buy-in price: _"
    } else {
        &app.buy_price_buf
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
    let box_h = 26_u16.min(area.height.saturating_sub(2));
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
            Constraint::Length(1), // [13] notifications label
            Constraint::Length(1), // [14] notifications value
            Constraint::Length(1), // [15] blank
            Constraint::Length(1), // [16] ntfy topic label
            Constraint::Length(1), // [17] ntfy topic value
            Constraint::Length(1), // [18] blank
            Constraint::Length(1), // [19] hint
            Constraint::Min(0),
        ])
        .split(inner);

    // -- Currency field --
    draw_cycle_field(f, t, chunks[1], chunks[2],
        app.settings_field == SettingsField::Currency,
        "Currency",
        &format!("{} ({})", CURRENCIES[app.settings_currency_idx].to_uppercase(), currency_symbol(CURRENCIES[app.settings_currency_idx])),
    );

    // -- Theme field --
    draw_cycle_field(f, t, chunks[4], chunks[5],
        app.settings_field == SettingsField::Theme,
        "Theme",
        THEME_NAMES[app.settings_theme_idx],
    );

    // -- API key fields --
    let api_fields: [(SettingsField, &String, usize, usize); 2] = [
        (SettingsField::CoingeckoApiKey, &app.settings_coingecko_key, 7, 8),
        (SettingsField::CoinmarketcapApiKey, &app.settings_cmc_key, 10, 11),
    ];

    for (field, value, label_row, value_row) in &api_fields {
        draw_text_field(f, t, chunks[*label_row], chunks[*value_row],
            app.settings_field == *field, app.settings_editing && app.settings_field == *field,
            field.label(), value, true,
        );
    }

    // -- Notification method --
    draw_cycle_field(f, t, chunks[13], chunks[14],
        app.settings_field == SettingsField::Notifications,
        "Notifications",
        NOTIFICATION_METHODS[app.settings_notification_idx],
    );

    // -- Ntfy topic --
    draw_text_field(f, t, chunks[16], chunks[17],
        app.settings_field == SettingsField::NtfyTopic,
        app.settings_editing && app.settings_field == SettingsField::NtfyTopic,
        "Ntfy Topic",
        &app.settings_ntfy_topic,
        false,
    );

    let hint = if app.settings_editing {
        "  Enter/Esc finish editing"
    } else if app.settings_field.is_cycle_field() {
        "  h/l change | s save & close | Esc cancel"
    } else {
        "  Enter edit | s save & close | Esc cancel"
    };
    let hint_p = Paragraph::new(hint)
        .style(Style::default().fg(t.dim));
    f.render_widget(hint_p, chunks[19]);
}

fn draw_cycle_field(f: &mut Frame, t: &crate::theme::Theme, label_area: Rect, value_area: Rect, is_selected: bool, label: &str, value: &str) {
    let label_style = if is_selected {
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.dim)
    };
    let marker = if is_selected { "\u{25b8} " } else { "  " };
    f.render_widget(Paragraph::new(format!("{}{}", marker, label)).style(label_style), label_area);

    let val_spans = if is_selected {
        Line::from(vec![
            Span::styled("    \u{25c2} ", Style::default().fg(t.dim)),
            Span::styled(value, Style::default().fg(t.fg).add_modifier(Modifier::BOLD)),
            Span::styled(" \u{25b8}", Style::default().fg(t.dim)),
        ])
    } else {
        Line::from(Span::styled(format!("    {}", value), Style::default().fg(t.accent)))
    };
    f.render_widget(Paragraph::new(val_spans), value_area);
}

fn draw_text_field(f: &mut Frame, t: &crate::theme::Theme, label_area: Rect, value_area: Rect, is_selected: bool, is_editing: bool, label: &str, value: &str, mask: bool) {
    let label_style = if is_selected {
        Style::default().fg(t.fg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.dim)
    };
    let marker = if is_selected { "\u{25b8} " } else { "  " };
    f.render_widget(Paragraph::new(format!("{}{}", marker, label)).style(label_style), label_area);

    let display_val = if value.is_empty() {
        if is_editing { "_".to_string() } else { "(not set)".to_string() }
    } else if mask {
        mask_key(value)
    } else {
        value.to_string()
    };

    let val_text = if is_editing {
        format!("    {}_", if value.is_empty() { "" } else { value })
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

    f.render_widget(Paragraph::new(val_text).style(val_style), value_area);
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
