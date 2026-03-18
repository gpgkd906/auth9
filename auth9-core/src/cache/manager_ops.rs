//! impl CacheOperations for CacheManager

use super::{manager::CacheManager, CacheOperations};
use crate::error::Result;
use crate::models::rbac::UserRolesInTenant;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
impl CacheOperations for CacheManager {
    async fn ping(&self) -> Result<()> {
        CacheManager::ping(self).await
    }

    async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        CacheManager::get_user_roles(self, user_id, tenant_id).await
    }

    async fn set_user_roles(&self, roles: &UserRolesInTenant) -> Result<()> {
        CacheManager::set_user_roles(self, roles).await
    }

    async fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        CacheManager::get_user_roles_for_service(self, user_id, tenant_id, service_id).await
    }

    async fn set_user_roles_for_service(
        &self,
        roles: &UserRolesInTenant,
        service_id: Uuid,
    ) -> Result<()> {
        CacheManager::set_user_roles_for_service(self, roles, service_id).await
    }

    async fn invalidate_user_roles(&self, user_id: Uuid, tenant_id: Option<Uuid>) -> Result<()> {
        CacheManager::invalidate_user_roles(self, user_id, tenant_id).await
    }

    async fn invalidate_user_roles_for_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<()> {
        CacheManager::invalidate_user_roles_for_tenant(self, user_id, tenant_id).await
    }

    async fn invalidate_all_user_roles(&self) -> Result<()> {
        CacheManager::invalidate_all_user_roles(self).await
    }

    async fn add_to_token_blacklist(&self, jti: &str, ttl_secs: u64) -> Result<()> {
        CacheManager::add_to_token_blacklist(self, jti, ttl_secs).await
    }

    async fn is_token_blacklisted(&self, jti: &str) -> Result<bool> {
        CacheManager::is_token_blacklisted(self, jti).await
    }

    async fn store_webauthn_reg_state(
        &self,
        user_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        CacheManager::store_webauthn_reg_state(self, user_id, state, ttl_secs).await
    }

    async fn get_webauthn_reg_state(&self, user_id: &str) -> Result<Option<String>> {
        CacheManager::get_webauthn_reg_state(self, user_id).await
    }

    async fn remove_webauthn_reg_state(&self, user_id: &str) -> Result<()> {
        CacheManager::remove_webauthn_reg_state(self, user_id).await
    }

    async fn store_webauthn_auth_state(
        &self,
        challenge_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        CacheManager::store_webauthn_auth_state(self, challenge_id, state, ttl_secs).await
    }

    async fn get_webauthn_auth_state(&self, challenge_id: &str) -> Result<Option<String>> {
        CacheManager::get_webauthn_auth_state(self, challenge_id).await
    }

    async fn remove_webauthn_auth_state(&self, challenge_id: &str) -> Result<()> {
        CacheManager::remove_webauthn_auth_state(self, challenge_id).await
    }

    async fn store_oidc_state(
        &self,
        state_nonce: &str,
        payload: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        CacheManager::store_oidc_state(self, state_nonce, payload, ttl_secs).await
    }

    async fn consume_oidc_state(&self, state_nonce: &str) -> Result<Option<String>> {
        CacheManager::consume_oidc_state(self, state_nonce).await
    }

    async fn bind_refresh_token_session(
        &self,
        refresh_token: &str,
        session_id: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        CacheManager::bind_refresh_token_session(self, refresh_token, session_id, ttl_secs).await
    }

    async fn get_refresh_token_session(&self, refresh_token: &str) -> Result<Option<String>> {
        CacheManager::get_refresh_token_session(self, refresh_token).await
    }

    async fn remove_refresh_token_session(&self, refresh_token: &str) -> Result<()> {
        CacheManager::remove_refresh_token_session(self, refresh_token).await
    }

    async fn remove_all_refresh_sessions_for_session(&self, session_id: &str) -> Result<()> {
        CacheManager::remove_all_refresh_sessions_for_session(self, session_id).await
    }

    async fn check_and_mark_webhook_event(&self, event_key: &str, ttl_secs: u64) -> Result<bool> {
        CacheManager::check_and_mark_webhook_event(self, event_key, ttl_secs).await
    }

    // ==================== OTP ====================

    async fn store_otp(&self, key: &str, code: &str, ttl_secs: u64) -> Result<()> {
        CacheManager::store_otp(self, key, code, ttl_secs).await
    }

    async fn get_otp(&self, key: &str) -> Result<Option<String>> {
        CacheManager::get_otp(self, key).await
    }

    async fn remove_otp(&self, key: &str) -> Result<()> {
        CacheManager::remove_otp(self, key).await
    }

    async fn increment_counter(&self, key: &str, ttl_secs: u64) -> Result<u64> {
        CacheManager::increment_counter(self, key, ttl_secs).await
    }

    async fn get_counter(&self, key: &str) -> Result<u64> {
        CacheManager::get_counter(self, key).await
    }

    async fn set_flag(&self, key: &str, ttl_secs: u64) -> Result<bool> {
        CacheManager::set_flag(self, key, ttl_secs).await
    }

    // ==================== TOTP ====================

    async fn store_totp_setup(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        CacheManager::store_totp_setup(self, token, data, ttl_secs).await
    }

    async fn get_totp_setup(&self, token: &str) -> Result<Option<String>> {
        CacheManager::get_totp_setup(self, token).await
    }

    async fn remove_totp_setup(&self, token: &str) -> Result<()> {
        CacheManager::remove_totp_setup(self, token).await
    }

    async fn is_totp_code_used(&self, user_id: &str, time_step: u64) -> Result<bool> {
        CacheManager::is_totp_code_used(self, user_id, time_step).await
    }

    async fn mark_totp_code_used(&self, user_id: &str, time_step: u64, ttl_secs: u64) -> Result<()> {
        CacheManager::mark_totp_code_used(self, user_id, time_step, ttl_secs).await
    }

    // ==================== MFA Session ====================

    async fn store_mfa_session(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        CacheManager::store_mfa_session(self, token, data, ttl_secs).await
    }

    async fn get_mfa_session(&self, token: &str) -> Result<Option<String>> {
        CacheManager::get_mfa_session(self, token).await
    }

    async fn consume_mfa_session(&self, token: &str) -> Result<Option<String>> {
        CacheManager::consume_mfa_session(self, token).await
    }
}
