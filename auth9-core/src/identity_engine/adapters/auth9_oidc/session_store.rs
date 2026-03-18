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
    async fn delete_user_session(&self, session_id: &str) -> Result<()> {
        tracing::debug!(session_id, "auth9_oidc: session deletion handled by application layer");
        Ok(())
    }

    async fn logout_user(&self, user_id: &str) -> Result<()> {
        tracing::debug!(user_id, "auth9_oidc: user logout handled by application layer");
        Ok(())
    }
}
