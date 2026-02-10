//! Auth9 Core - CLI Entry Point
//!
//! Commands:
//!   serve   - Start the API server (default)
//!   init    - Run migrations and seed default data
//!   migrate - Run database migrations only
//!   seed    - Seed Keycloak with default data only
//!   reset   - Reset database (drop all tables)

use anyhow::Result;
use auth9_core::{config::Config, migration, server, telemetry};
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "auth9-core")]
#[command(about = "Auth9 Identity Service Backend", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server (HTTP + gRPC)
    Serve,
    /// Run migrations and seed default data (migrate + seed)
    Init,
    /// Run database migrations only
    Migrate,
    /// Seed Keycloak with default data only
    Seed,
    /// Reset database (drop all tables)
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration first (telemetry init needs config)
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    // Initialise telemetry (metrics + tracing + structured logging)
    let prometheus_handle = telemetry::init(&config.telemetry);

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            info!("Running init (migrate + seed)...");
            migration::run_migrations(&config).await?;
            migration::seed_keycloak(&config).await?;
            info!("Init completed successfully");
        }
        Some(Commands::Migrate) => {
            info!("Running database migrations...");
            migration::run_migrations(&config).await?;
            info!("Migrations completed successfully");
        }
        Some(Commands::Seed) => {
            info!("Seeding Keycloak with default data...");
            migration::seed_keycloak(&config).await?;
            info!("Seed completed successfully");
        }
        Some(Commands::Reset) => {
            info!("Resetting database (dropping all tables)...");
            migration::reset_database(&config).await?;
            info!("Database reset completed");
        }
        Some(Commands::Serve) | None => {
            info!("Starting Auth9 Core Service");
            info!("HTTP server listening on {}", config.http_addr());
            info!("gRPC server listening on {}", config.grpc_addr());
            server::run(config, prometheus_handle).await?;
        }
    }

    Ok(())
}
