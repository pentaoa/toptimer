# Toptimer

Toptimer is a terminal countdown TUI for conference deadlines. It is built with Rust, Ratatui, and a small set of built-in fixed-width digit fonts designed to avoid jitter during countdown refreshes.

## Features

- Centered terminal UI for timers, settings, and config viewing
- Large countdown digits that scale to the available terminal space
- Fixed-width digit fonts: `tube`, `block`, `compact`
- Global language setting: Chinese, Japanese, English
- Shared timezone preset system, including AOE (`Etc/GMT+12`), UTC, China, Japan, US, and Europe presets
- Precision modes: minutes, seconds, tenths, hundredths, milliseconds
- Config stored at `~/.config/toptimer/config.json`

## Install

From the repository:

```bash
cargo build --release
cp target/release/toptimer ~/.local/bin/toptimer
```

For local development:

```bash
cargo run
```

## Usage

```bash
toptimer
```

Useful commands:

```bash
toptimer init
toptimer config
toptimer fonts
```

Timer screen shortcuts:

- `←` / `→`: switch digit font
- `↑` / `↓`: switch timer item
- `r`: reload config
- `q`: back

## Configuration

Config file:

```text
~/.config/toptimer/config.json
```

Core settings:

```json
{
  "settings": {
    "home_timezone": "Asia/Shanghai",
    "display_precision": "seconds",
    "default_add_timezone": "Asia/Shanghai",
    "language": "zh"
  }
}
```

Supported language codes:

- `zh`: Chinese
- `ja`: Japanese
- `en`: English

## Development

```bash
cargo fmt
cargo build
cargo test
```

## License

MIT