//! API integration tests infrastructure
//!
//! This module provides test utilities for API handler testing without
//! external dependencies (no database, no Redis, no Keycloak).

pub mod http;
pub mod role_api_test;
pub mod tenant_api_test;
pub mod user_api_test;

// Re-export MockKeycloakServer for use in http tests
pub use http::mock_keycloak::MockKeycloakServer;

use async_trait::async_trait;
use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::JwtConfig;
use auth9_core::domain::{
    AddUserToTenantInput, AlertSeverity, AssignRolesInput, Client, CreateInvitationInput,
    CreateLinkedIdentityInput, CreateLoginEventInput, CreatePasswordResetTokenInput,
    CreatePasskeyInput, CreatePermissionInput, CreateRoleInput, CreateSecurityAlertInput,
    CreateServiceInput, CreateSessionInput, CreateTenantInput, CreateUserInput, CreateWebhookInput,
    Invitation, InvitationStatus, LinkedIdentity, LoginEvent, LoginEventType, LoginStats,
    PasswordResetToken, Permission, Role, SecurityAlert, Service, ServiceStatus, Session,
    StoredPasskey, StringUuid, SystemSettingRow, Tenant, TenantSettings, TenantStatus, TenantUser,
    UpdateRoleInput, UpdateServiceInput, UpdateTenantInput, UpdateUserInput, UpdateWebhookInput,
    UpsertSystemSettingInput, User, UserRolesInTenant, Webhook,
};
use auth9_core::error::{AppError, Result};
use auth9_core::jwt::JwtManager;
use auth9_core::repository::audit::{
    AuditLog, AuditLogQuery, AuditRepository, CreateAuditLogInput,
};
use auth9_core::repository::{
    InvitationRepository, LinkedIdentityRepository, LoginEventRepository, PasswordResetRepository,
    RbacRepository, SecurityAlertRepository, ServiceRepository, SessionRepository,
    SystemSettingsRepository, TenantRepository, UserRepository, WebAuthnRepository,
    WebhookRepository,
};
use auth9_core::service::{ClientService, RbacService, TenantService, UserService};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Test Configuration
// ============================================================================

pub fn test_jwt_config() -> JwtConfig {
    JwtConfig {
        secret: "test-secret-key-for-api-testing-purposes".to_string(),
        issuer: "https://auth9.test".to_string(),
        access_token_ttl_secs: 3600,
        refresh_token_ttl_secs: 604800,
        private_key_pem: None,
        public_key_pem: None,
    }
}

pub fn create_test_jwt_manager() -> JwtManager {
    JwtManager::new(test_jwt_config())
}

/// Create an identity token for platform-level testing (platform admin)
///
/// Uses "admin@auth9.local" to match the `platform_admin_emails` allowlist
/// in test configs, so the token passes `is_platform_admin_email()` checks.
pub fn create_test_identity_token() -> String {
    let jwt_manager = create_test_jwt_manager();
    let user_id = Uuid::new_v4();
    jwt_manager
        .create_identity_token(user_id, "admin@auth9.local", Some("Platform Admin"))
        .expect("Failed to create test identity token")
}

/// Create an identity token for a specific user ID (platform-level)
pub fn create_test_identity_token_for_user(user_id: Uuid) -> String {
    let jwt_manager = create_test_jwt_manager();
    jwt_manager
        .create_identity_token(user_id, "test-user@test.com", Some("Test User"))
        .expect("Failed to create test identity token")
}

#[allow(dead_code)]
pub fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

// ============================================================================
// Test Repository Implementations
// ============================================================================

/// Configurable test tenant repository
pub struct TestTenantRepository {
    tenants: RwLock<Vec<Tenant>>,
}

impl TestTenantRepository {
    pub fn new() -> Self {
        Self {
            tenants: RwLock::new(vec![]),
        }
    }

    pub async fn add_tenant(&self, tenant: Tenant) {
        self.tenants.write().await.push(tenant);
    }

    #[allow(dead_code)]
    pub async fn set_tenants(&self, tenants: Vec<Tenant>) {
        *self.tenants.write().await = tenants;
    }

    #[allow(dead_code)]
    pub async fn clear(&self) {
        self.tenants.write().await.clear();
    }
}

impl Default for TestTenantRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantRepository for TestTenantRepository {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant> {
        let tenant = Tenant {
            id: StringUuid::new_v4(),
            name: input.name.clone(),
            slug: input.slug.clone(),
            logo_url: input.logo_url.clone(),
            settings: input.settings.clone().unwrap_or_default(),
            status: TenantStatus::Active,
            password_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.tenants.write().await.push(tenant.clone());
        Ok(tenant)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.iter().find(|t| t.id == id).cloned())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.iter().find(|t| t.slug == slug).cloned())
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(tenants
            .iter()
            .skip(start)
            .take(end - start)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<i64> {
        Ok(self.tenants.read().await.len() as i64)
    }

    async fn search(&self, query: &str, offset: i64, limit: i64) -> Result<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        let query_lower = query.to_lowercase();
        let filtered: Vec<Tenant> = tenants
            .iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.slug.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(filtered.into_iter().skip(start).take(end - start).collect())
    }

    async fn count_search(&self, query: &str) -> Result<i64> {
        let tenants = self.tenants.read().await;
        let query_lower = query.to_lowercase();
        let count = tenants
            .iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.slug.to_lowercase().contains(&query_lower)
            })
            .count();
        Ok(count as i64)
    }

    async fn update(&self, id: StringUuid, input: &UpdateTenantInput) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;
        let tenant = tenants
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;

        if let Some(name) = &input.name {
            tenant.name = name.clone();
        }
        if let Some(logo_url) = &input.logo_url {
            tenant.logo_url = Some(logo_url.clone());
        }
        if let Some(settings) = &input.settings {
            tenant.settings = settings.clone();
        }
        if let Some(status) = &input.status {
            tenant.status = status.clone();
        }
        tenant.updated_at = Utc::now();
        Ok(tenant.clone())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        let pos = tenants
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;
        tenants.remove(pos);
        Ok(())
    }

    async fn update_password_policy(
        &self,
        id: StringUuid,
        policy: &auth9_core::domain::PasswordPolicy,
    ) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;
        let tenant = tenants
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;
        tenant.password_policy = Some(policy.clone());
        tenant.updated_at = Utc::now();
        Ok(tenant.clone())
    }
}

/// Configurable test user repository
pub struct TestUserRepository {
    users: RwLock<Vec<User>>,
    tenant_users: RwLock<Vec<TenantUser>>,
}

impl TestUserRepository {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(vec![]),
            tenant_users: RwLock::new(vec![]),
        }
    }

    pub async fn add_user(&self, user: User) {
        self.users.write().await.push(user);
    }

    #[allow(dead_code)]
    pub async fn set_users(&self, users: Vec<User>) {
        *self.users.write().await = users;
    }

    pub async fn add_tenant_user(&self, tenant_user: TenantUser) {
        self.tenant_users.write().await.push(tenant_user);
    }
}

impl Default for TestUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for TestUserRepository {
    async fn create(&self, keycloak_id: &str, input: &CreateUserInput) -> Result<User> {
        let user = User {
            id: StringUuid::new_v4(),
            email: input.email.clone(),
            display_name: input.display_name.clone(),
            avatar_url: input.avatar_url.clone(),
            keycloak_id: keycloak_id.to_string(),
            mfa_enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.users.write().await.push(user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.email == email).cloned())
    }

    async fn find_by_keycloak_id(&self, keycloak_id: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.iter().find(|u| u.keycloak_id == keycloak_id).cloned())
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<User>> {
        let users = self.users.read().await;
        let start = offset as usize;
        Ok(users
            .iter()
            .skip(start)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<i64> {
        Ok(self.users.read().await.len() as i64)
    }

    async fn search(&self, query: &str, offset: i64, limit: i64) -> Result<Vec<User>> {
        let users = self.users.read().await;
        let query_lower = query.to_lowercase();
        let filtered: Vec<User> = users
            .iter()
            .filter(|u| {
                u.email.to_lowercase().contains(&query_lower)
                    || u.display_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        let start = offset as usize;
        Ok(filtered.into_iter().skip(start).take(limit as usize).collect())
    }

    async fn search_count(&self, query: &str) -> Result<i64> {
        let users = self.users.read().await;
        let query_lower = query.to_lowercase();
        let count = users
            .iter()
            .filter(|u| {
                u.email.to_lowercase().contains(&query_lower)
                    || u.display_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .count();
        Ok(count as i64)
    }

    async fn update(&self, id: StringUuid, input: &UpdateUserInput) -> Result<User> {
        let mut users = self.users.write().await;
        let user = users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

        if let Some(display_name) = &input.display_name {
            user.display_name = Some(display_name.clone());
        }
        if let Some(avatar_url) = &input.avatar_url {
            user.avatar_url = Some(avatar_url.clone());
        }
        user.updated_at = Utc::now();
        Ok(user.clone())
    }

    async fn update_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User> {
        let mut users = self.users.write().await;
        let user = users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
        user.mfa_enabled = enabled;
        user.updated_at = Utc::now();
        Ok(user.clone())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let mut users = self.users.write().await;
        let pos = users
            .iter()
            .position(|u| u.id == id)
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
        users.remove(pos);
        Ok(())
    }

    async fn add_to_tenant(&self, input: &AddUserToTenantInput) -> Result<TenantUser> {
        let tenant_user = TenantUser {
            id: StringUuid::new_v4(),
            user_id: StringUuid::from(input.user_id),
            tenant_id: StringUuid::from(input.tenant_id),
            role_in_tenant: input.role_in_tenant.clone(),
            joined_at: Utc::now(),
        };
        self.tenant_users.write().await.push(tenant_user.clone());
        Ok(tenant_user)
    }

    async fn update_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role: &str,
    ) -> Result<TenantUser> {
        let mut tenant_users = self.tenant_users.write().await;
        let tu = tenant_users
            .iter_mut()
            .find(|tu| tu.user_id == user_id && tu.tenant_id == tenant_id)
            .ok_or_else(|| AppError::NotFound("User-tenant relationship not found".to_string()))?;
        tu.role_in_tenant = role.to_string();
        Ok(tu.clone())
    }

    async fn remove_from_tenant(&self, user_id: StringUuid, tenant_id: StringUuid) -> Result<()> {
        let mut tenant_users = self.tenant_users.write().await;
        let pos = tenant_users
            .iter()
            .position(|tu| tu.user_id == user_id && tu.tenant_id == tenant_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("User {} not in tenant {}", user_id, tenant_id))
            })?;
        tenant_users.remove(pos);
        Ok(())
    }

    async fn find_tenant_users(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>> {
        let tenant_users = self.tenant_users.read().await;
        let users = self.users.read().await;
        let user_ids: Vec<StringUuid> = tenant_users
            .iter()
            .filter(|tu| tu.tenant_id == tenant_id)
            .map(|tu| tu.user_id)
            .collect();
        Ok(users
            .iter()
            .filter(|u| user_ids.contains(&u.id))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn find_user_tenants(&self, user_id: StringUuid) -> Result<Vec<TenantUser>> {
        let tenant_users = self.tenant_users.read().await;
        Ok(tenant_users
            .iter()
            .filter(|tu| tu.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn find_user_tenants_with_tenant(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<auth9_core::domain::TenantUserWithTenant>> {
        // Create mock TenantUserWithTenant from TenantUser data
        let tenant_users = self.tenant_users.read().await;
        Ok(tenant_users
            .iter()
            .filter(|tu| tu.user_id == user_id)
            .map(|tu| auth9_core::domain::TenantUserWithTenant {
                id: tu.id,
                tenant_id: tu.tenant_id,
                user_id: tu.user_id,
                role_in_tenant: tu.role_in_tenant.clone(),
                joined_at: tu.joined_at,
                tenant: auth9_core::domain::TenantInfo {
                    id: tu.tenant_id,
                    name: format!("Tenant {}", tu.tenant_id),
                    slug: format!("tenant-{}", tu.tenant_id),
                    logo_url: None,
                    status: "active".to_string(),
                },
            })
            .collect())
    }

    async fn delete_all_tenant_memberships(&self, user_id: StringUuid) -> Result<u64> {
        let mut tenant_users = self.tenant_users.write().await;
        let before = tenant_users.len();
        tenant_users.retain(|tu| tu.user_id != user_id);
        Ok((before - tenant_users.len()) as u64)
    }

    async fn list_tenant_user_ids(&self, user_id: StringUuid) -> Result<Vec<StringUuid>> {
        let tenant_users = self.tenant_users.read().await;
        Ok(tenant_users
            .iter()
            .filter(|tu| tu.user_id == user_id)
            .map(|tu| tu.id)
            .collect())
    }

    async fn list_tenant_user_ids_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Vec<StringUuid>> {
        let tenant_users = self.tenant_users.read().await;
        Ok(tenant_users
            .iter()
            .filter(|tu| tu.tenant_id == tenant_id)
            .map(|tu| tu.id)
            .collect())
    }

    async fn delete_tenant_memberships_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let mut tenant_users = self.tenant_users.write().await;
        let before = tenant_users.len();
        tenant_users.retain(|tu| tu.tenant_id != tenant_id);
        Ok((before - tenant_users.len()) as u64)
    }
}

/// Configurable test service repository
pub struct TestServiceRepository {
    services: RwLock<Vec<Service>>,
    clients: RwLock<Vec<Client>>,
}

impl TestServiceRepository {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(vec![]),
            clients: RwLock::new(vec![]),
        }
    }

    pub async fn add_service(&self, service: Service) {
        self.services.write().await.push(service);
    }

    pub async fn add_client(&self, client: Client) {
        self.clients.write().await.push(client);
    }
}

impl Default for TestServiceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServiceRepository for TestServiceRepository {
    async fn create(&self, input: &CreateServiceInput) -> Result<Service> {
        let service = Service {
            id: StringUuid::new_v4(),
            tenant_id: input.tenant_id.map(StringUuid::from),
            name: input.name.clone(),
            base_url: input.base_url.clone(),
            redirect_uris: input.redirect_uris.clone(),
            logout_uris: input.logout_uris.clone().unwrap_or_default(),
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.services.write().await.push(service.clone());
        Ok(service)
    }

    async fn create_client(
        &self,
        service_id: Uuid,
        client_id: &str,
        secret_hash: &str,
        name: Option<String>,
    ) -> Result<Client> {
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: StringUuid::from(service_id),
            client_id: client_id.to_string(),
            name,
            client_secret_hash: secret_hash.to_string(),
            created_at: Utc::now(),
        };
        self.clients.write().await.push(client.clone());
        Ok(client)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>> {
        let services = self.services.read().await;
        Ok(services.iter().find(|s| s.id.0 == id).cloned())
    }

    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Service>> {
        let clients = self.clients.read().await;
        let client = clients.iter().find(|c| c.client_id == client_id);
        if let Some(c) = client {
            let services = self.services.read().await;
            Ok(services.iter().find(|s| s.id == c.service_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn find_client_by_client_id(&self, client_id: &str) -> Result<Option<Client>> {
        let clients = self.clients.read().await;
        Ok(clients.iter().find(|c| c.client_id == client_id).cloned())
    }

    async fn list(&self, tenant_id: Option<Uuid>, offset: i64, limit: i64) -> Result<Vec<Service>> {
        let services = self.services.read().await;
        let filtered: Vec<Service> = if let Some(tid) = tenant_id {
            services
                .iter()
                .filter(|s| s.tenant_id.map(|t| t.0) == Some(tid))
                .cloned()
                .collect()
        } else {
            services.clone()
        };
        Ok(filtered
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn list_clients(&self, service_id: Uuid) -> Result<Vec<Client>> {
        let clients = self.clients.read().await;
        Ok(clients
            .iter()
            .filter(|c| c.service_id.0 == service_id)
            .cloned()
            .collect())
    }

    async fn count(&self, tenant_id: Option<Uuid>) -> Result<i64> {
        let services = self.services.read().await;
        if let Some(tid) = tenant_id {
            Ok(services
                .iter()
                .filter(|s| s.tenant_id.map(|t| t.0) == Some(tid))
                .count() as i64)
        } else {
            Ok(services.len() as i64)
        }
    }

    async fn update(&self, id: Uuid, input: &UpdateServiceInput) -> Result<Service> {
        let mut services = self.services.write().await;
        let service = services
            .iter_mut()
            .find(|s| s.id.0 == id)
            .ok_or_else(|| AppError::NotFound(format!("Service {} not found", id)))?;

        if let Some(name) = &input.name {
            service.name = name.clone();
        }
        if let Some(base_url) = &input.base_url {
            service.base_url = Some(base_url.clone());
        }
        if let Some(redirect_uris) = &input.redirect_uris {
            service.redirect_uris = redirect_uris.clone();
        }
        if let Some(logout_uris) = &input.logout_uris {
            service.logout_uris = logout_uris.clone();
        }
        if let Some(status) = &input.status {
            service.status = status.clone();
        }
        service.updated_at = Utc::now();
        Ok(service.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut services = self.services.write().await;
        let pos = services
            .iter()
            .position(|s| s.id.0 == id)
            .ok_or_else(|| AppError::NotFound(format!("Service {} not found", id)))?;
        services.remove(pos);
        Ok(())
    }

    async fn delete_client(&self, _service_id: Uuid, client_id: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        let pos = clients
            .iter()
            .position(|c| c.client_id == client_id)
            .ok_or_else(|| AppError::NotFound(format!("Client {} not found", client_id)))?;
        clients.remove(pos);
        Ok(())
    }

    async fn update_client_secret_hash(
        &self,
        client_id: &str,
        new_secret_hash: &str,
    ) -> Result<()> {
        let mut clients = self.clients.write().await;
        let client = clients
            .iter_mut()
            .find(|c| c.client_id == client_id)
            .ok_or_else(|| AppError::NotFound(format!("Client {} not found", client_id)))?;
        client.client_secret_hash = new_secret_hash.to_string();
        Ok(())
    }

    async fn list_by_tenant(&self, tenant_id: Uuid) -> Result<Vec<Service>> {
        let services = self.services.read().await;
        Ok(services
            .iter()
            .filter(|s| s.tenant_id.map(|t| t.0) == Some(tenant_id))
            .cloned()
            .collect())
    }

    async fn delete_clients_by_service(&self, service_id: Uuid) -> Result<u64> {
        let mut clients = self.clients.write().await;
        let before = clients.len();
        clients.retain(|c| c.service_id.0 != service_id);
        Ok((before - clients.len()) as u64)
    }
}

/// Configurable test RBAC repository
pub struct TestRbacRepository {
    permissions: RwLock<Vec<Permission>>,
    roles: RwLock<Vec<Role>>,
    role_permissions: RwLock<Vec<(StringUuid, StringUuid)>>, // (role_id, permission_id)
    user_roles: RwLock<Vec<(Uuid, Uuid, UserRolesInTenant)>>,
    user_roles_for_service: RwLock<Vec<(Uuid, Uuid, Uuid, UserRolesInTenant)>>,
    tenant_user_roles: RwLock<Vec<(StringUuid, StringUuid)>>, // (tenant_user_id, role_id)
}

impl TestRbacRepository {
    pub fn new() -> Self {
        Self {
            permissions: RwLock::new(vec![]),
            roles: RwLock::new(vec![]),
            role_permissions: RwLock::new(vec![]),
            user_roles: RwLock::new(vec![]),
            user_roles_for_service: RwLock::new(vec![]),
            tenant_user_roles: RwLock::new(vec![]),
        }
    }

    pub async fn add_role(&self, role: Role) {
        self.roles.write().await.push(role);
    }

    pub async fn add_permission(&self, permission: Permission) {
        self.permissions.write().await.push(permission);
    }

    pub async fn set_user_roles(&self, user_id: Uuid, tenant_id: Uuid, roles: UserRolesInTenant) {
        self.user_roles
            .write()
            .await
            .push((user_id, tenant_id, roles));
    }

    pub async fn set_user_roles_for_service(
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
}

impl Default for TestRbacRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RbacRepository for TestRbacRepository {
    async fn create_permission(&self, input: &CreatePermissionInput) -> Result<Permission> {
        let permission = Permission {
            id: StringUuid::new_v4(),
            service_id: StringUuid::from(input.service_id),
            code: input.code.clone(),
            name: input.name.clone(),
            description: input.description.clone(),
        };
        self.permissions.write().await.push(permission.clone());
        Ok(permission)
    }

    async fn find_permission_by_id(&self, id: StringUuid) -> Result<Option<Permission>> {
        let permissions = self.permissions.read().await;
        Ok(permissions.iter().find(|p| p.id == id).cloned())
    }

    async fn find_permissions_by_service(&self, service_id: StringUuid) -> Result<Vec<Permission>> {
        let permissions = self.permissions.read().await;
        Ok(permissions
            .iter()
            .filter(|p| p.service_id == service_id)
            .cloned()
            .collect())
    }

    async fn delete_permission(&self, id: StringUuid) -> Result<()> {
        let mut permissions = self.permissions.write().await;
        let pos = permissions
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Permission {} not found", id)))?;
        permissions.remove(pos);
        Ok(())
    }

    async fn create_role(&self, input: &CreateRoleInput) -> Result<Role> {
        let role = Role {
            id: StringUuid::new_v4(),
            service_id: StringUuid::from(input.service_id),
            name: input.name.clone(),
            description: input.description.clone(),
            parent_role_id: input.parent_role_id.map(StringUuid::from),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.roles.write().await.push(role.clone());
        Ok(role)
    }

    async fn find_role_by_id(&self, id: StringUuid) -> Result<Option<Role>> {
        let roles = self.roles.read().await;
        Ok(roles.iter().find(|r| r.id == id).cloned())
    }

    async fn find_roles_by_service(&self, service_id: StringUuid) -> Result<Vec<Role>> {
        let roles = self.roles.read().await;
        Ok(roles
            .iter()
            .filter(|r| r.service_id == service_id)
            .cloned()
            .collect())
    }

    async fn update_role(&self, id: StringUuid, input: &UpdateRoleInput) -> Result<Role> {
        let mut roles = self.roles.write().await;
        let role = roles
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;

        if let Some(name) = &input.name {
            role.name = name.clone();
        }
        if let Some(description) = &input.description {
            role.description = Some(description.clone());
        }
        role.updated_at = Utc::now();
        Ok(role.clone())
    }

    async fn delete_role(&self, id: StringUuid) -> Result<()> {
        let mut roles = self.roles.write().await;
        let pos = roles
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;
        roles.remove(pos);
        Ok(())
    }

    async fn assign_permission_to_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()> {
        self.role_permissions
            .write()
            .await
            .push((role_id, permission_id));
        Ok(())
    }

    async fn remove_permission_from_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()> {
        let mut role_permissions = self.role_permissions.write().await;
        let pos = role_permissions
            .iter()
            .position(|(rid, pid)| *rid == role_id && *pid == permission_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Permission {} not assigned to role {}",
                    permission_id, role_id
                ))
            })?;
        role_permissions.remove(pos);
        Ok(())
    }

    async fn find_role_permissions(&self, role_id: StringUuid) -> Result<Vec<Permission>> {
        let role_permissions = self.role_permissions.read().await;
        let permissions = self.permissions.read().await;
        let permission_ids: Vec<StringUuid> = role_permissions
            .iter()
            .filter(|(rid, _)| *rid == role_id)
            .map(|(_, pid)| *pid)
            .collect();
        Ok(permissions
            .iter()
            .filter(|p| permission_ids.contains(&p.id))
            .cloned()
            .collect())
    }

    async fn assign_roles_to_user(
        &self,
        input: &AssignRolesInput,
        _granted_by: Option<StringUuid>,
    ) -> Result<()> {
        let mut tenant_user_roles = self.tenant_user_roles.write().await;
        // Use user_id as a stand-in for tenant_user_id in tests
        let tenant_user_id = StringUuid::from(input.user_id);
        for role_id in &input.role_ids {
            tenant_user_roles.push((tenant_user_id, StringUuid::from(*role_id)));
        }
        Ok(())
    }

    async fn remove_role_from_user(
        &self,
        tenant_user_id: StringUuid,
        role_id: StringUuid,
    ) -> Result<()> {
        let mut tenant_user_roles = self.tenant_user_roles.write().await;
        let pos = tenant_user_roles
            .iter()
            .position(|(tuid, rid)| *tuid == tenant_user_id && *rid == role_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Role {} not assigned to tenant user {}",
                    role_id, tenant_user_id
                ))
            })?;
        tenant_user_roles.remove(pos);
        Ok(())
    }

    async fn find_tenant_user_id(
        &self,
        _user_id: StringUuid,
        _tenant_id: StringUuid,
    ) -> Result<Option<StringUuid>> {
        // Simplified: return a new UUID for testing
        Ok(Some(StringUuid::new_v4()))
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
        Ok(self.roles.read().await.clone())
    }

    async fn delete_permissions_by_service(&self, service_id: StringUuid) -> Result<u64> {
        let mut permissions = self.permissions.write().await;
        let before = permissions.len();
        permissions.retain(|p| p.service_id != service_id);
        Ok((before - permissions.len()) as u64)
    }

    async fn delete_roles_by_service(&self, service_id: StringUuid) -> Result<u64> {
        let mut roles = self.roles.write().await;
        let before = roles.len();
        // First, collect role IDs to delete
        let role_ids_to_delete: Vec<StringUuid> = roles
            .iter()
            .filter(|r| r.service_id == service_id)
            .map(|r| r.id)
            .collect();

        // Remove role_permissions for these roles
        {
            let mut role_permissions = self.role_permissions.write().await;
            role_permissions.retain(|(rid, _)| !role_ids_to_delete.contains(rid));
        }

        // Remove tenant_user_roles for these roles
        {
            let mut tenant_user_roles = self.tenant_user_roles.write().await;
            tenant_user_roles.retain(|(_, rid)| !role_ids_to_delete.contains(rid));
        }

        // Remove the roles
        roles.retain(|r| r.service_id != service_id);
        Ok((before - roles.len()) as u64)
    }

    async fn clear_parent_role_references(&self, service_id: StringUuid) -> Result<u64> {
        let mut roles = self.roles.write().await;
        let mut count = 0u64;
        for role in roles.iter_mut() {
            if role.service_id == service_id && role.parent_role_id.is_some() {
                role.parent_role_id = None;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_user_roles_by_tenant_user(&self, tenant_user_id: StringUuid) -> Result<u64> {
        let mut tenant_user_roles = self.tenant_user_roles.write().await;
        let before = tenant_user_roles.len();
        tenant_user_roles.retain(|(tuid, _)| *tuid != tenant_user_id);
        Ok((before - tenant_user_roles.len()) as u64)
    }

    async fn clear_parent_role_reference_by_id(&self, role_id: StringUuid) -> Result<u64> {
        let mut roles = self.roles.write().await;
        let mut count = 0u64;
        for role in roles.iter_mut() {
            if role.parent_role_id == Some(role_id) {
                role.parent_role_id = None;
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Configurable test audit repository
pub struct TestAuditRepository {
    logs: RwLock<Vec<AuditLog>>,
    next_id: RwLock<i64>,
}

impl TestAuditRepository {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(vec![]),
            next_id: RwLock::new(1),
        }
    }

    #[allow(dead_code)]
    pub async fn get_logs(&self) -> Vec<AuditLog> {
        self.logs.read().await.clone()
    }
}

impl Default for TestAuditRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditRepository for TestAuditRepository {
    async fn create(&self, input: &CreateAuditLogInput) -> Result<()> {
        let mut next_id = self.next_id.write().await;
        let log = AuditLog {
            id: *next_id,
            actor_id: input.actor_id.map(|id| id.to_string()),
            action: input.action.clone(),
            resource_type: input.resource_type.clone(),
            resource_id: input.resource_id.map(|id| id.to_string()),
            old_value: input.old_value.clone(),
            new_value: input.new_value.clone(),
            ip_address: input.ip_address.clone(),
            created_at: Utc::now(),
        };
        *next_id += 1;
        self.logs.write().await.push(log);
        Ok(())
    }

    async fn find(&self, query: &AuditLogQuery) -> Result<Vec<AuditLog>> {
        let logs = self.logs.read().await;
        let filtered: Vec<AuditLog> = logs
            .iter()
            .filter(|log| {
                if let Some(actor_id) = query.actor_id {
                    if log.actor_id.as_ref().map(|id| id.as_str()) != Some(&actor_id.to_string()) {
                        return false;
                    }
                }
                if let Some(ref resource_type) = query.resource_type {
                    if &log.resource_type != resource_type {
                        return false;
                    }
                }
                if let Some(ref action) = query.action {
                    if &log.action != action {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(50) as usize;
        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self, query: &AuditLogQuery) -> Result<i64> {
        // Count should return total matching records WITHOUT pagination (like the real impl)
        let logs = self.logs.read().await;
        let count = logs
            .iter()
            .filter(|log| {
                if let Some(actor_id) = query.actor_id {
                    if log.actor_id.as_ref().map(|id| id.as_str()) != Some(&actor_id.to_string()) {
                        return false;
                    }
                }
                if let Some(ref resource_type) = query.resource_type {
                    if &log.resource_type != resource_type {
                        return false;
                    }
                }
                if let Some(ref action) = query.action {
                    if &log.action != action {
                        return false;
                    }
                }
                true
            })
            .count();
        Ok(count as i64)
    }

    async fn find_with_actor(
        &self,
        query: &AuditLogQuery,
    ) -> Result<Vec<auth9_core::repository::audit::AuditLogWithActor>> {
        // Convert AuditLog to AuditLogWithActor (without actor email for tests)
        let logs = self.find(query).await?;
        Ok(logs
            .into_iter()
            .map(|log| auth9_core::repository::audit::AuditLogWithActor {
                id: log.id,
                actor_id: log.actor_id,
                actor_email: None,
                actor_display_name: None,
                action: log.action,
                resource_type: log.resource_type,
                resource_id: log.resource_id,
                old_value: log.old_value,
                new_value: log.new_value,
                ip_address: log.ip_address,
                created_at: log.created_at,
            })
            .collect())
    }

    async fn nullify_actor_id(&self, user_id: StringUuid) -> Result<u64> {
        let mut logs = self.logs.write().await;
        let user_id_str = user_id.to_string();
        let mut count = 0u64;
        for log in logs.iter_mut() {
            if log.actor_id.as_ref() == Some(&user_id_str) {
                log.actor_id = None;
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Configurable test system settings repository
pub struct TestSystemSettingsRepository {
    settings: RwLock<Vec<SystemSettingRow>>,
    next_id: RwLock<i32>,
}

impl TestSystemSettingsRepository {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(vec![]),
            next_id: RwLock::new(1),
        }
    }

    pub async fn add_setting(&self, setting: SystemSettingRow) {
        self.settings.write().await.push(setting);
    }

    #[allow(dead_code)]
    pub async fn clear(&self) {
        self.settings.write().await.clear();
    }
}

impl Default for TestSystemSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemSettingsRepository for TestSystemSettingsRepository {
    async fn get(&self, category: &str, key: &str) -> Result<Option<SystemSettingRow>> {
        let settings = self.settings.read().await;
        Ok(settings
            .iter()
            .find(|s| s.category == category && s.setting_key == key)
            .cloned())
    }

    async fn list_by_category(&self, category: &str) -> Result<Vec<SystemSettingRow>> {
        let settings = self.settings.read().await;
        Ok(settings
            .iter()
            .filter(|s| s.category == category)
            .cloned()
            .collect())
    }

    async fn upsert(&self, input: &UpsertSystemSettingInput) -> Result<SystemSettingRow> {
        let mut settings = self.settings.write().await;

        // Check if exists
        if let Some(existing) = settings
            .iter_mut()
            .find(|s| s.category == input.category && s.setting_key == input.setting_key)
        {
            existing.value = input.value.clone();
            existing.encrypted = input.encrypted;
            existing.description = input.description.clone();
            existing.updated_at = Utc::now();
            return Ok(existing.clone());
        }

        // Create new
        let mut next_id = self.next_id.write().await;
        let setting = SystemSettingRow {
            id: *next_id,
            category: input.category.clone(),
            setting_key: input.setting_key.clone(),
            value: input.value.clone(),
            encrypted: input.encrypted,
            description: input.description.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        *next_id += 1;
        settings.push(setting.clone());
        Ok(setting)
    }

    async fn delete(&self, category: &str, key: &str) -> Result<()> {
        let mut settings = self.settings.write().await;
        settings.retain(|s| !(s.category == category && s.setting_key == key));
        Ok(())
    }
}

// ============================================================================
// Test Password Reset Repository
// ============================================================================

/// Configurable test password reset repository
pub struct TestPasswordResetRepository {
    tokens: RwLock<Vec<PasswordResetToken>>,
}

impl TestPasswordResetRepository {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(vec![]),
        }
    }

    #[allow(dead_code)]
    pub async fn add_token(&self, token: PasswordResetToken) {
        self.tokens.write().await.push(token);
    }
}

impl Default for TestPasswordResetRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PasswordResetRepository for TestPasswordResetRepository {
    async fn create(&self, input: &CreatePasswordResetTokenInput) -> Result<PasswordResetToken> {
        let token = PasswordResetToken {
            id: StringUuid::new_v4(),
            user_id: input.user_id,
            token_hash: input.token_hash.clone(),
            expires_at: input.expires_at,
            used_at: None,
            created_at: Utc::now(),
        };
        self.tokens.write().await.push(token.clone());
        Ok(token)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>> {
        let tokens = self.tokens.read().await;
        let now = Utc::now();
        Ok(tokens
            .iter()
            .find(|t| t.token_hash == token_hash && t.used_at.is_none() && t.expires_at > now)
            .cloned())
    }

    async fn find_valid_by_user(&self, user_id: StringUuid) -> Result<Option<PasswordResetToken>> {
        let tokens = self.tokens.read().await;
        let now = Utc::now();
        Ok(tokens
            .iter()
            .find(|t| t.user_id == user_id && t.used_at.is_none() && t.expires_at > now)
            .cloned())
    }

    async fn mark_used(&self, id: StringUuid) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        let token = tokens
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Password reset token not found".to_string()))?;
        token.used_at = Some(Utc::now());
        Ok(())
    }

    async fn delete_expired(&self) -> Result<u64> {
        let mut tokens = self.tokens.write().await;
        let now = Utc::now();
        let before = tokens.len();
        tokens.retain(|t| t.expires_at > now && t.used_at.is_none());
        Ok((before - tokens.len()) as u64)
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<()> {
        let mut tokens = self.tokens.write().await;
        tokens.retain(|t| t.user_id != user_id);
        Ok(())
    }

    async fn replace_for_user(
        &self,
        input: &CreatePasswordResetTokenInput,
    ) -> Result<PasswordResetToken> {
        // Atomically delete old tokens and create new one
        let mut tokens = self.tokens.write().await;
        tokens.retain(|t| t.user_id != input.user_id);
        let token = PasswordResetToken {
            id: StringUuid::new_v4(),
            user_id: input.user_id,
            token_hash: input.token_hash.clone(),
            expires_at: input.expires_at,
            used_at: None,
            created_at: Utc::now(),
        };
        tokens.push(token.clone());
        Ok(token)
    }
}

// ============================================================================
// Test Session Repository
// ============================================================================

/// Configurable test session repository
pub struct TestSessionRepository {
    sessions: RwLock<Vec<Session>>,
}

impl TestSessionRepository {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(vec![]),
        }
    }

    pub async fn add_session(&self, session: Session) {
        self.sessions.write().await.push(session);
    }
}

impl Default for TestSessionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionRepository for TestSessionRepository {
    async fn create(&self, input: &CreateSessionInput) -> Result<Session> {
        let now = Utc::now();
        let session = Session {
            id: StringUuid::new_v4(),
            user_id: input.user_id,
            keycloak_session_id: input.keycloak_session_id.clone(),
            device_type: input.device_type.clone(),
            device_name: input.device_name.clone(),
            ip_address: input.ip_address.clone(),
            location: input.location.clone(),
            user_agent: input.user_agent.clone(),
            last_active_at: now,
            created_at: now,
            revoked_at: None,
        };
        self.sessions.write().await.push(session.clone());
        Ok(session)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.iter().find(|s| s.id == id).cloned())
    }

    async fn find_by_keycloak_session(&self, keycloak_session_id: &str) -> Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .iter()
            .find(|s| s.keycloak_session_id.as_deref() == Some(keycloak_session_id))
            .cloned())
    }

    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .iter()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn list_active_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .iter()
            .filter(|s| s.user_id == user_id && s.revoked_at.is_none())
            .cloned()
            .collect())
    }

    async fn update_last_active(&self, id: StringUuid) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.iter_mut().find(|s| s.id == id) {
            session.last_active_at = Utc::now();
        }
        Ok(())
    }

    async fn revoke(&self, id: StringUuid) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .iter_mut()
            .find(|s| s.id == id && s.revoked_at.is_none())
            .ok_or_else(|| {
                AppError::NotFound("Session not found or already revoked".to_string())
            })?;
        session.revoked_at = Some(Utc::now());
        Ok(())
    }

    async fn revoke_all_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let mut count = 0u64;
        for session in sessions.iter_mut() {
            if session.user_id == user_id && session.revoked_at.is_none() {
                session.revoked_at = Some(now);
                count += 1;
            }
        }
        Ok(count)
    }

    async fn revoke_all_except(&self, user_id: StringUuid, except_id: StringUuid) -> Result<u64> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let mut count = 0u64;
        for session in sessions.iter_mut() {
            if session.user_id == user_id && session.id != except_id && session.revoked_at.is_none()
            {
                session.revoked_at = Some(now);
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let mut sessions = self.sessions.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = sessions.len();
        sessions.retain(|s| s.created_at > cutoff);
        Ok((before - sessions.len()) as u64)
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|s| s.user_id != user_id);
        Ok((before - sessions.len()) as u64)
    }

    async fn count_active_by_user(&self, user_id: StringUuid) -> Result<i64> {
        let sessions = self.sessions.read().await;
        let count = sessions
            .iter()
            .filter(|s| s.user_id == user_id && s.revoked_at.is_none())
            .count();
        Ok(count as i64)
    }

    async fn find_oldest_active_by_user(&self, user_id: StringUuid) -> Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        let oldest = sessions
            .iter()
            .filter(|s| s.user_id == user_id && s.revoked_at.is_none())
            .min_by_key(|s| s.created_at)
            .cloned();
        Ok(oldest)
    }
}

// ============================================================================
// Test Linked Identity Repository
// ============================================================================

/// Configurable test linked identity repository
pub struct TestLinkedIdentityRepository {
    identities: RwLock<Vec<LinkedIdentity>>,
}

impl TestLinkedIdentityRepository {
    pub fn new() -> Self {
        Self {
            identities: RwLock::new(vec![]),
        }
    }

    pub async fn add_identity(&self, identity: LinkedIdentity) {
        self.identities.write().await.push(identity);
    }
}

impl Default for TestLinkedIdentityRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LinkedIdentityRepository for TestLinkedIdentityRepository {
    async fn create(&self, input: &CreateLinkedIdentityInput) -> Result<LinkedIdentity> {
        let identity = LinkedIdentity {
            id: StringUuid::new_v4(),
            user_id: input.user_id,
            provider_type: input.provider_type.clone(),
            provider_alias: input.provider_alias.clone(),
            external_user_id: input.external_user_id.clone(),
            external_email: input.external_email.clone(),
            linked_at: Utc::now(),
        };
        self.identities.write().await.push(identity.clone());
        Ok(identity)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<LinkedIdentity>> {
        let identities = self.identities.read().await;
        Ok(identities.iter().find(|i| i.id == id).cloned())
    }

    async fn find_by_provider(
        &self,
        provider_alias: &str,
        external_user_id: &str,
    ) -> Result<Option<LinkedIdentity>> {
        let identities = self.identities.read().await;
        Ok(identities
            .iter()
            .find(|i| i.provider_alias == provider_alias && i.external_user_id == external_user_id)
            .cloned())
    }

    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<LinkedIdentity>> {
        let identities = self.identities.read().await;
        Ok(identities
            .iter()
            .filter(|i| i.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let mut identities = self.identities.write().await;
        let pos = identities
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| AppError::NotFound("Linked identity not found".to_string()))?;
        identities.remove(pos);
        Ok(())
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let mut identities = self.identities.write().await;
        let before = identities.len();
        identities.retain(|i| i.user_id != user_id);
        Ok((before - identities.len()) as u64)
    }
}

// ============================================================================
// Test Webhook Repository
// ============================================================================

/// Configurable test webhook repository
pub struct TestWebhookRepository {
    webhooks: RwLock<Vec<Webhook>>,
}

impl TestWebhookRepository {
    pub fn new() -> Self {
        Self {
            webhooks: RwLock::new(vec![]),
        }
    }

    pub async fn add_webhook(&self, webhook: Webhook) {
        self.webhooks.write().await.push(webhook);
    }
}

impl Default for TestWebhookRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookRepository for TestWebhookRepository {
    async fn create(&self, tenant_id: StringUuid, input: &CreateWebhookInput) -> Result<Webhook> {
        let now = Utc::now();
        let webhook = Webhook {
            id: StringUuid::new_v4(),
            tenant_id,
            name: input.name.clone(),
            url: input.url.clone(),
            secret: input.secret.clone(),
            events: input.events.clone(),
            enabled: input.enabled,
            last_triggered_at: None,
            failure_count: 0,
            created_at: now,
            updated_at: now,
        };
        self.webhooks.write().await.push(webhook.clone());
        Ok(webhook)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Webhook>> {
        let webhooks = self.webhooks.read().await;
        Ok(webhooks.iter().find(|w| w.id == id).cloned())
    }

    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Webhook>> {
        let webhooks = self.webhooks.read().await;
        Ok(webhooks
            .iter()
            .filter(|w| w.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn list_enabled_for_event(&self, event: &str) -> Result<Vec<Webhook>> {
        let webhooks = self.webhooks.read().await;
        Ok(webhooks
            .iter()
            .filter(|w| w.enabled && w.events.contains(&event.to_string()))
            .cloned()
            .collect())
    }

    async fn update(&self, id: StringUuid, input: &UpdateWebhookInput) -> Result<Webhook> {
        let mut webhooks = self.webhooks.write().await;
        let webhook = webhooks
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| AppError::NotFound(format!("Webhook {} not found", id)))?;

        if let Some(name) = &input.name {
            webhook.name = name.clone();
        }
        if let Some(url) = &input.url {
            webhook.url = url.clone();
        }
        if let Some(secret) = &input.secret {
            webhook.secret = Some(secret.clone());
        }
        if let Some(events) = &input.events {
            webhook.events = events.clone();
        }
        if let Some(enabled) = input.enabled {
            webhook.enabled = enabled;
        }
        webhook.updated_at = Utc::now();
        Ok(webhook.clone())
    }

    async fn update_triggered(&self, id: StringUuid, success: bool) -> Result<()> {
        let mut webhooks = self.webhooks.write().await;
        if let Some(webhook) = webhooks.iter_mut().find(|w| w.id == id) {
            webhook.last_triggered_at = Some(Utc::now());
            if success {
                webhook.failure_count = 0;
            } else {
                webhook.failure_count += 1;
            }
        }
        Ok(())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let mut webhooks = self.webhooks.write().await;
        let pos = webhooks
            .iter()
            .position(|w| w.id == id)
            .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))?;
        webhooks.remove(pos);
        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let mut webhooks = self.webhooks.write().await;
        let before = webhooks.len();
        webhooks.retain(|w| w.tenant_id != tenant_id);
        Ok((before - webhooks.len()) as u64)
    }
}

// ============================================================================
// Test Invitation Repository
// ============================================================================

/// Configurable test invitation repository
pub struct TestInvitationRepository {
    invitations: RwLock<HashMap<StringUuid, Invitation>>,
}

impl TestInvitationRepository {
    pub fn new() -> Self {
        Self {
            invitations: RwLock::new(HashMap::new()),
        }
    }

    #[allow(dead_code)]
    pub async fn add_invitation(&self, invitation: Invitation) {
        self.invitations
            .write()
            .await
            .insert(invitation.id, invitation);
    }
}

impl Default for TestInvitationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InvitationRepository for TestInvitationRepository {
    async fn create(
        &self,
        tenant_id: StringUuid,
        invited_by: StringUuid,
        input: &CreateInvitationInput,
        token_hash: &str,
    ) -> Result<Invitation> {
        let now = Utc::now();
        let expires_in = input.expires_in_hours.unwrap_or(72);
        let invitation = Invitation {
            id: StringUuid::new_v4(),
            tenant_id,
            email: input.email.clone(),
            role_ids: input.role_ids.clone(),
            invited_by,
            token_hash: token_hash.to_string(),
            status: InvitationStatus::Pending,
            expires_at: now + chrono::Duration::hours(expires_in),
            accepted_at: None,
            created_at: now,
            updated_at: now,
        };
        self.invitations
            .write()
            .await
            .insert(invitation.id, invitation.clone());
        Ok(invitation)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Invitation>> {
        let invitations = self.invitations.read().await;
        Ok(invitations.get(&id).cloned())
    }

    async fn find_by_email_and_tenant(
        &self,
        email: &str,
        tenant_id: StringUuid,
    ) -> Result<Option<Invitation>> {
        let invitations = self.invitations.read().await;
        Ok(invitations
            .values()
            .find(|i| {
                i.email == email
                    && i.tenant_id == tenant_id
                    && i.status == InvitationStatus::Pending
            })
            .cloned())
    }

    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Invitation>> {
        let invitations = self.invitations.read().await;
        let mut filtered: Vec<_> = invitations
            .values()
            .filter(|i| {
                i.tenant_id == tenant_id && status.as_ref().map_or(true, |s| &i.status == s)
            })
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(filtered
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn count_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
    ) -> Result<i64> {
        let invitations = self.invitations.read().await;
        Ok(invitations
            .values()
            .filter(|i| {
                i.tenant_id == tenant_id && status.as_ref().map_or(true, |s| &i.status == s)
            })
            .count() as i64)
    }

    async fn update_status(&self, id: StringUuid, status: InvitationStatus) -> Result<Invitation> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;
        invitation.status = status;
        invitation.updated_at = Utc::now();
        Ok(invitation.clone())
    }

    async fn list_pending(&self) -> Result<Vec<Invitation>> {
        let invitations = self.invitations.read().await;
        Ok(invitations
            .values()
            .filter(|i| i.status == InvitationStatus::Pending)
            .cloned()
            .collect())
    }

    async fn mark_accepted(&self, id: StringUuid) -> Result<Invitation> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;
        invitation.status = InvitationStatus::Accepted;
        invitation.accepted_at = Some(Utc::now());
        invitation.updated_at = Utc::now();
        Ok(invitation.clone())
    }

    async fn update_token_hash(&self, id: StringUuid, token_hash: &str) -> Result<Invitation> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;
        invitation.token_hash = token_hash.to_string();
        invitation.updated_at = Utc::now();
        Ok(invitation.clone())
    }

    async fn touch_updated_at(&self, id: StringUuid) -> Result<Invitation> {
        let mut invitations = self.invitations.write().await;
        let invitation = invitations
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;
        invitation.updated_at = Utc::now();
        Ok(invitation.clone())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let mut invitations = self.invitations.write().await;
        invitations
            .remove(&id)
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;
        Ok(())
    }

    async fn expire_pending(&self) -> Result<u64> {
        let mut invitations = self.invitations.write().await;
        let now = Utc::now();
        let mut count = 0u64;
        for invitation in invitations.values_mut() {
            if invitation.status == InvitationStatus::Pending && invitation.expires_at <= now {
                invitation.status = InvitationStatus::Expired;
                invitation.updated_at = now;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let mut invitations = self.invitations.write().await;
        let before = invitations.len();
        invitations.retain(|_, i| i.tenant_id != tenant_id);
        Ok((before - invitations.len()) as u64)
    }
}

// ============================================================================
// Test Login Event Repository
// ============================================================================

/// Configurable test login event repository
pub struct TestLoginEventRepository {
    events: RwLock<Vec<LoginEvent>>,
    next_id: RwLock<i64>,
}

impl TestLoginEventRepository {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(vec![]),
            next_id: RwLock::new(1),
        }
    }

    pub async fn add_event(&self, event: LoginEvent) {
        self.events.write().await.push(event);
    }
}

impl Default for TestLoginEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoginEventRepository for TestLoginEventRepository {
    async fn create(&self, input: &CreateLoginEventInput) -> Result<i64> {
        let mut next_id = self.next_id.write().await;
        let event = LoginEvent {
            id: *next_id,
            user_id: input.user_id,
            email: input.email.clone(),
            tenant_id: input.tenant_id,
            event_type: input.event_type.clone(),
            ip_address: input.ip_address.clone(),
            user_agent: input.user_agent.clone(),
            device_type: input.device_type.clone(),
            location: input.location.clone(),
            session_id: input.session_id,
            failure_reason: input.failure_reason.clone(),
            created_at: Utc::now(),
        };
        self.events.write().await.push(event);
        let id = *next_id;
        *next_id += 1;
        Ok(id)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<LoginEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_by_user(
        &self,
        user_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.user_id == Some(user_id))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.tenant_id == Some(tenant_id))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<i64> {
        Ok(self.events.read().await.len() as i64)
    }

    async fn count_by_user(&self, user_id: StringUuid) -> Result<i64> {
        let events = self.events.read().await;
        Ok(events.iter().filter(|e| e.user_id == Some(user_id)).count() as i64)
    }

    async fn count_by_tenant(&self, tenant_id: StringUuid) -> Result<i64> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| e.tenant_id == Some(tenant_id))
            .count() as i64)
    }

    async fn get_stats(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<LoginStats> {
        let events = self.events.read().await;
        let filtered: Vec<_> = events
            .iter()
            .filter(|e| e.created_at >= start && e.created_at <= end)
            .collect();

        let total_logins = filtered.len() as i64;
        let successful_logins = filtered
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    LoginEventType::Success | LoginEventType::Social
                )
            })
            .count() as i64;
        let failed_logins = filtered
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    LoginEventType::FailedPassword
                        | LoginEventType::FailedMfa
                        | LoginEventType::Locked
                )
            })
            .count() as i64;

        let unique_users: std::collections::HashSet<_> =
            filtered.iter().filter_map(|e| e.user_id).collect();

        let mut by_event_type: HashMap<String, i64> = HashMap::new();
        let mut by_device_type: HashMap<String, i64> = HashMap::new();

        for event in &filtered {
            *by_event_type
                .entry(format!("{:?}", event.event_type).to_lowercase())
                .or_insert(0) += 1;
            let device = event
                .device_type
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            *by_device_type.entry(device).or_insert(0) += 1;
        }

        Ok(LoginStats {
            total_logins,
            successful_logins,
            failed_logins,
            unique_users: unique_users.len() as i64,
            by_event_type,
            by_device_type,
            period_start: start,
            period_end: end,
        })
    }

    async fn count_failed_by_ip(&self, ip_address: &str, since: DateTime<Utc>) -> Result<i64> {
        let events = self.events.read().await;
        Ok(events
            .iter()
            .filter(|e| {
                e.ip_address.as_deref() == Some(ip_address)
                    && e.created_at >= since
                    && matches!(
                        e.event_type,
                        LoginEventType::FailedPassword | LoginEventType::FailedMfa
                    )
            })
            .count() as i64)
    }

    async fn count_failed_by_ip_multi_user(
        &self,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let events = self.events.read().await;
        let unique_users: std::collections::HashSet<_> = events
            .iter()
            .filter(|e| {
                e.ip_address.as_deref() == Some(ip_address)
                    && e.created_at >= since
                    && matches!(
                        e.event_type,
                        LoginEventType::FailedPassword | LoginEventType::FailedMfa
                    )
            })
            .filter_map(|e| {
                e.user_id
                    .or_else(|| e.email.as_ref().map(|_| StringUuid::new_v4()))
            })
            .collect();
        Ok(unique_users.len() as i64)
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let mut events = self.events.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = events.len();
        events.retain(|e| e.created_at > cutoff);
        Ok((before - events.len()) as u64)
    }

    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64> {
        let mut events = self.events.write().await;
        let mut count = 0u64;
        for event in events.iter_mut() {
            if event.user_id == Some(user_id) {
                event.user_id = None;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let mut events = self.events.write().await;
        let before = events.len();
        events.retain(|e| e.tenant_id != Some(tenant_id));
        Ok((before - events.len()) as u64)
    }
}

// ============================================================================
// Test Security Alert Repository
// ============================================================================

/// Configurable test security alert repository
pub struct TestSecurityAlertRepository {
    alerts: RwLock<Vec<SecurityAlert>>,
}

impl TestSecurityAlertRepository {
    pub fn new() -> Self {
        Self {
            alerts: RwLock::new(vec![]),
        }
    }

    pub async fn add_alert(&self, alert: SecurityAlert) {
        self.alerts.write().await.push(alert);
    }
}

impl Default for TestSecurityAlertRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityAlertRepository for TestSecurityAlertRepository {
    async fn create(&self, input: &CreateSecurityAlertInput) -> Result<SecurityAlert> {
        let alert = SecurityAlert {
            id: StringUuid::new_v4(),
            user_id: input.user_id,
            tenant_id: input.tenant_id,
            alert_type: input.alert_type.clone(),
            severity: input.severity.clone(),
            details: input.details.clone(),
            resolved_at: None,
            resolved_by: None,
            created_at: Utc::now(),
        };
        self.alerts.write().await.push(alert.clone());
        Ok(alert)
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<SecurityAlert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts.iter().find(|a| a.id == id).cloned())
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_unresolved(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts
            .iter()
            .filter(|a| a.resolved_at.is_none())
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_by_user(
        &self,
        user_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SecurityAlert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts
            .iter()
            .filter(|a| a.user_id == Some(user_id))
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_by_severity(
        &self,
        severity: AlertSeverity,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<SecurityAlert>> {
        let alerts = self.alerts.read().await;
        Ok(alerts
            .iter()
            .filter(|a| a.severity == severity)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<i64> {
        Ok(self.alerts.read().await.len() as i64)
    }

    async fn count_unresolved(&self) -> Result<i64> {
        let alerts = self.alerts.read().await;
        Ok(alerts.iter().filter(|a| a.resolved_at.is_none()).count() as i64)
    }

    async fn resolve(&self, id: StringUuid, resolved_by: StringUuid) -> Result<SecurityAlert> {
        let mut alerts = self.alerts.write().await;
        let alert = alerts
            .iter_mut()
            .find(|a| a.id == id && a.resolved_at.is_none())
            .ok_or_else(|| {
                AppError::NotFound("Security alert not found or already resolved".to_string())
            })?;
        alert.resolved_at = Some(Utc::now());
        alert.resolved_by = Some(resolved_by);
        Ok(alert.clone())
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let mut alerts = self.alerts.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let before = alerts.len();
        alerts.retain(|a| a.resolved_at.is_none() || a.created_at > cutoff);
        Ok((before - alerts.len()) as u64)
    }

    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64> {
        let mut alerts = self.alerts.write().await;
        let mut count = 0u64;
        for alert in alerts.iter_mut() {
            if alert.user_id == Some(user_id) {
                alert.user_id = None;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let mut alerts = self.alerts.write().await;
        let before = alerts.len();
        alerts.retain(|a| a.tenant_id != Some(tenant_id));
        Ok((before - alerts.len()) as u64)
    }
}

// ============================================================================
// Test WebAuthn Repository
// ============================================================================

pub struct TestWebAuthnRepository {
    credentials: RwLock<Vec<StoredPasskey>>,
}

impl TestWebAuthnRepository {
    pub fn new() -> Self {
        Self {
            credentials: RwLock::new(vec![]),
        }
    }
}

impl Default for TestWebAuthnRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebAuthnRepository for TestWebAuthnRepository {
    async fn create(&self, input: &CreatePasskeyInput) -> Result<StoredPasskey> {
        let stored = StoredPasskey {
            id: input.id.clone(),
            user_id: input.user_id.clone(),
            credential_id: input.credential_id.clone(),
            credential_data: input.credential_data.clone(),
            user_label: input.user_label.clone(),
            aaguid: input.aaguid.clone(),
            created_at: Utc::now(),
            last_used_at: None,
        };
        self.credentials.write().await.push(stored.clone());
        Ok(stored)
    }

    async fn find_by_credential_id(&self, credential_id: &str) -> Result<Option<StoredPasskey>> {
        let creds = self.credentials.read().await;
        Ok(creds
            .iter()
            .find(|c| c.credential_id == credential_id)
            .cloned())
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<StoredPasskey>> {
        let creds = self.credentials.read().await;
        Ok(creds
            .iter()
            .filter(|c| c.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let mut creds = self.credentials.write().await;
        let before = creds.len();
        creds.retain(|c| !(c.id == id && c.user_id == user_id));
        if creds.len() == before {
            return Err(AppError::NotFound(
                "WebAuthn credential not found".to_string(),
            ));
        }
        Ok(())
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<u64> {
        let mut creds = self.credentials.write().await;
        let before = creds.len();
        creds.retain(|c| c.user_id != user_id);
        Ok((before - creds.len()) as u64)
    }

    async fn update_last_used(&self, id: &str) -> Result<()> {
        let mut creds = self.credentials.write().await;
        if let Some(cred) = creds.iter_mut().find(|c| c.id == id) {
            cred.last_used_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn update_credential_data(&self, id: &str, data: &serde_json::Value) -> Result<()> {
        let mut creds = self.credentials.write().await;
        if let Some(cred) = creds.iter_mut().find(|c| c.id == id) {
            cred.credential_data = data.clone();
        }
        Ok(())
    }
}

// ============================================================================
// Test Data Helpers
// ============================================================================

pub fn create_test_tenant(id: Option<Uuid>) -> Tenant {
    Tenant {
        id: StringUuid::from(id.unwrap_or_else(Uuid::new_v4)),
        name: "Test Tenant".to_string(),
        slug: "test-tenant".to_string(),
        logo_url: None,
        settings: TenantSettings::default(),
        status: TenantStatus::Active,
        password_policy: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_user(id: Option<Uuid>) -> User {
    User {
        id: StringUuid::from(id.unwrap_or_else(Uuid::new_v4)),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        avatar_url: None,
        keycloak_id: "kc-user-test".to_string(),
        mfa_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_service(id: Option<Uuid>, tenant_id: Option<Uuid>) -> Service {
    Service {
        id: StringUuid::from(id.unwrap_or_else(Uuid::new_v4)),
        tenant_id: tenant_id.map(StringUuid::from),
        name: "Test Service".to_string(),
        base_url: Some("https://test.example.com".to_string()),
        redirect_uris: vec!["https://test.example.com/callback".to_string()],
        logout_uris: vec![],
        status: ServiceStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_role(id: Option<Uuid>, service_id: Uuid) -> Role {
    Role {
        id: StringUuid::from(id.unwrap_or_else(Uuid::new_v4)),
        service_id: StringUuid::from(service_id),
        name: "test-role".to_string(),
        description: Some("Test role description".to_string()),
        parent_role_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn create_test_permission(id: Option<Uuid>, service_id: Uuid) -> Permission {
    Permission {
        id: StringUuid::from(id.unwrap_or_else(Uuid::new_v4)),
        service_id: StringUuid::from(service_id),
        code: "test:permission".to_string(),
        name: "Test Permission".to_string(),
        description: Some("Test permission".to_string()),
    }
}

// ============================================================================
// Test Services Builder
// ============================================================================

/// Builder for creating test services with mocked repositories
#[allow(dead_code)]
#[allow(dead_code)]
pub struct TestServicesBuilder {
    pub tenant_repo: Arc<TestTenantRepository>,
    pub user_repo: Arc<TestUserRepository>,
    pub service_repo: Arc<TestServiceRepository>,
    pub rbac_repo: Arc<TestRbacRepository>,
    pub audit_repo: Arc<TestAuditRepository>,
    pub webhook_repo: Arc<TestWebhookRepository>,
    #[allow(dead_code)]
    pub invitation_repo: Arc<TestInvitationRepository>,
    pub session_repo: Arc<TestSessionRepository>,
    #[allow(dead_code)]
    pub password_reset_repo: Arc<TestPasswordResetRepository>,
    pub linked_identity_repo: Arc<TestLinkedIdentityRepository>,
    pub login_event_repo: Arc<TestLoginEventRepository>,
    pub security_alert_repo: Arc<TestSecurityAlertRepository>,
}

#[allow(dead_code)]
impl TestServicesBuilder {
    pub fn new() -> Self {
        Self {
            tenant_repo: Arc::new(TestTenantRepository::new()),
            user_repo: Arc::new(TestUserRepository::new()),
            service_repo: Arc::new(TestServiceRepository::new()),
            rbac_repo: Arc::new(TestRbacRepository::new()),
            audit_repo: Arc::new(TestAuditRepository::new()),
            webhook_repo: Arc::new(TestWebhookRepository::new()),
            invitation_repo: Arc::new(TestInvitationRepository::new()),
            session_repo: Arc::new(TestSessionRepository::new()),
            password_reset_repo: Arc::new(TestPasswordResetRepository::new()),
            linked_identity_repo: Arc::new(TestLinkedIdentityRepository::new()),
            login_event_repo: Arc::new(TestLoginEventRepository::new()),
            security_alert_repo: Arc::new(TestSecurityAlertRepository::new()),
        }
    }

    pub fn build_tenant_service(
        &self,
    ) -> TenantService<
        TestTenantRepository,
        TestServiceRepository,
        TestWebhookRepository,
        TestInvitationRepository,
        TestUserRepository,
        TestRbacRepository,
        TestLoginEventRepository,
        TestSecurityAlertRepository,
    > {
        TenantService::new(
            self.tenant_repo.clone(),
            self.service_repo.clone(),
            self.webhook_repo.clone(),
            self.invitation_repo.clone(),
            self.user_repo.clone(),
            self.rbac_repo.clone(),
            self.login_event_repo.clone(),
            self.security_alert_repo.clone(),
            None,
        )
    }

    pub fn build_user_service(
        &self,
    ) -> UserService<
        TestUserRepository,
        TestSessionRepository,
        TestPasswordResetRepository,
        TestLinkedIdentityRepository,
        TestLoginEventRepository,
        TestSecurityAlertRepository,
        TestAuditRepository,
        TestRbacRepository,
    > {
        UserService::new(
            self.user_repo.clone(),
            self.session_repo.clone(),
            self.password_reset_repo.clone(),
            self.linked_identity_repo.clone(),
            self.login_event_repo.clone(),
            self.security_alert_repo.clone(),
            self.audit_repo.clone(),
            self.rbac_repo.clone(),
            None,
            None, // webhook_publisher
        )
    }

    pub fn build_client_service(&self) -> ClientService<TestServiceRepository, TestRbacRepository> {
        ClientService::new(self.service_repo.clone(), self.rbac_repo.clone(), None)
    }

    pub fn build_rbac_service(&self) -> RbacService<TestRbacRepository> {
        RbacService::new(self.rbac_repo.clone(), None)
    }
}

impl Default for TestServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_repository_crud() {
        let repo = TestTenantRepository::new();

        // Create
        let input = CreateTenantInput {
            name: "Test".to_string(),
            slug: "test".to_string(),
            logo_url: None,
            settings: None,
        };
        let tenant = repo.create(&input).await.unwrap();
        assert_eq!(tenant.name, "Test");

        // Read
        let found = repo.find_by_id(tenant.id).await.unwrap();
        assert!(found.is_some());

        // List
        let list = repo.list(0, 10).await.unwrap();
        assert_eq!(list.len(), 1);

        // Update
        let update_input = UpdateTenantInput {
            name: Some("Updated".to_string()),
            logo_url: None,
            settings: None,
            status: None,
        };
        let updated = repo.update(tenant.id, &update_input).await.unwrap();
        assert_eq!(updated.name, "Updated");

        // Delete
        repo.delete(tenant.id).await.unwrap();
        let deleted = repo.find_by_id(tenant.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_user_repository_crud() {
        let repo = TestUserRepository::new();

        // Create
        let input = CreateUserInput {
            email: "test@example.com".to_string(),
            display_name: Some("Test".to_string()),
            avatar_url: None,
        };
        let user = repo.create("kc-123", &input).await.unwrap();
        assert_eq!(user.email, "test@example.com");

        // Read
        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_some());

        // Find by email
        let by_email = repo.find_by_email("test@example.com").await.unwrap();
        assert!(by_email.is_some());
    }

    #[tokio::test]
    async fn test_service_repository_crud() {
        let repo = TestServiceRepository::new();

        // Create
        let input = CreateServiceInput {
            tenant_id: Some(Uuid::new_v4()),
            name: "Test Service".to_string(),
            client_id: "test-client".to_string(),
            base_url: Some("https://test.com".to_string()),
            redirect_uris: vec!["https://test.com/cb".to_string()],
            logout_uris: None,
        };
        let service = repo.create(&input).await.unwrap();
        assert_eq!(service.name, "Test Service");

        // Create client
        let client = repo
            .create_client(*service.id, "client-1", "hash", Some("Client".to_string()))
            .await
            .unwrap();
        assert_eq!(client.client_id, "client-1");
    }

    #[tokio::test]
    async fn test_rbac_repository_operations() {
        let repo = TestRbacRepository::new();
        let service_id = Uuid::new_v4();

        // Create role
        let role_input = CreateRoleInput {
            service_id,
            name: "admin".to_string(),
            description: Some("Admin role".to_string()),
            parent_role_id: None,
            permission_ids: None,
        };
        let role = repo.create_role(&role_input).await.unwrap();
        assert_eq!(role.name, "admin");

        // Create permission
        let perm_input = CreatePermissionInput {
            service_id,
            code: "user:read".to_string(),
            name: "Read Users".to_string(),
            description: Some("Read users".to_string()),
        };
        let perm = repo.create_permission(&perm_input).await.unwrap();
        assert_eq!(perm.code, "user:read");

        // Assign permission to role
        repo.assign_permission_to_role(role.id, perm.id)
            .await
            .unwrap();

        // Find role permissions
        let perms = repo.find_role_permissions(role.id).await.unwrap();
        assert_eq!(perms.len(), 1);
    }

    #[tokio::test]
    async fn test_services_builder() {
        let builder = TestServicesBuilder::new();

        // Add test data
        let tenant = create_test_tenant(None);
        builder.tenant_repo.add_tenant(tenant.clone()).await;

        // Build service and verify
        let tenant_service = builder.build_tenant_service();
        let result = tenant_service.get(tenant.id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Tenant");
    }
}
