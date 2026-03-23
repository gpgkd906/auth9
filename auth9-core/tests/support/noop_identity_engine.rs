//! No-op / stub implementations of all IdentityEngine sub-traits.
//!
//! Used in HTTP handler tests where the identity engine is wired into
//! `AppState` but never actually called.

use async_trait::async_trait;
use auth9_core::error::Result;
use auth9_core::identity_engine::{
    FederationBroker, IdentityActionStore, IdentityClientStore, IdentityCredentialStore,
    IdentityEngine, IdentityEventSource, IdentitySessionStore, IdentityUserStore,
    IdentityVerificationStore,
};
use auth9_core::identity_engine::{
    IdentityCredentialRepresentation, IdentityProviderRepresentation,
    IdentitySamlClientRepresentation, IdentityUserCreateInput, IdentityUserRepresentation,
    IdentityUserUpdateInput, OidcClientRepresentation, PendingActionInfo, RealmSettingsUpdate,
    VerificationTokenInfo,
};
use std::collections::HashMap;

// ============================================================================
// NoOpUserStore
// ============================================================================

pub struct NoOpUserStore;

#[async_trait]
impl IdentityUserStore for NoOpUserStore {
    async fn create_user(&self, _input: &IdentityUserCreateInput) -> Result<String> {
        Ok("test-id".to_string())
    }

    async fn get_user(&self, _user_id: &str) -> Result<IdentityUserRepresentation> {
        Ok(IdentityUserRepresentation {
            id: Some("test-id".to_string()),
            username: "test-user".to_string(),
            email: None,
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            attributes: HashMap::new(),
        })
    }

    async fn update_user(&self, _user_id: &str, _input: &IdentityUserUpdateInput) -> Result<()> {
        Ok(())
    }

    async fn delete_user(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }

    async fn set_user_password(
        &self,
        _user_id: &str,
        _password: &str,
        _temporary: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn admin_set_user_password(
        &self,
        _user_id: &str,
        _password: &str,
        _temporary: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn validate_user_password(&self, _user_id: &str, _password: &str) -> Result<bool> {
        Ok(true)
    }

    async fn get_user_password_hash(&self, _identity_subject: &str) -> Result<Option<String>> {
        Ok(None)
    }
}

// ============================================================================
// NoOpClientStore
// ============================================================================

pub struct NoOpClientStore;

#[async_trait]
impl IdentityClientStore for NoOpClientStore {
    async fn create_oidc_client(&self, _client: &OidcClientRepresentation) -> Result<String> {
        Ok("test-id".to_string())
    }

    async fn get_client_secret(&self, _client_uuid: &str) -> Result<String> {
        Ok("test-secret".to_string())
    }

    async fn regenerate_client_secret(&self, _client_uuid: &str) -> Result<String> {
        Ok("test-secret".to_string())
    }

    async fn get_client_uuid_by_client_id(&self, _client_id: &str) -> Result<String> {
        Ok("test-id".to_string())
    }

    async fn get_client_by_client_id(&self, _client_id: &str) -> Result<OidcClientRepresentation> {
        Ok(OidcClientRepresentation {
            id: Some("test-id".to_string()),
            client_id: "test-client".to_string(),
            name: None,
            enabled: true,
            public_client: false,
            redirect_uris: vec![],
            web_origins: vec![],
            secret: None,
            protocol: None,
            base_url: None,
            root_url: None,
            admin_url: None,
            attributes: None,
        })
    }

    async fn update_oidc_client(
        &self,
        _client_uuid: &str,
        _client: &OidcClientRepresentation,
    ) -> Result<()> {
        Ok(())
    }

    async fn delete_oidc_client(&self, _client_uuid: &str) -> Result<()> {
        Ok(())
    }

    async fn create_saml_client(
        &self,
        _client: &IdentitySamlClientRepresentation,
    ) -> Result<String> {
        Ok("test-id".to_string())
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
        Ok(String::new())
    }

    async fn get_active_signing_certificate(&self) -> Result<String> {
        Ok(String::new())
    }

    fn saml_sso_url(&self) -> String {
        "https://localhost/saml/sso".to_string()
    }
}

// ============================================================================
// NoOpSessionStore
// ============================================================================

pub struct NoOpSessionStore;

#[async_trait]
impl IdentitySessionStore for NoOpSessionStore {
    async fn delete_user_session(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    async fn logout_user(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// NoOpCredentialStore
// ============================================================================

pub struct NoOpCredentialStore;

#[async_trait]
impl IdentityCredentialStore for NoOpCredentialStore {
    async fn list_user_credentials(
        &self,
        _user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        Ok(vec![])
    }

    async fn remove_totp_credentials(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }

    async fn list_webauthn_credentials(
        &self,
        _user_id: &str,
    ) -> Result<Vec<IdentityCredentialRepresentation>> {
        Ok(vec![])
    }

    async fn delete_user_credential(&self, _user_id: &str, _credential_id: &str) -> Result<()> {
        Ok(())
    }

    async fn is_password_temporary(&self, _user_id: &str) -> Result<bool> {
        Ok(false)
    }
}

// ============================================================================
// NoOpFederationBroker
// ============================================================================

pub struct NoOpFederationBroker;

#[async_trait]
impl FederationBroker for NoOpFederationBroker {
    async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
        Ok(vec![])
    }

    async fn get_identity_provider(&self, _alias: &str) -> Result<IdentityProviderRepresentation> {
        Ok(IdentityProviderRepresentation {
            alias: String::new(),
            display_name: None,
            provider_id: String::new(),
            enabled: false,
            trust_email: false,
            store_token: false,
            link_only: false,
            first_login_policy: "auto_merge".to_string(),
            first_broker_login_flow_alias: None,
            config: HashMap::new(),
            extra: HashMap::new(),
        })
    }

    async fn create_identity_provider(
        &self,
        _provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        Ok(())
    }

    async fn update_identity_provider(
        &self,
        _alias: &str,
        _provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        Ok(())
    }

    async fn delete_identity_provider(&self, _alias: &str) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// NoOpActionStore
// ============================================================================

pub struct NoOpActionStore;

#[async_trait]
impl IdentityActionStore for NoOpActionStore {
    async fn get_pending_actions(&self, _user_id: &str) -> Result<Vec<PendingActionInfo>> {
        Ok(vec![])
    }

    async fn create_action(
        &self,
        _user_id: &str,
        _action_type: &str,
        _metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        Ok("test-id".to_string())
    }

    async fn complete_action(&self, _action_id: &str) -> Result<()> {
        Ok(())
    }

    async fn cancel_action(&self, _action_id: &str) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// NoOpVerificationStore
// ============================================================================

pub struct NoOpVerificationStore;

#[async_trait]
impl IdentityVerificationStore for NoOpVerificationStore {
    async fn get_verification_status(&self, _user_id: &str) -> Result<bool> {
        Ok(false)
    }

    async fn set_email_verified(&self, _user_id: &str, _verified: bool) -> Result<()> {
        Ok(())
    }

    async fn create_verification_token(
        &self,
        _user_id: &str,
        _token_hash: &str,
        _expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<VerificationTokenInfo> {
        Ok(VerificationTokenInfo {
            id: "test-id".to_string(),
            user_id: "test-user-id".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            used_at: None,
            created_at: chrono::Utc::now(),
        })
    }

    async fn find_valid_token(&self, _token_hash: &str) -> Result<Option<VerificationTokenInfo>> {
        Ok(None)
    }

    async fn mark_token_used(&self, _token_id: &str) -> Result<()> {
        Ok(())
    }

    async fn invalidate_user_tokens(&self, _user_id: &str) -> Result<u64> {
        Ok(0)
    }
}

// ============================================================================
// NoOpEventSource
// ============================================================================

pub struct NoOpEventSource;

#[async_trait]
impl IdentityEventSource for NoOpEventSource {}

// ============================================================================
// NoOpIdentityEngine — top-level composite
// ============================================================================

pub struct NoOpIdentityEngine;

#[async_trait]
impl IdentityEngine for NoOpIdentityEngine {
    fn user_store(&self) -> &dyn IdentityUserStore {
        &NoOpUserStore
    }

    fn client_store(&self) -> &dyn IdentityClientStore {
        &NoOpClientStore
    }

    fn session_store(&self) -> &dyn IdentitySessionStore {
        &NoOpSessionStore
    }

    fn credential_store(&self) -> &dyn IdentityCredentialStore {
        &NoOpCredentialStore
    }

    fn federation_broker(&self) -> &dyn FederationBroker {
        &NoOpFederationBroker
    }

    fn event_source(&self) -> &dyn IdentityEventSource {
        &NoOpEventSource
    }

    fn action_store(&self) -> &dyn IdentityActionStore {
        &NoOpActionStore
    }

    fn verification_store(&self) -> &dyn IdentityVerificationStore {
        &NoOpVerificationStore
    }

    async fn update_realm(&self, _settings: &RealmSettingsUpdate) -> Result<()> {
        Ok(())
    }
}
