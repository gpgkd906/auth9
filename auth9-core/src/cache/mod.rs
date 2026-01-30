//! Redis cache layer

use crate::config::RedisConfig;
use crate::domain::UserRolesInTenant;
use crate::error::{AppError, Result};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
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
}

/// Cache key prefixes
mod keys {
    pub const USER_ROLES: &str = "auth9:user_roles";
    pub const USER_ROLES_SERVICE: &str = "auth9:user_roles_service";
    pub const SERVICE_CONFIG: &str = "auth9:service";
    pub const TENANT_CONFIG: &str = "auth9:tenant";
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

    /// Get a value from cache
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.conn.clone();
        let value: Option<String> = conn.get(key).await?;

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
}

/// No-op cache manager for testing without Redis
#[derive(Clone)]
pub struct NoOpCacheManager;

impl NoOpCacheManager {
    pub fn new() -> Self {
        Self
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
}

impl Default for NoOpCacheManager {
    fn default() -> Self {
        Self::new()
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
}
