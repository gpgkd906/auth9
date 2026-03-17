use crate::error::Result;
use crate::identity_engine::IdentitySessionStore;
use async_trait::async_trait;

#[derive(Default)]
pub struct Auth9OidcSessionStoreAdapter;

impl Auth9OidcSessionStoreAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl IdentitySessionStore for Auth9OidcSessionStoreAdapter {
    async fn delete_user_session(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    async fn logout_user(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }
}
