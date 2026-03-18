//! TOTP (Time-based One-Time Password) service
//!
//! Handles TOTP enrollment, verification, and lifecycle management.
//! Uses the local credential store (auth9-oidc) for secret storage
//! and Redis for enrollment state and replay protection.

use crate::cache::CacheOperations;
use crate::crypto::{self, EncryptionKey};
use crate::error::{AppError, Result};
use auth9_oidc::models::credential::{CreateCredentialInput, CredentialType, TotpCredentialData};
use auth9_oidc::repository::credential::CredentialRepository;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use totp_rs::{Algorithm, Secret, TOTP};

/// TOTP enrollment response returned to the client
#[derive(Debug, Serialize)]
pub struct TotpEnrollmentResponse {
    /// Setup token for completing enrollment
    pub setup_token: String,
    /// otpauth:// URI for QR code generation
    pub otpauth_uri: String,
    /// Base32-encoded secret for manual entry
    pub secret: String,
}

/// TOTP enrollment setup data stored in Redis
#[derive(Debug, Serialize, Deserialize)]
struct TotpSetupData {
    user_id: String,
    secret_base32: String,
}

const TOTP_DIGITS: usize = 6;
const TOTP_PERIOD: u64 = 30;
const TOTP_SKEW: u8 = 1; // Allow ±1 time step
const TOTP_SETUP_TTL_SECS: u64 = 300;
const TOTP_REPLAY_TTL_SECS: u64 = 90;

pub struct TotpService {
    credential_repo: Arc<dyn CredentialRepository>,
    cache: Arc<dyn CacheOperations>,
    encryption_key: EncryptionKey,
}

impl TotpService {
    pub fn new(
        credential_repo: Arc<dyn CredentialRepository>,
        cache: Arc<dyn CacheOperations>,
        encryption_key: EncryptionKey,
    ) -> Self {
        Self {
            credential_repo,
            cache,
            encryption_key,
        }
    }

    /// Generate a random 160-bit (20 byte) TOTP secret
    fn generate_secret() -> Vec<u8> {
        let mut secret = vec![0u8; 20];
        rand::thread_rng().fill_bytes(&mut secret);
        secret
    }

    /// Build a TOTP instance from a raw secret
    fn build_totp(secret_bytes: &[u8], email: &str) -> Result<TOTP> {
        let secret = Secret::Raw(secret_bytes.to_vec());
        TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_PERIOD,
            secret.to_bytes().map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Invalid TOTP secret: {}", e))
            })?,
            Some("Auth9".to_string()),
            email.to_string(),
        )
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create TOTP: {}", e)))
    }

    /// Start TOTP enrollment: generate secret, store temporarily, return URI
    pub async fn start_enrollment(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<TotpEnrollmentResponse> {
        let secret_bytes = Self::generate_secret();
        let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret_bytes);
        let totp = Self::build_totp(&secret_bytes, email)?;
        let otpauth_uri = totp.get_url();

        let setup_token = uuid::Uuid::new_v4().to_string();
        let setup_data = TotpSetupData {
            user_id: user_id.to_string(),
            secret_base32: secret_base32.clone(),
        };
        let setup_json = serde_json::to_string(&setup_data)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize setup data: {}", e)))?;

        self.cache
            .store_totp_setup(&setup_token, &setup_json, TOTP_SETUP_TTL_SECS)
            .await?;

        Ok(TotpEnrollmentResponse {
            setup_token,
            otpauth_uri,
            secret: secret_base32,
        })
    }

    /// Complete TOTP enrollment: verify code, store encrypted secret
    pub async fn complete_enrollment(
        &self,
        user_id: &str,
        setup_token: &str,
        code: &str,
    ) -> Result<()> {
        // Retrieve and consume setup state
        let setup_json = self
            .cache
            .get_totp_setup(setup_token)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(
                    "No pending TOTP enrollment found. Please start enrollment again.".to_string(),
                )
            })?;

        let setup_data: TotpSetupData = serde_json::from_str(&setup_json)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse setup data: {}", e)))?;

        // Verify the setup token belongs to this user
        if setup_data.user_id != user_id {
            return Err(AppError::BadRequest(
                "Setup token does not belong to this user.".to_string(),
            ));
        }

        // Decode base32 secret
        let secret_bytes = base32::decode(
            base32::Alphabet::Rfc4648 { padding: false },
            &setup_data.secret_base32,
        )
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Invalid base32 secret")))?;

        // Verify the code
        let totp = Self::build_totp(&secret_bytes, "")?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if !totp.check(code, now) {
            return Err(AppError::BadRequest(
                "Invalid TOTP code. Please check your authenticator app and try again.".to_string(),
            ));
        }

        // Remove setup state
        self.cache.remove_totp_setup(setup_token).await?;

        // Encrypt the secret for storage
        let encrypted_secret = crypto::encrypt(&self.encryption_key, &setup_data.secret_base32)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encrypt TOTP secret: {}", e)))?;

        // Delete any existing TOTP credential for this user
        let _ = self
            .credential_repo
            .delete_by_user_and_type(user_id, CredentialType::Totp)
            .await;

        // Store new TOTP credential
        let credential_data = serde_json::to_value(TotpCredentialData {
            secret_encrypted: encrypted_secret,
            algorithm: "SHA1".to_string(),
            digits: TOTP_DIGITS as u8,
            period: TOTP_PERIOD as u32,
        })
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize credential data: {}", e)))?;

        self.credential_repo
            .create(&CreateCredentialInput {
                user_id: user_id.to_string(),
                credential_type: CredentialType::Totp,
                credential_data,
                user_label: Some("TOTP".to_string()),
            })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store TOTP credential: {}", e)))?;

        Ok(())
    }

    /// Verify a TOTP code with clock skew tolerance and replay protection
    pub async fn verify_code(&self, user_id: &str, code: &str) -> Result<bool> {
        // Load TOTP credential
        let credentials = self
            .credential_repo
            .find_by_user_and_type(user_id, CredentialType::Totp)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to load TOTP credential: {}", e)))?;

        let credential = match credentials.into_iter().find(|c| c.is_active) {
            Some(c) => c,
            None => return Ok(false),
        };

        let totp_data: TotpCredentialData = credential.parse_totp_data().map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to parse TOTP credential data: {}", e))
        })?;

        // Decrypt the secret
        let secret_base32 =
            crypto::decrypt(&self.encryption_key, &totp_data.secret_encrypted).map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to decrypt TOTP secret: {}", e))
            })?;

        let secret_bytes = base32::decode(
            base32::Alphabet::Rfc4648 { padding: false },
            &secret_base32,
        )
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Invalid base32 secret in credential")))?;

        let totp = Self::build_totp(&secret_bytes, "")?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if !totp.check(code, now) {
            return Ok(false);
        }

        // Replay protection: check if this time step was already used
        let time_step = now / TOTP_PERIOD;
        // Check current and adjacent time steps (since skew=1, any of t-1, t, t+1 could match)
        for step in [time_step.saturating_sub(1), time_step, time_step + 1] {
            if self.cache.is_totp_code_used(user_id, step).await? {
                // A code in this window was already consumed
                return Err(AppError::BadRequest(
                    "This TOTP code has already been used. Please wait for a new code.".to_string(),
                ));
            }
        }

        // Mark the current time step as used
        self.cache
            .mark_totp_code_used(user_id, time_step, TOTP_REPLAY_TTL_SECS)
            .await?;

        Ok(true)
    }

    /// Remove TOTP credential for a user
    pub async fn remove_totp(&self, user_id: &str) -> Result<()> {
        self.credential_repo
            .delete_by_user_and_type(user_id, CredentialType::Totp)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to remove TOTP credential: {}", e)))?;
        Ok(())
    }

    /// Check if user has an active TOTP credential
    pub async fn has_totp(&self, user_id: &str) -> Result<bool> {
        let credentials = self
            .credential_repo
            .find_by_user_and_type(user_id, CredentialType::Totp)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to check TOTP credentials: {}", e)))?;

        Ok(credentials.iter().any(|c| c.is_active))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoOpCacheManager;
    use auth9_oidc::models::credential::{Credential, CreateCredentialInput, CredentialType as CT};
    use auth9_oidc::repository::credential::CredentialRepository;
    use chrono::Utc;

    // Define mock locally since auth9-oidc's mock is behind #[cfg(test)] which only
    // applies when auth9-oidc itself is compiled in test mode.
    mockall::mock! {
        pub CredRepo {}
        #[async_trait::async_trait]
        impl CredentialRepository for CredRepo {
            async fn create(&self, input: &CreateCredentialInput) -> auth9_oidc::error::Result<Credential>;
            async fn find_by_id(&self, id: &str) -> auth9_oidc::error::Result<Option<Credential>>;
            async fn find_by_user_and_type(&self, user_id: &str, credential_type: CT) -> auth9_oidc::error::Result<Vec<Credential>>;
            async fn update_data(&self, id: &str, data: &serde_json::Value) -> auth9_oidc::error::Result<()>;
            async fn deactivate(&self, id: &str) -> auth9_oidc::error::Result<()>;
            async fn activate(&self, id: &str) -> auth9_oidc::error::Result<()>;
            async fn delete(&self, id: &str) -> auth9_oidc::error::Result<()>;
            async fn delete_all_by_user(&self, user_id: &str) -> auth9_oidc::error::Result<u64>;
            async fn delete_by_user_and_type(&self, user_id: &str, credential_type: CT) -> auth9_oidc::error::Result<u64>;
        }
    }

    fn test_encryption_key() -> EncryptionKey {
        EncryptionKey::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ])
    }

    fn create_test_service(mock_repo: MockCredRepo) -> TotpService {
        TotpService::new(
            Arc::new(mock_repo),
            Arc::new(NoOpCacheManager::new()),
            test_encryption_key(),
        )
    }

    #[test]
    fn test_generate_secret_length() {
        let secret = TotpService::generate_secret();
        assert_eq!(secret.len(), 20);
    }

    #[test]
    fn test_generate_secret_unique() {
        let s1 = TotpService::generate_secret();
        let s2 = TotpService::generate_secret();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_build_totp() {
        let secret = TotpService::generate_secret();
        let totp = TotpService::build_totp(&secret, "test@example.com");
        assert!(totp.is_ok());
    }

    #[test]
    fn test_totp_generate_and_verify() {
        let secret = TotpService::generate_secret();
        let totp = TotpService::build_totp(&secret, "test@example.com").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code = totp.generate(now);
        assert_eq!(code.len(), 6);
        assert!(totp.check(&code, now));
    }

    #[test]
    fn test_totp_clock_skew_plus_one() {
        let secret = TotpService::generate_secret();
        // Build with skew=1
        let totp = TotpService::build_totp(&secret, "test@example.com").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Generate code for one period in the future
        let future_code = totp.generate(now + TOTP_PERIOD);
        // Should be accepted at current time due to skew tolerance
        assert!(totp.check(&future_code, now));
    }

    #[test]
    fn test_totp_clock_skew_minus_one() {
        let secret = TotpService::generate_secret();
        let totp = TotpService::build_totp(&secret, "test@example.com").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Generate code for one period in the past
        let past_code = totp.generate(now.saturating_sub(TOTP_PERIOD));
        // Should be accepted at current time due to skew tolerance
        assert!(totp.check(&past_code, now));
    }

    #[test]
    fn test_totp_outside_skew_rejected() {
        let secret = TotpService::generate_secret();
        let totp = TotpService::build_totp(&secret, "test@example.com").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Generate code for two periods in the future (outside skew)
        let far_future_code = totp.generate(now + TOTP_PERIOD * 2);
        assert!(!totp.check(&far_future_code, now));
    }

    #[test]
    fn test_totp_wrong_code_rejected() {
        let secret = TotpService::generate_secret();
        let totp = TotpService::build_totp(&secret, "test@example.com").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(!totp.check("000000", now));
    }

    #[tokio::test]
    async fn test_start_enrollment() {
        let mock_repo = MockCredRepo::new();
        let service = create_test_service(mock_repo);

        let result = service
            .start_enrollment("user-1", "test@example.com")
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(!response.setup_token.is_empty());
        assert!(response.otpauth_uri.starts_with("otpauth://totp/"));
        assert!(!response.secret.is_empty());
    }

    #[tokio::test]
    async fn test_has_totp_false_when_no_credentials() {
        let mut mock_repo = MockCredRepo::new();
        mock_repo
            .expect_find_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::Totp)
            .returning(|_, _| Ok(vec![]));

        let service = create_test_service(mock_repo);
        let result = service.has_totp("user-1").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_has_totp_true_when_active_credential() {
        let mut mock_repo = MockCredRepo::new();
        mock_repo
            .expect_find_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::Totp)
            .returning(|_, _| {
                Ok(vec![Credential {
                    id: "cred-1".to_string(),
                    user_id: "user-1".to_string(),
                    credential_type: CredentialType::Totp,
                    credential_data: serde_json::json!({}),
                    user_label: Some("TOTP".to_string()),
                    is_active: true,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                }])
            });

        let service = create_test_service(mock_repo);
        let result = service.has_totp("user-1").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_has_totp_false_when_inactive_credential() {
        let mut mock_repo = MockCredRepo::new();
        mock_repo
            .expect_find_by_user_and_type()
            .returning(|_, _| {
                Ok(vec![Credential {
                    id: "cred-1".to_string(),
                    user_id: "user-1".to_string(),
                    credential_type: CredentialType::Totp,
                    credential_data: serde_json::json!({}),
                    user_label: None,
                    is_active: false,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                }])
            });

        let service = create_test_service(mock_repo);
        let result = service.has_totp("user-1").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_code_no_credential() {
        let mut mock_repo = MockCredRepo::new();
        mock_repo
            .expect_find_by_user_and_type()
            .returning(|_, _| Ok(vec![]));

        let service = create_test_service(mock_repo);
        let result = service.verify_code("user-1", "123456").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_complete_enrollment_no_setup_state() {
        let mock_repo = MockCredRepo::new();
        let service = create_test_service(mock_repo);

        let result = service
            .complete_enrollment("user-1", "nonexistent-token", "123456")
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No pending TOTP enrollment"));
    }

    #[tokio::test]
    async fn test_remove_totp() {
        let mut mock_repo = MockCredRepo::new();
        mock_repo
            .expect_delete_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::Totp)
            .returning(|_, _| Ok(1));

        let service = create_test_service(mock_repo);
        let result = service.remove_totp("user-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enrollment_and_verify_roundtrip() {
        let mut mock_repo = MockCredRepo::new();

        // Capture the stored credential data for verification
        let stored_data = Arc::new(std::sync::Mutex::new(None::<serde_json::Value>));
        let stored_data_clone = stored_data.clone();

        mock_repo
            .expect_delete_by_user_and_type()
            .returning(|_, _| Ok(0));

        mock_repo.expect_create().returning(move |input| {
            let mut guard = stored_data_clone.lock().unwrap();
            *guard = Some(input.credential_data.clone());
            Ok(Credential {
                id: "cred-new".to_string(),
                user_id: input.user_id.clone(),
                credential_type: input.credential_type,
                credential_data: input.credential_data.clone(),
                user_label: input.user_label.clone(),
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let encryption_key = test_encryption_key();
        let cache = Arc::new(NoOpCacheManager::new());
        let service = TotpService::new(Arc::new(mock_repo), cache.clone(), encryption_key.clone());

        // Start enrollment
        let enrollment = service
            .start_enrollment("user-1", "test@example.com")
            .await
            .unwrap();

        // Generate a valid code from the secret
        let secret_bytes = base32::decode(
            base32::Alphabet::Rfc4648 { padding: false },
            &enrollment.secret,
        )
        .unwrap();
        let totp = TotpService::build_totp(&secret_bytes, "").unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code = totp.generate(now);

        // Complete enrollment with valid code
        let result = service
            .complete_enrollment("user-1", &enrollment.setup_token, &code)
            .await;
        assert!(result.is_ok());

        // Verify credential was stored with encrypted secret
        let guard = stored_data.lock().unwrap();
        let data = guard.as_ref().unwrap();
        let totp_data: TotpCredentialData = serde_json::from_value(data.clone()).unwrap();
        assert_eq!(totp_data.algorithm, "SHA1");
        assert_eq!(totp_data.digits, 6);
        assert_eq!(totp_data.period, 30);
        assert!(!totp_data.secret_encrypted.is_empty());

        // Verify the encrypted secret can be decrypted back
        let decrypted = crypto::decrypt(&encryption_key, &totp_data.secret_encrypted).unwrap();
        assert_eq!(decrypted, enrollment.secret);
    }
}
