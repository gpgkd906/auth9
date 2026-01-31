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
    /// Public URL for Auth9 Core API (e.g., https://api.auth9.example.com)
    pub core_public_url: Option<String>,
    /// Public URL for Auth9 Portal (e.g., https://auth9.example.com)
    pub portal_url: Option<String>,
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

                // Read production URLs for portal client redirect URIs
                let core_public_url = env::var("AUTH9_CORE_PUBLIC_URL").ok();
                let portal_url = env::var("AUTH9_PORTAL_URL").ok();

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
                    core_public_url,
                    portal_url,
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

    fn test_config() -> Config {
        Config {
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
                core_public_url: None,
                portal_url: None,
            },
        }
    }

    #[test]
    fn test_config_addresses() {
        let config = test_config();

        assert_eq!(config.http_addr(), "127.0.0.1:8080");
        assert_eq!(config.grpc_addr(), "127.0.0.1:50051");
    }

    #[test]
    fn test_config_http_addr_ipv6() {
        let mut config = test_config();
        config.http_host = "::1".to_string();
        config.http_port = 3000;

        assert_eq!(config.http_addr(), "::1:3000");
    }

    #[test]
    fn test_config_grpc_addr_custom() {
        let mut config = test_config();
        config.grpc_host = "0.0.0.0".to_string();
        config.grpc_port = 9000;

        assert_eq!(config.grpc_addr(), "0.0.0.0:9000");
    }

    #[test]
    fn test_config_clone() {
        let config1 = test_config();
        let config2 = config1.clone();

        assert_eq!(config1.http_host, config2.http_host);
        assert_eq!(config1.http_port, config2.http_port);
        assert_eq!(config1.database.url, config2.database.url);
    }

    #[test]
    fn test_config_debug() {
        let config = test_config();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("http_host"));
        assert!(debug_str.contains("127.0.0.1"));
    }

    #[test]
    fn test_database_config_clone() {
        let db = DatabaseConfig {
            url: "mysql://user:pass@host/db".to_string(),
            max_connections: 20,
            min_connections: 5,
        };
        let db2 = db.clone();

        assert_eq!(db.url, db2.url);
        assert_eq!(db.max_connections, db2.max_connections);
        assert_eq!(db.min_connections, db2.min_connections);
    }

    #[test]
    fn test_database_config_debug() {
        let db = DatabaseConfig {
            url: "mysql://localhost/test".to_string(),
            max_connections: 10,
            min_connections: 2,
        };
        let debug_str = format!("{:?}", db);

        assert!(debug_str.contains("DatabaseConfig"));
        assert!(debug_str.contains("max_connections"));
    }

    #[test]
    fn test_redis_config_clone() {
        let redis = RedisConfig {
            url: "redis://localhost:6379".to_string(),
        };
        let redis2 = redis.clone();

        assert_eq!(redis.url, redis2.url);
    }

    #[test]
    fn test_redis_config_debug() {
        let redis = RedisConfig {
            url: "redis://localhost:6379".to_string(),
        };
        let debug_str = format!("{:?}", redis);

        assert!(debug_str.contains("RedisConfig"));
        assert!(debug_str.contains("redis://localhost:6379"));
    }

    #[test]
    fn test_jwt_config_with_rsa_keys() {
        let jwt = JwtConfig {
            secret: "fallback-secret".to_string(),
            issuer: "https://auth9.example.com".to_string(),
            access_token_ttl_secs: 1800,
            refresh_token_ttl_secs: 86400,
            private_key_pem: Some(
                "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----".to_string(),
            ),
            public_key_pem: Some(
                "-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----".to_string(),
            ),
        };

        assert!(jwt.private_key_pem.is_some());
        assert!(jwt.public_key_pem.is_some());
    }

    #[test]
    fn test_jwt_config_clone() {
        let jwt = JwtConfig {
            secret: "secret".to_string(),
            issuer: "issuer".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 604800,
            private_key_pem: None,
            public_key_pem: None,
        };
        let jwt2 = jwt.clone();

        assert_eq!(jwt.secret, jwt2.secret);
        assert_eq!(jwt.issuer, jwt2.issuer);
    }

    #[test]
    fn test_jwt_config_debug() {
        let jwt = JwtConfig {
            secret: "secret".to_string(),
            issuer: "https://issuer.com".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 604800,
            private_key_pem: None,
            public_key_pem: None,
        };
        let debug_str = format!("{:?}", jwt);

        assert!(debug_str.contains("JwtConfig"));
        assert!(debug_str.contains("issuer"));
    }

    #[test]
    fn test_keycloak_config_clone() {
        let kc = KeycloakConfig {
            url: "http://keycloak:8080".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "secret".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
        };
        let kc2 = kc.clone();

        assert_eq!(kc.url, kc2.url);
        assert_eq!(kc.public_url, kc2.public_url);
        assert_eq!(kc.realm, kc2.realm);
    }

    #[test]
    fn test_keycloak_config_debug() {
        let kc = KeycloakConfig {
            url: "http://keycloak:8080".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "secret".to_string(),
            ssl_required: "external".to_string(),
            core_public_url: None,
            portal_url: None,
        };
        let debug_str = format!("{:?}", kc);

        assert!(debug_str.contains("KeycloakConfig"));
        assert!(debug_str.contains("realm"));
    }

    #[test]
    fn test_keycloak_ssl_required_options() {
        let ssl_options = ["none", "external", "all"];

        for opt in &ssl_options {
            let kc = KeycloakConfig {
                url: "http://localhost:8081".to_string(),
                public_url: "http://localhost:8081".to_string(),
                realm: "test".to_string(),
                admin_client_id: "admin".to_string(),
                admin_client_secret: "secret".to_string(),
                ssl_required: opt.to_string(),
                core_public_url: None,
                portal_url: None,
            };
            assert_eq!(kc.ssl_required, *opt);
        }
    }

    #[test]
    fn test_config_different_hosts() {
        let config = Config {
            http_host: "192.168.1.100".to_string(),
            http_port: 3000,
            grpc_host: "192.168.1.100".to_string(),
            grpc_port: 4000,
            database: DatabaseConfig {
                url: "mysql://db.example.com/prod".to_string(),
                max_connections: 50,
                min_connections: 10,
            },
            redis: RedisConfig {
                url: "redis://cache.example.com:6379".to_string(),
            },
            jwt: JwtConfig {
                secret: "production-secret".to_string(),
                issuer: "https://auth.example.com".to_string(),
                access_token_ttl_secs: 900,
                refresh_token_ttl_secs: 2592000,
                private_key_pem: None,
                public_key_pem: None,
            },
            keycloak: KeycloakConfig {
                url: "http://keycloak.internal:8080".to_string(),
                public_url: "https://auth.example.com".to_string(),
                realm: "production".to_string(),
                admin_client_id: "auth9-admin".to_string(),
                admin_client_secret: "admin-secret".to_string(),
                ssl_required: "all".to_string(),
                core_public_url: None,
                portal_url: None,
            },
        };

        assert_eq!(config.http_addr(), "192.168.1.100:3000");
        assert_eq!(config.grpc_addr(), "192.168.1.100:4000");
    }
}
