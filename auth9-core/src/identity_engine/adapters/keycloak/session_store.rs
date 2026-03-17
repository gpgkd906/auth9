use crate::error::Result;
use crate::identity_engine::IdentitySessionStore;
use crate::keycloak::KeycloakClient;
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakSessionStoreAdapter {
    client: Arc<KeycloakClient>,
}

impl KeycloakSessionStoreAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl IdentitySessionStore for KeycloakSessionStoreAdapter {
    async fn delete_user_session(&self, session_id: &str) -> Result<()> {
        self.client.delete_user_session(session_id).await
    }

    async fn logout_user(&self, user_id: &str) -> Result<()> {
        self.client.logout_user(user_id).await
    }
}
