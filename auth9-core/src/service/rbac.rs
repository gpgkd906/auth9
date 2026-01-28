//! RBAC business logic

use crate::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, Permission, Role,
    RoleWithPermissions, UpdateRoleInput, UserRolesInTenant,
};
use crate::error::{AppError, Result};
use crate::repository::RbacRepository;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

pub struct RbacService<R: RbacRepository> {
    repo: Arc<R>,
}

impl<R: RbacRepository> RbacService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    // ==================== Permissions ====================

    pub async fn create_permission(&self, input: CreatePermissionInput) -> Result<Permission> {
        input.validate()?;
        self.repo.create_permission(&input).await
    }

    pub async fn get_permission(&self, id: Uuid) -> Result<Permission> {
        self.repo
            .find_permission_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Permission {} not found", id)))
    }

    pub async fn list_permissions(&self, service_id: Uuid) -> Result<Vec<Permission>> {
        self.repo.find_permissions_by_service(service_id).await
    }

    pub async fn delete_permission(&self, id: Uuid) -> Result<()> {
        let _ = self.get_permission(id).await?;
        self.repo.delete_permission(id).await
    }

    // ==================== Roles ====================

    pub async fn create_role(&self, input: CreateRoleInput) -> Result<Role> {
        input.validate()?;
        self.repo.create_role(&input).await
    }

    pub async fn get_role(&self, id: Uuid) -> Result<Role> {
        self.repo
            .find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))
    }

    pub async fn get_role_with_permissions(&self, id: Uuid) -> Result<RoleWithPermissions> {
        let role = self.get_role(id).await?;
        let permissions = self.repo.find_role_permissions(id).await?;
        Ok(RoleWithPermissions { role, permissions })
    }

    pub async fn list_roles(&self, service_id: Uuid) -> Result<Vec<Role>> {
        self.repo.find_roles_by_service(service_id).await
    }

    pub async fn update_role(&self, id: Uuid, input: UpdateRoleInput) -> Result<Role> {
        input.validate()?;
        let _ = self.get_role(id).await?;
        self.repo.update_role(id, &input).await
    }

    pub async fn delete_role(&self, id: Uuid) -> Result<()> {
        let _ = self.get_role(id).await?;
        self.repo.delete_role(id).await
    }

    // ==================== Role-Permission ====================

    pub async fn assign_permission_to_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()> {
        let _ = self.get_role(role_id).await?;
        let _ = self.get_permission(permission_id).await?;
        self.repo.assign_permission_to_role(role_id, permission_id).await
    }

    pub async fn remove_permission_from_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()> {
        self.repo.remove_permission_from_role(role_id, permission_id).await
    }

    // ==================== User-Tenant-Role ====================

    pub async fn assign_roles(&self, input: AssignRolesInput, granted_by: Option<Uuid>) -> Result<()> {
        input.validate()?;
        self.repo.assign_roles_to_user(&input, granted_by).await
    }

    pub async fn get_user_roles(&self, user_id: Uuid, tenant_id: Uuid) -> Result<UserRolesInTenant> {
        self.repo.find_user_roles_in_tenant(user_id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::rbac::MockRbacRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_get_user_roles() {
        let mut mock = MockRbacRepository::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        
        mock.expect_find_user_roles_in_tenant()
            .with(eq(user_id), eq(tenant_id))
            .returning(|uid, tid| {
                Ok(UserRolesInTenant {
                    user_id: uid,
                    tenant_id: tid,
                    roles: vec!["admin".to_string()],
                    permissions: vec!["user:read".to_string(), "user:write".to_string()],
                })
            });
        
        let service = RbacService::new(Arc::new(mock));
        
        let result = service.get_user_roles(user_id, tenant_id).await;
        assert!(result.is_ok());
        
        let roles = result.unwrap();
        assert_eq!(roles.roles, vec!["admin"]);
        assert_eq!(roles.permissions.len(), 2);
    }
}
