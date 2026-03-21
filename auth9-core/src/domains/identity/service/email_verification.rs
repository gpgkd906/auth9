//! Email verification service.
//!
//! Handles sending verification emails, validating tokens, and updating
//! email verification status through the IdentityEngine abstraction.

use crate::error::{AppError, Result};
use crate::identity_engine::IdentityEngine;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Default token TTL: 24 hours.
const DEFAULT_TOKEN_TTL_HOURS: i64 = 24;

pub struct EmailVerificationService {
    identity_engine: Arc<dyn IdentityEngine>,
    portal_url: String,
}

impl EmailVerificationService {
    pub fn new(identity_engine: Arc<dyn IdentityEngine>, portal_url: String) -> Self {
        Self {
            identity_engine,
            portal_url,
        }
    }

    /// Generate a random verification token and return (raw_token, token_hash).
    fn generate_token() -> (String, String) {
        let bytes: [u8; 32] = rand::thread_rng().gen();
        let raw_token = URL_SAFE_NO_PAD.encode(bytes);
        let hash = Self::hash_token(&raw_token);
        (raw_token, hash)
    }

    /// SHA-256 hash a raw token for storage.
    fn hash_token(raw_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Build the verification link URL.
    fn build_verification_link(&self, raw_token: &str) -> String {
        format!("{}/verify-email?token={}", self.portal_url, raw_token)
    }

    /// Create a verification token and return the raw token string.
    /// The caller is responsible for sending the email.
    pub async fn create_verification_token(&self, user_id: &str) -> Result<(String, String)> {
        let store = self.identity_engine.verification_store();

        // Invalidate existing tokens for this user
        let _ = store.invalidate_user_tokens(user_id).await;

        // Ensure verification status row exists
        let _ = store.get_verification_status(user_id).await;

        let (raw_token, token_hash) = Self::generate_token();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(DEFAULT_TOKEN_TTL_HOURS);

        store
            .create_verification_token(user_id, &token_hash, expires_at)
            .await?;

        let link = self.build_verification_link(&raw_token);
        Ok((raw_token, link))
    }

    /// Verify an email using a raw token string.
    /// Returns the user_id on success.
    pub async fn verify_email(&self, raw_token: &str) -> Result<String> {
        let store = self.identity_engine.verification_store();
        let token_hash = Self::hash_token(raw_token);

        let token_info = store.find_valid_token(&token_hash).await?.ok_or_else(|| {
            AppError::BadRequest("Invalid or expired verification token.".to_string())
        })?;

        // Mark token as used (replay protection)
        store.mark_token_used(&token_info.id).await?;

        // Update verification status
        store.set_email_verified(&token_info.user_id, true).await?;

        Ok(token_info.user_id)
    }

    /// Check if a user's email is verified.
    pub async fn is_verified(&self, user_id: &str) -> Result<bool> {
        self.identity_engine
            .verification_store()
            .get_verification_status(user_id)
            .await
    }

    /// Get the portal URL for building links.
    pub fn portal_url(&self) -> &str {
        &self.portal_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_token_produces_unique_pairs() {
        let (raw1, hash1) = EmailVerificationService::generate_token();
        let (raw2, hash2) = EmailVerificationService::generate_token();

        assert_ne!(raw1, raw2);
        assert_ne!(hash1, hash2);
        // Raw token is URL-safe base64, roughly 43 chars for 32 bytes
        assert!(raw1.len() >= 40);
    }

    #[test]
    fn hash_token_is_deterministic() {
        let raw = "test-token-value";
        let h1 = EmailVerificationService::hash_token(raw);
        let h2 = EmailVerificationService::hash_token(raw);
        assert_eq!(h1, h2);
        // SHA-256 hex is 64 chars
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hash_token_differs_for_different_inputs() {
        let h1 = EmailVerificationService::hash_token("token-a");
        let h2 = EmailVerificationService::hash_token("token-b");
        assert_ne!(h1, h2);
    }

    #[tokio::test]
    async fn build_verification_link_format() {
        use crate::identity_engine::adapters::auth9_oidc::Auth9OidcIdentityEngineAdapter;
        use crate::repository::social_provider::MockSocialProviderRepository;

        let pool = sqlx::MySqlPool::connect_lazy("mysql://fake:fake@localhost/fake").unwrap();
        let social_repo: Arc<dyn crate::repository::SocialProviderRepository> =
            Arc::new(MockSocialProviderRepository::new());
        let engine: Arc<dyn IdentityEngine> =
            Arc::new(Auth9OidcIdentityEngineAdapter::new(pool, social_repo, None));

        let service = EmailVerificationService::new(engine, "https://auth.example.com".to_string());

        let link = service.build_verification_link("abc123");
        assert_eq!(link, "https://auth.example.com/verify-email?token=abc123");
    }
}
