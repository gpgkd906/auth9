use crate::identity_engine::IdentityUserStore;
use crate::keycloak::KeycloakClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakUserStoreAdapter {
    #[allow(dead_code)]
    client: Arc<KeycloakClient>,
}

impl KeycloakUserStoreAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentityUserStore for KeycloakUserStoreAdapter {}
