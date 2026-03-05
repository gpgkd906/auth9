//! Redis cache layer

use crate::domain::UserRolesInTenant;
use crate::error::Result;
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
}

/// Default TTLs
pub(crate) mod ttl {
    pub const USER_ROLES_SECS: u64 = 300; // 5 minutes
    pub const USER_ROLES_SERVICE_SECS: u64 = 300;
    pub const SERVICE_CONFIG_SECS: u64 = 600; // 10 minutes
    pub const TENANT_CONFIG_SECS: u64 = 600; // 10 minutes
}
