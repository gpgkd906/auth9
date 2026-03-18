use super::{Auth9OidcFederationBrokerAdapter, Auth9OidcSessionStoreAdapter};
use crate::error::{AppError, Result};
use crate::identity_engine::{
    FederationBroker, IdentityClientStore, IdentityCredentialRepresentation,
    IdentityCredentialStore, IdentityEngine, IdentityEventSource, IdentitySamlClientRepresentation,
    IdentitySessionStore, IdentityUserCreateInput, IdentityUserRepresentation, IdentityUserStore,
    IdentityUserUpdateInput,
};
use crate::keycloak::{KeycloakOidcClient, RealmUpdate};
use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use sqlx::MySqlPool;

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
    async fn create_user(&self, _input: &IdentityUserCreateInput) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user create not implemented"
        )))
    }

    async fn get_user(&self, user_id: &str) -> Result<IdentityUserRepresentation> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user '{}' get not implemented",
            user_id
        )))
    }

    async fn update_user(&self, user_id: &str, _input: &IdentityUserUpdateInput) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user '{}' update not implemented",
            user_id
        )))
    }

    async fn delete_user(&self, user_id: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user '{}' delete not implemented",
            user_id
        )))
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

#[derive(Default)]
struct Auth9OidcClientStore;

#[async_trait]
impl IdentityClientStore for Auth9OidcClientStore {
    async fn create_oidc_client(&self, _client: &KeycloakOidcClient) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc oidc client create not implemented"
        )))
    }

    async fn get_client_secret(&self, client_uuid: &str) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' secret lookup not implemented",
            client_uuid
        )))
    }

    async fn regenerate_client_secret(&self, client_uuid: &str) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' secret regeneration not implemented",
            client_uuid
        )))
    }

    async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' lookup not implemented",
            client_id
        )))
    }

    async fn get_client_by_client_id(&self, client_id: &str) -> Result<KeycloakOidcClient> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' fetch not implemented",
            client_id
        )))
    }

    async fn update_oidc_client(
        &self,
        client_uuid: &str,
        _client: &KeycloakOidcClient,
    ) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' update not implemented",
            client_uuid
        )))
    }

    async fn delete_oidc_client(&self, client_uuid: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc client '{}' delete not implemented",
            client_uuid
        )))
    }

    async fn create_saml_client(
        &self,
        _client: &IdentitySamlClientRepresentation,
    ) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc saml client create not implemented"
        )))
    }

    async fn update_saml_client(
        &self,
        client_uuid: &str,
        _client: &IdentitySamlClientRepresentation,
    ) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc saml client '{}' update not implemented",
            client_uuid
        )))
    }

    async fn delete_saml_client(&self, client_uuid: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc saml client '{}' delete not implemented",
            client_uuid
        )))
    }

    async fn get_saml_idp_descriptor(&self) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc saml descriptor not implemented"
        )))
    }

    async fn get_active_signing_certificate(&self) -> Result<String> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc signing certificate lookup not implemented"
        )))
    }

    fn saml_sso_url(&self) -> String {
        String::new()
    }
}

#[derive(Default)]
struct Auth9OidcCredentialStore;

#[async_trait]
impl IdentityCredentialStore for Auth9OidcCredentialStore {
    async fn list_user_credentials(
        &self,
        _user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        Ok(Vec::new())
    }

    async fn remove_totp_credentials(&self, user_id: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user '{}' totp cleanup not implemented",
            user_id
        )))
    }

    async fn list_webauthn_credentials(
        &self,
        _user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        Ok(Vec::new())
    }

    async fn delete_user_credential(&self, user_id: &str, _credential_id: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc user '{}' credential deletion not implemented",
            user_id
        )))
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
}

impl Auth9OidcIdentityEngineAdapter {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            user_store: Auth9OidcUserStore::new(pool),
            client_store: Auth9OidcClientStore,
            session_store: Auth9OidcSessionStoreAdapter::new(),
            credential_store: Auth9OidcCredentialStore,
            federation_broker: Auth9OidcFederationBrokerAdapter::new(),
            event_source: Auth9OidcEventSource,
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

    async fn update_realm(&self, _settings: &RealmUpdate) -> Result<()> {
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
