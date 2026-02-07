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

/// Build redirect URIs for database (JSON array)
fn build_db_redirect_uris(_core_public_url: Option<&str>, portal_url: Option<&str>) -> String {
    let mut uris = vec![
        "http://localhost:3000/dashboard".to_string(),
        "http://localhost:3000/auth/callback".to_string(),
        "http://127.0.0.1:3000/dashboard".to_string(),
        "http://127.0.0.1:3000/auth/callback".to_string(),
    ];

    if let Some(portal_url_str) = portal_url {
        uris.push(format!("{}/dashboard", portal_url_str));
        uris.push(format!("{}/auth/callback", portal_url_str));
    }

    serde_json::to_string(&uris).unwrap()
}

/// Build logout URIs for database (JSON array)
fn build_db_logout_uris(portal_url: Option<&str>) -> String {
    let mut uris = vec![
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
    ];

    if let Some(portal_url_str) = portal_url {
        uris.push(portal_url_str.to_string());
    }

    serde_json::to_string(&uris).unwrap()
}

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

/// Reset database by dropping and recreating it
pub async fn reset_database(config: &Config) -> Result<()> {
    let db_name =
        extract_db_name(&config.database.url).context("Invalid DATABASE_URL: no database name")?;

    let base_url = get_base_url(&config.database.url);

    info!("Connecting to MySQL server...");
    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&base_url)
        .await
        .context("Failed to connect to MySQL server")?;

    info!("Dropping database '{}'...", db_name);
    let drop_query = format!("DROP DATABASE IF EXISTS `{}`", db_name);
    pool.execute(drop_query.as_str())
        .await
        .context("Failed to drop database")?;

    info!("Creating database '{}'...", db_name);
    let create_query = format!("CREATE DATABASE `{}`", db_name);
    pool.execute(create_query.as_str())
        .await
        .context("Failed to create database")?;

    pool.close().await;
    info!("Database '{}' has been reset", db_name);
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
    info!(
        "Seeding admin client in realm '{}'...",
        config.keycloak.realm
    );
    seeder
        .seed_admin_client()
        .await
        .context("Failed to seed admin client in Keycloak")?;

    // Seed portal service in database
    info!("Seeding portal service in database...");
    seed_portal_service(config)
        .await
        .context("Failed to seed portal service in database")?;

    // Seed dev email config if in dev environment
    seed_dev_email_config(config).await?;

    info!("Keycloak seeding completed");
    Ok(())
}

/// Seed portal service in the database (idempotent - uses INSERT IGNORE to prevent duplicates)
///
/// This function is safe to call multiple times, even concurrently, due to:
/// 1. Unique constraint on clients.client_id
/// 2. Unique constraint on services(tenant_id_key, name) from migration
/// 3. Use of INSERT IGNORE to gracefully handle constraint violations
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

    // Build URIs for portal service
    let redirect_uris = build_db_redirect_uris(
        config.keycloak.core_public_url.as_deref(),
        config.keycloak.portal_url.as_deref(),
    );

    let logout_uris = build_db_logout_uris(config.keycloak.portal_url.as_deref());

    let base_url = config
        .keycloak
        .portal_url
        .as_deref()
        .unwrap_or("http://localhost:3000");

    // Generate UUIDs for service and client
    let service_id = uuid::Uuid::new_v4().to_string();
    let client_id_record = uuid::Uuid::new_v4().to_string();
    let placeholder_hash = "public-client-no-secret";

    // Use INSERT IGNORE to prevent duplicate key errors from race conditions
    // The unique constraint on services(tenant_id_key, name) prevents duplicates
    let service_result = sqlx::query(
        r#"
        INSERT IGNORE INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
        VALUES (?, NULL, ?, ?, ?, ?, 'active', NOW(), NOW())
        "#,
    )
    .bind(&service_id)
    .bind(DEFAULT_PORTAL_NAME)
    .bind(base_url)
    .bind(&redirect_uris)
    .bind(&logout_uris)
    .execute(&pool)
    .await
    .context("Failed to create portal service")?;

    // If service was not inserted (already exists), get the existing service_id
    let actual_service_id = if service_result.rows_affected() == 0 {
        info!("Portal service already exists, using existing record");
        let row: (String,) =
            sqlx::query_as("SELECT id FROM services WHERE tenant_id IS NULL AND name = ?")
                .bind(DEFAULT_PORTAL_NAME)
                .fetch_one(&pool)
                .await
                .context("Failed to get existing portal service")?;
        row.0
    } else {
        service_id
    };

    // Create client using INSERT IGNORE (client_id has UNIQUE constraint)
    let client_result = sqlx::query(
        r#"
        INSERT IGNORE INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
        VALUES (?, ?, ?, ?, 'Portal Client', NOW())
        "#,
    )
    .bind(&client_id_record)
    .bind(&actual_service_id)
    .bind(DEFAULT_PORTAL_CLIENT_ID)
    .bind(placeholder_hash)
    .execute(&pool)
    .await
    .context("Failed to create portal client")?;

    pool.close().await;

    if client_result.rows_affected() > 0 {
        info!(
            "Created portal service and client '{}' in database",
            DEFAULT_PORTAL_CLIENT_ID
        );
    } else {
        info!(
            "Portal client '{}' already exists (created by concurrent process)",
            DEFAULT_PORTAL_CLIENT_ID
        );
    }

    Ok(())
}

/// Seed dev email config in database and sync to Keycloak when DEV_SMTP_HOST is set
///
/// This configures the system to use Mailpit for email testing in dev environment.
/// Also syncs SMTP settings directly to Keycloak realm so password reset emails work.
async fn seed_dev_email_config(config: &Config) -> Result<()> {
    let smtp_host = match std::env::var("DEV_SMTP_HOST") {
        Ok(host) => host,
        Err(_) => {
            // Not in dev environment, skip
            return Ok(());
        }
    };

    info!(
        "Dev environment detected, configuring email to use {}...",
        smtp_host
    );

    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    let email_config = serde_json::json!({
        "type": "smtp",
        "host": smtp_host,
        "port": 1025,
        "from_email": "noreply@auth9.local",
        "from_name": "Auth9",
        "use_tls": false
    });

    // Try to update existing setting first
    let result = sqlx::query(
        "UPDATE system_settings SET value = ? WHERE category = 'email' AND setting_key = 'provider'"
    )
    .bind(email_config.to_string())
    .execute(&pool)
    .await
    .context("Failed to update email config")?;

    if result.rows_affected() == 0 {
        // Setting doesn't exist, insert it (id is AUTO_INCREMENT)
        sqlx::query(
            "INSERT INTO system_settings (category, setting_key, value, created_at, updated_at) VALUES ('email', 'provider', ?, NOW(), NOW())"
        )
        .bind(email_config.to_string())
        .execute(&pool)
        .await
        .context("Failed to insert email config")?;

        info!("Dev email config inserted (SMTP: {}:1025)", smtp_host);
    } else {
        info!("Dev email config updated (SMTP: {}:1025)", smtp_host);
    }

    pool.close().await;

    // Sync SMTP settings directly to Keycloak realm so password reset emails work
    sync_smtp_to_keycloak(config, &smtp_host).await?;

    Ok(())
}

/// Sync SMTP configuration directly to Keycloak realm via Admin API
///
/// This ensures Keycloak can send password reset and verification emails
/// using the dev SMTP server (Mailpit).
async fn sync_smtp_to_keycloak(config: &Config, smtp_host: &str) -> Result<()> {
    use tracing::warn;

    info!("Syncing SMTP config to Keycloak realm...");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Get admin token
    let admin_username = std::env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
    let admin_password =
        std::env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    let token_url = format!(
        "{}/realms/master/protocol/openid-connect/token",
        config.keycloak.url
    );

    let token_response = http_client
        .post(&token_url)
        .form(&[
            ("grant_type", "password"),
            ("client_id", "admin-cli"),
            ("username", &admin_username),
            ("password", &admin_password),
        ])
        .send()
        .await
        .context("Failed to get Keycloak admin token for SMTP sync")?;

    if !token_response.status().is_success() {
        warn!("Failed to get admin token for SMTP sync, skipping Keycloak email config");
        return Ok(());
    }

    let token_json: serde_json::Value = token_response.json().await?;
    let token = token_json["access_token"]
        .as_str()
        .context("Missing access_token in Keycloak response")?;

    // Update realm with SMTP settings
    let realm_url = format!(
        "{}/admin/realms/{}",
        config.keycloak.url, config.keycloak.realm
    );

    let smtp_update = serde_json::json!({
        "smtpServer": {
            "host": smtp_host,
            "port": "1025",
            "from": "noreply@auth9.local",
            "fromDisplayName": "Auth9",
            "auth": "false",
            "ssl": "false",
            "starttls": "false"
        }
    });

    let response = http_client
        .put(&realm_url)
        .bearer_auth(token)
        .json(&smtp_update)
        .send()
        .await
        .context("Failed to update Keycloak realm SMTP settings")?;

    if response.status().is_success() {
        info!("Synced SMTP config to Keycloak realm ({}:1025)", smtp_host);
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        warn!("Failed to sync SMTP to Keycloak: {} - {}", status, body);
    }

    Ok(())
}
