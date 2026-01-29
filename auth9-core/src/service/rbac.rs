//! RBAC business logic

use crate::cache::CacheManager;
use crate::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, Permission, Role,
    RoleWithPermissions, StringUuid, UpdateRoleInput, UserRolesInTenant,
};
use crate::error::{AppError, Result};
use crate::repository::RbacRepository;
use std::sync::Arc;
use validator::Validate;

pub struct RbacService<R: RbacRepository> {
    repo: Arc<R>,
    cache_manager: Option<CacheManager>,
}

impl<R: RbacRepository> RbacService<R> {
    pub fn new(repo: Arc<R>, cache_manager: Option<CacheManager>) -> Self {
        Self {
            repo,
            cache_manager,
        }
    }

    // ==================== Permissions ====================

    pub async fn create_permission(&self, input: CreatePermissionInput) -> Result<Permission> {
        input.validate()?;
        let permission = self.repo.create_permission(&input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(permission)
    }

    pub async fn get_permission(&self, id: StringUuid) -> Result<Permission> {
        self.repo
            .find_permission_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Permission {} not found", id)))
    }

    pub async fn list_permissions(&self, service_id: StringUuid) -> Result<Vec<Permission>> {
        self.repo.find_permissions_by_service(service_id).await
    }

    pub async fn delete_permission(&self, id: StringUuid) -> Result<()> {
        let _ = self.get_permission(id).await?;
        self.repo.delete_permission(id).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(())
    }

    // ==================== Roles ====================

    pub async fn create_role(&self, input: CreateRoleInput) -> Result<Role> {
        input.validate()?;
        let role = self.repo.create_role(&input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(role)
    }

    pub async fn get_role(&self, id: StringUuid) -> Result<Role> {
        self.repo
            .find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))
    }

    pub async fn get_role_with_permissions(&self, id: StringUuid) -> Result<RoleWithPermissions> {
        let role = self.get_role(id).await?;
        let permissions = self.repo.find_role_permissions(id).await?;
        Ok(RoleWithPermissions { role, permissions })
    }

    pub async fn list_roles(&self, service_id: StringUuid) -> Result<Vec<Role>> {
        self.repo.find_roles_by_service(service_id).await
    }

    pub async fn update_role(&self, id: StringUuid, input: UpdateRoleInput) -> Result<Role> {
        input.validate()?;
        let _ = self.get_role(id).await?;
        let role = self.repo.update_role(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(role)
    }

    pub async fn delete_role(&self, id: StringUuid) -> Result<()> {
        let _ = self.get_role(id).await?;
        self.repo.delete_role(id).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(())
    }

    // ==================== Role-Permission ====================

    pub async fn assign_permission_to_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()> {
        let _ = self.get_role(role_id).await?;
        let _ = self.get_permission(permission_id).await?;
        self.repo
            .assign_permission_to_role(role_id, permission_id)
            .await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(())
    }

    pub async fn remove_permission_from_role(
        &self,
        role_id: StringUuid,
        permission_id: StringUuid,
    ) -> Result<()> {
        self.repo
            .remove_permission_from_role(role_id, permission_id)
            .await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(())
    }

    // ==================== User-Tenant-Role ====================

    pub async fn assign_roles(
        &self,
        input: AssignRolesInput,
        granted_by: Option<StringUuid>,
    ) -> Result<()> {
        input.validate()?;
        self.repo.assign_roles_to_user(&input, granted_by).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache
                .invalidate_user_roles_for_tenant(input.user_id, input.tenant_id)
                .await;
        }
        Ok(())
    }

    pub async fn get_user_roles(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<UserRolesInTenant> {
        self.repo
            .find_user_roles_in_tenant(user_id, tenant_id)
            .await
    }

    pub async fn get_user_role_records(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<Vec<Role>> {
        self.repo
            .find_user_role_records_in_tenant(user_id, tenant_id, None)
            .await
    }

    /// Remove role from user in tenant
    pub async fn unassign_role(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role_id: StringUuid,
    ) -> Result<()> {
        // The repository method needs tenant_user_id, so we need to look it up
        let tenant_user_id = self
            .repo
            .find_tenant_user_id(user_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not in tenant".to_string()))?;

        self.repo
            .remove_role_from_user(tenant_user_id, role_id)
            .await?;

        if let Some(cache) = &self.cache_manager {
            let _ = cache
                .invalidate_user_roles_for_tenant(*user_id, *tenant_id)
                .await;
        }
        Ok(())
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
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        mock.expect_find_user_roles_in_tenant()
            .with(eq(user_id), eq(tenant_id))
            .returning(|uid, tid| {
                Ok(UserRolesInTenant {
                    user_id: *uid,
                    tenant_id: *tid,
                    roles: vec!["admin".to_string()],
                    permissions: vec!["user:read".to_string(), "user:write".to_string()],
                })
            });

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_user_roles(user_id, tenant_id).await;
        assert!(result.is_ok());

        let roles = result.unwrap();
        assert_eq!(roles.roles, vec!["admin"]);
        assert_eq!(roles.permissions.len(), 2);
    }
}
