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

/// Default seed data constants
const DEFAULT_PLATFORM_TENANT_NAME: &str = "Auth9 Platform";
const DEFAULT_PLATFORM_TENANT_SLUG: &str = "auth9-platform";
const DEFAULT_DEMO_TENANT_NAME: &str = "Demo Organization";
const DEFAULT_DEMO_TENANT_SLUG: &str = "demo";
const SEED_ADMIN_DISPLAY_NAME: &str = "Admin User";

/// Default M2M test client for client_credentials flow testing
const DEFAULT_M2M_CLIENT_ID: &str = "auth9-m2m-test";
const DEFAULT_M2M_CLIENT_SECRET: &str = "m2m-test-secret-do-not-use-in-production";
const DEFAULT_M2M_SERVICE_NAME: &str = "Auth9 M2M Test Service";

/// Default demo client configuration
const DEFAULT_DEMO_CLIENT_ID: &str = "auth9-demo";
const DEFAULT_DEMO_SERVICE_NAME: &str = "Auth9 Demo Service";

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

    // Create realm if not exists (no settings applied yet)
    info!("Ensuring realm '{}' exists...", config.keycloak.realm);
    seeder
        .ensure_realm_exists()
        .await
        .context("Failed to create realm")?;

    // Seed default admin user BEFORE applying any realm settings.
    // Keycloak 23 rejects user creation with credentials (POST /users returns 400
    // "Password policy not met") when a password policy is active.
    info!("Seeding default admin user...");
    if let Err(e) = seeder.seed_admin_user().await {
        warn!("Failed to seed admin user (non-fatal): {}", e);
    }

    // Apply ALL realm settings (events, SSL, login theme, password policy, brute force)
    // This must happen AFTER admin user seeding due to Keycloak 23 password policy bug.
    info!("Applying realm settings...");
    seeder
        .apply_realm_settings()
        .await
        .context("Failed to apply realm settings")?;

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

    // Seed demo client in Keycloak
    info!("Seeding demo client in Keycloak...");
    seeder
        .seed_demo_client()
        .await
        .context("Failed to seed demo client in Keycloak")?;

    // Seed demo service in database
    info!("Seeding demo service in database...");
    seed_demo_service(config)
        .await
        .context("Failed to seed demo service in database")?;

    // Seed M2M test service (client_credentials flow)
    info!("Seeding M2M test service in database...");
    seed_m2m_test_service(config)
        .await
        .context("Failed to seed M2M test service in database")?;

    // Seed dev email config if in dev environment
    seed_dev_email_config(config).await?;

    // Seed initial data (tenants, admin user, associations)
    info!("Seeding initial data...");
    seed_initial_data(config)
        .await
        .context("Failed to seed initial data")?;

    info!("Keycloak seeding completed");
    Ok(())
}

/// Seed demo service in the database (idempotent)
async fn seed_demo_service(config: &Config) -> Result<()> {
    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    // Check if demo client already exists
    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients WHERE client_id = ?")
        .bind(DEFAULT_DEMO_CLIENT_ID)
        .fetch_one(&pool)
        .await
        .context("Failed to check demo client")?;

    if exists.0 > 0 {
        info!(
            "Demo client '{}' already exists in database, skipping",
            DEFAULT_DEMO_CLIENT_ID
        );
        pool.close().await;
        return Ok(());
    }

    let service_id = uuid::Uuid::new_v4().to_string();
    let client_record_id = uuid::Uuid::new_v4().to_string();
    // Public client: hash the placeholder so PasswordHash::new() can parse it
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    let placeholder_hash = Argon2::default()
        .hash_password(b"public-client-no-secret", &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash placeholder secret: {}", e))?
        .to_string();

    // Create service (tenant_id will be linked later)
    let service_result = sqlx::query(
        r#"
        INSERT IGNORE INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
        VALUES (?, NULL, ?, 'http://localhost:3002', '["http://localhost:3002/auth/callback"]', '["http://localhost:3002"]', 'active', NOW(), NOW())
        "#,
    )
    .bind(&service_id)
    .bind(DEFAULT_DEMO_SERVICE_NAME)
    .execute(&pool)
    .await
    .context("Failed to create demo service")?;

    // Get actual service ID
    let actual_service_id = if service_result.rows_affected() == 0 {
        let row: (String,) = sqlx::query_as("SELECT id FROM services WHERE name = ?")
            .bind(DEFAULT_DEMO_SERVICE_NAME)
            .fetch_one(&pool)
            .await
            .context("Failed to get existing demo service")?;
        row.0
    } else {
        service_id
    };

    // Create client
    let client_result = sqlx::query(
        r#"
        INSERT IGNORE INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
        VALUES (?, ?, ?, ?, 'Auth9 Demo Client', NOW())
        "#,
    )
    .bind(&client_record_id)
    .bind(&actual_service_id)
    .bind(DEFAULT_DEMO_CLIENT_ID)
    .bind(placeholder_hash)
    .execute(&pool)
    .await
    .context("Failed to create demo client")?;

    pool.close().await;

    if client_result.rows_affected() > 0 {
        info!(
            "Created demo service and client '{}' in database",
            DEFAULT_DEMO_CLIENT_ID
        );
    } else {
        info!("Demo client '{}' already exists", DEFAULT_DEMO_CLIENT_ID);
    }

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
    // Public client: hash the placeholder so PasswordHash::new() can parse it
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    let placeholder_hash = Argon2::default()
        .hash_password(b"public-client-no-secret", &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash placeholder secret: {}", e))?
        .to_string();

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
        let row: (String,) = sqlx::query_as("SELECT id FROM services WHERE name = ?")
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

/// Seed M2M test service with a confidential client for client_credentials flow testing
///
/// Creates a service with a known client_id and client_secret so that
/// client_credentials grant can be tested in QA/dev environments.
/// Idempotent via INSERT IGNORE on unique constraints.
async fn seed_m2m_test_service(config: &Config) -> Result<()> {
    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    // Check if M2M client already exists
    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM clients WHERE client_id = ?")
        .bind(DEFAULT_M2M_CLIENT_ID)
        .fetch_one(&pool)
        .await
        .context("Failed to check M2M client")?;

    if exists.0 > 0 {
        info!(
            "M2M test client '{}' already exists in database, skipping",
            DEFAULT_M2M_CLIENT_ID
        );
        pool.close().await;
        return Ok(());
    }

    // Hash the known test secret with Argon2
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let secret_hash = argon2
        .hash_password(DEFAULT_M2M_CLIENT_SECRET.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash M2M secret: {}", e))?
        .to_string();

    let service_id = uuid::Uuid::new_v4().to_string();
    let client_record_id = uuid::Uuid::new_v4().to_string();

    // Create service (tenant_id will be linked later by seed_initial_data)
    sqlx::query(
        r#"INSERT IGNORE INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
        VALUES (?, NULL, ?, 'http://localhost:8080', '[]', '[]', 'active', NOW(), NOW())"#,
    )
    .bind(&service_id)
    .bind(DEFAULT_M2M_SERVICE_NAME)
    .execute(&pool)
    .await
    .context("Failed to create M2M test service")?;

    // Get actual service ID (may have been created before via unique constraint)
    let actual_service_id: String =
        match sqlx::query_as::<_, (String,)>("SELECT id FROM services WHERE name = ?")
            .bind(DEFAULT_M2M_SERVICE_NAME)
            .fetch_optional(&pool)
            .await
            .context("Failed to query M2M service")?
        {
            Some((id,)) => id,
            None => service_id,
        };

    // Create confidential client with known secret
    let client_result = sqlx::query(
        r#"INSERT IGNORE INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
        VALUES (?, ?, ?, ?, 'M2M Test Client', NOW())"#,
    )
    .bind(&client_record_id)
    .bind(&actual_service_id)
    .bind(DEFAULT_M2M_CLIENT_ID)
    .bind(&secret_hash)
    .execute(&pool)
    .await
    .context("Failed to create M2M test client")?;

    pool.close().await;

    if client_result.rows_affected() > 0 {
        info!(
            "Created M2M test service and client '{}' (secret: '{}') in database",
            DEFAULT_M2M_CLIENT_ID, DEFAULT_M2M_CLIENT_SECRET
        );
    } else {
        info!(
            "M2M test client '{}' already exists (created by concurrent process)",
            DEFAULT_M2M_CLIENT_ID
        );
    }

    Ok(())
}

/// Seed initial data: platform tenant, demo tenant, admin user, and associations
///
/// This function is safe to call multiple times (idempotent):
/// - Tenants: INSERT IGNORE (slug UNIQUE constraint)
/// - Users: INSERT ... ON DUPLICATE KEY UPDATE keycloak_id (handles Keycloak reset)
/// - Tenant users: INSERT IGNORE (tenant_id, user_id UNIQUE constraint)
/// - Tenant services: INSERT ... ON DUPLICATE KEY UPDATE enabled = TRUE
async fn seed_initial_data(config: &Config) -> Result<()> {
    use tracing::warn;

    // 1. Query Keycloak for admin user's keycloak_id and email
    let seeder = KeycloakSeeder::new(&config.keycloak);
    let (keycloak_id, admin_email) = match seeder.get_admin_user_keycloak_id().await? {
        Some((id, email)) => (id, email),
        None => {
            warn!("Admin user not found in Keycloak, skipping initial data seed");
            return Ok(());
        }
    };

    info!(
        "Found admin user in Keycloak: keycloak_id={}, email={}",
        keycloak_id, admin_email
    );

    let pool: Pool<MySql> = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database.url)
        .await
        .context("Failed to connect to database")?;

    let default_settings = serde_json::json!({"require_mfa": false, "allowed_auth_methods": [], "session_timeout_secs": 3600, "branding": {}});
    let settings_json = default_settings.to_string();

    // 2. INSERT IGNORE two tenants
    let platform_tenant_id = uuid::Uuid::new_v4().to_string();
    let demo_tenant_id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT IGNORE INTO tenants (id, name, slug, settings, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, 'active', NOW(), NOW())"#,
    )
    .bind(&platform_tenant_id)
    .bind(DEFAULT_PLATFORM_TENANT_NAME)
    .bind(DEFAULT_PLATFORM_TENANT_SLUG)
    .bind(&settings_json)
    .execute(&pool)
    .await
    .context("Failed to seed platform tenant")?;

    sqlx::query(
        r#"INSERT IGNORE INTO tenants (id, name, slug, settings, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, 'active', NOW(), NOW())"#,
    )
    .bind(&demo_tenant_id)
    .bind(DEFAULT_DEMO_TENANT_NAME)
    .bind(DEFAULT_DEMO_TENANT_SLUG)
    .bind(&settings_json)
    .execute(&pool)
    .await
    .context("Failed to seed demo tenant")?;

    // 3. Upsert admin user (handles Keycloak reset where keycloak_id changes)
    // First try to find existing admin by display_name (stable across resets)
    let existing_admin: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE display_name = ? LIMIT 1")
            .bind(SEED_ADMIN_DISPLAY_NAME)
            .fetch_optional(&pool)
            .await
            .context("Failed to check existing admin user")?;

    if let Some((existing_id,)) = existing_admin {
        // Update existing admin user's keycloak_id and email
        sqlx::query(
            r#"UPDATE users SET keycloak_id = ?, email = ?, updated_at = NOW()
            WHERE id = ?"#,
        )
        .bind(&keycloak_id)
        .bind(&admin_email)
        .bind(&existing_id)
        .execute(&pool)
        .await
        .context("Failed to update admin user keycloak_id")?;

        info!("Updated existing admin user keycloak_id to {}", keycloak_id);
    } else {
        // Insert new admin user
        let admin_user_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO users (id, keycloak_id, email, display_name, mfa_enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, FALSE, NOW(), NOW())
            ON DUPLICATE KEY UPDATE keycloak_id = VALUES(keycloak_id), email = VALUES(email), mfa_enabled = FALSE"#,
        )
        .bind(&admin_user_id)
        .bind(&keycloak_id)
        .bind(&admin_email)
        .bind(SEED_ADMIN_DISPLAY_NAME)
        .execute(&pool)
        .await
        .context("Failed to seed admin user")?;

        info!("Created new admin user with keycloak_id {}", keycloak_id);
    }

    // 4. SELECT actual IDs (handles case where records already existed)
    let (actual_platform_id,): (String,) = sqlx::query_as("SELECT id FROM tenants WHERE slug = ?")
        .bind(DEFAULT_PLATFORM_TENANT_SLUG)
        .fetch_one(&pool)
        .await
        .context("Failed to get platform tenant ID")?;

    let (actual_demo_id,): (String,) = sqlx::query_as("SELECT id FROM tenants WHERE slug = ?")
        .bind(DEFAULT_DEMO_TENANT_SLUG)
        .fetch_one(&pool)
        .await
        .context("Failed to get demo tenant ID")?;

    let (actual_user_id,): (String,) = sqlx::query_as("SELECT id FROM users WHERE keycloak_id = ?")
        .bind(&keycloak_id)
        .fetch_one(&pool)
        .await
        .context("Failed to get admin user ID")?;

    // 5. INSERT IGNORE tenant_users (admin → both tenants)
    sqlx::query(
        r#"INSERT IGNORE INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)
        VALUES (?, ?, ?, 'admin', NOW())"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&actual_platform_id)
    .bind(&actual_user_id)
    .execute(&pool)
    .await
    .context("Failed to seed platform tenant_user")?;

    sqlx::query(
        r#"INSERT IGNORE INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)
        VALUES (?, ?, ?, 'admin', NOW())"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&actual_demo_id)
    .bind(&actual_user_id)
    .execute(&pool)
    .await
    .context("Failed to seed demo tenant_user")?;

    // 6. Link Portal service to Auth9 Platform tenant and seed tenant_services
    let portal_service: Option<(String,)> =
        sqlx::query_as("SELECT id FROM services WHERE name = ?")
            .bind(DEFAULT_PORTAL_NAME)
            .fetch_optional(&pool)
            .await
            .context("Failed to query portal service")?;

    if let Some((service_id,)) = portal_service {
        // Assign Portal service to Auth9 Platform tenant (the primary tenant for Portal)
        sqlx::query("UPDATE services SET tenant_id = ? WHERE id = ? AND tenant_id IS NULL")
            .bind(&actual_platform_id)
            .bind(&service_id)
            .execute(&pool)
            .await
            .context("Failed to assign portal service to platform tenant")?;
        sqlx::query(
            r#"INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
            VALUES (?, ?, TRUE, NOW(), NOW())
            ON DUPLICATE KEY UPDATE enabled = TRUE"#,
        )
        .bind(&actual_platform_id)
        .bind(&service_id)
        .execute(&pool)
        .await
        .context("Failed to seed platform tenant_service")?;

        sqlx::query(
            r#"INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
            VALUES (?, ?, TRUE, NOW(), NOW())
            ON DUPLICATE KEY UPDATE enabled = TRUE"#,
        )
        .bind(&actual_demo_id)
        .bind(&service_id)
        .execute(&pool)
        .await
        .context("Failed to seed demo tenant_service")?;

        info!("Seeded tenant_services for both tenants → Auth9 Admin Portal");

        // 7. Seed RBAC: default roles, permissions, and assignments for admin user
        seed_rbac_for_service(
            &pool,
            &service_id,
            &actual_platform_id,
            &actual_demo_id,
            &actual_user_id,
        )
        .await?;
    } else {
        warn!("Auth9 Admin Portal service not found in database, skipping tenant_services seed");
    }

    // 8. Link M2M test service to Demo tenant
    let m2m_service: Option<(String,)> = sqlx::query_as("SELECT id FROM services WHERE name = ?")
        .bind(DEFAULT_M2M_SERVICE_NAME)
        .fetch_optional(&pool)
        .await
        .context("Failed to query M2M service")?;

    if let Some((m2m_service_id,)) = m2m_service {
        sqlx::query("UPDATE services SET tenant_id = ? WHERE id = ? AND tenant_id IS NULL")
            .bind(&actual_demo_id)
            .bind(&m2m_service_id)
            .execute(&pool)
            .await
            .context("Failed to assign M2M service to demo tenant")?;

        sqlx::query(
            r#"INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
            VALUES (?, ?, TRUE, NOW(), NOW())
            ON DUPLICATE KEY UPDATE enabled = TRUE"#,
        )
        .bind(&actual_demo_id)
        .bind(&m2m_service_id)
        .execute(&pool)
        .await
        .context("Failed to seed demo tenant_service for M2M")?;

        info!("Seeded tenant_service for Demo tenant → M2M Test Service");
    }

    // 9. Link Demo service to Demo tenant
    let demo_service: Option<(String,)> = sqlx::query_as("SELECT id FROM services WHERE name = ?")
        .bind(DEFAULT_DEMO_SERVICE_NAME)
        .fetch_optional(&pool)
        .await
        .context("Failed to query Demo service")?;

    if let Some((demo_service_id,)) = demo_service {
        // Assign Demo service to Demo tenant
        sqlx::query("UPDATE services SET tenant_id = ? WHERE id = ? AND tenant_id IS NULL")
            .bind(&actual_demo_id)
            .bind(&demo_service_id)
            .execute(&pool)
            .await
            .context("Failed to assign Demo service to demo tenant")?;

        sqlx::query(
            r#"INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
            VALUES (?, ?, TRUE, NOW(), NOW())
            ON DUPLICATE KEY UPDATE enabled = TRUE"#,
        )
        .bind(&actual_demo_id)
        .bind(&demo_service_id)
        .execute(&pool)
        .await
        .context("Failed to seed demo tenant_service for Demo Service")?;

        info!("Seeded tenant_service for Demo tenant → Auth9 Demo Service");
    }

    pool.close().await;

    info!(
        "Initial data seeded: tenants=[{}, {}], admin_user={}, email={}",
        DEFAULT_PLATFORM_TENANT_SLUG, DEFAULT_DEMO_TENANT_SLUG, keycloak_id, admin_email
    );

    Ok(())
}

/// Seed RBAC data: admin role with full permission, assigned to admin user in both tenants
async fn seed_rbac_for_service(
    pool: &Pool<MySql>,
    service_id: &str,
    platform_tenant_id: &str,
    demo_tenant_id: &str,
    admin_user_id: &str,
) -> Result<()> {
    // Create "admin:full" permission (idempotent via INSERT IGNORE on unique key)
    let permission_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT IGNORE INTO permissions (id, service_id, code, name, description)
        VALUES (?, ?, 'admin:full', 'Full Admin Access', 'Full administrative access to all resources')"#,
    )
    .bind(&permission_id)
    .bind(service_id)
    .execute(pool)
    .await
    .context("Failed to seed admin permission")?;

    // Get actual permission ID (may already exist)
    let (actual_permission_id,): (String,) =
        sqlx::query_as("SELECT id FROM permissions WHERE service_id = ? AND code = 'admin:full'")
            .bind(service_id)
            .fetch_one(pool)
            .await
            .context("Failed to get admin permission ID")?;

    // Create "admin" role (idempotent via INSERT IGNORE on unique key)
    let role_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT IGNORE INTO roles (id, service_id, name, description, created_at, updated_at)
        VALUES (?, ?, 'admin', 'Administrator role with full access', NOW(), NOW())"#,
    )
    .bind(&role_id)
    .bind(service_id)
    .execute(pool)
    .await
    .context("Failed to seed admin role")?;

    // Get actual role ID (may already exist)
    let (actual_role_id,): (String,) =
        sqlx::query_as("SELECT id FROM roles WHERE service_id = ? AND name = 'admin'")
            .bind(service_id)
            .fetch_one(pool)
            .await
            .context("Failed to get admin role ID")?;

    // Link permission to role (idempotent via INSERT IGNORE on composite PK)
    sqlx::query("INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
        .bind(&actual_role_id)
        .bind(&actual_permission_id)
        .execute(pool)
        .await
        .context("Failed to seed role_permission")?;

    // Assign admin role to admin user in both tenants
    for tenant_id in [platform_tenant_id, demo_tenant_id] {
        // Get tenant_user_id for this user+tenant pair
        let tenant_user: Option<(String,)> =
            sqlx::query_as("SELECT id FROM tenant_users WHERE tenant_id = ? AND user_id = ?")
                .bind(tenant_id)
                .bind(admin_user_id)
                .fetch_optional(pool)
                .await
                .context("Failed to query tenant_user")?;

        if let Some((tenant_user_id,)) = tenant_user {
            sqlx::query(
                r#"INSERT IGNORE INTO user_tenant_roles (id, tenant_user_id, role_id, granted_at)
                VALUES (?, ?, ?, NOW())"#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&tenant_user_id)
            .bind(&actual_role_id)
            .execute(pool)
            .await
            .context("Failed to seed user_tenant_role")?;
        }
    }

    info!("Seeded RBAC: admin role with admin:full permission assigned to admin user");
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

    // Upsert dev email config: insert if not exists, update if current config is "none"
    let existing: Option<(serde_json::Value,)> = sqlx::query_as(
        "SELECT value FROM system_settings WHERE category = 'email' AND setting_key = 'provider'",
    )
    .fetch_optional(&pool)
    .await
    .context("Failed to check existing email config")?;

    match existing {
        Some((value,)) if value.get("type").and_then(|t| t.as_str()) != Some("none") => {
            info!(
                "Email config already configured (not 'none'), skipping dev seed (SMTP: {}:1025)",
                smtp_host
            );
        }
        Some(_) => {
            // Existing config is "none" (from migration default), update it
            sqlx::query(
                "UPDATE system_settings SET value = ?, updated_at = NOW() WHERE category = 'email' AND setting_key = 'provider'"
            )
            .bind(email_config.to_string())
            .execute(&pool)
            .await
            .context("Failed to update email config")?;

            info!(
                "Dev email config updated from 'none' to SMTP ({}:1025)",
                smtp_host
            );
        }
        None => {
            sqlx::query(
                "INSERT INTO system_settings (category, setting_key, value, created_at, updated_at) VALUES ('email', 'provider', ?, NOW(), NOW())"
            )
            .bind(email_config.to_string())
            .execute(&pool)
            .await
            .context("Failed to insert email config")?;

            info!("Dev email config inserted (SMTP: {}:1025)", smtp_host);
        }
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
