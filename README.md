# bags

A crypto price and portfolio tracker for the terminal.

```
 bags ⚉ Markets · Favourites · Portfolio │ MCap:3.2T BTC:61.2% F&G:72 Greed    32s ago
────────────────────────────────────────────────────────────────────────────────────────
 #   Name           Ticker  Price      1h%     24h%    7d%     24h Hi     24h Lo     Volume   MCap
 1   Bitcoin        BTC     97,342.10  +0.3%   +2.1%   +5.4%   98,100.00  95,200.00  32.1B    1.9T
 2   Ethereum       ETH     3,241.55   -0.1%   +1.8%   +3.2%   3,300.10   3,180.00   18.4B    389.2B
 ...
```

## Features

- **Three tabs** -- Markets (top 50), Favourites, Portfolio
- **Encrypted storage** -- SQLCipher-encrypted local database, password unlock on launch
- **Price charts** -- Sparkline graphs with 1D/7D/30D views, supply info, active alerts
- **Portfolio tracking** -- Add holdings, see total value, P&L and % gain per coin
- **Profit & loss** -- Auto-records buy price on first add; override with `b`
- **Price alerts** -- Set target price above/below; terminal bell + row flash on trigger
- **Global market stats** -- Total market cap, BTC dominance, Fear & Greed index in the top bar
- **Sort columns** -- Press `s` then a column key to sort by price, 24h%, mcap, etc.
- **Filter** -- Press `/` to fuzzy-filter coins by name or ticker in real time
- **Mouse support** -- Click rows, scroll wheel, click tabs
- **24h range** -- High/low columns in the table
- **Notifications** -- Desktop (notify-rust) and/or push (ntfy.sh), configurable in settings
- **Custom coins** -- Search and add any coin from CoinGecko
- **API key support** -- Optional CoinGecko Pro / CoinMarketCap keys
- **Multi-currency** -- USD, EUR, GBP, JPY, BTC, ETH, and more
- **11 color themes** -- Dark, dark-blue, dark-green, light, bubblegum, no-color, ...
- **Configurable refresh** -- Default 60s, minimum 30s
- **Error logging** -- Errors logged to `~/.config/bags/errors.log` with timestamps
- **All data stays local** -- Nothing leaves your machine

## Install

```sh
cargo install --path .
```

Requires Rust toolchain. SQLCipher is bundled automatically.

## Usage

```sh
bags
```

First run prompts you to set a password. Subsequent runs unlock with that password.

## Keybindings

| Key | Action |
|---|---|
| `j` / `k` | Scroll up/down |
| `PgUp` / `PgDn` | Page up/down |
| `g` / `G` | Jump to top/bottom |
| `Tab` / `1` `2` `3` | Switch tabs |
| `Enter` | Coin detail + chart |
| `h` / `l` | Cycle chart view (1D/7D/30D) |
| `f` | Toggle favourite |
| `a` | Add/edit holding amount |
| `d` | Remove holding |
| `b` | Edit buy-in price (Portfolio) |
| `A` | Set price alert on selected coin |
| `/` | Filter coins by name/ticker |
| `s` | Sort by column (then `r`/`n`/`p`/`1`/`2`/`7`/`v`/`m`) |
| `c` | Search & add custom coin |
| `r` | Force refresh |
| `S` | Settings |
| `Esc` | Clear filter / close popup / quit |
| `q` | Quit |
| Mouse | Click rows, scroll wheel, click tabs |

## Config

`~/.config/bags/config.yaml`

```yaml
refresh_interval_secs: 60
currency: usd
theme: dark
```

## Data

- Database: `~/.local/share/bags/bags.db` (SQLCipher encrypted)
- Config: `~/.config/bags/config.yaml`
- Error log: `~/.config/bags/errors.log`

## Settings

Press `S` to open settings. Use `j`/`k` to navigate, `h`/`l` to cycle options, `Enter` to edit text fields, `s` to save.

- **Currency** -- Cycle through 13 currencies
- **Theme** -- Live preview while cycling through 11 themes
- **CoinGecko API Key** -- Optional, for higher rate limits
- **CoinMarketCap API Key** -- Optional
- **Notifications** -- none / desktop / ntfy / both
- **Ntfy Topic** -- Your ntfy.sh topic for push alerts

---

> *"Everyone has a plan until they check their portfolio."*
> -- Mike Tyson, mass holder of bags
