//! Database migration and Keycloak seeding module
//!
//! Provides functionality for:
//! - Running database migrations
//! - Seeding Keycloak with realm and default admin user
//! - Seeding default services in database

use crate::config::Config;
use crate::keycloak::KeycloakSeeder;
use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Executor, MySql, Pool};
use tracing::info;

/// Default portal service configuration
const DEFAULT_PORTAL_CLIENT_ID: &str = "auth9-portal";
const DEFAULT_PORTAL_NAME: &str = "Auth9 Admin Portal";

/// Extract database name from DATABASE_URL
fn extract_db_name(url: &str) -> Option<&str> {
    // URL format: mysql://user:pass@host:port/dbname
    url.rsplit('/').next()
}

/// Get base URL without database name
fn get_base_url(url: &str) -> String {
    if let Some(pos) = url.rfind('/') {
        url[..pos].to_string()
    } else {
        url.to_string()
    }
}

/// Ensure database exists, create if not
async fn ensure_database_exists(config: &Config) -> Result<()> {
    let db_name =
        extract_db_name(&config.database.url).context("Invalid DATABASE_URL: no database name")?;

    let base_url = get_base_url(&config.database.url);

    info!("Connecting to MySQL server...");
    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&base_url)
        .await
        .context("Failed to connect to MySQL server")?;

    info!("Creating database '{}' if not exists...", db_name);
    let query = format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name);
    pool.execute(query.as_str())
        .await
        .context("Failed to create database")?;

    pool.close().await;
    info!("Database '{}' is ready", db_name);
    Ok(())
}

/// Run database migrations
pub async fn run_migrations(config: &Config) -> Result<()> {
    // First ensure database exists
    ensure_database_exists(config).await?;

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

    pool.close().await;
    info!("Database migrations completed");
    Ok(())
}

/// Seed Keycloak with default realm, admin user, and portal client
pub async fn seed_keycloak(config: &Config) -> Result<()> {
    use tracing::warn;

    info!("Initializing Keycloak seeder...");

    let seeder = KeycloakSeeder::new(&config.keycloak);

    // 🆕 Priority: Create admin client in master realm first
    info!("Seeding admin client in master realm...");
    seeder
        .seed_master_admin_client()
        .await
        .context("Failed to seed admin client in master realm")?;

    // Create realm if not exists
    info!("Ensuring realm '{}' exists...", config.keycloak.realm);
    seeder
        .ensure_realm_exists()
        .await
        .context("Failed to create realm")?;

    // Seed default admin user (non-fatal if fails)
    info!("Seeding default admin user...");
    if let Err(e) = seeder.seed_admin_user().await {
        warn!("Failed to seed admin user (non-fatal): {}", e);
    }

    // Seed portal client (OIDC client for auth9-portal)
    info!("Seeding portal client in Keycloak...");
    seeder
        .seed_portal_client()
        .await
        .context("Failed to seed portal client in Keycloak")?;

    // Seed admin client in configured realm (Confidential client for realm-level operations)
    info!("Seeding admin client in realm '{}'...", config.keycloak.realm);
    seeder
        .seed_admin_client()
        .await
        .context("Failed to seed admin client in Keycloak")?;

    // Seed portal service in database
    info!("Seeding portal service in database...");
    seed_portal_service(config)
        .await
        .context("Failed to seed portal service in database")?;

    info!("Keycloak seeding completed");
    Ok(())
}

/// Seed portal service in the database (idempotent - only creates if not exists)
async fn seed_portal_service(config: &Config) -> Result<()> {
    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    // Check if portal client already exists (via clients table)
    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients WHERE client_id = ?")
        .bind(DEFAULT_PORTAL_CLIENT_ID)
        .fetch_one(&pool)
        .await
        .context("Failed to check portal client")?;

    if exists.0 > 0 {
        info!(
            "Portal client '{}' already exists in database, skipping",
            DEFAULT_PORTAL_CLIENT_ID
        );
        pool.close().await;
        return Ok(());
    }

    // Create portal service first
    let redirect_uris = serde_json::to_string(&vec![
        "http://localhost:3000/dashboard",
        "http://localhost:3000/callback",
        "http://127.0.0.1:3000/dashboard",
        "http://127.0.0.1:3000/callback",
    ])
    .unwrap();
    let logout_uris =
        serde_json::to_string(&vec!["http://localhost:3000", "http://127.0.0.1:3000"]).unwrap();

    // Generate a UUID for the service
    let service_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        r#"
        INSERT INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
        VALUES (?, NULL, ?, 'http://localhost:3000', ?, ?, 'active', NOW(), NOW())
        "#,
    )
    .bind(&service_id)
    .bind(DEFAULT_PORTAL_NAME)
    .bind(&redirect_uris)
    .bind(&logout_uris)
    .execute(&pool)
    .await
    .context("Failed to create portal service")?;

    // Create a client for the portal service (public client - no secret)
    let client_id_record = uuid::Uuid::new_v4().to_string();
    let placeholder_hash = "public-client-no-secret";

    sqlx::query(
        r#"
        INSERT INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
        VALUES (?, ?, ?, ?, 'Portal Client', NOW())
        "#,
    )
    .bind(&client_id_record)
    .bind(&service_id)
    .bind(DEFAULT_PORTAL_CLIENT_ID)
    .bind(placeholder_hash)
    .execute(&pool)
    .await
    .context("Failed to create portal client")?;

    pool.close().await;
    info!(
        "Created portal service and client '{}' in database",
        DEFAULT_PORTAL_CLIENT_ID
    );
    Ok(())
}
