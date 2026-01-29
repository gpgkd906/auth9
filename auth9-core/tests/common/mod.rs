//! Common test utilities

use auth9_core::config::{Config, DatabaseConfig, JwtConfig, KeycloakConfig, RedisConfig};
use sqlx::MySqlPool;
use std::net::SocketAddr;

#[allow(dead_code)]
pub struct TestApp {
    pub addr: SocketAddr,
    pub db_pool: MySqlPool,
    pub config: Config,
}

#[allow(dead_code)]
impl TestApp {
    /// Create a test configuration
    pub fn test_config() -> Config {
        Config {
            http_host: "127.0.0.1".to_string(),
            http_port: 0, // Random port
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 0, // Random port
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                    "mysql://root:password@localhost:3306/auth9_test".to_string()
                }),
                max_connections: 5,
                min_connections: 1,
            },
            redis: RedisConfig {
                url: std::env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            },
            jwt: JwtConfig {
                secret: "test-secret-key-for-testing-purposes".to_string(),
                issuer: "https://auth9.test".to_string(),
                access_token_ttl_secs: 3600,
                refresh_token_ttl_secs: 604800,
                private_key_pem: None,
                public_key_pem: None,
            },
            keycloak: KeycloakConfig {
                url: "http://localhost:8081".to_string(),
                public_url: "http://localhost:8081".to_string(),
                realm: "test".to_string(),
                admin_client_id: "admin-cli".to_string(),
                admin_client_secret: "secret".to_string(),
                ssl_required: "none".to_string(),
            },
        }
    }

    /// Create HTTP client for testing
    pub fn http_client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    }

    /// Get API base URL
    pub fn api_url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

/// Setup test database (run migrations)
pub async fn setup_database(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    // Run migrations
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Clean up test data
pub async fn cleanup_database(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    // Delete all test data in reverse order of foreign key dependencies
    sqlx::query("DELETE FROM user_tenant_roles")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM role_permissions")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM roles").execute(pool).await?;
    sqlx::query("DELETE FROM permissions").execute(pool).await?;
    sqlx::query("DELETE FROM tenant_users")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM services").execute(pool).await?;
    sqlx::query("DELETE FROM users").execute(pool).await?;
    sqlx::query("DELETE FROM tenants").execute(pool).await?;
    sqlx::query("DELETE FROM audit_logs").execute(pool).await?;
    Ok(())
}
