use crate::identity_engine::IdentityClientStore;
use crate::keycloak::KeycloakClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakClientStoreAdapter {
    #[allow(dead_code)]
    client: Arc<KeycloakClient>,
}

impl KeycloakClientStoreAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentityClientStore for KeycloakClientStoreAdapter {}
