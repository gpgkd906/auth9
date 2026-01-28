//! JWT token handling

use crate::config::JwtConfig;
use crate::error::{AppError, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identity Token claims (issued after initial authentication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
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

/// JWT token manager
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());
        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create an identity token
    pub fn create_identity_token(
        &self,
        user_id: Uuid,
        email: &str,
        name: Option<&str>,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.access_token_ttl_secs);

        let claims = IdentityClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            name: name.map(String::from),
            iss: self.config.issuer.clone(),
            aud: "auth9".to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(e.into()))
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
            tenant_id: tenant_id.to_string(),
            roles,
            permissions,
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(e.into()))
    }

    /// Verify and decode an identity token
    pub fn verify_identity_token(&self, token: &str) -> Result<IdentityClaims> {
        let mut validation = Validation::default();
        validation.set_audience(&["auth9"]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<IdentityClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Verify and decode a tenant access token
    pub fn verify_tenant_access_token(
        &self,
        token: &str,
        expected_audience: Option<&str>,
    ) -> Result<TenantAccessClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        
        if let Some(aud) = expected_audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        let token_data = decode::<TenantAccessClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Get token expiration TTL in seconds
    pub fn access_token_ttl(&self) -> i64 {
        self.config.access_token_ttl_secs
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
            .verify_tenant_access_token(&token, Some("my-service"))
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
        
        let result = manager.verify_tenant_access_token(&token, Some("other-service"));
        assert!(result.is_err());
    }
}
