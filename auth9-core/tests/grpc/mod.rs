//! gRPC Token Exchange Service Tests
//!
//! Tests for the TokenExchange gRPC service methods using shared test repositories
//! from `tests/api/mod.rs` and a mock cache manager for testing cache behavior.

pub mod exchange_token_test;
pub mod get_user_roles_test;
pub mod introspect_token_test;
pub mod validate_token_test;

use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::JwtConfig;
use auth9_core::domain::{
    Client, Permission, Role, Service, ServiceStatus, StringUuid, User, UserRolesInTenant,
};
use auth9_core::grpc::token_exchange::TokenExchangeCache;
use auth9_core::grpc::TokenExchangeService;
use auth9_core::jwt::JwtManager;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Re-export test repositories from api module
pub use crate::api::{TestRbacRepository, TestServiceRepository, TestUserRepository};

// ============================================================================
// Test Configuration
// ============================================================================

pub fn test_jwt_config() -> JwtConfig {
    JwtConfig {
        secret: "test-secret-key-for-testing-purposes-only".to_string(),
        issuer: "https://auth9.test".to_string(),
        access_token_ttl_secs: 3600,
        refresh_token_ttl_secs: 604800,
        private_key_pem: None,
        public_key_pem: None,
        previous_public_key_pem: None,
    }
}

pub fn create_test_jwt_manager() -> JwtManager {
    JwtManager::new(test_jwt_config())
}

#[allow(dead_code)]
pub fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

// ============================================================================
// Mock Cache Manager for testing cache hit/miss behavior
// ============================================================================

/// Mock cache manager that tracks cache hits and allows pre-populating cache
pub struct MockCacheManager {
    /// Cached roles by (user_id, tenant_id)
    pub cached_roles: RwLock<HashMap<(Uuid, Uuid), UserRolesInTenant>>,
    /// Cached roles by (user_id, tenant_id, service_id)
    pub cached_roles_for_service: RwLock<HashMap<(Uuid, Uuid, Uuid), UserRolesInTenant>>,
    /// Count of cache get operations
    pub get_count: AtomicU32,
    /// Count of cache set operations
    pub set_count: AtomicU32,
}

impl MockCacheManager {
    pub fn new() -> Self {
        Self {
            cached_roles: RwLock::new(HashMap::new()),
            cached_roles_for_service: RwLock::new(HashMap::new()),
            get_count: AtomicU32::new(0),
            set_count: AtomicU32::new(0),
        }
    }

    /// Pre-populate cache with roles for testing cache hits
    pub async fn seed_roles(&self, user_id: Uuid, tenant_id: Uuid, roles: UserRolesInTenant) {
        self.cached_roles
            .write()
            .await
            .insert((user_id, tenant_id), roles);
    }

    /// Pre-populate cache with roles for service
    pub async fn seed_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
        roles: UserRolesInTenant,
    ) {
        self.cached_roles_for_service
            .write()
            .await
            .insert((user_id, tenant_id, service_id), roles);
    }

    #[allow(dead_code)]
    pub fn get_count(&self) -> u32 {
        self.get_count.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn set_count(&self) -> u32 {
        self.set_count.load(Ordering::Relaxed)
    }
}

impl Default for MockCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenExchangeCache for MockCacheManager {
    async fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> auth9_core::error::Result<Option<UserRolesInTenant>> {
        self.get_count.fetch_add(1, Ordering::Relaxed);
        let cache = self.cached_roles_for_service.read().await;
        Ok(cache.get(&(user_id, tenant_id, service_id)).cloned())
    }

    async fn set_user_roles_for_service(
        &self,
        roles: &UserRolesInTenant,
        service_id: Uuid,
    ) -> auth9_core::error::Result<()> {
        self.set_count.fetch_add(1, Ordering::Relaxed);
        self.cached_roles_for_service
            .write()
            .await
            .insert((roles.user_id, roles.tenant_id, service_id), roles.clone());
        Ok(())
    }

    async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> auth9_core::error::Result<Option<UserRolesInTenant>> {
        self.get_count.fetch_add(1, Ordering::Relaxed);
        let cache = self.cached_roles.read().await;
        Ok(cache.get(&(user_id, tenant_id)).cloned())
    }

    async fn set_user_roles(&self, roles: &UserRolesInTenant) -> auth9_core::error::Result<()> {
        self.set_count.fetch_add(1, Ordering::Relaxed);
        self.cached_roles
            .write()
            .await
            .insert((roles.user_id, roles.tenant_id), roles.clone());
        Ok(())
    }
}

// ============================================================================
// Test Data Helpers
// ============================================================================

pub fn create_test_user(id: Uuid) -> User {
    User {
        id: StringUuid::from(id),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        keycloak_id: "kc-user-id".to_string(),
        mfa_enabled: false,
        password_changed_at: None,
        locked_until: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

pub fn create_test_user_with_email(id: Uuid, email: &str) -> User {
    User {
        id: StringUuid::from(id),
        email: email.to_string(),
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        keycloak_id: format!("kc-{}", id),
        mfa_enabled: false,
        password_changed_at: None,
        locked_until: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

pub fn create_test_service(id: Uuid, tenant_id: Uuid) -> Service {
    Service {
        id: StringUuid::from(id),
        tenant_id: Some(StringUuid::from(tenant_id)),
        name: "Test Service".to_string(),
        base_url: Some("https://test.example.com".to_string()),
        redirect_uris: vec!["https://test.example.com/callback".to_string()],
        logout_uris: vec![],
        status: ServiceStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

pub fn create_test_service_without_tenant(id: Uuid) -> Service {
    Service {
        id: StringUuid::from(id),
        tenant_id: None,
        name: "Global Service".to_string(),
        base_url: Some("https://global.example.com".to_string()),
        redirect_uris: vec!["https://global.example.com/callback".to_string()],
        logout_uris: vec![],
        status: ServiceStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

pub fn create_test_client(id: Uuid, service_id: Uuid, client_id: &str) -> Client {
    Client {
        id: StringUuid::from(id),
        service_id: StringUuid::from(service_id),
        client_id: client_id.to_string(),
        name: Some("Test Client".to_string()),
        client_secret_hash: "hash".to_string(),
        created_at: chrono::Utc::now(),
    }
}

pub fn create_test_role(id: Uuid, service_id: Uuid, name: &str) -> Role {
    Role {
        id: StringUuid::from(id),
        service_id: StringUuid::from(service_id),
        name: name.to_string(),
        description: Some(format!("{} role", name)),
        parent_role_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[allow(dead_code)]
pub fn create_test_permission(id: Uuid, service_id: Uuid, name: &str) -> Permission {
    Permission {
        id: StringUuid::from(id),
        service_id: StringUuid::from(service_id),
        code: name.to_string(),
        name: name.to_string(),
        description: Some(format!("{} permission", name)),
    }
}

pub fn create_user_roles(
    user_id: Uuid,
    tenant_id: Uuid,
    roles: Vec<String>,
    permissions: Vec<String>,
) -> UserRolesInTenant {
    UserRolesInTenant {
        user_id,
        tenant_id,
        roles,
        permissions,
    }
}

// ============================================================================
// Service Builder for tests
// ============================================================================

pub struct GrpcTestBuilder {
    pub user_repo: Arc<TestUserRepository>,
    pub service_repo: Arc<TestServiceRepository>,
    pub rbac_repo: Arc<TestRbacRepository>,
    pub jwt_manager: JwtManager,
}

impl GrpcTestBuilder {
    pub fn new() -> Self {
        Self {
            user_repo: Arc::new(TestUserRepository::new()),
            service_repo: Arc::new(TestServiceRepository::new()),
            rbac_repo: Arc::new(TestRbacRepository::new()),
            jwt_manager: create_test_jwt_manager(),
        }
    }

    #[allow(dead_code)]
    pub fn with_jwt_config(mut self, config: JwtConfig) -> Self {
        self.jwt_manager = JwtManager::new(config);
        self
    }

    pub async fn with_user(self, user: User) -> Self {
        self.user_repo.add_user(user).await;
        self
    }

    pub async fn with_service(self, service: Service) -> Self {
        self.service_repo.add_service(service).await;
        self
    }

    pub async fn with_client(self, client: Client) -> Self {
        self.service_repo.add_client(client).await;
        self
    }

    pub async fn with_user_roles(
        self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
        roles: UserRolesInTenant,
    ) -> Self {
        self.rbac_repo
            .set_user_roles_for_service(user_id, tenant_id, service_id, roles)
            .await;
        self
    }

    #[allow(dead_code)]
    pub async fn with_role_record(self, role: Role) -> Self {
        self.rbac_repo.add_role(role).await;
        self
    }

    pub fn build_with_noop_cache(
        self,
    ) -> TokenExchangeService<
        TestUserRepository,
        TestServiceRepository,
        TestRbacRepository,
        NoOpCacheManager,
    > {
        TokenExchangeService::new(
            self.jwt_manager,
            NoOpCacheManager::new(),
            self.user_repo,
            self.service_repo,
            self.rbac_repo,
            false,
        )
    }

    pub fn build_with_mock_cache(
        self,
        cache: MockCacheManager,
    ) -> TokenExchangeService<
        TestUserRepository,
        TestServiceRepository,
        TestRbacRepository,
        MockCacheManager,
    > {
        TokenExchangeService::new(
            self.jwt_manager,
            cache,
            self.user_repo,
            self.service_repo,
            self.rbac_repo,
            false,
        )
    }
}

impl Default for GrpcTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
