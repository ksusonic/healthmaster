# Healthmaster

Small Rust app that loads health-check configuration from `config.toml` using `serde` + `toml`.

## Layout

- `src/lib.rs` - public library surface (`app`, `config`)
- `src/app.rs` - app orchestration (`run_default`, `run`, `load_config`)
- `src/config.rs` - config structs and TOML deserialization
- `src/main.rs` - thin binary entrypoint
- `tests/config_loading.rs` - integration test for loading config from file

## Config shape

```toml
interval_seconds = 30

[telegram]
chat_id = 123456789

[[targets]]
name = "google"
url = "https://google.com"
timeout_ms = 3000
```

## Quick start

```bash
cargo run
cargo test
```

