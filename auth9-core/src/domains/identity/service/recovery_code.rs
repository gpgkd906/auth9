//! Recovery code service
//!
//! Generates, stores, verifies, and consumes recovery codes.
//! Each code is SHA-256 hashed before storage. Codes are one-time use.

use crate::error::{AppError, Result};
use auth9_oidc::models::credential::{CreateCredentialInput, CredentialType, RecoveryCodeData};
use auth9_oidc::repository::credential::CredentialRepository;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const RECOVERY_CODE_COUNT: usize = 8;
const RECOVERY_CODE_LENGTH: usize = 10;
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

pub struct RecoveryCodeService {
    credential_repo: Arc<dyn CredentialRepository>,
}

impl RecoveryCodeService {
    pub fn new(credential_repo: Arc<dyn CredentialRepository>) -> Self {
        Self { credential_repo }
    }

    /// Generate a single random recovery code (10 chars, a-z0-9)
    fn generate_single_code() -> String {
        let mut rng = rand::thread_rng();
        (0..RECOVERY_CODE_LENGTH)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect()
    }

    /// Hash a recovery code with SHA-256
    fn hash_code(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Generate a new set of recovery codes for a user.
    /// Replaces any existing recovery codes.
    /// Returns the plaintext codes (displayed once to the user).
    pub async fn generate_codes(&self, user_id: &str) -> Result<Vec<String>> {
        // Delete existing recovery codes
        let _ = self
            .credential_repo
            .delete_by_user_and_type(user_id, CredentialType::RecoveryCode)
            .await;

        let mut plaintext_codes = Vec::with_capacity(RECOVERY_CODE_COUNT);

        for _ in 0..RECOVERY_CODE_COUNT {
            let code = Self::generate_single_code();
            let code_hash = Self::hash_code(&code);

            let credential_data = serde_json::to_value(RecoveryCodeData {
                code_hash,
                used: false,
            })
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "Failed to serialize recovery code data: {}",
                    e
                ))
            })?;

            self.credential_repo
                .create(&CreateCredentialInput {
                    user_id: user_id.to_string(),
                    credential_type: CredentialType::RecoveryCode,
                    credential_data,
                    user_label: None,
                })
                .await
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Failed to store recovery code: {}", e))
                })?;

            plaintext_codes.push(code);
        }

        Ok(plaintext_codes)
    }

    /// Verify and consume a recovery code. Returns true if valid and consumed.
    pub async fn verify_and_consume(&self, user_id: &str, code: &str) -> Result<bool> {
        let code_hash = Self::hash_code(code);

        let credentials = self
            .credential_repo
            .find_by_user_and_type(user_id, CredentialType::RecoveryCode)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to load recovery codes: {}", e))
            })?;

        for credential in credentials {
            if !credential.is_active {
                continue;
            }

            let data: RecoveryCodeData = match credential.parse_recovery_code_data() {
                Ok(d) => d,
                Err(_) => continue,
            };

            if data.used {
                continue;
            }

            if data.code_hash == code_hash {
                // Mark as used
                let updated_data = serde_json::to_value(RecoveryCodeData {
                    code_hash: data.code_hash,
                    used: true,
                })
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!(
                        "Failed to serialize updated recovery code: {}",
                        e
                    ))
                })?;

                self.credential_repo
                    .update_data(&credential.id, &updated_data)
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to update recovery code: {}", e))
                    })?;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Count remaining unused recovery codes
    pub async fn remaining_count(&self, user_id: &str) -> Result<usize> {
        let credentials = self
            .credential_repo
            .find_by_user_and_type(user_id, CredentialType::RecoveryCode)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to load recovery codes: {}", e))
            })?;

        let count = credentials
            .iter()
            .filter(|c| c.is_active)
            .filter_map(|c| c.parse_recovery_code_data().ok())
            .filter(|d| !d.used)
            .count();

        Ok(count)
    }

    /// Revoke all recovery codes for a user
    pub async fn revoke_all(&self, user_id: &str) -> Result<()> {
        self.credential_repo
            .delete_by_user_and_type(user_id, CredentialType::RecoveryCode)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to revoke recovery codes: {}", e))
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth9_oidc::models::credential::{CreateCredentialInput, Credential, CredentialType as CT};
    use auth9_oidc::repository::credential::CredentialRepository;
    use chrono::Utc;
    use std::collections::HashSet;

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

    fn make_recovery_credential(id: &str, code_hash: &str, used: bool) -> Credential {
        Credential {
            id: id.to_string(),
            user_id: "user-1".to_string(),
            credential_type: CredentialType::RecoveryCode,
            credential_data: serde_json::to_value(RecoveryCodeData {
                code_hash: code_hash.to_string(),
                used,
            })
            .unwrap(),
            user_label: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_generate_single_code_format() {
        let code = RecoveryCodeService::generate_single_code();
        assert_eq!(code.len(), RECOVERY_CODE_LENGTH);
        assert!(code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_codes_unique() {
        let codes: HashSet<String> = (0..100)
            .map(|_| RecoveryCodeService::generate_single_code())
            .collect();
        // With 36^10 possible codes, 100 draws should all be unique
        assert_eq!(codes.len(), 100);
    }

    #[test]
    fn test_hash_code_deterministic() {
        let hash1 = RecoveryCodeService::hash_code("testcode01");
        let hash2 = RecoveryCodeService::hash_code("testcode01");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_code_different_inputs() {
        let hash1 = RecoveryCodeService::hash_code("testcode01");
        let hash2 = RecoveryCodeService::hash_code("testcode02");
        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_generate_codes_count() {
        let mut mock = MockCredRepo::new();
        mock.expect_delete_by_user_and_type()
            .returning(|_, _| Ok(0));
        mock.expect_create().returning(|input| {
            Ok(Credential {
                id: uuid::Uuid::new_v4().to_string(),
                user_id: input.user_id.clone(),
                credential_type: input.credential_type,
                credential_data: input.credential_data.clone(),
                user_label: None,
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let service = RecoveryCodeService::new(Arc::new(mock));
        let codes = service.generate_codes("user-1").await.unwrap();
        assert_eq!(codes.len(), RECOVERY_CODE_COUNT);

        // All codes should be unique
        let unique: HashSet<&String> = codes.iter().collect();
        assert_eq!(unique.len(), RECOVERY_CODE_COUNT);
    }

    #[tokio::test]
    async fn test_verify_and_consume_valid_code() {
        let code = "testcode01";
        let code_hash = RecoveryCodeService::hash_code(code);

        let mut mock = MockCredRepo::new();
        mock.expect_find_by_user_and_type()
            .returning(move |_, _| Ok(vec![make_recovery_credential("cred-1", &code_hash, false)]));
        mock.expect_update_data()
            .withf(|id, _| id == "cred-1")
            .returning(|_, _| Ok(()));

        let service = RecoveryCodeService::new(Arc::new(mock));
        let result = service.verify_and_consume("user-1", code).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_verify_and_consume_already_used() {
        let code = "testcode01";
        let code_hash = RecoveryCodeService::hash_code(code);

        let mut mock = MockCredRepo::new();
        mock.expect_find_by_user_and_type()
            .returning(move |_, _| Ok(vec![make_recovery_credential("cred-1", &code_hash, true)]));

        let service = RecoveryCodeService::new(Arc::new(mock));
        let result = service.verify_and_consume("user-1", code).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_and_consume_wrong_code() {
        let code_hash = RecoveryCodeService::hash_code("correctcode");

        let mut mock = MockCredRepo::new();
        mock.expect_find_by_user_and_type()
            .returning(move |_, _| Ok(vec![make_recovery_credential("cred-1", &code_hash, false)]));

        let service = RecoveryCodeService::new(Arc::new(mock));
        let result = service
            .verify_and_consume("user-1", "wrongcode0")
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_verify_and_consume_no_codes() {
        let mut mock = MockCredRepo::new();
        mock.expect_find_by_user_and_type()
            .returning(|_, _| Ok(vec![]));

        let service = RecoveryCodeService::new(Arc::new(mock));
        let result = service
            .verify_and_consume("user-1", "anycode000")
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_remaining_count() {
        let hash1 = RecoveryCodeService::hash_code("code1");
        let hash2 = RecoveryCodeService::hash_code("code2");
        let hash3 = RecoveryCodeService::hash_code("code3");

        let mut mock = MockCredRepo::new();
        mock.expect_find_by_user_and_type().returning(move |_, _| {
            Ok(vec![
                make_recovery_credential("c1", &hash1, false),
                make_recovery_credential("c2", &hash2, true), // used
                make_recovery_credential("c3", &hash3, false),
            ])
        });

        let service = RecoveryCodeService::new(Arc::new(mock));
        let count = service.remaining_count("user-1").await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_revoke_all() {
        let mut mock = MockCredRepo::new();
        mock.expect_delete_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::RecoveryCode)
            .returning(|_, _| Ok(8));

        let service = RecoveryCodeService::new(Arc::new(mock));
        let result = service.revoke_all("user-1").await;
        assert!(result.is_ok());
    }
}
