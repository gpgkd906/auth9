//! Common test utilities

use auth9_core::config::{Config, DatabaseConfig, JwtConfig, KeycloakConfig, RedisConfig};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use std::net::SocketAddr;
use std::sync::Once;
use testcontainers::clients;
use testcontainers_modules::mysql::Mysql;
use tokio::sync::OnceCell;

/// Ensure .env file is loaded once
static ENV_INIT: Once = Once::new();

fn init_env() {
    ENV_INIT.call_once(|| {
        // Load .env file if it exists (for local development)
        let _ = dotenvy::dotenv();
    });
}

/// Global test container port
static MYSQL_PORT: OnceCell<u16> = OnceCell::const_new();

/// Get port of the shared MySQL test container (starts it if needed)
async fn get_mysql_port() -> u16 {
    // Ensure environment is initialized
    init_env();
    
    *MYSQL_PORT
        .get_or_init(|| async {
            // Check if DATABASE_URL is already set
            if let Ok(url) = std::env::var("DATABASE_URL") {
                eprintln!("Using existing DATABASE_URL: {}", url);
                // Try to extract port from URL or just verify connection
                // Here we assume if DATABASE_URL is set, we don't need the container port
                // But for this logic, we return 0 to indicate "use env var"
                return 0;
            }

            eprintln!("Starting MySQL test container...");

            // Use spawn_blocking to run synchronous testcontainers code
            let port = tokio::task::spawn_blocking(|| {
                let docker = clients::Cli::default();
                // Leak the docker client to keep it alive for the duration of tests
                let docker = Box::leak(Box::new(docker));
                
                let container = docker.run(Mysql::default());
                let port = container.get_host_port_ipv4(3306);
                
                eprintln!("MySQL container started on port {}", port);
                
                // Leak the container to prevent it from being dropped (and killing the container)
                // This keeps the DB alive for all tests
                Box::leak(Box::new(container));
                
                port
            })
            .await
            .expect("Failed to start MySQL container");

            port
        })
        .await
}

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

/// Get a database pool connected to the testcontainer MySQL
/// This will automatically start a MySQL container if needed
pub async fn get_test_pool() -> Result<MySqlPool, sqlx::Error> {
    // Ensure environment is initialized
    init_env();
    
    // First check if DATABASE_URL is set
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return MySqlPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await;
    }

    // Otherwise, use testcontainers
    let port = get_mysql_port().await;
    
    // If port is 0, it means we somehow fell back to env var logic inside init
    // but here we know env var wasn't set. This shouldn't happen with current logic.
    let url = format!("mysql://root@127.0.0.1:{}/test", port);

    MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
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
    sqlx::query("DELETE FROM clients").execute(pool).await?;
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
