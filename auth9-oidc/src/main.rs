use anyhow::Result;
use auth9_oidc::{config::Config, db, server};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".to_string()),
        )
        .init();

    let config = Config::from_env()?;
    info!("Starting auth9-oidc on {}", config.http_addr());
    info!("Current identity backend: {}", config.identity_backend);
    let db_pool = db::connect(&config.database_url).await?;
    server::run(config, db_pool).await
}
