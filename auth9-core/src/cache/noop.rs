//! NoOpCacheManager struct and inherent methods

use crate::error::Result;
use crate::models::rbac::UserRolesInTenant;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// No-op cache manager for testing without Redis
#[derive(Clone)]
pub struct NoOpCacheManager {
    oidc_states: Arc<RwLock<HashMap<String, String>>>,
    refresh_sessions: Arc<RwLock<HashMap<String, String>>>,
    otp_store: Arc<RwLock<HashMap<String, String>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    flags: Arc<RwLock<HashMap<String, bool>>>,
    audiences: Arc<RwLock<HashSet<String>>>,
}

impl NoOpCacheManager {
    pub fn new() -> Self {
        Self {
            oidc_states: Arc::new(RwLock::new(HashMap::new())),
            refresh_sessions: Arc::new(RwLock::new(HashMap::new())),
            otp_store: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            flags: Arc::new(RwLock::new(HashMap::new())),
            audiences: Arc::new(RwLock::new(HashSet::new())),
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

    // ==================== OTP ====================

    pub async fn store_otp(&self, key: &str, code: &str, _ttl_secs: u64) -> Result<()> {
        self.otp_store
            .write()
            .await
            .insert(key.to_string(), code.to_string());
        Ok(())
    }

    pub async fn get_otp(&self, key: &str) -> Result<Option<String>> {
        Ok(self.otp_store.read().await.get(key).cloned())
    }

    pub async fn remove_otp(&self, key: &str) -> Result<()> {
        self.otp_store.write().await.remove(key);
        Ok(())
    }

    pub async fn increment_counter(&self, key: &str, _ttl_secs: u64) -> Result<u64> {
        let mut counters = self.counters.write().await;
        let entry = counters.entry(key.to_string()).or_insert(0);
        *entry += 1;
        Ok(*entry)
    }

    pub async fn get_counter(&self, key: &str) -> Result<u64> {
        Ok(self.counters.read().await.get(key).copied().unwrap_or(0))
    }

    pub async fn set_flag(&self, key: &str, _ttl_secs: u64) -> Result<bool> {
        let mut flags = self.flags.write().await;
        if flags.contains_key(key) {
            Ok(true)
        } else {
            flags.insert(key.to_string(), true);
            Ok(false)
        }
    }

    // ==================== TOTP ====================

    pub async fn store_totp_setup(&self, token: &str, data: &str, _ttl_secs: u64) -> Result<()> {
        self.otp_store
            .write()
            .await
            .insert(format!("totp_setup:{}", token), data.to_string());
        Ok(())
    }

    pub async fn get_totp_setup(&self, token: &str) -> Result<Option<String>> {
        Ok(self
            .otp_store
            .read()
            .await
            .get(&format!("totp_setup:{}", token))
            .cloned())
    }

    pub async fn remove_totp_setup(&self, token: &str) -> Result<()> {
        self.otp_store
            .write()
            .await
            .remove(&format!("totp_setup:{}", token));
        Ok(())
    }

    pub async fn is_totp_code_used(&self, user_id: &str, time_step: u64) -> Result<bool> {
        Ok(self
            .flags
            .read()
            .await
            .contains_key(&format!("totp_used:{}:{}", user_id, time_step)))
    }

    pub async fn mark_totp_code_used(
        &self,
        user_id: &str,
        time_step: u64,
        _ttl_secs: u64,
    ) -> Result<()> {
        self.flags
            .write()
            .await
            .insert(format!("totp_used:{}:{}", user_id, time_step), true);
        Ok(())
    }

    // ==================== MFA Session ====================

    pub async fn store_mfa_session(&self, token: &str, data: &str, _ttl_secs: u64) -> Result<()> {
        self.otp_store
            .write()
            .await
            .insert(format!("mfa_session:{}", token), data.to_string());
        Ok(())
    }

    pub async fn get_mfa_session(&self, token: &str) -> Result<Option<String>> {
        Ok(self
            .otp_store
            .read()
            .await
            .get(&format!("mfa_session:{}", token))
            .cloned())
    }

    pub async fn consume_mfa_session(&self, token: &str) -> Result<Option<String>> {
        Ok(self
            .otp_store
            .write()
            .await
            .remove(&format!("mfa_session:{}", token)))
    }

    // ==================== Login Challenge ====================

    pub async fn store_login_challenge(&self, id: &str, data: &str, _ttl_secs: u64) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(format!("login_challenge:{}", id), data.to_string());
        Ok(())
    }

    pub async fn consume_login_challenge(&self, id: &str) -> Result<Option<String>> {
        Ok(self
            .oidc_states
            .write()
            .await
            .remove(&format!("login_challenge:{}", id)))
    }

    // ==================== Authorization Code ====================

    pub async fn store_authorization_code(
        &self,
        code: &str,
        data: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(format!("auth_code:{}", code), data.to_string());
        Ok(())
    }

    pub async fn consume_authorization_code(&self, code: &str) -> Result<Option<String>> {
        Ok(self
            .oidc_states
            .write()
            .await
            .remove(&format!("auth_code:{}", code)))
    }

    // ==================== Social Login State ====================

    pub async fn store_social_login_state(
        &self,
        id: &str,
        data: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(format!("social_state:{}", id), data.to_string());
        Ok(())
    }

    pub async fn consume_social_login_state(&self, id: &str) -> Result<Option<String>> {
        Ok(self
            .oidc_states
            .write()
            .await
            .remove(&format!("social_state:{}", id)))
    }

    // ==================== Enterprise SSO State ====================

    pub async fn store_enterprise_sso_state(
        &self,
        id: &str,
        data: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(format!("enterprise_sso_state:{}", id), data.to_string());
        Ok(())
    }

    pub async fn consume_enterprise_sso_state(&self, id: &str) -> Result<Option<String>> {
        Ok(self
            .oidc_states
            .write()
            .await
            .remove(&format!("enterprise_sso_state:{}", id)))
    }

    // ==================== Pending Merge ====================

    pub async fn store_pending_merge(&self, token: &str, data: &str, _ttl_secs: u64) -> Result<()> {
        self.oidc_states
            .write()
            .await
            .insert(format!("pending_merge:{}", token), data.to_string());
        Ok(())
    }

    pub async fn consume_pending_merge(&self, token: &str) -> Result<Option<String>> {
        Ok(self
            .oidc_states
            .write()
            .await
            .remove(&format!("pending_merge:{}", token)))
    }

    // ==================== Audience Validation ====================

    pub async fn is_valid_audience(&self, client_id: &str) -> Result<bool> {
        let audiences = self.audiences.read().await;
        // If audience set is empty (not initialized), accept all — allows tests
        // and fresh starts to work without pre-seeding
        if audiences.is_empty() {
            return Ok(true);
        }
        Ok(audiences.contains(client_id))
    }

    pub async fn refresh_audience_set(&self, client_ids: &[String]) -> Result<()> {
        let mut audiences = self.audiences.write().await;
        audiences.clear();
        audiences.extend(client_ids.iter().cloned());
        Ok(())
    }

    pub async fn add_audience(&self, client_id: &str) -> Result<()> {
        self.audiences.write().await.insert(client_id.to_string());
        Ok(())
    }

    pub async fn remove_audience(&self, client_id: &str) -> Result<()> {
        self.audiences.write().await.remove(client_id);
        Ok(())
    }
}

impl Default for NoOpCacheManager {
    fn default() -> Self {
        Self::new()
    }
}
