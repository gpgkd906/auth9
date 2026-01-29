//! Common test utilities

use auth9_core::config::{Config, DatabaseConfig, JwtConfig, KeycloakConfig, RedisConfig};
use auth9_core::server::{build_router, AppState};
use auth9_core::repository::{
    audit::AuditRepositoryImpl, rbac::RbacRepositoryImpl, service::ServiceRepositoryImpl,
    tenant::TenantRepositoryImpl, user::UserRepositoryImpl,
};
use auth9_core::service::{ClientService, RbacService, TenantService, UserService};
use auth9_core::cache::CacheManager;
use auth9_core::jwt::JwtManager;
use auth9_core::keycloak::KeycloakClient;

use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use std::net::SocketAddr;
use std::sync::{Arc, Once};
use testcontainers::clients;
use testcontainers_modules::mysql::Mysql;
use testcontainers_modules::redis::Redis;
use tokio::sync::OnceCell;
use tokio::net::TcpListener;
use wiremock::MockServer;

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
                if !url.trim().is_empty() {
                    eprintln!("Using existing DATABASE_URL: {}", url);
                    return 0;
                }
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
                
                // Leak the container to prevent it from being dropped
                Box::leak(Box::new(container));
                
                port
            })
            .await
            .expect("Failed to start MySQL container");

            port
        })
        .await
}

/// Global test container port for Redis
static REDIS_PORT: OnceCell<u16> = OnceCell::const_new();

async fn get_redis_port() -> u16 {
    // Ensure environment is initialized
    init_env();

    *REDIS_PORT
        .get_or_init(|| async {
            if let Ok(url) = std::env::var("REDIS_URL") {
                if !url.trim().is_empty() {
                    return 0;
                }
            }

            eprintln!("Starting Redis test container...");
            let port = tokio::task::spawn_blocking(|| {
                let docker = clients::Cli::default();
                let docker = Box::leak(Box::new(docker));
                let container = docker.run(Redis::default());
                let port = container.get_host_port_ipv4(6379);
                Box::leak(Box::new(container));
                port
            })
            .await
            .expect("Failed to start Redis container");

            port
        })
        .await
}

#[allow(dead_code)]
pub struct TestApp {
    pub addr: SocketAddr,
    pub db_pool: MySqlPool,
    pub config: Config,
    pub mock_server: MockServer, // Keep mock server alive
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
        //    .redirect(reqwest::redirect::Policy::none()) // Don't follow redirects automatically for auth tests
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    }

    pub fn api_url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub async fn spawn() -> Self {
        init_env();
        // Start mock server for Keycloak
        let mock_server = MockServer::start().await;

        let mut config = Self::test_config();
        
        // Update Keycloak config to point to mock server
        config.keycloak.url = mock_server.uri();
        config.keycloak.public_url = mock_server.uri();

        // Setup DB
        let db_pool = get_test_pool().await.expect("Failed to get DB pool");
        setup_database(&db_pool).await.expect("Failed to setup DB");

        // Setup Redis
        let redis_port = get_redis_port().await;
        if redis_port != 0 {
            config.redis.url = format!("redis://127.0.0.1:{}", redis_port);
        }

        // Initialize components
        let cache_manager = CacheManager::new(&config.redis).await.expect("Failed to create CacheManager");
        let tenant_repo = Arc::new(TenantRepositoryImpl::new(db_pool.clone()));
        let user_repo = Arc::new(UserRepositoryImpl::new(db_pool.clone()));
        let service_repo = Arc::new(ServiceRepositoryImpl::new(db_pool.clone()));
        let rbac_repo = Arc::new(RbacRepositoryImpl::new(db_pool.clone()));
        let audit_repo = Arc::new(AuditRepositoryImpl::new(db_pool.clone()));

        let tenant_service = Arc::new(TenantService::new(tenant_repo.clone(), Some(cache_manager.clone())));
        let user_service = Arc::new(UserService::new(user_repo.clone()));
        let client_service = Arc::new(ClientService::new(service_repo.clone(), Some(cache_manager.clone())));
        let rbac_service = Arc::new(RbacService::new(rbac_repo.clone(), Some(cache_manager.clone())));
        
        // Mock Keycloak admin authentication response within the app initialization?
        // The KeycloakClient constructor might try to authenticate.
        // auth9_core::keycloak::KeycloakClient::new just stores config, doesn't connect immediately.
        
        let jwt_manager = JwtManager::new(config.jwt.clone());
        let keycloak_client = KeycloakClient::new(config.keycloak.clone());

        let state = AppState {
            config: Arc::new(config.clone()),
            db_pool: db_pool.clone(),
            tenant_service,
            user_service,
            client_service,
            rbac_service,
            audit_repo,
            jwt_manager,
            cache_manager,
            keycloak_client,
        };

        let app = build_router(state);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind random port");
        let addr = listener.local_addr().expect("Failed to get local address");

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        TestApp {
            addr,
            db_pool,
            config,
            mock_server,
        }
    }
}

/// Get a database pool connected to the testcontainer MySQL
/// This will automatically start a MySQL container if needed
pub async fn get_test_pool() -> Result<MySqlPool, sqlx::Error> {
    // Ensure environment is initialized
    init_env();
    
    // First check if DATABASE_URL is set
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return MySqlPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await;
        }
    }

    // Otherwise, use testcontainers
    let port = get_mysql_port().await;
    
    // If port is 0, it means we somehow fell back to env var logic inside init
    // but here we know env var wasn't set.
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
