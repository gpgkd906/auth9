//! RBAC repository

use crate::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, Permission, Role, StringUuid,
    UpdateRoleInput, UserRolesInTenant,
};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::MySqlPool;

mod impl_repo;

#[cfg(test)]
mod tests;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RbacRepository: Send + Sync {
    // Permissions
    async fn create_permission(&self, input: &CreatePermissionInput) -> Result<Permission>;
    async fn find_permission_by_id(&self, id: StringUuid) -> Result<Option<Permission>>;
    async fn find_permissions_by_service(&self, service_id: StringUuid) -> Result<Vec<Permission>>;
    async fn delete_permission(&self, id: StringUuid) -> Result<()>;

    // Roles
    async fn create_role(&self, input: &CreateRoleInput) -> Result<Role>;
    async fn find_role_by_id(&self, id: StringUuid) -> Result<Option<Role>>;
    async fn find_roles_by_service(&self, service_id: StringUuid) -> Result<Vec<Role>>;
    async fn update_role(&self, id: StringUuid, input: &UpdateRoleInput) -> Result<Role>;
    async fn delete_role(&self, id: StringUuid) -> Result<()>;

    // Role-Permission mapping
    async fn assign_permission_to_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()>;
    async fn remove_permission_from_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()>;
    async fn find_role_permissions(&self, role_id: StringUuid) -> Result<Vec<Permission>>;

    // User-Tenant-Role
    async fn assign_roles_to_user(
        &self,
        input: &AssignRolesInput,
        granted_by: Option<StringUuid>,
    ) -> Result<()>;
    async fn remove_role_from_user(
        &self,
        tenant_user_id: StringUuid,
        role_id: StringUuid,
    ) -> Result<()>;
    async fn find_tenant_user_id(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<Option<StringUuid>>;
    /// Fetch the tenant-level role (owner/admin/member) from tenant_users
    async fn find_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<Option<String>>;
    async fn find_user_roles_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<UserRolesInTenant>;
    async fn find_user_roles_in_tenant_for_service(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        service_id: StringUuid,
    ) -> Result<UserRolesInTenant>;
    async fn find_user_role_records_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        service_id: Option<StringUuid>,
    ) -> Result<Vec<Role>>;

    // Cascade delete methods

    /// Delete all permissions for a service (including role_permissions mappings)
    async fn delete_permissions_by_service(&self, service_id: StringUuid) -> Result<u64>;

    /// Delete all roles for a service (including role_permissions and user_tenant_roles mappings)
    async fn delete_roles_by_service(&self, service_id: StringUuid) -> Result<u64>;

    /// Clear parent_role_id references for a service (SET NULL before deleting roles)
    async fn clear_parent_role_references(&self, service_id: StringUuid) -> Result<u64>;

    /// Delete all user role assignments for a tenant_user
    async fn delete_user_roles_by_tenant_user(&self, tenant_user_id: StringUuid) -> Result<u64>;

    /// Clear parent_role_id references for a specific role (SET NULL before deleting the role)
    async fn clear_parent_role_reference_by_id(&self, role_id: StringUuid) -> Result<u64>;

    /// Find user IDs that have a specific role assigned in a tenant (via user_tenant_roles)
    async fn find_user_ids_by_role_in_tenant(
        &self,
        tenant_id: StringUuid,
        role_id: StringUuid,
    ) -> Result<Vec<StringUuid>>;
}

pub struct RbacRepositoryImpl {
    pub(crate) pool: MySqlPool,
}

impl RbacRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
