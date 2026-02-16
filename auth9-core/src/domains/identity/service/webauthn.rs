//! WebAuthn/Passkey service
//!
//! Native WebAuthn registration and authentication using webauthn-rs.
//! During migration period, also supports listing/deleting Keycloak credentials.

use crate::cache::CacheOperations;
use crate::domain::{CreatePasskeyInput, WebAuthnCredential};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::repository::webauthn::WebAuthnRepository;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::sync::Arc;
use webauthn_rs::prelude::*;

/// Authentication result returned after successful passkey verification
#[derive(Debug)]
pub struct PasskeyAuthResult {
    /// The user_id from the stored credential
    pub user_id: String,
    /// The stored passkey ID (for updating last_used_at)
    pub stored_passkey_id: String,
}

/// Authentication start result
pub struct AuthenticationStartResult {
    /// Random challenge identifier for retrieving state later
    pub challenge_id: String,
    /// The challenge response to send to the browser
    pub options: RequestChallengeResponse,
}

pub struct WebAuthnService {
    webauthn: Arc<Webauthn>,
    repo: Arc<dyn WebAuthnRepository>,
    cache: Arc<dyn CacheOperations>,
    keycloak: Option<Arc<KeycloakClient>>,
    challenge_ttl_secs: u64,
}

impl WebAuthnService {
    pub fn new(
        webauthn: Arc<Webauthn>,
        repo: Arc<dyn WebAuthnRepository>,
        cache: Arc<dyn CacheOperations>,
        keycloak: Option<Arc<KeycloakClient>>,
        challenge_ttl_secs: u64,
    ) -> Self {
        Self {
            webauthn,
            repo,
            cache,
            keycloak,
            challenge_ttl_secs,
        }
    }

    // ==================== Registration ====================

    /// Start passkey registration ceremony
    ///
    /// Returns `CreationChallengeResponse` to send to the browser.
    /// The registration state is stored in Redis keyed by user_id.
    pub async fn start_registration(
        &self,
        user_id: &str,
        email: &str,
        display_name: Option<&str>,
    ) -> Result<CreationChallengeResponse> {
        // Build exclude list from existing credentials (CredentialIDs)
        let existing = self.repo.list_by_user(user_id).await?;
        let exclude_credentials: Vec<CredentialID> = existing
            .iter()
            .filter_map(|stored| {
                serde_json::from_value::<Passkey>(stored.credential_data.clone())
                    .ok()
                    .map(|pk| pk.cred_id().clone())
            })
            .collect();

        let user_unique_id = Uuid::parse_str(user_id)
            .map_err(|_| AppError::BadRequest("Invalid user_id format".to_string()))?;

        let exclude = if exclude_credentials.is_empty() {
            None
        } else {
            Some(exclude_credentials)
        };

        let (ccr, reg_state) = self
            .webauthn
            .start_passkey_registration(
                user_unique_id,
                email,
                display_name.unwrap_or(email),
                exclude,
            )
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("WebAuthn registration start failed: {}", e))
            })?;

        // Serialize registration state and store in Redis
        let state_json = serde_json::to_string(&reg_state).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to serialize registration state: {}",
                e
            ))
        })?;

        self.cache
            .store_webauthn_reg_state(user_id, &state_json, self.challenge_ttl_secs)
            .await?;

        Ok(ccr)
    }

    /// Complete passkey registration ceremony
    ///
    /// Verifies the browser's attestation response and stores the credential in TiDB.
    pub async fn complete_registration(
        &self,
        user_id: &str,
        credential: &RegisterPublicKeyCredential,
        label: Option<String>,
    ) -> Result<WebAuthnCredential> {
        // Retrieve registration state from Redis
        let state_json = self
            .cache
            .get_webauthn_reg_state(user_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(
                    "No pending registration found. Please start registration again.".to_string(),
                )
            })?;

        let reg_state: PasskeyRegistration = serde_json::from_str(&state_json).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to deserialize registration state: {}",
                e
            ))
        })?;

        // Remove state from Redis (one-time use)
        self.cache.remove_webauthn_reg_state(user_id).await?;

        // Verify attestation
        let passkey = self
            .webauthn
            .finish_passkey_registration(credential, &reg_state)
            .map_err(|e| {
                AppError::BadRequest(format!("WebAuthn registration verification failed: {}", e))
            })?;

        // Serialize the Passkey for storage
        let credential_data = serde_json::to_value(&passkey).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to serialize passkey: {}", e))
        })?;

        let credential_id_b64 = URL_SAFE_NO_PAD.encode(passkey.cred_id().as_ref());

        let input = CreatePasskeyInput {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            credential_id: credential_id_b64,
            credential_data,
            user_label: label,
            aaguid: None,
        };

        let stored = self.repo.create(&input).await?;
        Ok(stored.into())
    }

    // ==================== Authentication ====================

    /// Start discoverable passkey authentication
    ///
    /// Returns a challenge for the browser. No specific user is targeted -
    /// the browser will show available passkeys and the user picks one.
    pub async fn start_authentication(&self) -> Result<AuthenticationStartResult> {
        let (rcr, auth_state) = self
            .webauthn
            .start_discoverable_authentication()
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "WebAuthn authentication start failed: {}",
                    e
                ))
            })?;

        let challenge_id = uuid::Uuid::new_v4().to_string();

        let state_json = serde_json::to_string(&auth_state).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to serialize authentication state: {}",
                e
            ))
        })?;

        self.cache
            .store_webauthn_auth_state(&challenge_id, &state_json, self.challenge_ttl_secs)
            .await?;

        Ok(AuthenticationStartResult {
            challenge_id,
            options: rcr,
        })
    }

    /// Complete discoverable passkey authentication
    ///
    /// Verifies the browser's assertion response, looks up the credential in TiDB,
    /// and returns the associated user_id.
    pub async fn complete_authentication(
        &self,
        challenge_id: &str,
        credential: &PublicKeyCredential,
    ) -> Result<PasskeyAuthResult> {
        // Retrieve auth state from Redis
        let state_json = self
            .cache
            .get_webauthn_auth_state(challenge_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(
                    "No pending authentication found. Please start again.".to_string(),
                )
            })?;

        let auth_state: DiscoverableAuthentication =
            serde_json::from_str(&state_json).map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "Failed to deserialize authentication state: {}",
                    e
                ))
            })?;

        // Remove state from Redis (one-time use)
        self.cache.remove_webauthn_auth_state(challenge_id).await?;

        // Look up the credential by its ID
        // credential.raw_id is Base64UrlSafeData (binary), encode to base64url for DB lookup
        let cred_id_b64 = URL_SAFE_NO_PAD.encode(credential.raw_id.as_ref());

        let stored = self
            .repo
            .find_by_credential_id(&cred_id_b64)
            .await?
            .ok_or_else(|| {
                AppError::Unauthorized("Passkey not found. It may have been deleted.".to_string())
            })?;

        let mut passkey: Passkey =
            serde_json::from_value(stored.credential_data.clone()).map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "Failed to deserialize stored passkey: {}",
                    e
                ))
            })?;

        // Verify assertion using discoverable authentication
        let discoverable_key = DiscoverableKey::from(passkey.clone());
        let auth_result = self
            .webauthn
            .finish_discoverable_authentication(credential, auth_state, &[discoverable_key])
            .map_err(|e| {
                AppError::Unauthorized(format!("WebAuthn authentication failed: {}", e))
            })?;

        // Update counter in the passkey
        passkey.update_credential(&auth_result);

        // Persist updated credential data (counter increment)
        let updated_data = serde_json::to_value(&passkey).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to serialize updated passkey: {}",
                e
            ))
        })?;

        self.repo
            .update_credential_data(&stored.id, &updated_data)
            .await?;

        // Update last_used_at
        self.repo.update_last_used(&stored.id).await?;

        Ok(PasskeyAuthResult {
            user_id: stored.user_id,
            stored_passkey_id: stored.id,
        })
    }

    // ==================== Management ====================

    /// List credentials for a user (native + Keycloak during migration)
    pub async fn list_credentials(
        &self,
        user_id: &str,
        keycloak_user_id: Option<&str>,
    ) -> Result<Vec<WebAuthnCredential>> {
        // Native credentials from TiDB
        let native_creds: Vec<WebAuthnCredential> = self
            .repo
            .list_by_user(user_id)
            .await?
            .into_iter()
            .map(WebAuthnCredential::from)
            .collect();

        // Keycloak credentials (migration period)
        let mut all_creds = native_creds;
        if let (Some(kc), Some(kc_user_id)) = (&self.keycloak, keycloak_user_id) {
            if let Ok(kc_credentials) = kc.list_webauthn_credentials(kc_user_id).await {
                let kc_creds: Vec<WebAuthnCredential> = kc_credentials
                    .into_iter()
                    .map(|c| WebAuthnCredential {
                        id: format!("kc_{}", c.id),
                        credential_type: c.credential_type,
                        user_label: c.user_label,
                        created_at: c.created_date.map(|ts| {
                            chrono::DateTime::from_timestamp_millis(ts)
                                .unwrap_or_else(chrono::Utc::now)
                        }),
                    })
                    .collect();
                all_creds.extend(kc_creds);
            }
        }

        Ok(all_creds)
    }

    /// Delete a credential (handles both native and Keycloak)
    pub async fn delete_credential(
        &self,
        user_id: &str,
        credential_id: &str,
        keycloak_user_id: Option<&str>,
    ) -> Result<()> {
        // Check if it's a Keycloak credential (prefixed with kc_)
        if let Some(kc_id) = credential_id.strip_prefix("kc_") {
            if let (Some(kc), Some(kc_user_id)) = (&self.keycloak, keycloak_user_id) {
                return kc.delete_user_credential(kc_user_id, kc_id).await;
            }
            return Err(AppError::BadRequest(
                "Cannot delete Keycloak credential: Keycloak not configured".to_string(),
            ));
        }

        // Native credential
        self.repo.delete(credential_id, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoOpCacheManager;
    use crate::domain::StoredPasskey;
    use crate::repository::webauthn::MockWebAuthnRepository;

    fn create_test_webauthn() -> Arc<Webauthn> {
        let rp_id = "localhost";
        let rp_origin = url::Url::parse("http://localhost:3000").unwrap();
        let builder = WebauthnBuilder::new(rp_id, &rp_origin)
            .unwrap()
            .rp_name("Test Auth9");
        Arc::new(builder.build().unwrap())
    }

    fn create_test_service(mock_repo: MockWebAuthnRepository) -> WebAuthnService {
        WebAuthnService::new(
            create_test_webauthn(),
            Arc::new(mock_repo),
            Arc::new(NoOpCacheManager::new()),
            None,
            300,
        )
    }

    #[test]
    fn test_service_creation() {
        let mock_repo = MockWebAuthnRepository::new();
        let service = create_test_service(mock_repo);
        assert_eq!(service.challenge_ttl_secs, 300);
    }

    #[tokio::test]
    async fn test_start_registration() {
        let mut mock_repo = MockWebAuthnRepository::new();
        mock_repo.expect_list_by_user().returning(|_| Ok(vec![]));

        let service = create_test_service(mock_repo);

        let user_id = uuid::Uuid::new_v4().to_string();
        let result = service
            .start_registration(&user_id, "test@example.com", Some("Test User"))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_registration_invalid_user_id() {
        let mut mock_repo = MockWebAuthnRepository::new();
        mock_repo.expect_list_by_user().returning(|_| Ok(vec![]));

        let service = create_test_service(mock_repo);

        let result = service
            .start_registration("not-a-uuid", "test@example.com", None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_authentication() {
        let mock_repo = MockWebAuthnRepository::new();
        let service = create_test_service(mock_repo);

        let result = service.start_authentication().await;
        assert!(result.is_ok());

        let auth_start = result.unwrap();
        assert!(!auth_start.challenge_id.is_empty());
    }

    #[tokio::test]
    async fn test_complete_registration_no_pending_state() {
        let mock_repo = MockWebAuthnRepository::new();
        let service = create_test_service(mock_repo);

        // Create a dummy credential with valid base64url data (will fail at state lookup)
        let cred_json = serde_json::json!({
            "id": "dGVzdA",
            "rawId": "dGVzdA",
            "type": "public-key",
            "response": {
                "attestationObject": "o2NmbXRkbm9uZQ",
                "clientDataJSON": "eyJ0eXBlIjoiIn0"
            }
        });
        let credential: RegisterPublicKeyCredential =
            serde_json::from_value(cred_json).expect("test credential JSON should parse");

        let user_id = uuid::Uuid::new_v4().to_string();
        // NoOpCacheManager returns None for get, so this should fail
        // with "No pending registration found"
        let result = service
            .complete_registration(&user_id, &credential, None)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No pending registration"));
    }

    #[tokio::test]
    async fn test_complete_authentication_no_pending_state() {
        let mock_repo = MockWebAuthnRepository::new();
        let service = create_test_service(mock_repo);

        let cred_json = serde_json::json!({
            "id": "dGVzdA",
            "rawId": "dGVzdA",
            "type": "public-key",
            "response": {
                "authenticatorData": "dGVzdA",
                "clientDataJSON": "eyJ0eXBlIjoiIn0",
                "signature": "dGVzdA"
            }
        });
        let credential: PublicKeyCredential =
            serde_json::from_value(cred_json).expect("test credential JSON should parse");

        let result = service
            .complete_authentication("nonexistent-challenge", &credential)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No pending authentication"));
    }

    #[tokio::test]
    async fn test_list_credentials_native_only() {
        let mut mock_repo = MockWebAuthnRepository::new();
        mock_repo.expect_list_by_user().returning(|_| {
            Ok(vec![StoredPasskey {
                id: "pk-1".to_string(),
                user_id: "user-1".to_string(),
                credential_id: "cred-1".to_string(),
                credential_data: serde_json::json!({}),
                user_label: Some("My Key".to_string()),
                aaguid: None,
                created_at: chrono::Utc::now(),
                last_used_at: None,
            }])
        });

        let service = create_test_service(mock_repo);
        let result = service.list_credentials("user-1", None).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "pk-1");
        assert_eq!(result[0].credential_type, "webauthn");
    }

    #[tokio::test]
    async fn test_list_credentials_empty() {
        let mut mock_repo = MockWebAuthnRepository::new();
        mock_repo.expect_list_by_user().returning(|_| Ok(vec![]));

        let service = create_test_service(mock_repo);
        let result = service.list_credentials("user-1", None).await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_delete_native_credential() {
        let mut mock_repo = MockWebAuthnRepository::new();
        mock_repo
            .expect_delete()
            .withf(|id, user_id| id == "pk-1" && user_id == "user-1")
            .returning(|_, _| Ok(()));

        let service = create_test_service(mock_repo);
        let result = service.delete_credential("user-1", "pk-1", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_keycloak_credential_no_client() {
        let mock_repo = MockWebAuthnRepository::new();
        let service = create_test_service(mock_repo);

        // Trying to delete a kc_ prefixed credential without Keycloak client
        let result = service
            .delete_credential("user-1", "kc_some-kc-id", None)
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Keycloak not configured"));
    }

    #[test]
    fn test_passkey_auth_result() {
        let result = PasskeyAuthResult {
            user_id: "user-123".to_string(),
            stored_passkey_id: "pk-456".to_string(),
        };
        assert_eq!(result.user_id, "user-123");
        assert_eq!(result.stored_passkey_id, "pk-456");
    }

    #[test]
    fn test_auth_start_result() {
        // Minimal test for the struct
        let challenge_id = uuid::Uuid::new_v4().to_string();
        assert!(!challenge_id.is_empty());
    }
}
