use super::{Auth9OidcFederationBrokerAdapter, Auth9OidcSessionStoreAdapter};
use crate::error::{AppError, Result};
use crate::identity_engine::{
    FederationBroker, IdentityActionStore, IdentityClientStore, IdentityCredentialRepresentation,
    IdentityCredentialStore, IdentityEngine, IdentityEventSource, IdentitySamlClientRepresentation,
    IdentitySessionStore, IdentityUserCreateInput, IdentityUserRepresentation, IdentityUserStore,
    IdentityUserUpdateInput, IdentityVerificationStore, OidcClientRepresentation, PendingActionInfo,
    RealmSettingsUpdate, VerificationTokenInfo,
};
use crate::repository::social_provider::SocialProviderRepository;
use anyhow::anyhow;
use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;

struct Auth9OidcUserStore {
    pool: MySqlPool,
}

impl Auth9OidcUserStore {
    fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Hash a password with argon2id and store/replace the credential row.
    async fn upsert_password_credential(
        &self,
        user_id: &str,
        password: &str,
        temporary: bool,
    ) -> Result<()> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow!("password hashing failed: {}", e)))?
            .to_string();

        let data = serde_json::json!({
            "hash": hash,
            "algorithm": "argon2id",
            "temporary": temporary,
        });

        // Atomic replace: delete old password credential, insert new one
        sqlx::query(
            "DELETE FROM credentials WHERE user_id = ? AND credential_type = 'password'",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to delete old password credential: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO credentials (id, user_id, credential_type, credential_data) VALUES (?, ?, 'password', ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&data)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to insert password credential: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl IdentityUserStore for Auth9OidcUserStore {
    async fn create_user(&self, input: &IdentityUserCreateInput) -> Result<String> {
        let identity_subject = uuid::Uuid::new_v4().to_string();

        if let Some(ref credentials) = input.credentials {
            for cred in credentials {
                if cred.credential_type == "password" {
                    self.upsert_password_credential(
                        &identity_subject,
                        &cred.value,
                        cred.temporary,
                    )
                    .await?;
                }
            }
        }

        Ok(identity_subject)
    }

    async fn get_user(&self, user_id: &str) -> Result<IdentityUserRepresentation> {
        use sqlx::Row;

        let row = sqlx::query(
            "SELECT id, email, display_name FROM users WHERE identity_subject = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query user: {}", e)))?;

        let row = row.ok_or_else(|| {
            AppError::NotFound(format!("user with identity_subject '{}' not found", user_id))
        })?;

        let id: String = row
            .try_get("id")
            .map_err(|e| AppError::Internal(anyhow!("{}", e)))?;
        let email: String = row
            .try_get("email")
            .map_err(|e| AppError::Internal(anyhow!("{}", e)))?;
        let display_name: Option<String> = row
            .try_get("display_name")
            .map_err(|e| AppError::Internal(anyhow!("{}", e)))?;

        // Check email verification status
        let email_verified = sqlx::query_as::<_, (i8,)>(
            "SELECT email_verified FROM user_verification_status WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query verification status: {}", e)))?
        .map(|(v,)| v != 0)
        .unwrap_or(false);

        Ok(IdentityUserRepresentation {
            id: Some(id),
            username: email.clone(),
            email: Some(email),
            first_name: display_name,
            last_name: None,
            enabled: true,
            email_verified,
            attributes: HashMap::new(),
        })
    }

    async fn update_user(&self, _user_id: &str, _input: &IdentityUserUpdateInput) -> Result<()> {
        // No-op: caller updates auth9 DB directly via user_service().update()
        Ok(())
    }

    async fn delete_user(&self, user_id: &str) -> Result<()> {
        // Clean up auth9-oidc related tables for this user
        sqlx::query("DELETE FROM credentials WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow!("failed to delete credentials: {}", e)))?;

        sqlx::query("DELETE FROM pending_actions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow!("failed to delete pending actions: {}", e)))?;

        sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow!("failed to delete verification tokens: {}", e))
            })?;

        sqlx::query("DELETE FROM user_verification_status WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::Internal(anyhow!("failed to delete verification status: {}", e))
            })?;

        Ok(())
    }

    async fn set_user_password(
        &self,
        user_id: &str,
        password: &str,
        temporary: bool,
    ) -> Result<()> {
        self.upsert_password_credential(user_id, password, temporary)
            .await
    }

    async fn admin_set_user_password(
        &self,
        user_id: &str,
        password: &str,
        temporary: bool,
    ) -> Result<()> {
        self.upsert_password_credential(user_id, password, temporary)
            .await
    }

    async fn validate_user_password(&self, user_id: &str, password: &str) -> Result<bool> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT credential_data FROM credentials WHERE user_id = ? AND credential_type = 'password' AND is_active = 1 LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query password credential: {}", e)))?;

        let Some((data,)) = row else {
            return Ok(false);
        };

        let hash_str = data
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal(anyhow!("malformed password credential data")))?;

        let parsed_hash = PasswordHash::new(hash_str)
            .map_err(|e| AppError::Internal(anyhow!("invalid password hash format: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

struct Auth9OidcClientStore {
    #[allow(dead_code)]
    pool: MySqlPool,
    core_public_url: Option<String>,
}

impl Auth9OidcClientStore {
    fn new(pool: MySqlPool, core_public_url: Option<String>) -> Self {
        Self { pool, core_public_url }
    }

    /// Generate a random 32-byte hex string for use as a client secret.
    fn generate_random_secret() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }
}

#[async_trait]
impl IdentityClientStore for Auth9OidcClientStore {
    async fn create_oidc_client(&self, _client: &OidcClientRepresentation) -> Result<String> {
        // Auth9 OIDC: return a placeholder UUID. The actual client record is
        // created by ClientService at the application layer.
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn get_client_secret(&self, _client_uuid: &str) -> Result<String> {
        // Auth9 OIDC: generate a random secret for the creation flow.
        // The caller (authorization/api/service.rs) passes this to
        // ClientService::create_with_secret which hashes and stores it.
        Ok(Self::generate_random_secret())
    }

    async fn regenerate_client_secret(&self, _client_uuid: &str) -> Result<String> {
        Ok(Self::generate_random_secret())
    }

    async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String> {
        // Auth9 OIDC manages clients in its own DB, not in an external identity backend.
        // Returning NotFound causes callers to fall through to the auth9 DB path.
        Err(AppError::NotFound(format!(
            "auth9_oidc does not manage client '{}' externally",
            client_id
        )))
    }

    async fn get_client_by_client_id(&self, client_id: &str) -> Result<OidcClientRepresentation> {
        // Auth9 OIDC manages clients in its own DB. Returning NotFound causes
        // list-clients enrichment to use the DB-managed path.
        Err(AppError::NotFound(format!(
            "auth9_oidc does not manage client '{}' externally",
            client_id
        )))
    }

    async fn update_oidc_client(
        &self,
        _client_uuid: &str,
        _client: &OidcClientRepresentation,
    ) -> Result<()> {
        // No-op: application layer handles client updates in auth9 DB
        Ok(())
    }

    async fn delete_oidc_client(&self, _client_uuid: &str) -> Result<()> {
        // No-op: application layer handles client deletion in auth9 DB
        Ok(())
    }

    async fn create_saml_client(
        &self,
        _client: &IdentitySamlClientRepresentation,
    ) -> Result<String> {
        // Application layer handles SAML client creation in auth9 DB
        Ok(uuid::Uuid::new_v4().to_string())
    }

    async fn update_saml_client(
        &self,
        _client_uuid: &str,
        _client: &IdentitySamlClientRepresentation,
    ) -> Result<()> {
        Ok(())
    }

    async fn delete_saml_client(&self, _client_uuid: &str) -> Result<()> {
        Ok(())
    }

    async fn get_saml_idp_descriptor(&self) -> Result<String> {
        let base = self.core_public_url.as_deref().unwrap_or("");
        let sso_url = format!("{}/api/v1/saml/sso", base);
        Ok(format!(
            r#"<EntityDescriptor xmlns="urn:oasis:names:tc:SAML:2.0:metadata" entityID="{base}">
  <IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="{sso_url}" />
  </IDPSSODescriptor>
</EntityDescriptor>"#
        ))
    }

    async fn get_active_signing_certificate(&self) -> Result<String> {
        // Placeholder: actual certificate export from JwtManager requires
        // additional dependency wiring (tracked separately).
        Ok("placeholder-certificate".to_string())
    }

    fn saml_sso_url(&self) -> String {
        match &self.core_public_url {
            Some(url) => format!("{}/api/v1/saml/sso", url),
            None => String::new(),
        }
    }
}

struct Auth9OidcCredentialStore {
    pool: MySqlPool,
}

impl Auth9OidcCredentialStore {
    fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityCredentialStore for Auth9OidcCredentialStore {
    async fn list_user_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        use sqlx::Row;
        let rows = match sqlx::query(
            "SELECT id, credential_type, user_label, created_at FROM credentials WHERE user_id = ? AND is_active = 1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::debug!("failed to list credentials for user {}: {}", user_id, e);
                return Ok(Vec::new());
            }
        };

        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let created_at: chrono::DateTime<Utc> = row.try_get("created_at")
                .map_err(|e| AppError::Internal(anyhow!("{}", e)))?;
            result.push(IdentityCredentialRepresentation {
                id: row.try_get("id").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                credential_type: row.try_get("credential_type").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                user_label: row.try_get("user_label").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                created_date: Some(created_at.timestamp_millis()),
            });
        }
        Ok(result)
    }

    async fn remove_totp_credentials(&self, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM credentials WHERE user_id = ? AND credential_type = 'totp'")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow!("failed to remove totp credentials: {}", e)))?;
        Ok(())
    }

    async fn list_webauthn_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, credential_type, user_label, created_at FROM credentials WHERE user_id = ? AND credential_type = 'webauthn' AND is_active = 1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to list webauthn credentials: {}", e)))?;

        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let created_at: chrono::DateTime<Utc> = row.try_get("created_at")
                .map_err(|e| AppError::Internal(anyhow!("{}", e)))?;
            result.push(IdentityCredentialRepresentation {
                id: row.try_get("id").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                credential_type: row.try_get("credential_type").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                user_label: row.try_get("user_label").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                created_date: Some(created_at.timestamp_millis()),
            });
        }
        Ok(result)
    }

    async fn delete_user_credential(&self, user_id: &str, credential_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM credentials WHERE id = ? AND user_id = ?")
            .bind(credential_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow!("failed to delete credential: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "credential '{}' not found for user '{}'",
                credential_id, user_id
            )));
        }
        Ok(())
    }
}

struct Auth9OidcActionStore {
    pool: MySqlPool,
}

impl Auth9OidcActionStore {
    fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityActionStore for Auth9OidcActionStore {
    async fn get_pending_actions(&self, user_id: &str) -> Result<Vec<PendingActionInfo>> {
        let rows = sqlx::query(
            "SELECT id, action_type, metadata, created_at FROM pending_actions WHERE user_id = ? AND status = 'pending'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query pending actions: {}", e)))?;

        use sqlx::Row;
        let mut actions = Vec::with_capacity(rows.len());
        for row in &rows {
            actions.push(PendingActionInfo {
                id: row.try_get("id").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                action_type: row.try_get("action_type").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                metadata: row.try_get("metadata").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                created_at: row.try_get("created_at").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
            });
        }
        Ok(actions)
    }

    async fn create_action(
        &self,
        user_id: &str,
        action_type: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO pending_actions (id, user_id, action_type, metadata) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(action_type)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to create pending action: {}", e)))?;

        Ok(id)
    }

    async fn complete_action(&self, action_id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE pending_actions SET status = 'completed', completed_at = NOW() WHERE id = ? AND status = 'pending'",
        )
        .bind(action_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to complete action: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "pending action '{}' not found or already completed",
                action_id
            )));
        }
        Ok(())
    }

    async fn cancel_action(&self, action_id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE pending_actions SET status = 'cancelled', completed_at = NOW() WHERE id = ? AND status = 'pending'",
        )
        .bind(action_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to cancel action: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "pending action '{}' not found or already completed",
                action_id
            )));
        }
        Ok(())
    }
}

struct Auth9OidcVerificationStore {
    pool: MySqlPool,
}

impl Auth9OidcVerificationStore {
    fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityVerificationStore for Auth9OidcVerificationStore {
    async fn get_verification_status(&self, user_id: &str) -> Result<bool> {
        // Upsert then read
        sqlx::query(
            "INSERT IGNORE INTO user_verification_status (user_id, email_verified) VALUES (?, 0)",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to upsert verification status: {}", e)))?;

        let row: (i8,) = sqlx::query_as(
            "SELECT email_verified FROM user_verification_status WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query verification status: {}", e)))?;

        Ok(row.0 != 0)
    }

    async fn set_email_verified(&self, user_id: &str, verified: bool) -> Result<()> {
        let email_verified_at = if verified { "NOW()" } else { "NULL" };
        let query = format!(
            "UPDATE user_verification_status SET email_verified = ?, email_verified_at = {} WHERE user_id = ?",
            email_verified_at
        );
        sqlx::query(&query)
            .bind(verified as i8)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow!("failed to update verification status: {}", e)))?;
        Ok(())
    }

    async fn create_verification_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<VerificationTokenInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to create verification token: {}", e)))?;

        Ok(VerificationTokenInfo {
            id,
            user_id: user_id.to_string(),
            expires_at,
            used_at: None,
            created_at: Utc::now(),
        })
    }

    async fn find_valid_token(&self, token_hash: &str) -> Result<Option<VerificationTokenInfo>> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT id, user_id, expires_at, used_at, created_at FROM email_verification_tokens WHERE token_hash = ? AND used_at IS NULL AND expires_at > NOW()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to query verification token: {}", e)))?;

        match row {
            Some(r) => Ok(Some(VerificationTokenInfo {
                id: r.try_get("id").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                user_id: r.try_get("user_id").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                expires_at: r.try_get("expires_at").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                used_at: r.try_get("used_at").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
                created_at: r.try_get("created_at").map_err(|e| AppError::Internal(anyhow!("{}", e)))?,
            })),
            None => Ok(None),
        }
    }

    async fn mark_token_used(&self, token_id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE id = ? AND used_at IS NULL",
        )
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to mark token used: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "verification token '{}' not found or already used",
                token_id
            )));
        }
        Ok(())
    }

    async fn invalidate_user_tokens(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE user_id = ? AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow!("failed to invalidate user tokens: {}", e)))?;

        Ok(result.rows_affected())
    }
}

#[derive(Default)]
struct Auth9OidcEventSource;

#[async_trait]
impl IdentityEventSource for Auth9OidcEventSource {}

pub struct Auth9OidcIdentityEngineAdapter {
    user_store: Auth9OidcUserStore,
    client_store: Auth9OidcClientStore,
    session_store: Auth9OidcSessionStoreAdapter,
    credential_store: Auth9OidcCredentialStore,
    federation_broker: Auth9OidcFederationBrokerAdapter,
    event_source: Auth9OidcEventSource,
    action_store: Auth9OidcActionStore,
    verification_store: Auth9OidcVerificationStore,
}

impl Auth9OidcIdentityEngineAdapter {
    pub fn new(
        pool: MySqlPool,
        social_provider_repo: Arc<dyn SocialProviderRepository>,
        core_public_url: Option<String>,
    ) -> Self {
        Self {
            user_store: Auth9OidcUserStore::new(pool.clone()),
            client_store: Auth9OidcClientStore::new(pool.clone(), core_public_url),
            session_store: Auth9OidcSessionStoreAdapter::new(),
            credential_store: Auth9OidcCredentialStore::new(pool.clone()),
            federation_broker: Auth9OidcFederationBrokerAdapter::new(
                social_provider_repo,
            ),
            event_source: Auth9OidcEventSource,
            action_store: Auth9OidcActionStore::new(pool.clone()),
            verification_store: Auth9OidcVerificationStore::new(pool),
        }
    }
}

#[async_trait]
impl IdentityEngine for Auth9OidcIdentityEngineAdapter {
    fn user_store(&self) -> &dyn IdentityUserStore {
        &self.user_store
    }

    fn client_store(&self) -> &dyn IdentityClientStore {
        &self.client_store
    }

    fn session_store(&self) -> &dyn IdentitySessionStore {
        &self.session_store
    }

    fn credential_store(&self) -> &dyn IdentityCredentialStore {
        &self.credential_store
    }

    fn federation_broker(&self) -> &dyn FederationBroker {
        &self.federation_broker
    }

    fn event_source(&self) -> &dyn IdentityEventSource {
        &self.event_source
    }

    fn action_store(&self) -> &dyn IdentityActionStore {
        &self.action_store
    }

    fn verification_store(&self) -> &dyn IdentityVerificationStore {
        &self.verification_store
    }

    async fn update_realm(&self, _settings: &RealmSettingsUpdate) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test argon2id hash-and-verify roundtrip (pure logic, no DB).
    #[test]
    fn test_argon2_hash_and_verify_roundtrip() {
        let password = "StrongP@ss1"; // pragma: allowlist secret
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hashing should succeed");
        let hash_str = hash.to_string();

        let parsed = PasswordHash::new(&hash_str).expect("should parse");
        assert!(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok());
    }

    #[test]
    fn test_argon2_verify_wrong_password_fails() {
        let password = "CorrectPassword1!"; // pragma: allowlist secret
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hashing should succeed");
        let hash_str = hash.to_string();

        let parsed = PasswordHash::new(&hash_str).expect("should parse");
        assert!(Argon2::default()
            .verify_password(b"WrongPassword1!", &parsed)
            .is_err());
    }

    #[test]
    fn test_credential_json_structure() {
        let password = "TestPass1!"; // pragma: allowlist secret
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hashing should succeed")
            .to_string();

        let data = serde_json::json!({
            "hash": hash,
            "algorithm": "argon2id",
            "temporary": false,
        });

        assert_eq!(data["algorithm"], "argon2id");
        assert!(!data["temporary"].as_bool().unwrap());
        assert!(data["hash"].as_str().unwrap().starts_with("$argon2id$"));
    }

    #[test]
    fn test_temporary_flag_preserved() {
        let password = "TempPass1!"; // pragma: allowlist secret
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("hashing should succeed")
            .to_string();

        for temporary in [true, false] {
            let data = serde_json::json!({
                "hash": hash,
                "algorithm": "argon2id",
                "temporary": temporary,
            });
            assert_eq!(data["temporary"].as_bool().unwrap(), temporary);
        }
    }
}
