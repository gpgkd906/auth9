use anyhow::Result;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

pub async fn connect(database_url: &str) -> Result<MySqlPool> {
    Ok(MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?)
}
