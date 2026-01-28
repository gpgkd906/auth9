//! Redis cache layer

use crate::config::RedisConfig;
use crate::domain::UserRolesInTenant;
use crate::error::{AppError, Result};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Cache key prefixes
mod keys {
    pub const USER_ROLES: &str = "auth9:user_roles";
    pub const SERVICE_CONFIG: &str = "auth9:service";
    pub const TENANT_CONFIG: &str = "auth9:tenant";
}

/// Default TTLs
mod ttl {
    pub const USER_ROLES_SECS: u64 = 300; // 5 minutes
    pub const SERVICE_CONFIG_SECS: u64 = 600; // 10 minutes
    pub const TENANT_CONFIG_SECS: u64 = 600; // 10 minutes
}

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    conn: ConnectionManager,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new(config: &RedisConfig) -> Result<Self> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create Redis client: {}", e)))?;
        
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Redis: {}", e)))?;
        
        Ok(Self { conn })
    }

    /// Get a value from cache
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(key).await?;
        
        match value {
            Some(v) => {
                let parsed = serde_json::from_str(&v)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Cache deserialize error: {}", e)))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Set a value in cache with TTL
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let mut conn = self.conn.clone();
        let serialized = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Cache serialize error: {}", e)))?;
        
        let _: () = conn.set_ex(key, serialized, ttl.as_secs()).await?;
        Ok(())
    }

    /// Delete a key from cache
    async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        let _: () = conn.del(key).await?;
        Ok(())
    }

    /// Delete keys matching a pattern
    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut conn = self.conn.clone();
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await?;
        
        if !keys.is_empty() {
            conn.del::<_, ()>(keys).await?;
        }
        Ok(())
    }

    // ==================== User Roles Cache ====================

    /// Get cached user roles in tenant
    pub async fn get_user_roles(&self, user_id: Uuid, tenant_id: Uuid) -> Result<Option<UserRolesInTenant>> {
        let key = format!("{}:{}:{}", keys::USER_ROLES, user_id, tenant_id);
        self.get(&key).await
    }

    /// Cache user roles in tenant
    pub async fn set_user_roles(&self, roles: &UserRolesInTenant) -> Result<()> {
        let key = format!("{}:{}:{}", keys::USER_ROLES, roles.user_id, roles.tenant_id);
        self.set(&key, roles, Duration::from_secs(ttl::USER_ROLES_SECS)).await
    }

    /// Invalidate user roles cache
    pub async fn invalidate_user_roles(&self, user_id: Uuid, tenant_id: Option<Uuid>) -> Result<()> {
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

    // ==================== Service Config Cache ====================

    /// Get cached service config
    pub async fn get_service_config<T: DeserializeOwned>(&self, service_id: Uuid) -> Result<Option<T>> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.get(&key).await
    }

    /// Cache service config
    pub async fn set_service_config<T: Serialize>(&self, service_id: Uuid, config: &T) -> Result<()> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.set(&key, config, Duration::from_secs(ttl::SERVICE_CONFIG_SECS)).await
    }

    /// Invalidate service config cache
    pub async fn invalidate_service_config(&self, service_id: Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::SERVICE_CONFIG, service_id);
        self.delete(&key).await
    }

    // ==================== Tenant Config Cache ====================

    /// Get cached tenant config
    pub async fn get_tenant_config<T: DeserializeOwned>(&self, tenant_id: Uuid) -> Result<Option<T>> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.get(&key).await
    }

    /// Cache tenant config
    pub async fn set_tenant_config<T: Serialize>(&self, tenant_id: Uuid, config: &T) -> Result<()> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.set(&key, config, Duration::from_secs(ttl::TENANT_CONFIG_SECS)).await
    }

    /// Invalidate tenant config cache
    pub async fn invalidate_tenant_config(&self, tenant_id: Uuid) -> Result<()> {
        let key = format!("{}:{}", keys::TENANT_CONFIG, tenant_id);
        self.delete(&key).await
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
}
