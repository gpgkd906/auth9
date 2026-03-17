use super::{
    KeycloakClientStoreAdapter, KeycloakCredentialStoreAdapter, KeycloakEventSourceAdapter,
    KeycloakFederationBrokerAdapter, KeycloakSessionStoreAdapter, KeycloakUserStoreAdapter,
};
use crate::error::Result;
use crate::identity_engine::{
    FederationBroker, IdentityClientStore, IdentityCredentialStore, IdentityEngine,
    IdentityEventSource, IdentitySessionStore, IdentityUserStore,
};
use crate::keycloak::{KeycloakClient, RealmUpdate};
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakIdentityEngineAdapter {
    user_store: KeycloakUserStoreAdapter,
    client_store: KeycloakClientStoreAdapter,
    session_store: KeycloakSessionStoreAdapter,
    credential_store: KeycloakCredentialStoreAdapter,
    federation_broker: KeycloakFederationBrokerAdapter,
    event_source: KeycloakEventSourceAdapter,
    client: Arc<KeycloakClient>,
}

impl KeycloakIdentityEngineAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self {
            user_store: KeycloakUserStoreAdapter::new(client.clone()),
            client_store: KeycloakClientStoreAdapter::new(client.clone()),
            session_store: KeycloakSessionStoreAdapter::new(client.clone()),
            credential_store: KeycloakCredentialStoreAdapter::new(client.clone()),
            federation_broker: KeycloakFederationBrokerAdapter::new(client.clone()),
            event_source: KeycloakEventSourceAdapter::new(client.clone()),
            client,
        }
    }
}

#[async_trait]
impl IdentityEngine for KeycloakIdentityEngineAdapter {
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

    async fn update_realm(&self, settings: &RealmUpdate) -> Result<()> {
        self.client.update_realm(settings).await
    }
}
