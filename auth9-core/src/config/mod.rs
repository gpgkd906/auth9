//! Configuration management for Auth9 Core

use anyhow::{Context, Result};
use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP server host
    pub http_host: String,
    /// HTTP server port
    pub http_port: u16,
    /// gRPC server host
    pub grpc_host: String,
    /// gRPC server port
    pub grpc_port: u16,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Redis configuration
    pub redis: RedisConfig,
    /// JWT configuration
    pub jwt: JwtConfig,
    /// Keycloak configuration
    pub keycloak: KeycloakConfig,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub private_key_pem: Option<String>,
    pub public_key_pem: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeycloakConfig {
    /// Internal URL for server-to-server communication (e.g., http://keycloak:8080)
    pub url: String,
    /// Public URL for browser redirects (e.g., http://localhost:8081)
    pub public_url: String,
    pub realm: String,
    pub admin_client_id: String,
    pub admin_client_secret: String,
    /// SSL requirement for the realm: "none", "external", or "all"
    /// - "none": HTTP allowed (local dev only)
    /// - "external": HTTPS required for external requests (recommended for production)
    /// - "all": HTTPS required for all requests
    pub ssl_required: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            http_host: env::var("HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid HTTP_PORT")?,
            grpc_host: env::var("GRPC_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: env::var("GRPC_PORT")
                .unwrap_or_else(|_| "50051".to_string())
                .parse()
                .context("Invalid GRPC_PORT")?,
            database: DatabaseConfig {
                url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2),
            },
            redis: RedisConfig {
                url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET").context("JWT_SECRET is required")?,
                issuer: env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "https://auth9.example.com".to_string()),
                access_token_ttl_secs: env::var("JWT_ACCESS_TOKEN_TTL_SECS")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .unwrap_or(3600),
                refresh_token_ttl_secs: env::var("JWT_REFRESH_TOKEN_TTL_SECS")
                    .unwrap_or_else(|_| "604800".to_string())
                    .parse()
                    .unwrap_or(604800),
                private_key_pem: env::var("JWT_PRIVATE_KEY")
                    .ok()
                    .map(|value| value.replace("\\n", "\n")),
                public_key_pem: env::var("JWT_PUBLIC_KEY")
                    .ok()
                    .map(|value| value.replace("\\n", "\n")),
            },
            keycloak: {
                let url = env::var("KEYCLOAK_URL")
                    .unwrap_or_else(|_| "http://localhost:8081".to_string());
                let public_url = env::var("KEYCLOAK_PUBLIC_URL").unwrap_or_else(|_| url.clone());
                KeycloakConfig {
                    url,
                    public_url,
                    realm: env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "auth9".to_string()),
                    admin_client_id: env::var("KEYCLOAK_ADMIN_CLIENT_ID")
                        .unwrap_or_else(|_| "admin-cli".to_string()),
                    admin_client_secret: env::var("KEYCLOAK_ADMIN_CLIENT_SECRET")
                        .unwrap_or_else(|_| String::new()),
                    // Default to "external" for production safety
                    ssl_required: env::var("KEYCLOAK_SSL_REQUIRED")
                        .unwrap_or_else(|_| "external".to_string()),
                }
            },
        })
    }

    /// Get HTTP server address
    pub fn http_addr(&self) -> String {
        format!("{}:{}", self.http_host, self.http_port)
    }

    /// Get gRPC server address
    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_addresses() {
        let config = Config {
            http_host: "127.0.0.1".to_string(),
            http_port: 8080,
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 50051,
            database: DatabaseConfig {
                url: "mysql://localhost/test".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret".to_string(),
                issuer: "test".to_string(),
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
                ssl_required: "external".to_string(),
            },
        };

        assert_eq!(config.http_addr(), "127.0.0.1:8080");
        assert_eq!(config.grpc_addr(), "127.0.0.1:50051");
    }
}
