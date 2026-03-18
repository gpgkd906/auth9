use anyhow::Result;
use sqlx::{mysql::MySqlPoolOptions, Executor, MySqlPool};

/// Embedded migration SQL files (CREATE TABLE IF NOT EXISTS — safe to re-run).
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/20260318000001_create_credentials.sql"),
    include_str!("../migrations/20260318000002_create_user_verification_status.sql"),
    include_str!("../migrations/20260318000003_create_pending_actions.sql"),
    include_str!("../migrations/20260318000004_create_email_verification_tokens.sql"),
];

pub async fn connect(database_url: &str) -> Result<MySqlPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Run idempotent CREATE TABLE IF NOT EXISTS migrations on startup.
    // Uses raw SQL to avoid conflicts with auth9-core's _sqlx_migrations table.
    for sql in MIGRATIONS {
        pool.execute(*sql).await?;
    }
    tracing::info!("auth9-oidc database tables ensured");

    Ok(pool)
}
