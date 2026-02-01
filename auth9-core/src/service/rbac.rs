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

    /// Delete a role with cascade handling.
    ///
    /// Cascade order:
    /// 1. Clear parent_role_id references (other roles pointing to this one)
    /// 2. Delete role (repository handles role_permissions and user_tenant_roles)
    /// 3. Invalidate cache
    pub async fn delete_role(&self, id: StringUuid) -> Result<()> {
        let _ = self.get_role(id).await?;

        // 1. Clear parent_role_id references from other roles
        self.repo.clear_parent_role_reference_by_id(id).await?;

        // 2. Delete role (repository handles role_permissions and user_tenant_roles)
        self.repo.delete_role(id).await?;

        // 3. Invalidate cache
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
    use crate::domain::{Permission, Role};
    use crate::repository::rbac::MockRbacRepository;
    use mockall::predicate::*;
    use uuid::Uuid;

    // ==================== Permission Tests ====================

    #[tokio::test]
    async fn test_create_permission_success() {
        let mut mock = MockRbacRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_create_permission().returning(|input| {
            Ok(Permission {
                service_id: StringUuid::from(input.service_id),
                code: input.code.clone(),
                name: input.name.clone(),
                description: input.description.clone(),
                ..Default::default()
            })
        });

        let service = RbacService::new(Arc::new(mock), None);

        let input = CreatePermissionInput {
            service_id,
            code: "user:read".to_string(),
            name: "Read Users".to_string(),
            description: Some("Permission to read users".to_string()),
        };

        let result = service.create_permission(input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().code, "user:read");
    }

    #[tokio::test]
    async fn test_create_permission_invalid_code() {
        let mock = MockRbacRepository::new();
        let service = RbacService::new(Arc::new(mock), None);

        let input = CreatePermissionInput {
            service_id: Uuid::new_v4(),
            code: "invalid".to_string(), // Invalid format
            name: "Read Users".to_string(),
            description: None,
        };

        let result = service.create_permission(input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_get_permission_success() {
        let mut mock = MockRbacRepository::new();
        let permission = Permission::default();
        let permission_clone = permission.clone();
        let id = permission.id;

        mock.expect_find_permission_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(permission_clone.clone())));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_permission(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_permission_not_found() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_permission_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_permission(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_permissions() {
        let mut mock = MockRbacRepository::new();
        let service_id = StringUuid::new_v4();

        mock.expect_find_permissions_by_service()
            .with(eq(service_id))
            .returning(|_| {
                Ok(vec![
                    Permission {
                        code: "user:read".to_string(),
                        ..Default::default()
                    },
                    Permission {
                        code: "user:write".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.list_permissions(service_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_delete_permission_success() {
        let mut mock = MockRbacRepository::new();
        let permission = Permission::default();
        let permission_clone = permission.clone();
        let id = permission.id;

        mock.expect_find_permission_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(permission_clone.clone())));

        mock.expect_delete_permission()
            .with(eq(id))
            .returning(|_| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.delete_permission(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_permission_not_found() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_permission_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.delete_permission(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ==================== Role Tests ====================

    #[tokio::test]
    async fn test_create_role_success() {
        let mut mock = MockRbacRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_create_role().returning(|input| {
            Ok(Role {
                service_id: StringUuid::from(input.service_id),
                name: input.name.clone(),
                description: input.description.clone(),
                ..Default::default()
            })
        });

        let service = RbacService::new(Arc::new(mock), None);

        let input = CreateRoleInput {
            service_id,
            name: "Admin".to_string(),
            description: Some("Administrator role".to_string()),
            parent_role_id: None,
            permission_ids: None,
        };

        let result = service.create_role(input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Admin");
    }

    #[tokio::test]
    async fn test_create_role_invalid_name() {
        let mock = MockRbacRepository::new();
        let service = RbacService::new(Arc::new(mock), None);

        let input = CreateRoleInput {
            service_id: Uuid::new_v4(),
            name: "".to_string(), // Empty name is invalid
            description: None,
            parent_role_id: None,
            permission_ids: None,
        };

        let result = service.create_role(input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_get_role_success() {
        let mut mock = MockRbacRepository::new();
        let role = Role {
            name: "Admin".to_string(),
            ..Default::default()
        };
        let role_clone = role.clone();
        let id = role.id;

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_role(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Admin");
    }

    #[tokio::test]
    async fn test_get_role_not_found() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_role(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_role_with_permissions() {
        let mut mock = MockRbacRepository::new();
        let role = Role {
            name: "Admin".to_string(),
            ..Default::default()
        };
        let role_clone = role.clone();
        let id = role.id;

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        mock.expect_find_role_permissions()
            .with(eq(id))
            .returning(|_| {
                Ok(vec![Permission {
                    code: "user:read".to_string(),
                    ..Default::default()
                }])
            });

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_role_with_permissions(id).await;
        assert!(result.is_ok());
        let rwp = result.unwrap();
        assert_eq!(rwp.role.name, "Admin");
        assert_eq!(rwp.permissions.len(), 1);
    }

    #[tokio::test]
    async fn test_list_roles() {
        let mut mock = MockRbacRepository::new();
        let service_id = StringUuid::new_v4();

        mock.expect_find_roles_by_service()
            .with(eq(service_id))
            .returning(|_| {
                Ok(vec![
                    Role {
                        name: "Admin".to_string(),
                        ..Default::default()
                    },
                    Role {
                        name: "User".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.list_roles(service_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_update_role_success() {
        let mut mock = MockRbacRepository::new();
        let role = Role {
            name: "Admin".to_string(),
            ..Default::default()
        };
        let role_clone = role.clone();
        let id = role.id;

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        mock.expect_update_role().returning(|_, input| {
            Ok(Role {
                name: input.name.clone().unwrap_or_default(),
                ..Default::default()
            })
        });

        let service = RbacService::new(Arc::new(mock), None);

        let input = UpdateRoleInput {
            name: Some("Super Admin".to_string()),
            description: None,
            parent_role_id: None,
        };

        let result = service.update_role(id, input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Super Admin");
    }

    #[tokio::test]
    async fn test_update_role_not_found() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let input = UpdateRoleInput {
            name: Some("Updated".to_string()),
            description: None,
            parent_role_id: None,
        };

        let result = service.update_role(id, input).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_role_success() {
        let mut mock = MockRbacRepository::new();
        let role = Role::default();
        let role_clone = role.clone();
        let id = role.id;

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        // Clear parent_role_id references
        mock.expect_clear_parent_role_reference_by_id()
            .with(eq(id))
            .returning(|_| Ok(0));

        mock.expect_delete_role().with(eq(id)).returning(|_| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.delete_role(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_role_not_found() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.delete_role(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ==================== Role-Permission Tests ====================

    #[tokio::test]
    async fn test_assign_permission_to_role_success() {
        let mut mock = MockRbacRepository::new();
        let role = Role::default();
        let permission = Permission::default();
        let role_clone = role.clone();
        let permission_clone = permission.clone();
        let role_id = role.id;
        let permission_id = permission.id;

        mock.expect_find_role_by_id()
            .with(eq(role_id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        mock.expect_find_permission_by_id()
            .with(eq(permission_id))
            .returning(move |_| Ok(Some(permission_clone.clone())));

        mock.expect_assign_permission_to_role()
            .with(eq(role_id), eq(permission_id))
            .returning(|_, _| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service
            .assign_permission_to_role(role_id, permission_id)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assign_permission_to_role_role_not_found() {
        let mut mock = MockRbacRepository::new();
        let role_id = StringUuid::new_v4();
        let permission_id = StringUuid::new_v4();

        mock.expect_find_role_by_id()
            .with(eq(role_id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service
            .assign_permission_to_role(role_id, permission_id)
            .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_assign_permission_to_role_permission_not_found() {
        let mut mock = MockRbacRepository::new();
        let role = Role::default();
        let role_clone = role.clone();
        let role_id = role.id;
        let permission_id = StringUuid::new_v4();

        mock.expect_find_role_by_id()
            .with(eq(role_id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        mock.expect_find_permission_by_id()
            .with(eq(permission_id))
            .returning(|_| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service
            .assign_permission_to_role(role_id, permission_id)
            .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_remove_permission_from_role() {
        let mut mock = MockRbacRepository::new();
        let role_id = StringUuid::new_v4();
        let permission_id = StringUuid::new_v4();

        mock.expect_remove_permission_from_role()
            .with(eq(role_id), eq(permission_id))
            .returning(|_, _| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service
            .remove_permission_from_role(role_id, permission_id)
            .await;
        assert!(result.is_ok());
    }

    // ==================== User-Tenant-Role Tests ====================

    #[tokio::test]
    async fn test_assign_roles_success() {
        let mut mock = MockRbacRepository::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        mock.expect_assign_roles_to_user().returning(|_, _| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let input = AssignRolesInput {
            user_id,
            tenant_id,
            role_ids: vec![role_id],
        };

        let result = service.assign_roles(input, None).await;
        assert!(result.is_ok());
    }

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

    #[tokio::test]
    async fn test_get_user_role_records() {
        let mut mock = MockRbacRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        mock.expect_find_user_role_records_in_tenant()
            .with(eq(user_id), eq(tenant_id), eq(None))
            .returning(|_, _, _| {
                Ok(vec![Role {
                    name: "Admin".to_string(),
                    ..Default::default()
                }])
            });

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.get_user_role_records(user_id, tenant_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_unassign_role_success() {
        let mut mock = MockRbacRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();
        let role_id = StringUuid::new_v4();
        let tenant_user_id = StringUuid::new_v4();

        mock.expect_find_tenant_user_id()
            .with(eq(user_id), eq(tenant_id))
            .returning(move |_, _| Ok(Some(tenant_user_id)));

        mock.expect_remove_role_from_user()
            .with(eq(tenant_user_id), eq(role_id))
            .returning(|_, _| Ok(()));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.unassign_role(user_id, tenant_id, role_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unassign_role_user_not_in_tenant() {
        let mut mock = MockRbacRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();
        let role_id = StringUuid::new_v4();

        mock.expect_find_tenant_user_id()
            .with(eq(user_id), eq(tenant_id))
            .returning(|_, _| Ok(None));

        let service = RbacService::new(Arc::new(mock), None);

        let result = service.unassign_role(user_id, tenant_id, role_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
