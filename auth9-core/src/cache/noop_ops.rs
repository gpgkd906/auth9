//! impl CacheOperations for NoOpCacheManager

use super::{noop::NoOpCacheManager, CacheOperations};
use crate::error::Result;
use crate::models::rbac::UserRolesInTenant;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
impl CacheOperations for NoOpCacheManager {
    async fn ping(&self) -> Result<()> {
        Ok(())
    }

    async fn get_user_roles(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        Ok(None)
    }

    async fn set_user_roles(&self, _roles: &UserRolesInTenant) -> Result<()> {
        Ok(())
    }

    async fn get_user_roles_for_service(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
        _service_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        Ok(None)
    }

    async fn set_user_roles_for_service(
        &self,
        _roles: &UserRolesInTenant,
        _service_id: Uuid,
    ) -> Result<()> {
        Ok(())
    }

    async fn invalidate_user_roles(&self, _user_id: Uuid, _tenant_id: Option<Uuid>) -> Result<()> {
        Ok(())
    }

    async fn invalidate_user_roles_for_tenant(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
    ) -> Result<()> {
        Ok(())
    }

    async fn invalidate_all_user_roles(&self) -> Result<()> {
        Ok(())
    }

    async fn add_to_token_blacklist(&self, _jti: &str, _ttl_secs: u64) -> Result<()> {
        Ok(())
    }

    async fn is_token_blacklisted(&self, _jti: &str) -> Result<bool> {
        Ok(false)
    }

    async fn store_webauthn_reg_state(
        &self,
        _user_id: &str,
        _state: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        Ok(())
    }

    async fn get_webauthn_reg_state(&self, _user_id: &str) -> Result<Option<String>> {
        Ok(None)
    }

    async fn remove_webauthn_reg_state(&self, _user_id: &str) -> Result<()> {
        Ok(())
    }

    async fn store_webauthn_auth_state(
        &self,
        _challenge_id: &str,
        _state: &str,
        _ttl_secs: u64,
    ) -> Result<()> {
        Ok(())
    }

    async fn get_webauthn_auth_state(&self, _challenge_id: &str) -> Result<Option<String>> {
        Ok(None)
    }

    async fn remove_webauthn_auth_state(&self, _challenge_id: &str) -> Result<()> {
        Ok(())
    }

    async fn store_oidc_state(
        &self,
        state_nonce: &str,
        payload: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        NoOpCacheManager::store_oidc_state(self, state_nonce, payload, ttl_secs).await
    }

    async fn consume_oidc_state(&self, state_nonce: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_oidc_state(self, state_nonce).await
    }

    async fn bind_refresh_token_session(
        &self,
        refresh_token: &str,
        session_id: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        NoOpCacheManager::bind_refresh_token_session(self, refresh_token, session_id, ttl_secs)
            .await
    }

    async fn get_refresh_token_session(&self, refresh_token: &str) -> Result<Option<String>> {
        NoOpCacheManager::get_refresh_token_session(self, refresh_token).await
    }

    async fn remove_refresh_token_session(&self, refresh_token: &str) -> Result<()> {
        NoOpCacheManager::remove_refresh_token_session(self, refresh_token).await
    }

    async fn remove_all_refresh_sessions_for_session(&self, session_id: &str) -> Result<()> {
        NoOpCacheManager::remove_all_refresh_sessions_for_session(self, session_id).await
    }

    async fn check_and_mark_webhook_event(&self, event_key: &str, ttl_secs: u64) -> Result<bool> {
        NoOpCacheManager::check_and_mark_webhook_event(self, event_key, ttl_secs).await
    }

    // ==================== OTP ====================

    async fn store_otp(&self, key: &str, code: &str, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::store_otp(self, key, code, ttl_secs).await
    }

    async fn get_otp(&self, key: &str) -> Result<Option<String>> {
        NoOpCacheManager::get_otp(self, key).await
    }

    async fn remove_otp(&self, key: &str) -> Result<()> {
        NoOpCacheManager::remove_otp(self, key).await
    }

    async fn increment_counter(&self, key: &str, ttl_secs: u64) -> Result<u64> {
        NoOpCacheManager::increment_counter(self, key, ttl_secs).await
    }

    async fn get_counter(&self, key: &str) -> Result<u64> {
        NoOpCacheManager::get_counter(self, key).await
    }

    async fn set_flag(&self, key: &str, ttl_secs: u64) -> Result<bool> {
        NoOpCacheManager::set_flag(self, key, ttl_secs).await
    }

    // ==================== TOTP ====================

    async fn store_totp_setup(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::store_totp_setup(self, token, data, ttl_secs).await
    }

    async fn get_totp_setup(&self, token: &str) -> Result<Option<String>> {
        NoOpCacheManager::get_totp_setup(self, token).await
    }

    async fn remove_totp_setup(&self, token: &str) -> Result<()> {
        NoOpCacheManager::remove_totp_setup(self, token).await
    }

    async fn is_totp_code_used(&self, user_id: &str, time_step: u64) -> Result<bool> {
        NoOpCacheManager::is_totp_code_used(self, user_id, time_step).await
    }

    async fn mark_totp_code_used(&self, user_id: &str, time_step: u64, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::mark_totp_code_used(self, user_id, time_step, ttl_secs).await
    }

    // ==================== MFA Session ====================

    async fn store_mfa_session(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::store_mfa_session(self, token, data, ttl_secs).await
    }

    async fn get_mfa_session(&self, token: &str) -> Result<Option<String>> {
        NoOpCacheManager::get_mfa_session(self, token).await
    }

    async fn consume_mfa_session(&self, token: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_mfa_session(self, token).await
    }

    // ==================== Login Challenge ====================

    async fn store_login_challenge(&self, id: &str, data: &str, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::store_login_challenge(self, id, data, ttl_secs).await
    }

    async fn consume_login_challenge(&self, id: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_login_challenge(self, id).await
    }

    // ==================== Authorization Code ====================

    async fn store_authorization_code(&self, code: &str, data: &str, ttl_secs: u64) -> Result<()> {
        NoOpCacheManager::store_authorization_code(self, code, data, ttl_secs).await
    }

    async fn consume_authorization_code(&self, code: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_authorization_code(self, code).await
    }

    // ==================== Social Login State ====================

    async fn store_social_login_state(
        &self,
        id: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        NoOpCacheManager::store_social_login_state(self, id, data, ttl_secs).await
    }

    async fn consume_social_login_state(&self, id: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_social_login_state(self, id).await
    }

    // ==================== Enterprise SSO State ====================

    async fn store_enterprise_sso_state(
        &self,
        id: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        NoOpCacheManager::store_enterprise_sso_state(self, id, data, ttl_secs).await
    }

    async fn consume_enterprise_sso_state(&self, id: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_enterprise_sso_state(self, id).await
    }

    // ==================== Pending Merge ====================

    async fn store_pending_merge(
        &self,
        token: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        NoOpCacheManager::store_pending_merge(self, token, data, ttl_secs).await
    }

    async fn consume_pending_merge(&self, token: &str) -> Result<Option<String>> {
        NoOpCacheManager::consume_pending_merge(self, token).await
    }

    // ==================== Audience Validation ====================

    async fn is_valid_audience(&self, client_id: &str) -> Result<bool> {
        NoOpCacheManager::is_valid_audience(self, client_id).await
    }

    async fn refresh_audience_set(&self, client_ids: &[String]) -> Result<()> {
        NoOpCacheManager::refresh_audience_set(self, client_ids).await
    }

    async fn add_audience(&self, client_id: &str) -> Result<()> {
        NoOpCacheManager::add_audience(self, client_id).await
    }

    async fn remove_audience(&self, client_id: &str) -> Result<()> {
        NoOpCacheManager::remove_audience(self, client_id).await
    }
}
