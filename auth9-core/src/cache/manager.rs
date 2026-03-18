//! CacheManager struct and inherent methods

use super::{keys, ttl};
use crate::config::RedisConfig;
use crate::error::{AppError, Result};
use crate::models::rbac::UserRolesInTenant;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use uuid::Uuid;

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    conn: ConnectionManager,
}

impl CacheManager {
    pub(crate) fn refresh_token_hash(refresh_token: &str) -> String {
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
    pub(crate) async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
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
    pub(crate) async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
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
    pub(crate) async fn delete(&self, key: &str) -> Result<()> {
        let start = std::time::Instant::now();
        let mut conn = self.conn.clone();
        let _: () = conn.del(key).await?;
        metrics::counter!("auth9_redis_operations_total", "operation" => "del").increment(1);
        metrics::histogram!("auth9_redis_operation_duration_seconds", "operation" => "del")
            .record(start.elapsed().as_secs_f64());
        Ok(())
    }

    /// Delete keys matching a pattern using SCAN (non-blocking)
    pub(crate) async fn delete_pattern(&self, pattern: &str) -> Result<()> {
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

    // ==================== OTP ====================

    pub async fn store_otp(&self, key: &str, code: &str, ttl_secs: u64) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(key, code, ttl_secs).await?;
        Ok(())
    }

    pub async fn get_otp(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    pub async fn remove_otp(&self, key: &str) -> Result<()> {
        self.delete(key).await
    }

    pub async fn increment_counter(&self, key: &str, ttl_secs: u64) -> Result<u64> {
        let mut conn = self.conn.clone();
        let value: u64 = redis::cmd("INCR")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(AppError::from)?;
        // Set TTL only on first increment (when counter transitions from 0 to 1)
        if value == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(key)
                .arg(ttl_secs)
                .query_async(&mut conn)
                .await
                .map_err(AppError::from)?;
        }
        Ok(value)
    }

    pub async fn get_counter(&self, key: &str) -> Result<u64> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value.and_then(|v| v.parse().ok()).unwrap_or(0))
    }

    pub async fn set_flag(&self, key: &str, ttl_secs: u64) -> Result<bool> {
        let mut conn = self.conn.clone();
        let result: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Redis SETNX failed: {}", e)))?;
        // result is Some("OK") if newly set, None if already existed
        Ok(result.is_none())
    }

    // ==================== Webhook Event Deduplication ====================

    // ==================== TOTP ====================

    pub async fn store_totp_setup(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        let key = format!("{}:{}", keys::TOTP_SETUP, token);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, data, ttl_secs).await?;
        Ok(())
    }

    pub async fn get_totp_setup(&self, token: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::TOTP_SETUP, token);
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(&key).await?;
        Ok(value)
    }

    pub async fn remove_totp_setup(&self, token: &str) -> Result<()> {
        let key = format!("{}:{}", keys::TOTP_SETUP, token);
        self.delete(&key).await
    }

    pub async fn is_totp_code_used(&self, user_id: &str, time_step: u64) -> Result<bool> {
        let key = format!("{}:{}:{}", keys::TOTP_USED, user_id, time_step);
        let mut conn = self.conn.clone();
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        Ok(exists)
    }

    pub async fn mark_totp_code_used(
        &self,
        user_id: &str,
        time_step: u64,
        ttl_secs: u64,
    ) -> Result<()> {
        let key = format!("{}:{}:{}", keys::TOTP_USED, user_id, time_step);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, "1", ttl_secs).await?;
        Ok(())
    }

    // ==================== MFA Session ====================

    pub async fn store_mfa_session(&self, token: &str, data: &str, ttl_secs: u64) -> Result<()> {
        let key = format!("{}:{}", keys::MFA_SESSION, token);
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(&key, data, ttl_secs).await?;
        Ok(())
    }

    pub async fn get_mfa_session(&self, token: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::MFA_SESSION, token);
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(&key).await?;
        Ok(value)
    }

    pub async fn consume_mfa_session(&self, token: &str) -> Result<Option<String>> {
        let key = format!("{}:{}", keys::MFA_SESSION, token);
        let mut conn = self.conn.clone();
        redis::cmd("GETDEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(AppError::from)
    }

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
