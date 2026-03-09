use crate::config::Clickhouse;
use clickhouse::Client;
use std::error::Error;

pub async fn connect(config: Clickhouse) -> Result<Client, Box<dyn Error>> {
    let client = Client::default()
        .with_url(config.url)
        .with_user(config.user)
        .with_password(config.password);

    // Ensure client is connected by executing a simple query
    let _: u8 = client.query("SELECT 1").fetch_one().await?;

    Ok(client)
}
