//! Database migration and Keycloak seeding module
//!
//! Provides functionality for:
//! - Running database migrations
//! - Seeding Keycloak with realm and default admin user

use crate::config::Config;
use crate::keycloak::KeycloakSeeder;
use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use tracing::info;

/// Run database migrations
pub async fn run_migrations(config: &Config) -> Result<()> {
    info!("Connecting to database...");

    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    info!("Running database migrations...");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run migrations")?;

    info!("Database migrations completed");
    Ok(())
}

/// Seed Keycloak with default realm and admin user
pub async fn seed_keycloak(config: &Config) -> Result<()> {
    info!("Initializing Keycloak seeder...");

    let seeder = KeycloakSeeder::new(&config.keycloak);

    // Create realm if not exists
    info!("Ensuring realm '{}' exists...", config.keycloak.realm);
    seeder
        .ensure_realm_exists()
        .await
        .context("Failed to create realm")?;

    // Seed default admin user
    info!("Seeding default admin user...");
    seeder
        .seed_admin_user()
        .await
        .context("Failed to seed admin user")?;

    info!("Keycloak seeding completed");
    Ok(())
}
