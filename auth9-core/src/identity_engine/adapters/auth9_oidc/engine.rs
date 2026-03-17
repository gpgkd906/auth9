use super::{Auth9OidcFederationBrokerAdapter, Auth9OidcSessionStoreAdapter};
use crate::error::Result;
use crate::identity_engine::{
    FederationBroker, IdentityClientStore, IdentityCredentialStore, IdentityEngine,
    IdentityEventSource, IdentitySessionStore, IdentityUserStore,
};
use crate::keycloak::RealmUpdate;
use async_trait::async_trait;

#[derive(Default)]
struct Auth9OidcUserStore;

#[async_trait]
impl IdentityUserStore for Auth9OidcUserStore {}

#[derive(Default)]
struct Auth9OidcClientStore;

#[async_trait]
impl IdentityClientStore for Auth9OidcClientStore {}

#[derive(Default)]
struct Auth9OidcCredentialStore;

#[async_trait]
impl IdentityCredentialStore for Auth9OidcCredentialStore {}

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
    pub fn new() -> Self {
        Self {
            user_store: Auth9OidcUserStore,
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
