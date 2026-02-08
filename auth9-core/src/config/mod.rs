//! Configuration management for Auth9 Core

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use std::fmt;

/// WebAuthn configuration
#[derive(Debug, Clone)]
pub struct WebAuthnConfig {
    /// Relying Party ID (domain, e.g. "localhost" or "auth9.example.com")
    pub rp_id: String,
    /// Relying Party display name
    pub rp_name: String,
    /// Relying Party origin URL (e.g. "http://localhost:3000")
    pub rp_origin: String,
    /// Challenge TTL in seconds (default 300)
    pub challenge_ttl_secs: u64,
}

/// Application configuration
#[derive(Clone)]
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
    /// gRPC security configuration
    pub grpc_security: GrpcSecurityConfig,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// CORS configuration
    pub cors: CorsConfig,
    /// WebAuthn configuration
    pub webauthn: WebAuthnConfig,
    /// Platform admin email allowlist.
    ///
    /// Identity tokens are intentionally tenant-unscoped. Only Identity tokens whose
    /// email is in this allowlist are treated as platform admins.
    pub platform_admin_emails: Vec<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("http_host", &self.http_host)
            .field("http_port", &self.http_port)
            .field("grpc_host", &self.grpc_host)
            .field("grpc_port", &self.grpc_port)
            .field("database", &self.database)
            .field("redis", &self.redis)
            .field("jwt", &self.jwt)
            .field("keycloak", &self.keycloak)
            .field("grpc_security", &self.grpc_security)
            .field("rate_limit", &self.rate_limit)
            .field("cors", &self.cors)
            .field("webauthn", &self.webauthn)
            .field(
                "platform_admin_emails",
                &format!("[{} emails]", self.platform_admin_emails.len()),
            )
            .finish()
    }
}

/// CORS configuration (no sensitive fields)
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins (comma-separated in env var, or "*" for any)
    /// Default is restrictive (localhost only)
    pub allowed_origins: Vec<String>,
    /// Whether to allow credentials
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            // Default to common local development origins
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:5173".to_string(),
            ],
            allow_credentials: true,
        }
    }
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &"<REDACTED>")
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .finish()
    }
}

#[derive(Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl fmt::Debug for RedisConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisConfig")
            .field("url", &"<REDACTED>")
            .finish()
    }
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub private_key_pem: Option<String>,
    pub public_key_pem: Option<String>,
}

impl fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtConfig")
            .field("secret", &"<REDACTED>")
            .field("issuer", &self.issuer)
            .field("access_token_ttl_secs", &self.access_token_ttl_secs)
            .field("refresh_token_ttl_secs", &self.refresh_token_ttl_secs)
            .field(
                "private_key_pem",
                &self.private_key_pem.as_ref().map(|_| "<REDACTED>"),
            )
            .field(
                "public_key_pem",
                &self.public_key_pem.as_ref().map(|_| "<REDACTED>"),
            )
            .finish()
    }
}

#[derive(Clone)]
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
    /// Webhook secret for verifying Keycloak event webhook signatures (HMAC-SHA256)
    /// Required in production to prevent spoofed events
    pub webhook_secret: Option<String>,
}

impl fmt::Debug for KeycloakConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycloakConfig")
            .field("url", &self.url)
            .field("public_url", &self.public_url)
            .field("realm", &self.realm)
            .field("admin_client_id", &self.admin_client_id)
            .field("admin_client_secret", &"<REDACTED>")
            .field("ssl_required", &self.ssl_required)
            .field("core_public_url", &self.core_public_url)
            .field("portal_url", &self.portal_url)
            .field(
                "webhook_secret",
                &self.webhook_secret.as_ref().map(|_| "<REDACTED>"),
            )
            .finish()
    }
}

/// gRPC security configuration
#[derive(Clone)]
pub struct GrpcSecurityConfig {
    /// Authentication mode: "none", "api_key", or "mtls"
    pub auth_mode: String,
    /// API keys for api_key mode (comma-separated in env var)
    pub api_keys: Vec<String>,
    /// Path to TLS certificate for mTLS mode
    pub tls_cert_path: Option<String>,
    /// Path to TLS private key for mTLS mode
    pub tls_key_path: Option<String>,
    /// Path to CA certificate for client verification in mTLS mode
    pub tls_ca_cert_path: Option<String>,
    /// Whether to enable gRPC reflection (for debugging tools like grpcurl)
    pub enable_reflection: bool,
}

impl fmt::Debug for GrpcSecurityConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrpcSecurityConfig")
            .field("auth_mode", &self.auth_mode)
            .field("api_keys", &format!("[{} keys]", self.api_keys.len()))
            .field("tls_cert_path", &self.tls_cert_path)
            .field("tls_key_path", &self.tls_key_path)
            .field("tls_ca_cert_path", &self.tls_ca_cert_path)
            .field("enable_reflection", &self.enable_reflection)
            .finish()
    }
}

impl Default for GrpcSecurityConfig {
    fn default() -> Self {
        Self {
            auth_mode: "none".to_string(),
            api_keys: vec![],
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_cert_path: None,
            enable_reflection: false,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Default requests per window
    pub default_requests: u64,
    /// Default window size in seconds
    pub default_window_secs: u64,
    /// Per-endpoint overrides (JSON format in env var)
    pub endpoints: HashMap<String, RateLimitEndpointConfig>,
    /// Per-tenant multipliers (JSON format in env var)
    pub tenant_multipliers: HashMap<String, f64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            // Default to enabled for production security
            enabled: true,
            default_requests: 100,
            default_window_secs: 60,
            endpoints: HashMap::new(),
            tenant_multipliers: HashMap::new(),
        }
    }
}

/// Rate limit configuration for a specific endpoint
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RateLimitEndpointConfig {
    /// Maximum requests allowed
    pub requests: u64,
    /// Time window in seconds
    pub window_secs: u64,
}

impl Config {
    pub fn is_platform_admin_email(&self, email: &str) -> bool {
        let email = email.trim();
        self.platform_admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(email))
    }

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

                // Webhook secret for Keycloak event verification
                let webhook_secret = env::var("KEYCLOAK_WEBHOOK_SECRET").ok();

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
                    webhook_secret,
                }
            },
            grpc_security: GrpcSecurityConfig {
                auth_mode: env::var("GRPC_AUTH_MODE").unwrap_or_else(|_| "none".to_string()),
                api_keys: env::var("GRPC_API_KEYS")
                    .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
                    .unwrap_or_default(),
                tls_cert_path: env::var("GRPC_TLS_CERT_PATH").ok(),
                tls_key_path: env::var("GRPC_TLS_KEY_PATH").ok(),
                tls_ca_cert_path: env::var("GRPC_TLS_CA_CERT_PATH").ok(),
                enable_reflection: env::var("GRPC_ENABLE_REFLECTION")
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(false),
            },
            rate_limit: {
                let endpoints: HashMap<String, RateLimitEndpointConfig> =
                    env::var("RATE_LIMIT_ENDPOINTS")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                let tenant_multipliers: HashMap<String, f64> =
                    env::var("RATE_LIMIT_TENANT_MULTIPLIERS")
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                RateLimitConfig {
                    // Default to enabled for production security
                    // Set RATE_LIMIT_ENABLED=false to disable in development
                    enabled: env::var("RATE_LIMIT_ENABLED")
                        .map(|s| s.to_lowercase() != "false")
                        .unwrap_or(true),
                    default_requests: env::var("RATE_LIMIT_DEFAULT_REQUESTS")
                        .unwrap_or_else(|_| "100".to_string())
                        .parse()
                        .unwrap_or(100),
                    default_window_secs: env::var("RATE_LIMIT_DEFAULT_WINDOW_SECS")
                        .unwrap_or_else(|_| "60".to_string())
                        .parse()
                        .unwrap_or(60),
                    endpoints,
                    tenant_multipliers,
                }
            },
            cors: {
                // Parse CORS_ALLOWED_ORIGINS: comma-separated list or "*" for any
                let allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
                    .map(|s| {
                        if s == "*" {
                            // Special case: wildcard means allow any
                            vec!["*".to_string()]
                        } else {
                            s.split(',')
                                .map(|origin| origin.trim().to_string())
                                .filter(|origin| !origin.is_empty())
                                .collect()
                        }
                    })
                    .unwrap_or_else(|_| CorsConfig::default().allowed_origins);

                let allow_credentials = env::var("CORS_ALLOW_CREDENTIALS")
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(true);

                CorsConfig {
                    allowed_origins,
                    allow_credentials,
                }
            },
            webauthn: {
                let portal_url = env::var("AUTH9_PORTAL_URL")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string());
                WebAuthnConfig {
                    rp_id: env::var("WEBAUTHN_RP_ID")
                        .unwrap_or_else(|_| "localhost".to_string()),
                    rp_name: env::var("WEBAUTHN_RP_NAME")
                        .unwrap_or_else(|_| "Auth9".to_string()),
                    rp_origin: env::var("WEBAUTHN_RP_ORIGIN")
                        .unwrap_or(portal_url),
                    challenge_ttl_secs: env::var("WEBAUTHN_CHALLENGE_TTL_SECS")
                        .unwrap_or_else(|_| "300".to_string())
                        .parse()
                        .unwrap_or(300),
                }
            },
            platform_admin_emails: parse_csv_env(
                "PLATFORM_ADMIN_EMAILS",
                vec!["admin@auth9.local".to_string()],
            ),
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

fn parse_csv_env(key: &str, default: Vec<String>) -> Vec<String> {
    match env::var(key) {
        Ok(v) => {
            let items: Vec<String> = v
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if items.is_empty() {
                default
            } else {
                items
            }
        }
        Err(_) => default,
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
                webhook_secret: None,
            },
            grpc_security: GrpcSecurityConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            webauthn: WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "Auth9".to_string(),
                rp_origin: "http://localhost:3000".to_string(),
                challenge_ttl_secs: 300,
            },
            platform_admin_emails: vec!["admin@auth9.local".to_string()],
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
        // URL should be redacted for security
        assert!(debug_str.contains("<REDACTED>"));
        assert!(!debug_str.contains("redis://localhost:6379"));
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
            webhook_secret: None,
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
            webhook_secret: None,
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
                webhook_secret: None,
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
                webhook_secret: Some("production-webhook-secret".to_string()),
            },
            grpc_security: GrpcSecurityConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            webauthn: WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "Auth9".to_string(),
                rp_origin: "http://localhost:3000".to_string(),
                challenge_ttl_secs: 300,
            },
            platform_admin_emails: vec!["admin@auth9.local".to_string()],
        };

        assert_eq!(config.http_addr(), "192.168.1.100:3000");
        assert_eq!(config.grpc_addr(), "192.168.1.100:4000");
    }

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert!(config
            .allowed_origins
            .contains(&"http://localhost:3000".to_string()));
        assert!(config
            .allowed_origins
            .contains(&"http://localhost:5173".to_string()));
        assert!(config.allow_credentials);
    }

    #[test]
    fn test_cors_config_custom_origins() {
        let config = CorsConfig {
            allowed_origins: vec![
                "https://app.example.com".to_string(),
                "https://admin.example.com".to_string(),
            ],
            allow_credentials: true,
        };
        assert_eq!(config.allowed_origins.len(), 2);
        assert!(config
            .allowed_origins
            .contains(&"https://app.example.com".to_string()));
    }

    #[test]
    fn test_cors_config_wildcard() {
        let config = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allow_credentials: false,
        };
        assert_eq!(config.allowed_origins.len(), 1);
        assert_eq!(config.allowed_origins[0], "*");
        // Note: wildcard + credentials is a CORS spec violation, hence allow_credentials = false
    }

    #[test]
    fn test_grpc_security_config_default() {
        let config = GrpcSecurityConfig::default();
        assert_eq!(config.auth_mode, "none");
        assert!(config.api_keys.is_empty());
        assert!(config.tls_cert_path.is_none());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        // Rate limiting is enabled by default for production security
        assert!(config.enabled);
        assert_eq!(config.default_requests, 100);
        assert_eq!(config.default_window_secs, 60);
    }

    #[test]
    fn test_grpc_security_config_with_api_keys() {
        let config = GrpcSecurityConfig {
            auth_mode: "api_key".to_string(),
            api_keys: vec!["key1".to_string(), "key2".to_string()],
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_cert_path: None,
            enable_reflection: false,
        };

        assert_eq!(config.auth_mode, "api_key");
        assert_eq!(config.api_keys.len(), 2);
    }

    #[test]
    fn test_grpc_security_config_with_mtls() {
        let config = GrpcSecurityConfig {
            auth_mode: "mtls".to_string(),
            api_keys: vec![],
            tls_cert_path: Some("/path/to/server.crt".to_string()),
            tls_key_path: Some("/path/to/server.key".to_string()),
            tls_ca_cert_path: Some("/path/to/ca.crt".to_string()),
            enable_reflection: false,
        };

        assert_eq!(config.auth_mode, "mtls");
        assert!(config.tls_cert_path.is_some());
        assert!(config.tls_key_path.is_some());
        assert!(config.tls_ca_cert_path.is_some());
    }

    #[test]
    fn test_grpc_security_config_with_reflection() {
        let config = GrpcSecurityConfig {
            auth_mode: "none".to_string(),
            api_keys: vec![],
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_cert_path: None,
            enable_reflection: true,
        };

        assert!(config.enable_reflection);
    }

    #[test]
    fn test_rate_limit_endpoint_config_deserialize() {
        let json = r#"{"requests": 10, "window_secs": 60}"#;
        let config: RateLimitEndpointConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.requests, 10);
        assert_eq!(config.window_secs, 60);
    }

    // ==================== from_env() tests ====================
    // These tests use a Mutex to serialize env var access since Rust tests
    // run in parallel and env vars are process-wide.

    use std::sync::Mutex;
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper to save and restore env vars around a test closure
    fn with_env_vars<F>(vars: &[(&str, Option<&str>)], test_fn: F)
    where
        F: FnOnce(),
    {
        let _guard = ENV_MUTEX.lock().unwrap();

        // Save originals
        let originals: Vec<(&str, Option<String>)> = vars
            .iter()
            .map(|(key, _)| (*key, env::var(key).ok()))
            .collect();

        // Set new values
        for (key, value) in vars {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }

        // Run test
        test_fn();

        // Restore originals
        for (key, original) in originals {
            match original {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }

    #[test]
    fn test_from_env_missing_database_url() {
        with_env_vars(
            &[("DATABASE_URL", None), ("JWT_SECRET", Some("test-secret"))],
            || {
                let result = Config::from_env();
                assert!(result.is_err());
                let err_msg = format!("{}", result.unwrap_err());
                assert!(err_msg.contains("DATABASE_URL"));
            },
        );
    }

    #[test]
    fn test_from_env_missing_jwt_secret() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/test")),
                ("JWT_SECRET", None),
            ],
            || {
                let result = Config::from_env();
                assert!(result.is_err());
                let err_msg = format!("{}", result.unwrap_err());
                assert!(err_msg.contains("JWT_SECRET"));
            },
        );
    }

    #[test]
    fn test_from_env_with_minimal_config() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("minimal-test-secret")),
                ("RATE_LIMIT_ENABLED", None),
                ("GRPC_AUTH_MODE", None),
                ("HTTP_HOST", None),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.http_host, "0.0.0.0");
                assert_eq!(config.database.url, "mysql://test:test@localhost/testdb");
                assert_eq!(config.jwt.secret, "minimal-test-secret");
                assert!(config.rate_limit.enabled);
                assert_eq!(config.grpc_security.auth_mode, "none");
            },
        );
    }

    #[test]
    fn test_from_env_cors_wildcard() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("CORS_ALLOWED_ORIGINS", Some("*")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.cors.allowed_origins, vec!["*".to_string()]);
            },
        );
    }

    #[test]
    fn test_from_env_cors_comma_separated() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                (
                    "CORS_ALLOWED_ORIGINS",
                    Some("https://app.example.com, https://admin.example.com"),
                ),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.cors.allowed_origins.len(), 2);
                assert!(config
                    .cors
                    .allowed_origins
                    .contains(&"https://app.example.com".to_string()));
                assert!(config
                    .cors
                    .allowed_origins
                    .contains(&"https://admin.example.com".to_string()));
            },
        );
    }

    #[test]
    fn test_from_env_rate_limit_disabled() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("RATE_LIMIT_ENABLED", Some("false")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert!(!config.rate_limit.enabled);
            },
        );
    }

    #[test]
    fn test_from_env_grpc_api_keys() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("GRPC_AUTH_MODE", Some("api_key")),
                ("GRPC_API_KEYS", Some("key-alpha, key-beta , key-gamma")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.grpc_security.auth_mode, "api_key");
                assert_eq!(config.grpc_security.api_keys.len(), 3);
                assert_eq!(config.grpc_security.api_keys[0], "key-alpha");
                assert_eq!(config.grpc_security.api_keys[1], "key-beta");
                assert_eq!(config.grpc_security.api_keys[2], "key-gamma");
            },
        );
    }

    #[test]
    fn test_from_env_jwt_pem_newline_replacement() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                (
                    "JWT_PRIVATE_KEY",
                    Some("-----BEGIN PRIVATE KEY-----\\nMIIEvg\\n-----END PRIVATE KEY-----"),
                ),
                (
                    "JWT_PUBLIC_KEY",
                    Some("-----BEGIN PUBLIC KEY-----\\nMIIBIj\\n-----END PUBLIC KEY-----"),
                ),
            ],
            || {
                let config = Config::from_env().unwrap();
                let priv_key = config.jwt.private_key_pem.unwrap();
                let pub_key = config.jwt.public_key_pem.unwrap();
                assert!(priv_key.contains('\n'));
                assert!(!priv_key.contains("\\n"));
                assert!(pub_key.contains('\n'));
                assert!(!pub_key.contains("\\n"));
            },
        );
    }

    #[test]
    fn test_from_env_grpc_reflection_enabled() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("GRPC_ENABLE_REFLECTION", Some("true")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert!(config.grpc_security.enable_reflection);
            },
        );
    }

    #[test]
    fn test_from_env_cors_allow_credentials_false() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("CORS_ALLOW_CREDENTIALS", Some("false")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert!(!config.cors.allow_credentials);
            },
        );
    }

    #[test]
    fn test_from_env_keycloak_webhook_secret() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                ("KEYCLOAK_WEBHOOK_SECRET", Some("my-webhook-hmac-secret")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(
                    config.keycloak.webhook_secret.unwrap(),
                    "my-webhook-hmac-secret"
                );
            },
        );
    }

    #[test]
    fn test_from_env_rate_limit_endpoints_json() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                (
                    "RATE_LIMIT_ENDPOINTS",
                    Some(r#"{"POST:/api/v1/auth/token":{"requests":10,"window_secs":30}}"#),
                ),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.rate_limit.endpoints.len(), 1);
                let ep = config
                    .rate_limit
                    .endpoints
                    .get("POST:/api/v1/auth/token")
                    .unwrap();
                assert_eq!(ep.requests, 10);
                assert_eq!(ep.window_secs, 30);
            },
        );
    }

    #[test]
    fn test_from_env_rate_limit_tenant_multipliers_json() {
        with_env_vars(
            &[
                ("DATABASE_URL", Some("mysql://test:test@localhost/testdb")),
                ("JWT_SECRET", Some("test-secret")),
                (
                    "RATE_LIMIT_TENANT_MULTIPLIERS",
                    Some(r#"{"premium":2.0,"enterprise":5.0}"#),
                ),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.rate_limit.tenant_multipliers.len(), 2);
                assert_eq!(
                    config.rate_limit.tenant_multipliers.get("premium"),
                    Some(&2.0)
                );
                assert_eq!(
                    config.rate_limit.tenant_multipliers.get("enterprise"),
                    Some(&5.0)
                );
            },
        );
    }

    #[test]
    fn test_sensitive_data_redacted_in_debug() {
        // Create a config with sensitive data
        let config = Config {
            http_host: "127.0.0.1".to_string(),
            http_port: 8080,
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 50051,
            database: DatabaseConfig {
                url: "mysql://user:supersecretpassword@host/db".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            redis: RedisConfig {
                url: "redis://:redispassword@localhost:6379".to_string(),
            },
            jwt: JwtConfig {
                secret: "my-super-secret-jwt-key".to_string(),
                issuer: "https://auth9.example.com".to_string(),
                access_token_ttl_secs: 3600,
                refresh_token_ttl_secs: 604800,
                private_key_pem: Some(
                    "-----BEGIN PRIVATE KEY-----\nsecretkey\n-----END PRIVATE KEY-----".to_string(),
                ),
                public_key_pem: Some(
                    "-----BEGIN PUBLIC KEY-----\npublickey\n-----END PUBLIC KEY-----".to_string(),
                ),
            },
            keycloak: KeycloakConfig {
                url: "http://keycloak:8080".to_string(),
                public_url: "http://localhost:8081".to_string(),
                realm: "auth9".to_string(),
                admin_client_id: "admin-cli".to_string(),
                admin_client_secret: "keycloak-admin-secret".to_string(),
                ssl_required: "external".to_string(),
                core_public_url: None,
                portal_url: None,
                webhook_secret: Some("webhook-secret-key".to_string()),
            },
            grpc_security: GrpcSecurityConfig {
                auth_mode: "api_key".to_string(),
                api_keys: vec!["api-key-1".to_string(), "api-key-2".to_string()],
                tls_cert_path: None,
                tls_key_path: None,
                tls_ca_cert_path: None,
                enable_reflection: false,
            },
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            webauthn: WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "Auth9".to_string(),
                rp_origin: "http://localhost:3000".to_string(),
                challenge_ttl_secs: 300,
            },
            platform_admin_emails: vec!["admin@auth9.local".to_string()],
        };

        let debug_str = format!("{:?}", config);

        // Verify sensitive data is NOT in the debug output
        assert!(
            !debug_str.contains("supersecretpassword"),
            "Database password should be redacted"
        );
        assert!(
            !debug_str.contains("redispassword"),
            "Redis password should be redacted"
        );
        assert!(
            !debug_str.contains("my-super-secret-jwt-key"),
            "JWT secret should be redacted"
        );
        assert!(
            !debug_str.contains("secretkey"),
            "Private key should be redacted"
        );
        assert!(
            !debug_str.contains("keycloak-admin-secret"),
            "Keycloak admin secret should be redacted"
        );
        assert!(
            !debug_str.contains("webhook-secret-key"),
            "Webhook secret should be redacted"
        );
        assert!(
            !debug_str.contains("api-key-1"),
            "API keys should be redacted"
        );
        assert!(
            !debug_str.contains("api-key-2"),
            "API keys should be redacted"
        );

        // Verify non-sensitive data IS present
        assert!(
            debug_str.contains("127.0.0.1"),
            "HTTP host should be visible"
        );
        assert!(debug_str.contains("8080"), "HTTP port should be visible");
        assert!(debug_str.contains("auth9"), "Realm should be visible");
        assert!(
            debug_str.contains("https://auth9.example.com"),
            "Issuer should be visible"
        );

        // Verify redaction markers are present
        assert!(
            debug_str.contains("<REDACTED>"),
            "Should contain redaction markers"
        );
        assert!(
            debug_str.contains("[2 keys]"),
            "Should show API key count without values"
        );
    }
}
