//! NoOpCacheManager struct and inherent methods

use crate::domain::UserRolesInTenant;
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// No-op cache manager for testing without Redis
#[derive(Clone)]
pub struct NoOpCacheManager {
    oidc_states: Arc<RwLock<HashMap<String, String>>>,
    refresh_sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl NoOpCacheManager {
    pub fn new() -> Self {
        Self {
            oidc_states: Arc::new(RwLock::new(HashMap::new())),
            refresh_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn refresh_token_hash(refresh_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub async fn ping(&self) -> Result<()> {
        Ok(())
    }

    pub async fn get_user_roles(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        Ok(None)
    }

    pub async fn set_user_roles(&self, _roles: &UserRolesInTenant) -> Result<()> {
        Ok(())
    }

    pub async fn get_user_roles_for_service(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
        _service_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        Ok(None)
    }

    pub async fn set_user_roles_for_service(
        &self,
        _roles: &UserRolesInTenant,
        _service_id: Uuid,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn invalidate_user_roles(
        &self,
        _user_id: Uuid,
        _tenant_id: Option<Uuid>,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn invalidate_user_roles_for_tenant(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn invalidate_all_user_roles(&self) -> Result<()> {
        Ok(())
    }

    pub async fn add_to_token_blacklist(&self, _jti: &str, _ttl_secs: u64) -> Result<()> {
        Ok(())
    }

    pub async fn is_token_blacklisted(&self, _jti: &str) -> Result<bool> {
        Ok(false)
    }

    pub async fn store_webauthn_reg_state(
        &self,
        _user_id: &str,
        _state: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn get_webauthn_reg_state(&self, _user_id: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub async fn remove_webauthn_reg_state(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }

    pub async fn store_webauthn_auth_state(
        &self,
        _challenge_id: &str,
        _state: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn get_webauthn_auth_state(&self, _challenge_id: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub async fn remove_webauthn_auth_state(&self, _challenge_id: &str) -> Result<()> {
        Ok(())
    }

    pub async fn store_oidc_state(
        &self,
        state_nonce: &str,
        payload: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(state_nonce.to_string(), payload.to_string());
        Ok(())
    }

    pub async fn consume_oidc_state(&self, state_nonce: &str) -> Result<Option<String>> {
        Ok(self.oidc_states.write().await.remove(state_nonce))
    }

    pub async fn bind_refresh_token_session(
        &self,
        refresh_token: &str,
        session_id: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        let key = Self::refresh_token_hash(refresh_token);
        self.refresh_sessions
            .write()
            .await
            .insert(key, session_id.to_string());
        Ok(())
    }

    pub async fn get_refresh_token_session(&self, refresh_token: &str) -> Result<Option<String>> {
        let key = Self::refresh_token_hash(refresh_token);
        Ok(self.refresh_sessions.read().await.get(&key).cloned())
    }

    pub async fn remove_refresh_token_session(&self, refresh_token: &str) -> Result<()> {
        let key = Self::refresh_token_hash(refresh_token);
        self.refresh_sessions.write().await.remove(&key);
        Ok(())
    }

    pub async fn remove_all_refresh_sessions_for_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.refresh_sessions.write().await;
        sessions.retain(|_, v| v != session_id);
        Ok(())
    }

    pub async fn check_and_mark_webhook_event(
        &self,
        _event_key: &str,
        _ttl_secs: u64,
    ) -> Result<bool> {
        // NoOp: always report as not duplicate
        Ok(false)
    }
}

impl Default for NoOpCacheManager {
    fn default() -> Self {
        Self::new()
    }
}
