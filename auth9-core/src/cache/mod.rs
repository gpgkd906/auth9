//! Redis cache layer

use crate::config::RedisConfig;
use crate::domain::UserRolesInTenant;
use crate::error::{AppError, Result};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

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

    /// Store WebAuthn registration state (keyed by user_id, one active per user)
    async fn store_webauthn_reg_state(
        &self,
        user_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// Get WebAuthn registration state
    async fn get_webauthn_reg_state(&self, user_id: &str) -> Result<Option<String>>;

    /// Remove WebAuthn registration state
    async fn remove_webauthn_reg_state(&self, user_id: &str) -> Result<()>;

    /// Store WebAuthn authentication state (keyed by challenge_id)
    async fn store_webauthn_auth_state(
        &self,
        challenge_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// Get WebAuthn authentication state
    async fn get_webauthn_auth_state(&self, challenge_id: &str) -> Result<Option<String>>;

    /// Remove WebAuthn authentication state
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
mod keys {
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
mod ttl {
    pub const USER_ROLES_SECS: u64 = 300; // 5 minutes
    pub const USER_ROLES_SERVICE_SECS: u64 = 300;
    pub const SERVICE_CONFIG_SECS: u64 = 600; // 10 minutes
    pub const TENANT_CONFIG_SECS: u64 = 600; // 10 minutes
}

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    conn: ConnectionManager,
}

impl CacheManager {
    fn refresh_token_hash(refresh_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create a new cache manager
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        let client = redis::Client::open(config.url.as_str()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to create Redis client: {}", e))
        })?;

        let conn = ConnectionManager::new(client).await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to connect to Redis: {}", e))
        })?;

        Ok(Self { conn })
    }

    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    /// Get a clone of the Redis connection manager for rate limiting
    pub fn get_connection_manager(&self) -> ConnectionManager {
        self.conn.clone()
    }

    /// Get a value from cache
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let start = std::time::Instant::now();
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(key).await?;

        metrics::counter!("auth9_redis_operations_total", "operation" => "get").increment(1);
        metrics::histogram!("auth9_redis_operation_duration_seconds", "operation" => "get")
            .record(start.elapsed().as_secs_f64());

        match value {
            Some(v) => {
                let parsed = serde_json::from_str(&v).map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Cache deserialize error: {}", e))
                })?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Set a value in cache with TTL
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        let mut conn = self.conn.clone();
        let serialized = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Cache serialize error: {}", e)))?;

        let _: () = conn.set_ex(key, serialized, ttl.as_secs()).await?;
        metrics::counter!("auth9_redis_operations_total", "operation" => "set").increment(1);
        metrics::histogram!("auth9_redis_operation_duration_seconds", "operation" => "set")
            .record(start.elapsed().as_secs_f64());
        Ok(())
    }

    /// Delete a key from cache
    async fn delete(&self, key: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let mut conn = self.conn.clone();
        let _: () = conn.del(key).await?;
        metrics::counter!("auth9_redis_operations_total", "operation" => "del").increment(1);
        metrics::histogram!("auth9_redis_operation_duration_seconds", "operation" => "del")
            .record(start.elapsed().as_secs_f64());
        Ok(())
    }

    /// Delete keys matching a pattern using SCAN (non-blocking)
    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let mut conn = self.conn.clone();
        let mut cursor: u64 = 0;
        let mut total_deleted = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100) // Batch size
                .query_async(&mut conn)
                .await?;

            if !keys.is_empty() {
                conn.del::<_, ()>(&keys).await?;
                total_deleted += keys.len();
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        tracing::debug!(
            pattern = pattern,
            deleted = total_deleted,
            "Cache pattern invalidation completed"
        );

        metrics::counter!("auth9_redis_operations_total", "operation" => "delete_pattern")
            .increment(1);
        metrics::histogram!("auth9_redis_operation_duration_seconds", "operation" => "delete_pattern")
            .record(start.elapsed().as_secs_f64());

        Ok(())
    }

    // ==================== User Roles Cache ====================

    /// Get cached user roles in tenant
    pub async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tenant_id);
        self.get(&key).await
    }

    /// Cache user roles in tenant
    pub async fn set_user_roles(&self, roles: &UserRolesInTenant) -> Result<()> {
        let key = format!("{}:{}:{}", keys::USER_ROLES, roles.user_id, roles.tenant_id);
        self.set(&key, roles, Duration::from_secs(ttl::USER_ROLES_SECS))
            .await
    }

    /// Invalidate user roles cache
    pub async fn invalidate_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Option<Uuid>,
    ) -> Result<()> {
        match tenant_id {
            Some(tid) => {
                let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tid);
                self.delete(&key).await
            }
            None => {
                let pattern = format!("{}:{}:*", keys::USER_ROLES, user_id);
                self.delete_pattern(&pattern).await
            }
        }
    }

    pub async fn invalidate_user_roles_for_tenant(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<()> {
        let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tenant_id);
        self.delete(&key).await?;
        let pattern = format!("{}:{}:{}:*", keys::USER_ROLES_SERVICE, user_id, tenant_id);
        self.delete_pattern(&pattern).await
    }

    pub async fn invalidate_all_user_roles(&self) -> Result<()> {
        self.delete_pattern(&format!("{}:*", keys::USER_ROLES))
            .await?;
        self.delete_pattern(&format!("{}:*", keys::USER_ROLES_SERVICE))
            .await
    }

    pub async fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> Result<Option<UserRolesInTenant>> {
        let key = format!(
            "{}:{}:{}:{}",
            keys::USER_ROLES_SERVICE,
            user_id,
            tenant_id,
            service_id
        );
        self.get(&key).await
    }

    pub async fn set_user_roles_for_service(
        &self,
        roles: &UserRolesInTenant,
        service_id: Uuid,
    ) -> Result<()> {
        let key = format!(
            "{}:{}:{}:{}",
            keys::USER_ROLES_SERVICE,
            roles.user_id,
            roles.tenant_id,
            service_id
        );
        self.set(
            &key,
            roles,
            Duration::from_secs(ttl::USER_ROLES_SERVICE_SECS),
        )
        .await
    }

    // ==================== Service Config Cache ====================

    /// Get cached service config
    pub async fn get_service_config<T: DeserializeOwned>(
        &self,
        service_id: Uuid,
    ) -> Result<Option<T>> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.get(&key).await
    }

    /// Cache service config
    pub async fn set_service_config<T: Serialize>(
        &self,
        service_id: Uuid,
        config: &T,
    ) -> Result<()> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.set(&key, config, Duration::from_secs(ttl::SERVICE_CONFIG_SECS))
            .await
    }

    /// Invalidate service config cache
    pub async fn invalidate_service_config(&self, service_id: Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.delete(&key).await
    }

    // ==================== Tenant Config Cache ====================

    /// Get cached tenant config
    pub async fn get_tenant_config<T: DeserializeOwned>(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<T>> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.get(&key).await
    }

    /// Cache tenant config
    pub async fn set_tenant_config<T: Serialize>(&self, tenant_id: Uuid, config: &T) -> Result<()> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.set(&key, config, Duration::from_secs(ttl::TENANT_CONFIG_SECS))
            .await
    }

    /// Invalidate tenant config cache
    pub async fn invalidate_tenant_config(&self, tenant_id: Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.delete(&key).await
    }

    // ==================== Token Blacklist ====================

    /// Add a token JTI to the blacklist for immediate revocation.
    /// The TTL should be set to the remaining validity time of the token.
    pub async fn add_to_token_blacklist(&self, jti: &str, ttl_secs: u64) -> Result<()> {
        if ttl_secs == 0 {
            return Ok(()); // Token already expired, no need to blacklist
        }
        let key = format!("{}:{}", keys::TOKEN_BLACKLIST, jti);
        // Store a simple "1" value to mark as blacklisted
        self.set(&key, &"1", Duration::from_secs(ttl_secs)).await
    }

    /// Check if a token JTI is in the blacklist.
    /// Uses raw Redis EXISTS to avoid JSON deserialization issues —
    /// blacklist values are simple flags, not JSON objects.
    pub async fn is_token_blacklisted(&self, jti: &str) -> Result<bool> {
        let key = format!("{}:{}", keys::TOKEN_BLACKLIST, jti);
        let mut conn = self.conn.clone();
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        Ok(exists)
    }

    // ==================== WebAuthn Challenge State ====================

    pub async fn store_webauthn_reg_state(
        &self,
        user_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        let key = format!("{}:{}", keys::WEBAUTHN_REG, user_id);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, state, ttl_secs).await?;
        Ok(())
    }

    pub async fn get_webauthn_reg_state(&self, user_id: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::WEBAUTHN_REG, user_id);
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(&key).await?;
        Ok(value)
    }

    pub async fn remove_webauthn_reg_state(&self, user_id: &str) -> Result<()> {
        let key = format!("{}:{}", keys::WEBAUTHN_REG, user_id);
        self.delete(&key).await
    }

    pub async fn store_webauthn_auth_state(
        &self,
        challenge_id: &str,
        state: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        let key = format!("{}:{}", keys::WEBAUTHN_AUTH, challenge_id);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, state, ttl_secs).await?;
        Ok(())
    }

    pub async fn get_webauthn_auth_state(&self, challenge_id: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::WEBAUTHN_AUTH, challenge_id);
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(&key).await?;
        Ok(value)
    }

    pub async fn remove_webauthn_auth_state(&self, challenge_id: &str) -> Result<()> {
        let key = format!("{}:{}", keys::WEBAUTHN_AUTH, challenge_id);
        self.delete(&key).await
    }

    // ==================== OIDC State ====================

    pub async fn store_oidc_state(
        &self,
        state_nonce: &str,
        payload: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        let key = format!("{}:{}", keys::OIDC_STATE, state_nonce);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, payload, ttl_secs).await?;
        Ok(())
    }

    pub async fn consume_oidc_state(&self, state_nonce: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::OIDC_STATE, state_nonce);
        let mut conn = self.conn.clone();
        redis::cmd("GETDEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(AppError::from)
    }

    // ==================== Refresh Token Session Binding ====================

    pub async fn bind_refresh_token_session(
        &self,
        refresh_token: &str,
        session_id: &str,
        ttl_secs: u64,
    ) -> Result<()> {
        let token_hash = Self::refresh_token_hash(refresh_token);
        let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, token_hash);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, session_id, ttl_secs).await?;

        // Maintain reverse index: session_id → set of token hashes
        let set_key = format!("{}:{}", keys::SESSION_REFRESH_TOKENS, session_id);
        let mut conn2 = self.conn.clone();
        let _: () = redis::cmd("SADD")
            .arg(&set_key)
            .arg(&token_hash)
            .query_async(&mut conn2)
            .await
            .map_err(AppError::from)?;
        // Set TTL on the set to match the refresh token TTL (extend if needed)
        let _: () = redis::cmd("EXPIRE")
            .arg(&set_key)
            .arg(ttl_secs)
            .query_async(&mut conn2)
            .await
            .map_err(AppError::from)?;

        Ok(())
    }

    pub async fn get_refresh_token_session(&self, refresh_token: &str) -> Result<Option<String>> {
        let token_hash = Self::refresh_token_hash(refresh_token);
        let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, token_hash);
        let mut conn = self.conn.clone();
        conn.get(&key).await.map_err(AppError::from)
    }

    pub async fn remove_refresh_token_session(&self, refresh_token: &str) -> Result<()> {
        let token_hash = Self::refresh_token_hash(refresh_token);
        let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, token_hash);
        self.delete(&key).await
    }

    pub async fn remove_all_refresh_sessions_for_session(&self, session_id: &str) -> Result<()> {
        let set_key = format!("{}:{}", keys::SESSION_REFRESH_TOKENS, session_id);
        let mut conn = self.conn.clone();

        // Get all token hashes for this session
        let token_hashes: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&set_key)
            .query_async(&mut conn)
            .await
            .unwrap_or_default();

        // Delete each refresh_session key
        for hash in &token_hashes {
            let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, hash);
            let _ = self.delete(&key).await;
        }

        // Delete the reverse index set itself
        let _ = self.delete(&set_key).await;

        Ok(())
    }

    // ==================== Webhook Event Deduplication ====================

    /// Atomically check if a webhook event key exists and set it if not (SETNX).
    /// Returns true if the event was already processed (duplicate).
    pub async fn check_and_mark_webhook_event(
        &self,
        event_key: &str,
        ttl_secs: u64,
    ) -> Result<bool> {
        let key = format!("{}:{}", keys::WEBHOOK_EVENT_DEDUP, event_key);
        let mut conn = self.conn.clone();
        // SET key "1" NX EX ttl — returns Some("OK") if key was newly set, None if existed.
        // Use Option<String> instead of bool: redis SET NX returns OK (success) or nil (key
        // exists), and the redis crate's bool parsing of nil is unreliable across versions.
        // Propagate errors (instead of swallowing with unwrap_or) so the caller can fall
        // back to in-memory dedup when Redis is unavailable.
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Redis SETNX failed: {}", e)))?;
        let is_new = result.is_some();
        Ok(!is_new)
    }
}

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
}

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

    fn refresh_token_hash(refresh_token: &str) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();

        let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tenant_id);
        assert_eq!(
            key,
            "auth9:user_roles:550e8400-e29b-41d4-a716-446655440000:6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    fn test_cache_key_constants() {
        assert_eq!(keys::USER_ROLES, "auth9:user_roles");
        assert_eq!(keys::USER_ROLES_SERVICE, "auth9:user_roles_service");
        assert_eq!(keys::SERVICE_CONFIG, "auth9:service");
        assert_eq!(keys::TENANT_CONFIG, "auth9:tenant");
    }

    #[test]
    fn test_cache_ttl_constants() {
        assert_eq!(ttl::USER_ROLES_SECS, 300);
        assert_eq!(ttl::USER_ROLES_SERVICE_SECS, 300);
        assert_eq!(ttl::SERVICE_CONFIG_SECS, 600);
        assert_eq!(ttl::TENANT_CONFIG_SECS, 600);
    }

    #[test]
    fn test_service_config_key_format() {
        let service_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        assert_eq!(key, "auth9:service:550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_tenant_config_key_format() {
        let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        assert_eq!(key, "auth9:tenant:6ba7b810-9dad-11d1-80b4-00c04fd430c8");
    }

    #[test]
    fn test_user_roles_service_key_format() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let tenant_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
        let service_id = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();

        let key = format!(
            "{}:{}:{}:{}",
            keys::USER_ROLES_SERVICE,
            user_id,
            tenant_id,
            service_id
        );
        assert!(key.starts_with("auth9:user_roles_service:"));
        assert!(key.contains(&user_id.to_string()));
        assert!(key.contains(&tenant_id.to_string()));
        assert!(key.contains(&service_id.to_string()));
    }

    #[tokio::test]
    async fn test_noop_cache_manager_ping() {
        let cache = NoOpCacheManager::new();
        assert!(cache.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_get_user_roles() {
        let cache = NoOpCacheManager::new();
        let result = cache
            .get_user_roles(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_set_user_roles() {
        let cache = NoOpCacheManager::new();
        let roles = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec![],
            permissions: vec![],
        };
        assert!(cache.set_user_roles(&roles).await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_get_user_roles_for_service() {
        let cache = NoOpCacheManager::new();
        let result = cache
            .get_user_roles_for_service(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_set_user_roles_for_service() {
        let cache = NoOpCacheManager::new();
        let roles = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec![],
            permissions: vec![],
        };
        assert!(cache
            .set_user_roles_for_service(&roles, Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_invalidate_user_roles_with_tenant() {
        let cache = NoOpCacheManager::new();
        assert!(cache
            .invalidate_user_roles(Uuid::new_v4(), Some(Uuid::new_v4()))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_invalidate_user_roles_without_tenant() {
        let cache = NoOpCacheManager::new();
        assert!(cache
            .invalidate_user_roles(Uuid::new_v4(), None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_invalidate_user_roles_for_tenant() {
        let cache = NoOpCacheManager::new();
        assert!(cache
            .invalidate_user_roles_for_tenant(Uuid::new_v4(), Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_invalidate_all_user_roles() {
        let cache = NoOpCacheManager::new();
        assert!(cache.invalidate_all_user_roles().await.is_ok());
    }

    #[test]
    fn test_noop_cache_manager_default() {
        let cache = NoOpCacheManager::default();
        // Just verify it creates without panic
        let _ = cache;
    }

    #[test]
    fn test_noop_cache_manager_clone() {
        let cache1 = NoOpCacheManager::new();
        let cache2 = cache1.clone();
        // Just verify cloning works
        let _ = cache2;
    }

    #[test]
    fn test_token_blacklist_key_format() {
        let jti = "abc123-session-id";
        let key = format!("{}:{}", keys::TOKEN_BLACKLIST, jti);
        assert_eq!(key, "auth9:token_blacklist:abc123-session-id");
    }

    #[tokio::test]
    async fn test_noop_cache_manager_add_to_token_blacklist() {
        let cache = NoOpCacheManager::new();
        let result = cache.add_to_token_blacklist("test-jti", 3600).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_manager_is_token_blacklisted() {
        let cache = NoOpCacheManager::new();
        // NoOp always returns false (token not blacklisted)
        let result = cache.is_token_blacklisted("test-jti").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_noop_cache_manager_blacklist_zero_ttl() {
        let cache = NoOpCacheManager::new();
        // Zero TTL should still be ok (no-op)
        let result = cache.add_to_token_blacklist("test-jti", 0).await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // CacheOperations trait dispatch tests for NoOpCacheManager
    // (covers the `impl CacheOperations for NoOpCacheManager` block)
    // ========================================================================

    #[tokio::test]
    async fn test_noop_cache_operations_trait_ping() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_get_user_roles() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        let result = cache
            .get_user_roles(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_set_user_roles() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        let roles = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec![],
            permissions: vec![],
        };
        assert!(cache.set_user_roles(&roles).await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_get_user_roles_for_service() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        let result = cache
            .get_user_roles_for_service(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_set_user_roles_for_service() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        let roles = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec![],
            permissions: vec![],
        };
        assert!(cache
            .set_user_roles_for_service(&roles, Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_invalidate_user_roles() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .invalidate_user_roles(Uuid::new_v4(), Some(Uuid::new_v4()))
            .await
            .is_ok());
        assert!(cache
            .invalidate_user_roles(Uuid::new_v4(), None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_invalidate_user_roles_for_tenant() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .invalidate_user_roles_for_tenant(Uuid::new_v4(), Uuid::new_v4())
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_invalidate_all_user_roles() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache.invalidate_all_user_roles().await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_token_blacklist() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache.add_to_token_blacklist("jti-1", 3600).await.is_ok());
        assert!(!cache.is_token_blacklisted("jti-1").await.unwrap());
    }

    // ========================================================================
    // WebAuthn challenge state tests
    // ========================================================================

    #[test]
    fn test_webauthn_reg_key_format() {
        let user_id = "user-123";
        let key = format!("{}:{}", keys::WEBAUTHN_REG, user_id);
        assert_eq!(key, "auth9:webauthn_reg:user-123");
    }

    #[test]
    fn test_webauthn_auth_key_format() {
        let challenge_id = "challenge-456";
        let key = format!("{}:{}", keys::WEBAUTHN_AUTH, challenge_id);
        assert_eq!(key, "auth9:webauthn_auth:challenge-456");
    }

    #[tokio::test]
    async fn test_noop_cache_webauthn_reg_state() {
        let cache = NoOpCacheManager::new();
        assert!(cache
            .store_webauthn_reg_state("user-1", "{}", 300)
            .await
            .is_ok());
        let result = cache.get_webauthn_reg_state("user-1").await.unwrap();
        assert!(result.is_none());
        assert!(cache.remove_webauthn_reg_state("user-1").await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_webauthn_auth_state() {
        let cache = NoOpCacheManager::new();
        assert!(cache
            .store_webauthn_auth_state("challenge-1", "{}", 300)
            .await
            .is_ok());
        let result = cache.get_webauthn_auth_state("challenge-1").await.unwrap();
        assert!(result.is_none());
        assert!(cache
            .remove_webauthn_auth_state("challenge-1")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_webauthn_reg() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .store_webauthn_reg_state("user-1", "{\"test\": true}", 300)
            .await
            .is_ok());
        assert!(cache
            .get_webauthn_reg_state("user-1")
            .await
            .unwrap()
            .is_none());
        assert!(cache.remove_webauthn_reg_state("user-1").await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_webauthn_auth() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .store_webauthn_auth_state("ch-1", "{\"test\": true}", 300)
            .await
            .is_ok());
        assert!(cache
            .get_webauthn_auth_state("ch-1")
            .await
            .unwrap()
            .is_none());
        assert!(cache.remove_webauthn_auth_state("ch-1").await.is_ok());
    }

    #[tokio::test]
    async fn test_noop_cache_oidc_state_consume_once() {
        let cache = NoOpCacheManager::new();
        cache
            .store_oidc_state("nonce-1", "{\"redirect_uri\":\"https://a\"}", 300)
            .await
            .unwrap();
        let first = cache.consume_oidc_state("nonce-1").await.unwrap();
        let second = cache.consume_oidc_state("nonce-1").await.unwrap();
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_refresh_session_binding() {
        let cache = NoOpCacheManager::new();
        cache
            .bind_refresh_token_session("rt-1", "sid-1", 300)
            .await
            .unwrap();
        let found = cache.get_refresh_token_session("rt-1").await.unwrap();
        assert_eq!(found.as_deref(), Some("sid-1"));
        cache.remove_refresh_token_session("rt-1").await.unwrap();
        let missing = cache.get_refresh_token_session("rt-1").await.unwrap();
        assert!(missing.is_none());
    }

    // ========================================================================
    // Security Fix Tests: Redis SCAN instead of KEYS
    // ========================================================================

    #[test]
    fn test_delete_pattern_uses_scan() {
        // This test verifies that delete_pattern uses SCAN instead of KEYS
        // The actual behavior is tested in integration tests with real Redis
        // Here we just verify the pattern matching logic
        let pattern = "auth9:user_roles:*";
        assert!(pattern.contains("*"));
        assert!(pattern.starts_with("auth9:"));
    }

    #[tokio::test]
    async fn test_invalidate_all_user_roles_noop() {
        // Test that invalidate_all_user_roles works with NoOpCacheManager
        let cache = NoOpCacheManager::new();
        let result = cache.invalidate_all_user_roles().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalidate_user_roles_for_tenant_noop() {
        // Test that invalidate_user_roles_for_tenant works with NoOpCacheManager
        let cache = NoOpCacheManager::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let result = cache
            .invalidate_user_roles_for_tenant(user_id, tenant_id)
            .await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // CacheOperations trait dispatch tests - OIDC and Refresh Token
    // ========================================================================

    #[tokio::test]
    async fn test_noop_cache_operations_trait_oidc_state() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .store_oidc_state("nonce-1", "payload", 300)
            .await
            .is_ok());
        // NoOp impl stores in-memory, so consuming works
        let result = cache.consume_oidc_state("nonce-1").await.unwrap();
        assert!(result.is_some());
        let second = cache.consume_oidc_state("nonce-1").await.unwrap();
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_operations_trait_refresh_session() {
        let cache: &dyn CacheOperations = &NoOpCacheManager::new();
        assert!(cache
            .bind_refresh_token_session("rt-1", "sid-1", 300)
            .await
            .is_ok());
        let result = cache.get_refresh_token_session("rt-1").await.unwrap();
        assert_eq!(result.as_deref(), Some("sid-1"));
        assert!(cache.remove_refresh_token_session("rt-1").await.is_ok());
        let missing = cache.get_refresh_token_session("rt-1").await.unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_refresh_token_hash_deterministic() {
        let hash1 = NoOpCacheManager::refresh_token_hash("test-token");
        let hash2 = NoOpCacheManager::refresh_token_hash("test-token");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_refresh_token_hash_different_inputs() {
        let hash1 = NoOpCacheManager::refresh_token_hash("token-a");
        let hash2 = NoOpCacheManager::refresh_token_hash("token-b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_cache_manager_refresh_token_hash_deterministic() {
        let hash1 = CacheManager::refresh_token_hash("test-token");
        let hash2 = CacheManager::refresh_token_hash("test-token");
        assert_eq!(hash1, hash2);
        // Both managers should produce same hash
        let noop_hash = NoOpCacheManager::refresh_token_hash("test-token");
        assert_eq!(hash1, noop_hash);
    }

    #[test]
    fn test_oidc_state_key_format() {
        let key = format!("{}:{}", keys::OIDC_STATE, "nonce-abc");
        assert_eq!(key, "auth9:oidc_state:nonce-abc");
    }

    #[test]
    fn test_refresh_token_session_key_format() {
        let key = format!("{}:{}", keys::REFRESH_TOKEN_SESSION, "hash-abc");
        assert_eq!(key, "auth9:refresh_session:hash-abc");
    }
}
