use crate::identity_engine::IdentityEventSource;
use crate::keycloak::KeycloakClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakEventSourceAdapter {
    #[allow(dead_code)]
    client: Arc<KeycloakClient>,
}

impl KeycloakEventSourceAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentityEventSource for KeycloakEventSourceAdapter {}
