//! Redis cache layer

use crate::error::Result;
use crate::models::rbac::UserRolesInTenant;
use async_trait::async_trait;
use uuid::Uuid;

mod manager;
mod manager_ops;
mod noop;
mod noop_ops;

#[cfg(test)]
mod tests;

pub use manager::CacheManager;
pub use noop::NoOpCacheManager;

/// Cache operations trait for dependency injection and testing
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CacheOperations: Send + Sync {
    async fn ping(&self) -> Result<()>;
    async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>>;
    async fn set_user_roles(&self, roles: &UserRolesInTenant) -> Result<()>;
    async fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>>;
    async fn set_user_roles_for_service(
        &self,
        roles: &UserRolesInTenant,
        service_id: Uuid,
    ) -> Result<()>;
    async fn invalidate_user_roles(&self, user_id: Uuid, tenant_id: Option<Uuid>) -> Result<()>;
    async fn invalidate_user_roles_for_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<()>;
    async fn invalidate_all_user_roles(&self) -> Result<()>;

    /// Add a token JTI to the blacklist for immediate revocation.
    /// The TTL should be set to the remaining validity time of the token.
    async fn add_to_token_blacklist(&self, jti: &str, ttl_secs: u64) -> Result<()>;

    /// Check if a token JTI is in the blacklist.
    async fn is_token_blacklisted(&self, jti: &str) -> Result<bool>;

    // ==================== WebAuthn Challenge State ====================

    async fn store_webauthn_reg_state(
        &self,
        user_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()>;
    async fn get_webauthn_reg_state(&self, user_id: &str) -> Result<Option<String>>;
    async fn remove_webauthn_reg_state(&self, user_id: &str) -> Result<()>;

    async fn store_webauthn_auth_state(
        &self,
        challenge_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()>;
    async fn get_webauthn_auth_state(&self, challenge_id: &str) -> Result<Option<String>>;
    async fn remove_webauthn_auth_state(&self, challenge_id: &str) -> Result<()>;

    // ==================== OIDC State ====================

    async fn store_oidc_state(&self, state_nonce: &str, payload: &str, ttl_secs: u64)
        -> Result<()>;
    async fn consume_oidc_state(&self, state_nonce: &str) -> Result<Option<String>>;

    // ==================== Refresh Token Session Binding ====================

    async fn bind_refresh_token_session(
        &self,
        refresh_token: &str,
        session_id: &str,
        ttl_secs: u64,
    ) -> Result<()>;
    async fn get_refresh_token_session(&self, refresh_token: &str) -> Result<Option<String>>;
    async fn remove_refresh_token_session(&self, refresh_token: &str) -> Result<()>;

    /// Remove all refresh token sessions bound to a given session ID.
    /// Used during logout to clean up all refresh tokens for the session.
    async fn remove_all_refresh_sessions_for_session(&self, session_id: &str) -> Result<()>;

    // ==================== Webhook Event Deduplication ====================

    /// Check if a webhook event has already been processed (returns true if duplicate).
    /// If not a duplicate, marks it as processed with the given TTL.
    async fn check_and_mark_webhook_event(&self, event_key: &str, ttl_secs: u64) -> Result<bool>;

    // ==================== OTP ====================

    /// Store an OTP code with TTL
    async fn store_otp(&self, key: &str, code: &str, ttl_secs: u64) -> Result<()>;

    /// Get a stored OTP code
    async fn get_otp(&self, key: &str) -> Result<Option<String>>;

    /// Remove a stored OTP code (one-time consumption)
    async fn remove_otp(&self, key: &str) -> Result<()>;

    /// Increment a counter and return the new value. Sets TTL on first increment.
    async fn increment_counter(&self, key: &str, ttl_secs: u64) -> Result<u64>;

    /// Get the current value of a counter (0 if not set)
    async fn get_counter(&self, key: &str) -> Result<u64>;

    /// Set a flag key with TTL. Returns true if the key already existed.
    async fn set_flag(&self, key: &str, ttl_secs: u64) -> Result<bool>;

    // ==================== TOTP ====================

    /// Store TOTP enrollment setup state
    async fn store_totp_setup(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()>;

    /// Get TOTP enrollment setup state
    async fn get_totp_setup(&self, token: &str) -> Result<Option<String>>;

    /// Remove TOTP enrollment setup state
    async fn remove_totp_setup(&self, token: &str) -> Result<()>;

    /// Check if a TOTP code time step has been used (replay protection)
    async fn is_totp_code_used(&self, user_id: &str, time_step: u64) -> Result<bool>;

    /// Mark a TOTP code time step as used (replay protection)
    async fn mark_totp_code_used(&self, user_id: &str, time_step: u64, ttl_secs: u64) -> Result<()>;

    // ==================== MFA Session ====================

    /// Store MFA session data (temporary token after password auth, before MFA verification)
    async fn store_mfa_session(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()>;

    /// Get MFA session data
    async fn get_mfa_session(&self, token: &str) -> Result<Option<String>>;

    /// Consume (get + delete) MFA session data
    async fn consume_mfa_session(&self, token: &str) -> Result<Option<String>>;

    // ==================== Login Challenge ====================

    /// Store a login challenge (OIDC authorize → hosted login → authorize_complete)
    async fn store_login_challenge(&self, id: &str, data: &str, ttl_secs: u64) -> Result<()>;

    /// Consume (get + delete) a login challenge
    async fn consume_login_challenge(&self, id: &str) -> Result<Option<String>>;

    // ==================== Authorization Code ====================

    /// Store an authorization code (authorize_complete → token endpoint)
    async fn store_authorization_code(&self, code: &str, data: &str, ttl_secs: u64) -> Result<()>;

    /// Consume (get + delete) an authorization code (one-time use)
    async fn consume_authorization_code(&self, code: &str) -> Result<Option<String>>;

    // ==================== Social Login State ====================

    /// Store social login state (social authorize → provider → callback)
    async fn store_social_login_state(
        &self,
        id: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// Consume (get + delete) a social login state
    async fn consume_social_login_state(&self, id: &str) -> Result<Option<String>>;

    // ==================== Enterprise SSO State ====================

    /// Store enterprise SSO login state (enterprise authorize → IdP → callback)
    async fn store_enterprise_sso_state(
        &self,
        id: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// Consume (get + delete) an enterprise SSO login state
    async fn consume_enterprise_sso_state(&self, id: &str) -> Result<Option<String>>;

    // ==================== Pending Merge ====================

    /// Store pending merge state (confirm-link flow for first_login_policy=prompt_confirm)
    async fn store_pending_merge(
        &self,
        token: &str,
        data: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// Consume (get + delete) a pending merge state
    async fn consume_pending_merge(&self, token: &str) -> Result<Option<String>>;

    // ==================== Audience Validation ====================

    /// Check if a client_id is a registered audience (SISMEMBER on Redis SET).
    async fn is_valid_audience(&self, client_id: &str) -> Result<bool>;

    /// Replace the entire audience set with the given list (DEL + SADD).
    async fn refresh_audience_set(&self, client_ids: &[String]) -> Result<()>;

    /// Add a single audience to the set (SADD).
    async fn add_audience(&self, client_id: &str) -> Result<()>;

    /// Remove a single audience from the set (SREM).
    async fn remove_audience(&self, client_id: &str) -> Result<()>;
}

/// Cache key prefixes
pub(crate) mod keys {
    pub const USER_ROLES: &str = "auth9:user_roles";
    pub const USER_ROLES_SERVICE: &str = "auth9:user_roles_service";
    pub const SERVICE_CONFIG: &str = "auth9:service";
    pub const TENANT_CONFIG: &str = "auth9:tenant";
    pub const TOKEN_BLACKLIST: &str = "auth9:token_blacklist";
    pub const WEBAUTHN_REG: &str = "auth9:webauthn_reg";
    pub const WEBAUTHN_AUTH: &str = "auth9:webauthn_auth";
    pub const OIDC_STATE: &str = "auth9:oidc_state";
    pub const REFRESH_TOKEN_SESSION: &str = "auth9:refresh_session";
    pub const SESSION_REFRESH_TOKENS: &str = "auth9:session_tokens";
    pub const WEBHOOK_EVENT_DEDUP: &str = "auth9:webhook_dedup";
    pub const OTP: &str = "auth9:otp";
    pub const OTP_COOLDOWN: &str = "auth9:otp_cooldown";
    pub const OTP_DAILY: &str = "auth9:otp_daily";
    pub const OTP_FAIL: &str = "auth9:otp_fail";
    pub const TOTP_SETUP: &str = "auth9:totp_setup";
    pub const TOTP_USED: &str = "auth9:totp_used";
    pub const MFA_SESSION: &str = "auth9:mfa_session";
    pub const LOGIN_CHALLENGE: &str = "auth9:login_challenge";
    pub const AUTH_CODE: &str = "auth9:auth_code";
    pub const SOCIAL_STATE: &str = "auth9:social_state";
    pub const ENTERPRISE_SSO_STATE: &str = "auth9:enterprise_sso_state";
    pub const PENDING_MERGE: &str = "auth9:pending_merge";
    pub const VALID_AUDIENCES: &str = "auth9:valid_audiences";
}

/// Default TTLs
pub(crate) mod ttl {
    pub const USER_ROLES_SECS: u64 = 300; // 5 minutes
    pub const USER_ROLES_SERVICE_SECS: u64 = 300;
    pub const SERVICE_CONFIG_SECS: u64 = 600; // 10 minutes
    pub const TENANT_CONFIG_SECS: u64 = 600; // 10 minutes
}
