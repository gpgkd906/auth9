//! JWT token handling

use crate::config::JwtConfig;
use crate::error::{AppError, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identity Token claims (issued after initial authentication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Session ID (for session management)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    /// Email
    pub email: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Token type discriminator (prevents token confusion attacks)
    #[serde(default)]
    pub token_type: String,
    /// Custom claims (from Actions)
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
}

/// Tenant Access Token claims (issued after token exchange)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantAccessClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: String,
    /// Issuer
    pub iss: String,
    /// Audience (service client_id)
    pub aud: String,
    /// Token type discriminator (prevents token confusion attacks)
    #[serde(default)]
    pub token_type: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Roles in this tenant
    pub roles: Vec<String>,
    /// Permissions (derived from roles)
    pub permissions: Vec<String>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
}

/// Service Client Token claims (issued via client_credentials grant)
/// Uses a distinct audience ("auth9-service") so the auth middleware can distinguish
/// service tokens from user Identity tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceClientClaims {
    /// Subject (service ID, not a user ID)
    pub sub: String,
    /// Service email (synthetic, e.g., service+client_id@auth9.local)
    pub email: String,
    /// Issuer
    pub iss: String,
    /// Audience (always "auth9-service" to distinguish from Identity tokens)
    pub aud: String,
    /// Token type discriminator (prevents token confusion attacks)
    #[serde(default)]
    pub token_type: String,
    /// The tenant_id this service belongs to (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration (Unix timestamp)
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    /// Token type discriminator (prevents token confusion attacks)
    #[serde(default)]
    pub token_type: String,
    pub tenant_id: String,
    pub iat: i64,
    pub exp: i64,
}

/// JWT token manager
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    public_key_pem: Option<String>,
}

impl JwtManager {
    pub fn new(config: JwtConfig) -> Self {
        let algorithm = if config.private_key_pem.is_some() {
            Algorithm::RS256
        } else {
            Algorithm::HS256
        };
        let public_key_pem = config.public_key_pem.clone();
        let encoding_key = match config.private_key_pem.as_ref() {
            Some(private_key) => EncodingKey::from_rsa_pem(private_key.as_bytes())
                .expect("Failed to load JWT private key"),
            None => EncodingKey::from_secret(config.secret.as_bytes()),
        };
        let decoding_key = match config.public_key_pem.as_ref() {
            Some(public_key) => DecodingKey::from_rsa_pem(public_key.as_bytes())
                .expect("Failed to load JWT public key"),
            None => match config.private_key_pem.as_ref() {
                Some(private_key) => DecodingKey::from_rsa_pem(private_key.as_bytes())
                    .expect("Failed to load JWT private key"),
                None => DecodingKey::from_secret(config.secret.as_bytes()),
            },
        };
        Self {
            config,
            encoding_key,
            decoding_key,
            algorithm,
            public_key_pem,
        }
    }

    /// Create a Validation with a strict leeway (5 seconds) instead of the default 60 seconds.
    /// This ensures tokens expire promptly while still tolerating minor clock skew.
    fn strict_validation(&self) -> Validation {
        let mut v = Validation::new(self.algorithm);
        v.leeway = 5;
        v
    }

    /// Create an identity token
    pub fn create_identity_token(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
    ) -> Result<String> {
        self.create_identity_token_with_session(user_id, email, name, None)
    }

    /// Create an identity token with custom claims (from Actions)
    pub fn create_identity_token_with_claims(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
        custom_claims: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        self.create_identity_token_full(user_id, email, name, None, Some(custom_claims))
    }

    /// Create an identity token with both session ID and custom claims (from Actions)
    pub fn create_identity_token_with_session_and_claims(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
        session_id: Option<Uuid>,
        custom_claims: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        self.create_identity_token_full(user_id, email, name, session_id, Some(custom_claims))
    }

    /// Create an identity token with session ID
    pub fn create_identity_token_with_session(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
        session_id: Option<Uuid>,
    ) -> Result<String> {
        self.create_identity_token_full(user_id, email, name, session_id, None)
    }

    /// Create an identity token with all options
    fn create_identity_token_full(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
        session_id: Option<Uuid>,
        custom_claims: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.access_token_ttl_secs);

        let claims = IdentityClaims {
            sub: user_id.to_string(),
            sid: session_id.map(|id| id.to_string()),
            email: email.to_string(),
            name: name.map(String::from),
            iss: self.config.issuer.clone(),
            aud: "auth9".to_string(),
            token_type: "identity".to_string(),
            extra: custom_claims,
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(|e| AppError::Internal(e.into()))
    }

    /// Create a tenant access token
    pub fn create_tenant_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        tenant_id: Uuid,
        service_client_id: &str,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.access_token_ttl_secs);

        let claims = TenantAccessClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            iss: self.config.issuer.clone(),
            aud: service_client_id.to_string(),
            token_type: "access".to_string(),
            tenant_id: tenant_id.to_string(),
            roles,
            permissions,
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(|e| AppError::Internal(e.into()))
    }

    pub fn create_refresh_token(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_client_id: &str,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.refresh_token_ttl_secs);

        let claims = RefreshClaims {
            sub: user_id.to_string(),
            iss: self.config.issuer.clone(),
            aud: service_client_id.to_string(),
            token_type: "refresh".to_string(),
            tenant_id: tenant_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(|e| AppError::Internal(e.into()))
    }

    /// Create a service client token (for client_credentials grant)
    pub fn create_service_client_token(
        &self,
        service_id: Uuid,
        email: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.access_token_ttl_secs);

        let claims = ServiceClientClaims {
            sub: service_id.to_string(),
            email: email.to_string(),
            iss: self.config.issuer.clone(),
            aud: "auth9-service".to_string(),
            token_type: "service".to_string(),
            tenant_id: tenant_id.map(|t| t.to_string()),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key).map_err(|e| AppError::Internal(e.into()))
    }

    /// Verify and decode a service client token
    pub fn verify_service_client_token(&self, token: &str) -> Result<ServiceClientClaims> {
        let mut validation = self.strict_validation();
        validation.set_audience(&["auth9-service"]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<ServiceClientClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Verify and decode an identity token
    pub fn verify_identity_token(&self, token: &str) -> Result<IdentityClaims> {
        let mut validation = self.strict_validation();
        validation.set_audience(&["auth9"]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<IdentityClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Verify and decode a tenant access token with optional audience.
    ///
    /// Intended for gRPC/internal flows that may receive a per-request audience.
    /// For REST authentication, prefer `verify_tenant_access_token_strict`.
    pub fn verify_tenant_access_token_with_optional_audience(
        &self,
        token: &str,
        expected_audience: Option<&str>,
    ) -> Result<TenantAccessClaims> {
        let mut validation = self.strict_validation();
        validation.set_issuer(&[&self.config.issuer]);

        if let Some(aud) = expected_audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        let token_data = decode::<TenantAccessClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Verify and decode a tenant access token
    #[deprecated(note = "Use verify_tenant_access_token_strict for REST-side validation")]
    pub fn verify_tenant_access_token(
        &self,
        token: &str,
        expected_audience: Option<&str>,
    ) -> Result<TenantAccessClaims> {
        self.verify_tenant_access_token_with_optional_audience(token, expected_audience)
    }

    /// Verify and decode a tenant access token (strict audience allowlist).
    ///
    /// This is intended for REST-side authentication where the caller must be one of a known set
    /// of clients/services. In production, the allowlist must be non-empty.
    pub fn verify_tenant_access_token_strict(
        &self,
        token: &str,
        expected_audiences: &[String],
    ) -> Result<TenantAccessClaims> {
        if expected_audiences.is_empty() {
            return Err(AppError::Unauthorized(
                "Tenant access token audience allowlist is not configured".to_string(),
            ));
        }

        let mut validation = self.strict_validation();
        validation.set_issuer(&[&self.config.issuer]);

        let aud_refs: Vec<&str> = expected_audiences.iter().map(|s| s.as_str()).collect();
        validation.set_audience(&aud_refs);

        let token_data = decode::<TenantAccessClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Get token expiration TTL in seconds
    pub fn access_token_ttl(&self) -> i64 {
        self.config.access_token_ttl_secs
    }

    pub fn uses_rsa(&self) -> bool {
        self.algorithm == Algorithm::RS256
    }

    pub fn public_key_pem(&self) -> Option<&str> {
        self.public_key_pem.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-for-testing-purposes-only".to_string(),
            issuer: "https://auth9.test".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 604800,
            private_key_pem: None,
            public_key_pem: None,
        }
    }

    #[test]
    fn test_create_and_verify_identity_token() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();

        let token = manager
            .create_identity_token(user_id, "test@example.com", Some("Test User"))
            .unwrap();

        let claims = manager.verify_identity_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.name, Some("Test User".to_string()));
        assert_eq!(claims.aud, "auth9");
    }

    #[test]
    fn test_create_and_verify_tenant_access_token() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager
            .create_tenant_access_token(
                user_id,
                "test@example.com",
                tenant_id,
                "my-service",
                vec!["admin".to_string()],
                vec!["user:read".to_string(), "user:write".to_string()],
            )
            .unwrap();

        let claims = manager
            .verify_tenant_access_token_strict(&token, &["my-service".to_string()])
            .unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.permissions, vec!["user:read", "user:write"]);
    }

    #[test]
    fn test_invalid_token() {
        let manager = JwtManager::new(test_config());

        let result = manager.verify_identity_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_audience() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager
            .create_tenant_access_token(
                user_id,
                "test@example.com",
                tenant_id,
                "my-service",
                vec![],
                vec![],
            )
            .unwrap();

        let result =
            manager.verify_tenant_access_token_strict(&token, &["other-service".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_identity_token_without_name() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();

        let token = manager
            .create_identity_token(user_id, "noname@example.com", None)
            .unwrap();

        let claims = manager.verify_identity_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "noname@example.com");
        assert!(claims.name.is_none());
    }

    #[test]
    fn test_refresh_token_creation() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager
            .create_refresh_token(user_id, tenant_id, "my-service")
            .unwrap();

        // Refresh tokens should be valid JWT strings
        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_access_token_ttl() {
        let manager = JwtManager::new(test_config());
        assert_eq!(manager.access_token_ttl(), 3600);
    }

    #[test]
    fn test_uses_rsa_false_for_hmac() {
        let manager = JwtManager::new(test_config());
        assert!(!manager.uses_rsa());
    }

    #[test]
    fn test_public_key_pem_none_for_hmac() {
        let manager = JwtManager::new(test_config());
        assert!(manager.public_key_pem().is_none());
    }

    #[test]
    fn test_tenant_access_token_strict_audience_allowlist() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager
            .create_tenant_access_token(
                user_id,
                "test@example.com",
                tenant_id,
                "any-service",
                vec!["user".to_string()],
                vec!["read".to_string()],
            )
            .unwrap();

        let claims = manager
            .verify_tenant_access_token_strict(&token, &["any-service".to_string()])
            .unwrap();
        assert_eq!(claims.aud, "any-service");
    }

    #[test]
    fn test_tenant_access_token_empty_roles_and_permissions() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager
            .create_tenant_access_token(
                user_id,
                "minimal@example.com",
                tenant_id,
                "service",
                vec![],
                vec![],
            )
            .unwrap();

        let claims = manager
            .verify_tenant_access_token_strict(&token, &["service".to_string()])
            .unwrap();
        assert!(claims.roles.is_empty());
        assert!(claims.permissions.is_empty());
    }

    #[test]
    fn test_identity_claims_serialization() {
        let claims = IdentityClaims {
            sub: "user-123".to_string(),
            sid: Some("session-456".to_string()),
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            iss: "https://auth9.test".to_string(),
            aud: "auth9".to_string(),
            token_type: "identity".to_string(),
            iat: 1000000,
            exp: 1003600,
            extra: None,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"sub\":\"user-123\""));
        assert!(json.contains("\"email\":\"test@example.com\""));
        assert!(json.contains("\"name\":\"Test User\""));
        assert!(json.contains("\"sid\":\"session-456\""));
        assert!(json.contains("\"token_type\":\"identity\""));
    }

    #[test]
    fn test_identity_claims_serialization_without_name() {
        let claims = IdentityClaims {
            sub: "user-123".to_string(),
            sid: None,
            email: "test@example.com".to_string(),
            name: None,
            iss: "https://auth9.test".to_string(),
            aud: "auth9".to_string(),
            token_type: "identity".to_string(),
            iat: 1000000,
            exp: 1003600,
            extra: None,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(!json.contains("\"name\""));
        assert!(!json.contains("\"sid\""));
    }

    #[test]
    fn test_tenant_access_claims_serialization() {
        let claims = TenantAccessClaims {
            sub: "user-456".to_string(),
            email: "tenant@example.com".to_string(),
            iss: "https://auth9.test".to_string(),
            aud: "my-app".to_string(),
            token_type: "access".to_string(),
            tenant_id: "tenant-789".to_string(),
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            iat: 1000000,
            exp: 1003600,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"tenant_id\":\"tenant-789\""));
        assert!(json.contains("\"roles\":[\"admin\",\"user\"]"));
        assert!(json.contains("\"permissions\":[\"read\",\"write\"]"));
        assert!(json.contains("\"token_type\":\"access\""));
    }

    #[test]
    fn test_refresh_claims_serialization() {
        let claims = RefreshClaims {
            sub: "user-123".to_string(),
            iss: "https://auth9.test".to_string(),
            aud: "my-service".to_string(),
            token_type: "refresh".to_string(),
            tenant_id: "tenant-456".to_string(),
            iat: 1000000,
            exp: 1604800,
        };

        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"sub\":\"user-123\""));
        assert!(json.contains("\"tenant_id\":\"tenant-456\""));
        assert!(json.contains("\"token_type\":\"refresh\""));
    }

    #[test]
    fn test_identity_claims_deserialization() {
        let json = r#"{
            "sub": "user-123",
            "email": "test@example.com",
            "name": "Test User",
            "iss": "https://auth9.test",
            "aud": "auth9",
            "token_type": "identity",
            "iat": 1000000,
            "exp": 1003600
        }"#;

        let claims: IdentityClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_tenant_access_claims_deserialization() {
        let json = r#"{
            "sub": "user-456",
            "email": "tenant@example.com",
            "iss": "https://auth9.test",
            "aud": "my-app",
            "token_type": "access",
            "tenant_id": "tenant-789",
            "roles": ["admin"],
            "permissions": ["read", "write"],
            "iat": 1000000,
            "exp": 1003600
        }"#;

        let claims: TenantAccessClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.tenant_id, "tenant-789");
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.permissions, vec!["read", "write"]);
    }

    #[test]
    fn test_jwt_manager_clone() {
        let manager1 = JwtManager::new(test_config());
        let manager2 = manager1.clone();

        let user_id = Uuid::new_v4();
        let token = manager1
            .create_identity_token(user_id, "test@example.com", None)
            .unwrap();

        // Cloned manager should be able to verify the token
        let claims = manager2.verify_identity_token(&token).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
    }

    #[test]
    fn test_token_has_valid_structure() {
        let manager = JwtManager::new(test_config());
        let token = manager
            .create_identity_token(Uuid::new_v4(), "test@example.com", None)
            .unwrap();

        // JWT should have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Each part should be non-empty
        for part in parts {
            assert!(!part.is_empty());
        }
    }

    #[test]
    fn test_token_issuer_validation() {
        let manager = JwtManager::new(test_config());
        let user_id = Uuid::new_v4();

        let token = manager
            .create_identity_token(user_id, "test@example.com", None)
            .unwrap();

        let claims = manager.verify_identity_token(&token).unwrap();
        assert_eq!(claims.iss, "https://auth9.test");
    }

    #[test]
    fn test_custom_ttl_config() {
        let config = JwtConfig {
            secret: "test-secret".to_string(),
            issuer: "https://custom.issuer".to_string(),
            access_token_ttl_secs: 1800,
            refresh_token_ttl_secs: 86400,
            private_key_pem: None,
            public_key_pem: None,
        };

        let manager = JwtManager::new(config);
        assert_eq!(manager.access_token_ttl(), 1800);
    }
}
