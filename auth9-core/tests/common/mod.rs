//! Common test utilities
//!
//! Lightweight test helpers without testcontainers.
//! For unit tests, use mock repositories from the service layer.

use auth9_core::config::{Config, DatabaseConfig, JwtConfig, KeycloakConfig, RedisConfig};

/// Test configuration (no real connections needed)
pub fn test_config() -> Config {
    Config {
        http_host: "127.0.0.1".to_string(),
        http_port: 0,
        grpc_host: "127.0.0.1".to_string(),
        grpc_port: 0,
        database: DatabaseConfig {
            url: "mysql://test@localhost/test".to_string(),
            max_connections: 1,
            min_connections: 1,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_string(),
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
            core_public_url: None,
            portal_url: None,
        },
    }
}

/// Test JWT configuration helper
pub fn test_jwt_config() -> JwtConfig {
    JwtConfig {
        secret: "test-secret-key-for-testing-purposes-only".to_string(),
        issuer: "https://auth9.test".to_string(),
        access_token_ttl_secs: 3600,
        refresh_token_ttl_secs: 604800,
        private_key_pem: None,
        public_key_pem: None,
    }
}
