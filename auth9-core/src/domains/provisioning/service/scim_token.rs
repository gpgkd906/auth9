//! SCIM Token generation and validation service

use crate::domain::scim::{ScimRequestContext, ScimToken, ScimTokenResponse};
use crate::domain::StringUuid;
use crate::error::{AppError, Result};
use crate::repository::scim_token::ScimTokenRepository;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const TOKEN_PREFIX: &str = "scim_";
const TOKEN_RANDOM_BYTES: usize = 33; // ~44 base64 chars

pub struct ScimTokenService<R: ScimTokenRepository + 'static> {
    repo: Arc<R>,
}

impl<R: ScimTokenRepository + 'static> ScimTokenService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Generate a new SCIM token. Returns (raw_token, token_record).
    pub async fn create_token(
        &self,
        tenant_id: StringUuid,
        connector_id: StringUuid,
        description: Option<String>,
        expires_in_days: Option<i64>,
    ) -> Result<(String, ScimTokenResponse)> {
        // Generate random token
        let mut random_bytes = vec![0u8; TOKEN_RANDOM_BYTES];
        rand::thread_rng().fill_bytes(&mut random_bytes);
        let raw_token = format!("{}{}", TOKEN_PREFIX, URL_SAFE_NO_PAD.encode(&random_bytes));

        // Hash for storage
        let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
        let token_prefix_str = &raw_token[..std::cmp::min(raw_token.len(), 12)];

        let expires_at = expires_in_days.map(|days| Utc::now() + Duration::days(days));

        let token = ScimToken {
            id: StringUuid::new_v4(),
            tenant_id,
            connector_id,
            token_hash,
            token_prefix: token_prefix_str.to_string(),
            description,
            expires_at,
            last_used_at: None,
            revoked_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let created = self.repo.create(&token).await?;
        Ok((raw_token, ScimTokenResponse::from(created)))
    }

    /// Validate a bearer token and return the request context.
    pub async fn validate_token(
        &self,
        raw_token: &str,
        base_url: &str,
    ) -> Result<ScimRequestContext> {
        let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));

        let token = self
            .repo
            .find_by_hash(&token_hash)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid SCIM token".to_string()))?;

        // Check revoked
        if token.revoked_at.is_some() {
            return Err(AppError::Unauthorized(
                "SCIM token has been revoked".to_string(),
            ));
        }

        // Check expired
        if let Some(expires_at) = token.expires_at {
            if Utc::now() > expires_at {
                return Err(AppError::Unauthorized("SCIM token has expired".to_string()));
            }
        }

        // Update last_used_at (fire and forget - don't fail the request if this fails)
        let repo = self.repo.clone();
        let token_id = token.id;
        tokio::spawn(async move {
            let _ = repo.update_last_used(token_id).await;
        });

        Ok(ScimRequestContext {
            tenant_id: token.tenant_id,
            connector_id: token.connector_id,
            token_id: token.id,
            base_url: base_url.to_string(),
        })
    }

    /// List tokens for a connector.
    pub async fn list_tokens(&self, connector_id: StringUuid) -> Result<Vec<ScimTokenResponse>> {
        let tokens = self.repo.list_by_connector(connector_id).await?;
        Ok(tokens.into_iter().map(ScimTokenResponse::from).collect())
    }

    /// Revoke a token.
    pub async fn revoke_token(&self, token_id: StringUuid) -> Result<()> {
        self.repo.revoke(token_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::scim_token::MockScimTokenRepository;

    fn make_valid_token(hash: &str) -> ScimToken {
        ScimToken {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            connector_id: StringUuid::new_v4(),
            token_hash: hash.to_string(),
            token_prefix: "scim_abc".to_string(),
            description: None,
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_validate_token_success() {
        let raw = "scim_test_token_value";
        let hash = hex::encode(Sha256::digest(raw.as_bytes()));
        let token = make_valid_token(&hash);
        let expected_tenant = token.tenant_id;
        let expected_connector = token.connector_id;

        let mut mock = MockScimTokenRepository::new();
        mock.expect_find_by_hash()
            .returning(move |_| Ok(Some(token.clone())));
        mock.expect_update_last_used().returning(|_| Ok(()));

        let service = ScimTokenService::new(Arc::new(mock));
        let ctx = service
            .validate_token(raw, "https://example.com/scim/v2")
            .await
            .unwrap();

        assert_eq!(ctx.tenant_id, expected_tenant);
        assert_eq!(ctx.connector_id, expected_connector);
    }

    #[tokio::test]
    async fn test_validate_token_not_found() {
        let mut mock = MockScimTokenRepository::new();
        mock.expect_find_by_hash().returning(|_| Ok(None));

        let service = ScimTokenService::new(Arc::new(mock));
        let result = service
            .validate_token("invalid", "https://example.com")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_revoked() {
        let raw = "scim_revoked_token";
        let hash = hex::encode(Sha256::digest(raw.as_bytes()));
        let mut token = make_valid_token(&hash);
        token.revoked_at = Some(Utc::now());

        let mut mock = MockScimTokenRepository::new();
        mock.expect_find_by_hash()
            .returning(move |_| Ok(Some(token.clone())));

        let service = ScimTokenService::new(Arc::new(mock));
        let result = service.validate_token(raw, "https://example.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_expired() {
        let raw = "scim_expired_token";
        let hash = hex::encode(Sha256::digest(raw.as_bytes()));
        let mut token = make_valid_token(&hash);
        token.expires_at = Some(Utc::now() - Duration::hours(1));

        let mut mock = MockScimTokenRepository::new();
        mock.expect_find_by_hash()
            .returning(move |_| Ok(Some(token.clone())));

        let service = ScimTokenService::new(Arc::new(mock));
        let result = service.validate_token(raw, "https://example.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_token() {
        let mut mock = MockScimTokenRepository::new();
        mock.expect_create().returning(|t| Ok(t.clone()));

        let service = ScimTokenService::new(Arc::new(mock));
        let tenant_id = StringUuid::new_v4();
        let connector_id = StringUuid::new_v4();

        let (raw, resp) = service
            .create_token(tenant_id, connector_id, Some("Test".to_string()), Some(30))
            .await
            .unwrap();

        assert!(raw.starts_with("scim_"));
        assert_eq!(resp.tenant_id, tenant_id);
        assert_eq!(resp.connector_id, connector_id);
        assert!(resp.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_list_tokens() {
        let mut mock = MockScimTokenRepository::new();
        mock.expect_list_by_connector().returning(|_| Ok(vec![]));

        let service = ScimTokenService::new(Arc::new(mock));
        let result = service.list_tokens(StringUuid::new_v4()).await.unwrap();
        assert!(result.is_empty());
    }
}
