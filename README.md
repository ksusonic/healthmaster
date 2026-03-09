# Healthmaster

A Rust application that continuously monitors health check targets and stores results in ClickHouse.

## Features

- **Continuous Health Monitoring**: Automatically checks configured targets at specified intervals
- **ClickHouse Integration**: Stores all health check results for analysis and monitoring
- **Configurable Targets**: Define multiple targets with custom timeouts and check intervals
- **Async/Concurrent**: Each target runs in its own task for efficient parallel monitoring

## Layout

- `src/lib.rs` - public library surface (`app`, `config`)
- `src/app.rs` - app orchestration (`run_default`, `run`, `load_config`)
- `src/config.rs` - split config loading (TOML + env)
- `src/health_checker.rs` - health checking logic and ClickHouse storage
- `src/clickhouse.rs` - ClickHouse client connection
- `src/main.rs` - thin binary entrypoint
- `tests/config_loading.rs` - integration test for loading config from file

## Config shape (`config.toml`)

```toml
[[targets]]
name = "google"
url = "https://google.com"
timeout_ms = 3000
interval_seconds = 30  # Check every 30 seconds

[[targets]]
name = "github"
url = "https://github.com"
timeout_ms = 5000
interval_seconds = 60  # Check every 60 seconds
```

**Configuration Options:**
- `name`: Target identifier
- `url`: URL to check
- `timeout_ms`: Request timeout in milliseconds
- `interval_seconds`: How often to check (default: 60 seconds)

## Environment Variables

Required environment variables (set in `.env`):
- `TELEGRAM_BOT_TOKEN`: Telegram bot token for notifications
- `TELEGRAM_CHAT_ID`: Telegram chat ID for notifications
- `CLICKHOUSE_URL`: ClickHouse server URL
- `CLICKHOUSE_USER`: ClickHouse username
- `CLICKHOUSE_PASSWORD`: ClickHouse password

## Quick start

```bash
cp .env.example .env
# edit .env and set TELEGRAM_BOT_TOKEN and other values
cargo run
cargo test
```

## How it Works

1. The application loads configuration from `config.toml` and environment variables
2. Connects to ClickHouse database
3. Spawns an async task for each configured target
4. Each task runs in a loop:
   - Performs HTTP GET request to the target URL
   - Measures response time and status
   - Stores result in ClickHouse `health_checks` table
   - Waits for the configured interval
   - Repeats

## Database Schema

Health check results are stored in the `health_checks` table:
- `timestamp`: When the check was performed
- `target`: Target name
- `url`: Target URL
- `status`: HTTP status code (0 if request failed)
- `latency_ms`: Response time in milliseconds
- `success`: 1 if successful, 0 if failed
- `error`: Error message if request failed

