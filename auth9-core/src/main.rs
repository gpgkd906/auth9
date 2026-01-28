use anyhow::Result;
use auth9_core::{config::Config, server};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auth9_core=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    info!("Starting Auth9 Core Service");
    info!("HTTP server listening on {}", config.http_addr());
    info!("gRPC server listening on {}", config.grpc_addr());

    // Run the server
    server::run(config).await
}
