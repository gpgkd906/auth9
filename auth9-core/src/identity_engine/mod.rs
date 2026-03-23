use crate::error::Result;
use async_trait::async_trait;

pub mod adapters;
mod types;

pub use types::{
    IdentityCredentialInput, IdentityCredentialRepresentation,
    IdentityProtocolMapperRepresentation, IdentityProviderRepresentation,
    IdentitySamlClientRepresentation, IdentityUserCreateInput, IdentityUserRepresentation,
    IdentityUserUpdateInput, OidcClientRepresentation, PendingActionInfo, RealmSettingsUpdate,
    VerificationTokenInfo,
};

/// User lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityUserStore: Send + Sync {
    async fn create_user(&self, input: &IdentityUserCreateInput) -> Result<String>;
    async fn get_user(&self, user_id: &str) -> Result<IdentityUserRepresentation>;
    async fn update_user(&self, user_id: &str, input: &IdentityUserUpdateInput) -> Result<()>;
    async fn delete_user(&self, user_id: &str) -> Result<()>;
    async fn set_user_password(&self, user_id: &str, password: &str, temporary: bool)
        -> Result<()>;
    async fn admin_set_user_password(
        &self,
        user_id: &str,
        password: &str,
        temporary: bool,
    ) -> Result<()>;
    async fn validate_user_password(&self, user_id: &str, password: &str) -> Result<bool>;
    /// Get the current password hash for a user (for password history storage).
    /// Returns None if the user has no password credential.
    async fn get_user_password_hash(&self, user_id: &str) -> Result<Option<String>>;
}

/// OIDC/SAML client lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityClientStore: Send + Sync {
    async fn create_oidc_client(&self, client: &OidcClientRepresentation) -> Result<String>;
    async fn get_client_secret(&self, client_uuid: &str) -> Result<String>;
    async fn regenerate_client_secret(&self, client_uuid: &str) -> Result<String>;
    async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String>;
    async fn get_client_by_client_id(&self, client_id: &str) -> Result<OidcClientRepresentation>;
    async fn update_oidc_client(
        &self,
        client_uuid: &str,
        client: &OidcClientRepresentation,
    ) -> Result<()>;
    async fn delete_oidc_client(&self, client_uuid: &str) -> Result<()>;
    async fn create_saml_client(&self, client: &IdentitySamlClientRepresentation)
        -> Result<String>;
    async fn update_saml_client(
        &self,
        client_uuid: &str,
        client: &IdentitySamlClientRepresentation,
    ) -> Result<()>;
    async fn delete_saml_client(&self, client_uuid: &str) -> Result<()>;
    async fn get_saml_idp_descriptor(&self) -> Result<String>;
    async fn get_active_signing_certificate(&self) -> Result<String>;
    fn saml_sso_url(&self) -> String;
}

/// Session lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentitySessionStore: Send + Sync {
    async fn delete_user_session(&self, session_id: &str) -> Result<()>;
    async fn logout_user(&self, user_id: &str) -> Result<()>;
}

/// Credential lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityCredentialStore: Send + Sync {
    async fn list_user_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>>;
    async fn remove_totp_credentials(&self, user_id: &str) -> Result<()>;
    async fn list_webauthn_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>>;
    async fn delete_user_credential(&self, user_id: &str, credential_id: &str) -> Result<()>;
    /// Check if the user's password credential is marked as temporary.
    async fn is_password_temporary(&self, user_id: &str) -> Result<bool>;
}

/// Federation and broker management operations for an identity backend.
///
/// Note: User federated identity operations (get/remove) have been removed.
/// Auth9 now owns `linked_identities` as primary data via the repository layer.
#[async_trait]
pub trait FederationBroker: Send + Sync {
    async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>>;
    async fn get_identity_provider(&self, alias: &str) -> Result<IdentityProviderRepresentation>;
    async fn create_identity_provider(
        &self,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()>;
    async fn update_identity_provider(
        &self,
        alias: &str,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()>;
    async fn delete_identity_provider(&self, alias: &str) -> Result<()>;
}

/// Pending action lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityActionStore: Send + Sync {
    async fn get_pending_actions(&self, user_id: &str) -> Result<Vec<PendingActionInfo>>;
    async fn create_action(
        &self,
        user_id: &str,
        action_type: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<String>;
    async fn complete_action(&self, action_id: &str) -> Result<()>;
    async fn cancel_action(&self, action_id: &str) -> Result<()>;
}

/// Email verification operations for an identity backend.
#[async_trait]
pub trait IdentityVerificationStore: Send + Sync {
    async fn get_verification_status(&self, user_id: &str) -> Result<bool>;
    async fn set_email_verified(&self, user_id: &str, verified: bool) -> Result<()>;
    async fn create_verification_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<VerificationTokenInfo>;
    async fn find_valid_token(&self, token_hash: &str) -> Result<Option<VerificationTokenInfo>>;
    async fn mark_token_used(&self, token_id: &str) -> Result<()>;
    async fn invalidate_user_tokens(&self, user_id: &str) -> Result<u64>;
}

/// Event ingestion surface for an identity backend.
#[async_trait]
pub trait IdentityEventSource: Send + Sync {}

/// Top-level identity backend handle exposed through application state.
#[async_trait]
pub trait IdentityEngine: Send + Sync {
    fn user_store(&self) -> &dyn IdentityUserStore;
    fn client_store(&self) -> &dyn IdentityClientStore;
    fn session_store(&self) -> &dyn IdentitySessionStore;
    fn credential_store(&self) -> &dyn IdentityCredentialStore;
    fn federation_broker(&self) -> &dyn FederationBroker;
    fn event_source(&self) -> &dyn IdentityEventSource;
    fn action_store(&self) -> &dyn IdentityActionStore;
    fn verification_store(&self) -> &dyn IdentityVerificationStore;

    async fn update_realm(&self, settings: &RealmSettingsUpdate) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_engine::adapters::auth9_oidc::Auth9OidcIdentityEngineAdapter;
    use crate::repository::social_provider::MockSocialProviderRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn auth9_oidc_adapter_exposes_identity_engine_surfaces() {
        let pool = sqlx::MySqlPool::connect_lazy("mysql://fake:fake@localhost/fake").unwrap();
        let social_repo: Arc<dyn crate::repository::SocialProviderRepository> =
            Arc::new(MockSocialProviderRepository::new());
        let adapter = Auth9OidcIdentityEngineAdapter::new(pool, social_repo, None);
        let engine: &dyn IdentityEngine = &adapter;

        let _ = engine.user_store();
        let _ = engine.client_store();
        let _ = engine.session_store();
        let _ = engine.credential_store();
        let _ = engine.federation_broker();
        let _ = engine.event_source();
        let _ = engine.action_store();
        let _ = engine.verification_store();
    }
}
