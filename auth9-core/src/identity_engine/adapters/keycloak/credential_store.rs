use crate::identity_engine::IdentityCredentialStore;
use crate::keycloak::KeycloakClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakCredentialStoreAdapter {
    #[allow(dead_code)]
    client: Arc<KeycloakClient>,
}

impl KeycloakCredentialStoreAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentityCredentialStore for KeycloakCredentialStoreAdapter {}
