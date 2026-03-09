use healthmaster::app;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_config() -> PathBuf {
    let mut path = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    path.push(format!("healthmaster-config-{now}.toml"));

    let content = r#"
interval_seconds = 15

[telegram]
chat_id = 42

[[targets]]
name = "example"
url = "https://example.com"
timeout_ms = 1500
"#;

    fs::write(&path, content).expect("temporary config should be writable");
    path
}

#[test]
fn load_config_from_file() {
    const TEST_TOKEN: &str = "test_token_123";

    unsafe {
        std::env::set_var("TELEGRAM_BOT_TOKEN", TEST_TOKEN);
    }

    let path = write_temp_config();
    let config = app::load_config(&path).expect("config should load");

    assert_eq!(config.interval_seconds, 15);
    assert_eq!(config.telegram.chat_id, 42);
    assert_eq!(config.telegram.bot_token, TEST_TOKEN);
    assert_eq!(config.targets.len(), 1);
    assert_eq!(config.targets[0].name, "example");

    let _ = fs::remove_file(path);
}
