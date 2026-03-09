# Healthmaster

Small Rust app that loads health-check targets from `config.toml`.

## Layout

- `src/lib.rs` - public library surface (`app`, `config`)
- `src/app.rs` - app orchestration (`run_default`, `run`, `load_config`)
- `src/config.rs` - split config loading (TOML + env)
- `src/main.rs` - thin binary entrypoint
- `tests/config_loading.rs` - integration test for loading config from file

## Config shape (`config.toml`)

```toml
[[targets]]
name = "google"
url = "https://google.com"
timeout_ms = 3000
```

## Quick start

```bash
cp .env.example .env
# edit .env and set TELEGRAM_BOT_TOKEN (and other values if needed)
cargo run
cargo test
```
