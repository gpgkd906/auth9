use crate::error::Result;
use crate::keycloak::RealmUpdate;
use async_trait::async_trait;

pub mod adapters;
mod types;

pub use types::{FederatedIdentityRepresentation, IdentityProviderRepresentation};

/// User lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityUserStore: Send + Sync {}

/// OIDC/SAML client lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityClientStore: Send + Sync {}

/// Session lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentitySessionStore: Send + Sync {
    async fn delete_user_session(&self, session_id: &str) -> Result<()>;
    async fn logout_user(&self, user_id: &str) -> Result<()>;
}

/// Credential lifecycle operations for an identity backend.
#[async_trait]
pub trait IdentityCredentialStore: Send + Sync {}

/// Federation and broker management operations for an identity backend.
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
    async fn get_user_federated_identities(
        &self,
        user_id: &str,
    ) -> Result<Vec<FederatedIdentityRepresentation>>;
    async fn remove_user_federated_identity(
        &self,
        user_id: &str,
        provider_alias: &str,
    ) -> Result<()>;
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

    async fn update_realm(&self, settings: &RealmUpdate) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeycloakConfig;
    use crate::identity_engine::adapters::keycloak::KeycloakIdentityEngineAdapter;
    use crate::keycloak::KeycloakClient;
    use std::sync::Arc;

    #[test]
    fn keycloak_adapter_exposes_identity_engine_surfaces() {
        let client = Arc::new(KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8080".to_string(),
            public_url: "http://localhost:8080".to_string(),
            realm: "test".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "test-placeholder".to_string(), // pragma: allowlist secret
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        }));

        let adapter = KeycloakIdentityEngineAdapter::new(client);
        let engine: &dyn IdentityEngine = &adapter;

        let _ = engine.user_store();
        let _ = engine.client_store();
        let _ = engine.session_store();
        let _ = engine.credential_store();
        let _ = engine.federation_broker();
        let _ = engine.event_source();
    }
}
