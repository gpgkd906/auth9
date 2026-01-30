//! gRPC Token Exchange Service Unit Tests
//!
//! Tests for the TokenExchange gRPC service methods using mock repositories
//! and a no-op cache manager (no Redis required).

use async_trait::async_trait;
use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::JwtConfig;
use auth9_core::domain::{
    AddUserToTenantInput, AssignRolesInput, Client, CreatePermissionInput, CreateRoleInput,
    CreateServiceInput, CreateUserInput, Permission, Role, Service, ServiceStatus, StringUuid,
    TenantUser, UpdateRoleInput, UpdateServiceInput, UpdateUserInput, User, UserRolesInTenant,
};
use auth9_core::error::Result;
use auth9_core::grpc::proto::token_exchange_server::TokenExchange;
use auth9_core::grpc::proto::{
    ExchangeTokenRequest, GetUserRolesRequest, IntrospectTokenRequest, ValidateTokenRequest,
};
use auth9_core::grpc::TokenExchangeService;
use auth9_core::jwt::JwtManager;
use auth9_core::repository::{RbacRepository, ServiceRepository, UserRepository};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::Request;
use uuid::Uuid;

// ============================================================================
// Test Configuration
// ============================================================================

fn test_jwt_config() -> JwtConfig {
    JwtConfig {
        secret: "test-secret-key-for-testing-purposes-only".to_string(),
        issuer: "https://auth9.test".to_string(),
        access_token_ttl_secs: 3600,
        refresh_token_ttl_secs: 604800,
        private_key_pem: None,
        public_key_pem: None,
    }
}

fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

// ============================================================================
// Test Repository Implementations
// ============================================================================

/// Configurable test user repository
struct TestUserRepository {
    users: RwLock<Vec<User>>,
}

impl TestUserRepository {
    fn new() -> Self {
        Self {
            users: RwLock::new(vec![]),
        }
    }

    async fn add_user(&self, user: User) {
        self.users.write().await.push(user);
    }
}

#[async_trait]
impl UserRepository for TestUserRepository {
    async fn create(&self, _keycloak_id: &str, _input: &CreateUserInput) -> Result<User> {
        unimplemented!()
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.email == email).cloned())
    }

    async fn find_by_keycloak_id(&self, _keycloak_id: &str) -> Result<Option<User>> {
        Ok(None)
    }

    async fn list(&self, _offset: i64, _limit: i64) -> Result<Vec<User>> {
        Ok(vec![])
    }

    async fn count(&self) -> Result<i64> {
        Ok(0)
    }

    async fn update(&self, _id: StringUuid, _input: &UpdateUserInput) -> Result<User> {
        unimplemented!()
    }

    async fn update_mfa_enabled(&self, _id: StringUuid, _enabled: bool) -> Result<User> {
        unimplemented!()
    }

    async fn delete(&self, _id: StringUuid) -> Result<()> {
        unimplemented!()
    }

    async fn add_to_tenant(&self, _input: &AddUserToTenantInput) -> Result<TenantUser> {
        unimplemented!()
    }

    async fn remove_from_tenant(&self, _user_id: StringUuid, _tenant_id: StringUuid) -> Result<()> {
        unimplemented!()
    }

    async fn find_tenant_users(
        &self,
        _tenant_id: StringUuid,
        _offset: i64,
        _limit: i64,
    ) -> Result<Vec<User>> {
        Ok(vec![])
    }

    async fn find_user_tenants(&self, _user_id: StringUuid) -> Result<Vec<TenantUser>> {
        Ok(vec![])
    }
}

/// Configurable test service repository
struct TestServiceRepository {
    services: RwLock<Vec<Service>>,
    clients: RwLock<Vec<Client>>,
}

impl TestServiceRepository {
    fn new() -> Self {
        Self {
            services: RwLock::new(vec![]),
            clients: RwLock::new(vec![]),
        }
    }

    async fn add_service(&self, service: Service) {
        self.services.write().await.push(service);
    }

    async fn add_client(&self, client: Client) {
        self.clients.write().await.push(client);
    }
}

#[async_trait]
impl ServiceRepository for TestServiceRepository {
    async fn create(&self, _input: &CreateServiceInput) -> Result<Service> {
        unimplemented!()
    }

    async fn create_client(
        &self,
        _service_id: Uuid,
        _client_id: &str,
        _secret_hash: &str,
        _name: Option<String>,
    ) -> Result<Client> {
        unimplemented!()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>> {
        let services = self.services.read().await;
        Ok(services.iter().find(|s| s.id.0 == id).cloned())
    }

    async fn find_by_client_id(&self, _client_id: &str) -> Result<Option<Service>> {
        Ok(None)
    }

    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<Client>> {
        let clients = self.clients.read().await;
        Ok(clients.iter().find(|c| c.client_id == client_id).cloned())
    }

    async fn list(
        &self,
        _tenant_id: Option<Uuid>,
        _offset: i64,
        _limit: i64,
    ) -> Result<Vec<Service>> {
        Ok(vec![])
    }

    async fn list_clients(&self, _service_id: Uuid) -> Result<Vec<Client>> {
        Ok(vec![])
    }

    async fn count(&self, _tenant_id: Option<Uuid>) -> Result<i64> {
        Ok(0)
    }

    async fn update(&self, _id: Uuid, _input: &UpdateServiceInput) -> Result<Service> {
        unimplemented!()
    }

    async fn delete(&self, _id: Uuid) -> Result<()> {
        unimplemented!()
    }

    async fn delete_client(&self, _service_id: Uuid, _client_id: &str) -> Result<()> {
        unimplemented!()
    }

    async fn update_client_secret_hash(
        &self,
        _client_id: &str,
        _new_secret_hash: &str,
    ) -> Result<()> {
        unimplemented!()
    }
}

/// Configurable test RBAC repository
struct TestRbacRepository {
    user_roles: RwLock<Vec<(Uuid, Uuid, UserRolesInTenant)>>, // (user_id, tenant_id, roles)
    user_roles_for_service: RwLock<Vec<(Uuid, Uuid, Uuid, UserRolesInTenant)>>, // (user_id, tenant_id, service_id, roles)
    role_records: RwLock<Vec<Role>>,
}

impl TestRbacRepository {
    fn new() -> Self {
        Self {
            user_roles: RwLock::new(vec![]),
            user_roles_for_service: RwLock::new(vec![]),
            role_records: RwLock::new(vec![]),
        }
    }

    async fn set_user_roles(&self, user_id: Uuid, tenant_id: Uuid, roles: UserRolesInTenant) {
        self.user_roles
            .write()
            .await
            .push((user_id, tenant_id, roles));
    }

    async fn set_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
        roles: UserRolesInTenant,
    ) {
        self.user_roles_for_service
            .write()
            .await
            .push((user_id, tenant_id, service_id, roles));
    }

    async fn add_role_record(&self, role: Role) {
        self.role_records.write().await.push(role);
    }
}

#[async_trait]
impl RbacRepository for TestRbacRepository {
    async fn create_permission(&self, _input: &CreatePermissionInput) -> Result<Permission> {
        unimplemented!()
    }

    async fn find_permission_by_id(&self, _id: StringUuid) -> Result<Option<Permission>> {
        Ok(None)
    }

    async fn find_permissions_by_service(
        &self,
        _service_id: StringUuid,
    ) -> Result<Vec<Permission>> {
        Ok(vec![])
    }

    async fn delete_permission(&self, _id: StringUuid) -> Result<()> {
        unimplemented!()
    }

    async fn create_role(&self, _input: &CreateRoleInput) -> Result<Role> {
        unimplemented!()
    }

    async fn find_role_by_id(&self, _id: StringUuid) -> Result<Option<Role>> {
        Ok(None)
    }

    async fn find_roles_by_service(&self, _service_id: StringUuid) -> Result<Vec<Role>> {
        Ok(vec![])
    }

    async fn update_role(&self, _id: StringUuid, _input: &UpdateRoleInput) -> Result<Role> {
        unimplemented!()
    }

    async fn delete_role(&self, _id: StringUuid) -> Result<()> {
        unimplemented!()
    }

    async fn assign_permission_to_role(
        &self,
        _role_id: StringUuid,
        _permission_id: StringUuid,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn remove_permission_from_role(
        &self,
        _role_id: StringUuid,
        _permission_id: StringUuid,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn find_role_permissions(&self, _role_id: StringUuid) -> Result<Vec<Permission>> {
        Ok(vec![])
    }

    async fn assign_roles_to_user(
        &self,
        _input: &AssignRolesInput,
        _granted_by: Option<StringUuid>,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn remove_role_from_user(
        &self,
        _tenant_user_id: StringUuid,
        _role_id: StringUuid,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn find_tenant_user_id(
        &self,
        _user_id: StringUuid,
        _tenant_id: StringUuid,
    ) -> Result<Option<StringUuid>> {
        Ok(None)
    }

    async fn find_user_roles_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<UserRolesInTenant> {
        let roles = self.user_roles.read().await;
        for (uid, tid, r) in roles.iter() {
            if *uid == user_id.0 && *tid == tenant_id.0 {
                return Ok(r.clone());
            }
        }
        Ok(UserRolesInTenant {
            user_id: user_id.0,
            tenant_id: tenant_id.0,
            roles: vec![],
            permissions: vec![],
        })
    }

    async fn find_user_roles_in_tenant_for_service(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        service_id: StringUuid,
    ) -> Result<UserRolesInTenant> {
        let roles = self.user_roles_for_service.read().await;
        for (uid, tid, sid, r) in roles.iter() {
            if *uid == user_id.0 && *tid == tenant_id.0 && *sid == service_id.0 {
                return Ok(r.clone());
            }
        }
        Ok(UserRolesInTenant {
            user_id: user_id.0,
            tenant_id: tenant_id.0,
            roles: vec![],
            permissions: vec![],
        })
    }

    async fn find_user_role_records_in_tenant(
        &self,
        _user_id: StringUuid,
        _tenant_id: StringUuid,
        _service_id: Option<StringUuid>,
    ) -> Result<Vec<Role>> {
        Ok(self.role_records.read().await.clone())
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_user(id: Uuid) -> User {
    User {
        id: StringUuid::from(id),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        keycloak_id: "kc-user-id".to_string(),
        mfa_enabled: false,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn create_test_service(id: Uuid, tenant_id: Uuid) -> Service {
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

fn create_test_client(id: Uuid, service_id: Uuid) -> Client {
    Client {
        id: StringUuid::from(id),
        service_id: StringUuid::from(service_id),
        client_id: "test-client".to_string(),
        name: Some("Test Client".to_string()),
        client_secret_hash: "hash".to_string(),
        created_at: chrono::Utc::now(),
    }
}

// ============================================================================
// exchange_token tests
// ============================================================================

#[tokio::test]
async fn test_exchange_token_success() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    // Setup test repositories
    let user_repo = Arc::new(TestUserRepository::new());
    user_repo.add_user(create_test_user(user_id)).await;

    let service_repo = Arc::new(TestServiceRepository::new());
    service_repo
        .add_service(create_test_service(service_id, tenant_id))
        .await;
    service_repo
        .add_client(create_test_client(client_id, service_id))
        .await;

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec!["admin".to_string()],
                permissions: vec!["user:read".to_string(), "user:write".to_string()],
            },
        )
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert_eq!(response.token_type, "Bearer");
    assert!(response.expires_in > 0);
    assert!(!response.refresh_token.is_empty());
}

#[tokio::test]
async fn test_exchange_token_invalid_identity_token() {
    let cache_manager = create_test_cache();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token: "invalid-token".to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn test_exchange_token_user_not_found() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    // Empty user repository - user won't be found
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_exchange_token_client_not_found() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    // Add user but no client
    let user_repo = Arc::new(TestUserRepository::new());
    user_repo.add_user(create_test_user(user_id)).await;

    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "nonexistent-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_exchange_token_invalid_tenant_id() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: "invalid-uuid".to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

// ============================================================================
// validate_token tests
// ============================================================================

#[tokio::test]
async fn test_validate_token_success() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec!["admin".to_string()],
            vec!["user:read".to_string()],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "test-client".to_string(),
    });

    let response = grpc_service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.valid);
    assert_eq!(response.user_id, user_id.to_string());
    assert_eq!(response.tenant_id, tenant_id.to_string());
    assert!(response.error.is_empty());
}

#[tokio::test]
async fn test_validate_token_invalid_token() {
    let cache_manager = create_test_cache();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ValidateTokenRequest {
        access_token: "invalid-token".to_string(),
        audience: "test-client".to_string(),
    });

    let response = grpc_service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
    assert!(!response.error.is_empty());
}

#[tokio::test]
async fn test_validate_token_wrong_audience() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "service-a",
            vec![],
            vec![],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "service-b".to_string(),
    });

    let response = grpc_service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
}

#[tokio::test]
async fn test_validate_token_empty_audience() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec![],
            vec![],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: String::new(),
    });

    let response = grpc_service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.valid);
}

// ============================================================================
// get_user_roles tests
// ============================================================================

#[tokio::test]
async fn test_get_user_roles_success() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles(
            user_id,
            tenant_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec!["admin".to_string(), "viewer".to_string()],
                permissions: vec!["user:read".to_string(), "user:write".to_string()],
            },
        )
        .await;
    rbac_repo
        .add_role_record(Role {
            id: StringUuid::from(role_id),
            service_id: StringUuid::from(service_id),
            name: "admin".to_string(),
            description: Some("Administrator role".to_string()),
            parent_role_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.roles.is_empty());
    assert!(!response.permissions.is_empty());
}

#[tokio::test]
async fn test_get_user_roles_with_service_filter() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec!["admin".to_string()],
                permissions: vec!["service:manage".to_string()],
            },
        )
        .await;
    rbac_repo
        .add_role_record(Role {
            id: StringUuid::from(role_id),
            service_id: StringUuid::from(service_id),
            name: "admin".to_string(),
            description: None,
            parent_role_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: service_id.to_string(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.roles.len(), 1);
    assert_eq!(response.roles[0].name, "admin");
}

#[tokio::test]
async fn test_get_user_roles_invalid_user_id() {
    let cache_manager = create_test_cache();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: "invalid-uuid".to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_invalid_tenant_id() {
    let cache_manager = create_test_cache();
    let user_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: "invalid-uuid".to_string(),
        service_id: String::new(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

// ============================================================================
// introspect_token tests
// ============================================================================

#[tokio::test]
async fn test_introspect_tenant_access_token() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec!["admin".to_string()],
            vec!["user:read".to_string()],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = grpc_service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.sub, user_id.to_string());
    assert_eq!(response.email, "test@example.com");
    assert_eq!(response.tenant_id, tenant_id.to_string());
    assert_eq!(response.roles, vec!["admin"]);
    assert_eq!(response.permissions, vec!["user:read"]);
}

#[tokio::test]
async fn test_introspect_identity_token() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(IntrospectTokenRequest {
        token: identity_token,
    });

    let response = grpc_service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.sub, user_id.to_string());
    assert_eq!(response.email, "test@example.com");
    assert!(response.tenant_id.is_empty());
    assert!(response.roles.is_empty());
}

#[tokio::test]
async fn test_introspect_invalid_token() {
    let cache_manager = create_test_cache();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(IntrospectTokenRequest {
        token: "invalid-token".to_string(),
    });

    let response = grpc_service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.active);
    assert!(response.sub.is_empty());
    assert!(response.email.is_empty());
}

#[tokio::test]
async fn test_introspect_expired_token() {
    let cache_manager = create_test_cache();

    // Create a JWT manager with very negative TTL to ensure token is expired
    let mut config = test_jwt_config();
    config.access_token_ttl_secs = -3600;

    let expired_jwt_manager = JwtManager::new(config);
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let access_token = expired_jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec![],
            vec![],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    // Use fresh jwt_manager for verification (with normal TTL)
    let grpc_service = TokenExchangeService::new(
        JwtManager::new(test_jwt_config()),
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = grpc_service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.active);
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[tokio::test]
async fn test_exchange_token_with_empty_roles() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    user_repo.add_user(create_test_user(user_id)).await;

    let service_repo = Arc::new(TestServiceRepository::new());
    service_repo
        .add_service(create_test_service(service_id, tenant_id))
        .await;
    service_repo
        .add_client(create_test_client(client_id, service_id))
        .await;

    // Empty roles
    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec![],
                permissions: vec![],
            },
        )
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
}

#[tokio::test]
async fn test_get_user_roles_empty_result() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.roles.is_empty());
    assert!(response.permissions.is_empty());
}

#[tokio::test]
async fn test_get_user_roles_invalid_service_id() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: "not-a-uuid".to_string(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_err());

    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
}

// ============================================================================
// Additional coverage tests for edge cases
// ============================================================================

#[tokio::test]
async fn test_exchange_token_with_multiple_permissions() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "multi-perm@example.com", Some("Multi Perm User"))
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    user_repo
        .add_user(User {
            id: StringUuid::from(user_id),
            email: "multi-perm@example.com".to_string(),
            display_name: Some("Multi Perm User".to_string()),
            avatar_url: None,
            keycloak_id: "kc-multi".to_string(),
            mfa_enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;

    let service_repo = Arc::new(TestServiceRepository::new());
    service_repo
        .add_service(create_test_service(service_id, tenant_id))
        .await;
    service_repo
        .add_client(create_test_client(client_id, service_id))
        .await;

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec![
                    "admin".to_string(),
                    "editor".to_string(),
                    "viewer".to_string(),
                ],
                permissions: vec![
                    "users:read".to_string(),
                    "users:write".to_string(),
                    "users:delete".to_string(),
                    "tenants:read".to_string(),
                    "tenants:manage".to_string(),
                ],
            },
        )
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager.clone(),
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());

    // Verify the token contains the roles
    let claims = jwt_manager
        .verify_tenant_access_token(&response.access_token, Some("test-client"))
        .unwrap();
    assert_eq!(claims.roles.len(), 3);
    assert_eq!(claims.permissions.len(), 5);
}

#[tokio::test]
async fn test_validate_token_with_special_characters_in_audience() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "service-with-special-chars_123",
            vec![],
            vec![],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "service-with-special-chars_123".to_string(),
    });

    let response = grpc_service.validate_token(request).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().valid);
}

#[tokio::test]
async fn test_introspect_token_with_all_fields() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let access_token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "full@example.com",
            tenant_id,
            "full-service",
            vec!["super-admin".to_string(), "manager".to_string()],
            vec![
                "all:read".to_string(),
                "all:write".to_string(),
                "all:delete".to_string(),
            ],
        )
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = grpc_service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.email, "full@example.com");
    assert_eq!(response.roles, vec!["super-admin", "manager"]);
    assert_eq!(
        response.permissions,
        vec!["all:read", "all:write", "all:delete"]
    );
    assert!(response.exp > 0);
    assert!(response.iat > 0);
    assert!(!response.iss.is_empty());
    assert_eq!(response.aud, "full-service");
}

#[tokio::test]
async fn test_get_user_roles_with_multiple_role_records() {
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id_1 = Uuid::new_v4();
    let role_id_2 = Uuid::new_v4();
    let role_id_3 = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles(
            user_id,
            tenant_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec![
                    "admin".to_string(),
                    "editor".to_string(),
                    "viewer".to_string(),
                ],
                permissions: vec!["read".to_string(), "write".to_string()],
            },
        )
        .await;

    // Add multiple role records
    rbac_repo
        .add_role_record(Role {
            id: StringUuid::from(role_id_1),
            service_id: StringUuid::from(service_id),
            name: "admin".to_string(),
            description: Some("Full admin".to_string()),
            parent_role_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;
    rbac_repo
        .add_role_record(Role {
            id: StringUuid::from(role_id_2),
            service_id: StringUuid::from(service_id),
            name: "editor".to_string(),
            description: Some("Content editor".to_string()),
            parent_role_id: Some(StringUuid::from(role_id_1)),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;
    rbac_repo
        .add_role_record(Role {
            id: StringUuid::from(role_id_3),
            service_id: StringUuid::from(service_id),
            name: "viewer".to_string(),
            description: None,
            parent_role_id: Some(StringUuid::from(role_id_2)),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = grpc_service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.roles.len(), 3);
    assert_eq!(response.permissions.len(), 2);

    // Verify role details
    let role_names: Vec<&str> = response.roles.iter().map(|r| r.name.as_str()).collect();
    assert!(role_names.contains(&"admin"));
    assert!(role_names.contains(&"editor"));
    assert!(role_names.contains(&"viewer"));
}

#[tokio::test]
async fn test_exchange_token_service_lookup_chain() {
    // Test the full lookup chain: client -> service -> roles
    let cache_manager = create_test_cache();

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_id = Uuid::new_v4();

    let jwt_manager = JwtManager::new(test_jwt_config());
    let identity_token = jwt_manager
        .create_identity_token(user_id, "chain@example.com", None)
        .unwrap();

    let user_repo = Arc::new(TestUserRepository::new());
    user_repo.add_user(create_test_user(user_id)).await;

    let service_repo = Arc::new(TestServiceRepository::new());
    // Add service with specific tenant
    service_repo
        .add_service(Service {
            id: StringUuid::from(service_id),
            tenant_id: Some(StringUuid::from(tenant_id)),
            name: "Chain Test Service".to_string(),
            base_url: Some("https://chain.example.com".to_string()),
            redirect_uris: vec!["https://chain.example.com/cb".to_string()],
            logout_uris: vec!["https://chain.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;
    // Add client with custom client_id
    service_repo
        .add_client(Client {
            id: StringUuid::from(client_id),
            service_id: StringUuid::from(service_id),
            client_id: "chain-client-id".to_string(),
            name: Some("Chain Client".to_string()),
            client_secret_hash: "hash".to_string(),
            created_at: chrono::Utc::now(),
        })
        .await;

    let rbac_repo = Arc::new(TestRbacRepository::new());
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            UserRolesInTenant {
                user_id,
                tenant_id,
                roles: vec!["chain-role".to_string()],
                permissions: vec!["chain:execute".to_string()],
            },
        )
        .await;

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    // Use the custom client_id
    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "chain-client-id".to_string(),
    });

    let response = grpc_service.exchange_token(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert_eq!(response.token_type, "Bearer");
}
