//! Password management business logic

use crate::domain::{
    ChangePasswordInput, CreatePasswordResetTokenInput, ForgotPasswordInput, PasswordPolicy,
    ResetPasswordInput, StringUuid, UpdatePasswordPolicyInput,
};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::repository::{
    PasswordResetRepository, SystemSettingsRepository, TenantRepository, UserRepository,
};
use crate::service::EmailService;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use std::sync::Arc;
use validator::Validate;

pub struct PasswordService<
    P: PasswordResetRepository,
    U: UserRepository,
    S: SystemSettingsRepository,
    T: TenantRepository = crate::repository::tenant::TenantRepositoryImpl,
> {
    password_reset_repo: Arc<P>,
    user_repo: Arc<U>,
    email_service: Arc<EmailService<S>>,
    keycloak: Arc<KeycloakClient>,
    tenant_repo: Option<Arc<T>>,
    hmac_key: String,
}

impl<P: PasswordResetRepository, U: UserRepository, S: SystemSettingsRepository>
    PasswordService<P, U, S>
{
    pub fn new(
        password_reset_repo: Arc<P>,
        user_repo: Arc<U>,
        email_service: Arc<EmailService<S>>,
        keycloak: Arc<KeycloakClient>,
        hmac_key: String,
    ) -> Self {
        Self {
            password_reset_repo,
            user_repo,
            email_service,
            keycloak,
            tenant_repo: None,
            hmac_key,
        }
    }
}

impl<
        P: PasswordResetRepository,
        U: UserRepository,
        S: SystemSettingsRepository,
        T: TenantRepository,
    > PasswordService<P, U, S, T>
{
    pub fn with_tenant_repo(
        password_reset_repo: Arc<P>,
        user_repo: Arc<U>,
        email_service: Arc<EmailService<S>>,
        keycloak: Arc<KeycloakClient>,
        tenant_repo: Arc<T>,
        hmac_key: String,
    ) -> Self {
        Self {
            password_reset_repo,
            user_repo,
            email_service,
            keycloak,
            tenant_repo: Some(tenant_repo),
            hmac_key,
        }
    }

    /// Request a password reset email
    pub async fn request_reset(&self, input: ForgotPasswordInput) -> Result<()> {
        input.validate()?;

        // Find user by email
        let user = match self.user_repo.find_by_email(&input.email).await? {
            Some(u) => u,
            None => {
                // Don't reveal whether email exists
                return Ok(());
            }
        };

        // Generate a secure random token
        let token = generate_reset_token();
        let token_hash = hash_token(&token, self.hmac_key.as_bytes())?;

        // Atomically delete old tokens and create new one (prevents race condition)
        let expires_at = Utc::now() + Duration::hours(1);
        self.password_reset_repo
            .replace_for_user(&CreatePasswordResetTokenInput {
                user_id: user.id,
                token_hash,
                expires_at,
            })
            .await?;

        // Send the reset email
        // The token is passed to the email template which builds the reset URL
        self.email_service
            .send_password_reset(&input.email, &token, user.display_name.as_deref())
            .await?;

        Ok(())
    }

    /// Reset password using a token
    pub async fn reset_password(&self, input: ResetPasswordInput) -> Result<()> {
        input.validate()?;

        // Hash the provided token to look up in database
        let token_hash = hash_token(&input.token, self.hmac_key.as_bytes())?;

        // Find valid token
        let reset_token = self
            .password_reset_repo
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".to_string()))?;

        // Get the user
        let user = self
            .user_repo
            .find_by_id(reset_token.user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Reset password in Keycloak
        self.keycloak
            .reset_user_password(&user.keycloak_id, &input.new_password, false)
            .await?;

        // Mark token as used
        self.password_reset_repo.mark_used(reset_token.id).await?;

        // Send password changed notification
        self.email_service
            .send_password_changed(&user.email, user.display_name.as_deref())
            .await?;

        Ok(())
    }

    /// Change password for authenticated user
    pub async fn change_password(
        &self,
        user_id: StringUuid,
        input: ChangePasswordInput,
    ) -> Result<()> {
        input.validate()?;

        // Get the user
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Verify current password with Keycloak
        let is_valid = self
            .keycloak
            .validate_user_password(&user.keycloak_id, &input.current_password)
            .await?;

        if !is_valid {
            return Err(AppError::BadRequest(
                "Current password is incorrect".to_string(),
            ));
        }

        // Set new password in Keycloak
        self.keycloak
            .reset_user_password(&user.keycloak_id, &input.new_password, false)
            .await?;

        // Send password changed notification
        self.email_service
            .send_password_changed(&user.email, user.display_name.as_deref())
            .await?;

        Ok(())
    }

    /// Validate a password against a policy
    pub fn validate_against_policy(&self, password: &str, policy: &PasswordPolicy) -> Result<()> {
        match policy.validate_password(password) {
            Ok(()) => Ok(()),
            Err(errors) => Err(AppError::Validation(errors.join("; "))),
        }
    }

    /// Get password policy for a tenant
    pub async fn get_policy(&self, tenant_id: StringUuid) -> Result<PasswordPolicy> {
        if let Some(ref tenant_repo) = self.tenant_repo {
            let tenant = tenant_repo
                .find_by_id(tenant_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", tenant_id)))?;
            Ok(tenant.password_policy.unwrap_or_default())
        } else {
            Ok(PasswordPolicy::default())
        }
    }

    /// Update password policy for a tenant
    pub async fn update_policy(
        &self,
        tenant_id: StringUuid,
        input: UpdatePasswordPolicyInput,
    ) -> Result<PasswordPolicy> {
        input.validate()?;

        // Get current policy
        let current = self.get_policy(tenant_id).await?;

        // Apply updates
        let updated = PasswordPolicy {
            min_length: input.min_length.unwrap_or(current.min_length),
            require_uppercase: input.require_uppercase.unwrap_or(current.require_uppercase),
            require_lowercase: input.require_lowercase.unwrap_or(current.require_lowercase),
            require_numbers: input.require_numbers.unwrap_or(current.require_numbers),
            require_symbols: input.require_symbols.unwrap_or(current.require_symbols),
            max_age_days: input.max_age_days.unwrap_or(current.max_age_days),
            history_count: input.history_count.unwrap_or(current.history_count),
            lockout_threshold: input.lockout_threshold.unwrap_or(current.lockout_threshold),
            lockout_duration_mins: input
                .lockout_duration_mins
                .unwrap_or(current.lockout_duration_mins),
        };

        if let Some(ref tenant_repo) = self.tenant_repo {
            tenant_repo
                .update_password_policy(tenant_id, &updated)
                .await?;
        }

        Ok(updated)
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired_tokens(&self) -> Result<u64> {
        self.password_reset_repo.delete_expired().await
    }
}

/// Generate a secure random reset token
fn generate_reset_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Hash a token for secure storage using HMAC-SHA256
/// Uses a deterministic hash so the same token always produces the same hash,
/// enabling lookup by hash in the database.
fn hash_token(token: &str, key: &[u8]) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("HMAC init error: {}", e)))?;
    mac.update(token.as_bytes());
    let result = mac.finalize();
    Ok(hex::encode(result.into_bytes()))
}

/// Verify a token against a hash (legacy argon2 support)
#[allow(dead_code)]
fn verify_token(token: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid hash format: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(token.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PasswordResetToken;
    use crate::repository::password_reset::MockPasswordResetRepository;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use crate::repository::user::MockUserRepository;
    use crate::service::SystemSettingsService;
    use mockall::predicate::*;

    #[test]
    fn test_generate_reset_token() {
        let token1 = generate_reset_token();
        let token2 = generate_reset_token();

        // Token should be 64 characters (32 bytes hex encoded)
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Tokens should be unique
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_hash_token_deterministic() {
        let token = generate_reset_token();
        let key = b"test-hmac-key";
        let hash1 = hash_token(&token, key).unwrap();
        let hash2 = hash_token(&token, key).unwrap();

        // HMAC-SHA256 hash should be a hex string (64 chars = 32 bytes)
        assert_eq!(hash1.len(), 64);

        // Same token should produce same hash (deterministic)
        assert_eq!(hash1, hash2);

        // Different token should produce different hash
        let other_token = generate_reset_token();
        let other_hash = hash_token(&other_token, key).unwrap();
        assert_ne!(hash1, other_hash);
    }

    #[test]
    fn test_hash_token_different_keys() {
        let token = "test-token-123";
        let hash1 = hash_token(token, b"key1").unwrap();
        let hash2 = hash_token(token, b"key2").unwrap();
        assert_ne!(hash1, hash2, "Different keys should produce different hashes");
    }

    #[test]
    fn test_password_policy_validation() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            ..Default::default()
        };

        // Valid password
        assert!(policy.validate_password("StrongP@ss1").is_ok());

        // Too short
        let result = policy.validate_password("Sh0rt!");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("at least 8")));

        // Missing uppercase
        let result = policy.validate_password("weakpass1!");
        assert!(result.is_err());

        // Missing number
        let result = policy.validate_password("StrongPass!");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_policy_symbols() {
        let policy = PasswordPolicy {
            min_length: 6,
            require_uppercase: false,
            require_lowercase: false,
            require_numbers: false,
            require_symbols: true,
            ..Default::default()
        };

        // With symbol
        assert!(policy.validate_password("test!@").is_ok());

        // Without symbol
        let result = policy.validate_password("testab");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("symbol")));
    }

    #[test]
    fn test_password_policy_default() {
        let policy = PasswordPolicy::default();

        // Default policy requires uppercase + lowercase + numbers
        assert!(policy.validate_password("Simple123").is_ok());
        assert!(policy.validate_password("simple123").is_err()); // missing uppercase
        assert!(policy.validate_password("SIMPLE123").is_err()); // missing lowercase
        assert!(policy.validate_password("Simpleabc").is_err()); // missing number
    }

    #[test]
    fn test_validate_against_policy_standalone() {
        let policy = PasswordPolicy {
            min_length: 10,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            ..Default::default()
        };

        // Valid password
        assert!(policy.validate_password("ValidP@ss1234").is_ok());

        // Invalid password
        assert!(policy.validate_password("weak").is_err());
    }

    #[tokio::test]
    async fn test_request_reset_user_not_found() {
        let password_reset_mock = MockPasswordResetRepository::new();
        let mut user_mock = MockUserRepository::new();

        // User not found - should return Ok (don't reveal if email exists)
        user_mock
            .expect_find_by_email()
            .with(eq("notfound@example.com"))
            .returning(|_| Ok(None));

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let input = ForgotPasswordInput {
            email: "notfound@example.com".to_string(),
        };

        // Should succeed (not reveal user doesn't exist)
        let result = service.request_reset(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_reset_invalid_email() {
        let password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let input = ForgotPasswordInput {
            email: "invalid-email".to_string(),
        };

        // Should fail validation
        let result = service.request_reset(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reset_password_invalid_token() {
        let mut password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        // Token not found
        password_reset_mock
            .expect_find_by_token_hash()
            .returning(|_| Ok(None));

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let input = ResetPasswordInput {
            token: "invalid-token".to_string(),
            new_password: "NewPassword123!".to_string(),
        };

        let result = service.reset_password(input).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_reset_password_user_not_found() {
        let mut password_reset_mock = MockPasswordResetRepository::new();
        let mut user_mock = MockUserRepository::new();

        let user_id = StringUuid::new_v4();

        // Token found
        password_reset_mock
            .expect_find_by_token_hash()
            .returning(move |_| {
                Ok(Some(PasswordResetToken {
                    user_id,
                    ..Default::default()
                }))
            });

        // User not found
        user_mock
            .expect_find_by_id()
            .with(eq(user_id))
            .returning(|_| Ok(None));

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let input = ResetPasswordInput {
            token: "valid-token".to_string(),
            new_password: "NewPassword123!".to_string(),
        };

        let result = service.reset_password(input).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_change_password_user_not_found() {
        let password_reset_mock = MockPasswordResetRepository::new();
        let mut user_mock = MockUserRepository::new();

        let user_id = StringUuid::new_v4();

        user_mock
            .expect_find_by_id()
            .with(eq(user_id))
            .returning(|_| Ok(None));

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let input = ChangePasswordInput {
            current_password: "current123".to_string(),
            new_password: "NewPassword123!".to_string(),
        };

        let result = service.change_password(user_id, input).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() {
        let mut password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        password_reset_mock
            .expect_delete_expired()
            .returning(|| Ok(10));

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let count = service.cleanup_expired_tokens().await.unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_get_policy_returns_default() {
        let password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let tenant_id = StringUuid::new_v4();
        let policy = service.get_policy(tenant_id).await.unwrap();

        // Should return default policy
        assert_eq!(policy, PasswordPolicy::default());
    }

    #[tokio::test]
    async fn test_update_policy() {
        let password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        let (service, _) = create_test_password_service(password_reset_mock, user_mock);

        let tenant_id = StringUuid::new_v4();
        let input = UpdatePasswordPolicyInput {
            min_length: Some(12),
            require_uppercase: Some(true),
            require_lowercase: Some(true),
            require_numbers: Some(true),
            require_symbols: Some(true),
            max_age_days: None,
            history_count: None,
            lockout_threshold: None,
            lockout_duration_mins: None,
        };

        let policy = service.update_policy(tenant_id, input).await.unwrap();

        assert_eq!(policy.min_length, 12);
        assert!(policy.require_uppercase);
        assert!(policy.require_lowercase);
        assert!(policy.require_numbers);
        assert!(policy.require_symbols);
    }

    #[test]
    fn test_password_policy_min_length_only() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: false,
            require_lowercase: false,
            require_numbers: false,
            require_symbols: false,
            ..Default::default()
        };

        assert!(policy.validate_password("abcdefgh").is_ok());
        assert!(policy.validate_password("short").is_err());
    }

    #[test]
    fn test_password_policy_all_requirements() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            ..Default::default()
        };

        // All requirements met
        assert!(policy.validate_password("Test123!@").is_ok());

        // Missing symbol
        assert!(policy.validate_password("Test1234").is_err());
    }

    // Helper to create a test PasswordService with mock dependencies
    fn create_test_password_service(
        password_reset_mock: MockPasswordResetRepository,
        user_mock: MockUserRepository,
    ) -> (
        PasswordService<
            MockPasswordResetRepository,
            MockUserRepository,
            MockSystemSettingsRepository,
        >,
        Arc<KeycloakClient>,
    ) {
        let system_settings_mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(
            Arc::new(system_settings_mock),
            None, // No encryption key for tests
        ));
        let email_service = Arc::new(EmailService::new(settings_service));
        let keycloak = Arc::new(create_test_keycloak_client());

        let service = PasswordService::new(
            Arc::new(password_reset_mock),
            Arc::new(user_mock),
            email_service,
            keycloak.clone(),
            "test-password-reset-hmac-key".to_string(),
        );

        (service, keycloak)
    }

    // Helper to create a test KeycloakClient
    fn create_test_keycloak_client() -> KeycloakClient {
        use crate::config::KeycloakConfig;
        KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8081".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        })
    }

    // ========================================================================
    // Security Fix Tests: Configurable HMAC Key
    // ========================================================================

    #[test]
    fn test_hash_token_with_custom_key() {
        // Test that custom keys work correctly
        let token = "reset-token-12345";
        let key1 = b"production-key-1";
        let key2 = b"production-key-2";

        let hash1 = hash_token(token, key1).unwrap();
        let hash2 = hash_token(token, key2).unwrap();

        // Same token with different keys produces different hashes
        assert_ne!(hash1, hash2);

        // Same token with same key produces same hash
        let hash1_again = hash_token(token, key1).unwrap();
        assert_eq!(hash1, hash1_again);
    }

    #[test]
    fn test_hash_token_key_rotation() {
        // Simulate key rotation scenario
        let token = "user-reset-token";
        let old_key = b"old-production-key";
        let new_key = b"new-production-key";

        let old_hash = hash_token(token, old_key).unwrap();
        let new_hash = hash_token(token, new_key).unwrap();

        // After key rotation, same token produces different hash
        assert_ne!(old_hash, new_hash);

        // Old key still produces old hash (for transition period)
        let old_hash_verify = hash_token(token, old_key).unwrap();
        assert_eq!(old_hash, old_hash_verify);
    }

    #[tokio::test]
    async fn test_password_service_with_custom_hmac_key() {
        // Test that PasswordService uses the configured HMAC key
        let password_reset_mock = MockPasswordResetRepository::new();
        let user_mock = MockUserRepository::new();

        let system_settings_mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(
            Arc::new(system_settings_mock),
            None,
        ));
        let email_service = Arc::new(EmailService::new(settings_service));
        let keycloak = Arc::new(create_test_keycloak_client());

        // Create service with custom key
        let custom_key = "my-secure-production-key";
        let service = PasswordService::new(
            Arc::new(password_reset_mock),
            Arc::new(user_mock),
            email_service,
            keycloak,
            custom_key.to_string(),
        );

        // Verify service was created (HMAC key is stored internally)
        assert_eq!(service.hmac_key, custom_key);
    }
}
