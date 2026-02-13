# bags

A minimal crypto portfolio tracker for the terminal.

```
 [bags] │ Markets · Favourites · Portfolio
──────────────────────────────────────────────────
 #   Name           Ticker  Price      1h%     24h%    7d%     Volume   MCap
 1   Bitcoin        BTC     97,342.10  +0.3%   +2.1%   +5.4%   32.1B   1.9T
 2   Ethereum       ETH     3,241.55   -0.1%   +1.8%   +3.2%   18.4B   389.2B
 ...
```

## Features

- **Three tabs** -- Markets (top 50), Favourites, Portfolio
- **Encrypted storage** -- SQLCipher-encrypted local database, password on launch
- **Price charts** -- Sparkline graphs with 1D/7D/30D views
- **Portfolio tracking** -- Add holdings, see total value
- **Custom coins** -- Search and add any coin from CoinGecko
- **API key support** -- Optional CoinGecko Pro / CoinMarketCap keys
- **Multi-currency** -- USD, EUR, GBP, JPY, BTC, ETH, and more
- **Configurable refresh** -- Default 60s, minimum 30s
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
| `c` | Search & add custom coin |
| `r` | Force refresh |
| `S` | Settings (API keys, currency) |
| `q` / `Esc` | Quit / close popup |

## Config

`~/.config/bags/config.yaml`

```yaml
refresh_interval_secs: 60
currency: usd
```

## Data

- Database: `~/.local/share/bags/bags.db` (SQLCipher encrypted)
- Config: `~/.config/bags/config.yaml`

---

> *"The stock market is a device for transferring money from the impatient to the patient."*
> -- Warren Buffett, who probably doesn't hold any bags
