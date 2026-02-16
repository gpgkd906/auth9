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
        let permission = self.repo.create_permission(&input).await.map_err(|e| {
            // Convert database unique constraint error to user-friendly message
            if let AppError::Database(ref db_err) = e {
                let err_str = db_err.to_string().to_lowercase();
                if err_str.contains("duplicate") || err_str.contains("unique") {
                    return AppError::Conflict(format!(
                        "Permission code '{}' already exists in this service",
                        input.code
                    ));
                }
            }
            e
        })?;
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
        // Note: We don't invalidate user role cache when a permission is deleted.
        // Cached roles remain valid - the deleted permission simply won't be available.
        // Cache will naturally expire after TTL (5 minutes), and future queries will
        // reflect the deleted permission. This avoids expensive KEYS scan on Redis.
        Ok(())
    }

    // ==================== Roles ====================

    pub async fn create_role(&self, input: CreateRoleInput) -> Result<Role> {
        input.validate()?;

        // Check inheritance depth if a parent role is specified
        if let Some(parent_id) = input.parent_role_id {
            self.check_parent_chain_depth(StringUuid::from(parent_id))
                .await?;
        }

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

        // Check for circular inheritance if parent_role_id is being updated
        // input.parent_role_id is Option<Option<Uuid>>:
        // - Some(Some(id)) = explicitly set to a parent role
        // - Some(None) = explicitly cleared
        // - None = not provided, keep existing
        if let Some(Some(parent_id)) = input.parent_role_id {
            let parent_uuid = StringUuid::from(parent_id);
            self.check_circular_inheritance(id, parent_uuid).await?;
        }

        let role = self.repo.update_role(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_all_user_roles().await;
        }
        Ok(role)
    }

    /// Maximum allowed depth for role inheritance chains.
    const MAX_ROLE_INHERITANCE_DEPTH: usize = 10;

    /// Check for circular inheritance by traversing the parent chain.
    /// Returns error if setting `new_parent_id` as parent of `role_id` would create a cycle
    /// or if the resulting chain would exceed the maximum depth limit.
    async fn check_circular_inheritance(
        &self,
        role_id: StringUuid,
        new_parent_id: StringUuid,
    ) -> Result<()> {
        // A role cannot be its own parent
        if role_id == new_parent_id {
            return Err(AppError::BadRequest(
                "A role cannot be its own parent".to_string(),
            ));
        }

        // Traverse the parent chain from new_parent_id
        // If we encounter role_id, it means we have a cycle
        let mut current_id = Some(new_parent_id);
        let mut visited = std::collections::HashSet::new();
        visited.insert(role_id); // The role we're updating
        let mut depth: usize = 1; // Start at 1 since we already have role_id -> new_parent_id

        while let Some(parent_id) = current_id {
            if visited.contains(&parent_id) {
                return Err(AppError::BadRequest(
                    "Circular inheritance detected: this would create a cycle in the role hierarchy".to_string(),
                ));
            }

            depth += 1;
            if depth > Self::MAX_ROLE_INHERITANCE_DEPTH {
                return Err(AppError::BadRequest(format!(
                    "Role inheritance depth exceeds maximum limit of {}",
                    Self::MAX_ROLE_INHERITANCE_DEPTH
                )));
            }

            visited.insert(parent_id);

            // Get the parent role to find its parent
            match self.repo.find_role_by_id(parent_id).await? {
                Some(parent_role) => {
                    current_id = parent_role.parent_role_id;
                }
                None => {
                    // Parent role doesn't exist - this is a validation error
                    return Err(AppError::NotFound(format!(
                        "Parent role {} not found",
                        parent_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check that the parent chain depth from `parent_id` upward does not exceed
    /// the maximum allowed depth. Used during role creation where the new role
    /// does not yet have an ID.
    async fn check_parent_chain_depth(&self, parent_id: StringUuid) -> Result<()> {
        let mut current_id = Some(parent_id);
        let mut depth: usize = 1; // The new role itself counts as depth 1
        let mut visited = std::collections::HashSet::new();

        while let Some(id) = current_id {
            depth += 1;
            if depth > Self::MAX_ROLE_INHERITANCE_DEPTH {
                return Err(AppError::BadRequest(format!(
                    "Role inheritance depth exceeds maximum limit of {}",
                    Self::MAX_ROLE_INHERITANCE_DEPTH
                )));
            }

            if !visited.insert(id) {
                return Err(AppError::BadRequest(
                    "Circular inheritance detected in existing role hierarchy".to_string(),
                ));
            }

            match self.repo.find_role_by_id(id).await? {
                Some(role) => {
                    current_id = role.parent_role_id;
                }
                None => {
                    return Err(AppError::NotFound(format!("Parent role {} not found", id)));
                }
            }
        }

        Ok(())
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
        let role = self.get_role(role_id).await?;
        let permission = self.get_permission(permission_id).await?;

        // Validate that role and permission belong to the same service
        if role.service_id != permission.service_id {
            return Err(AppError::BadRequest(format!(
                "Cannot assign permission from service {} to role in service {}",
                permission.service_id, role.service_id
            )));
        }

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

    // ==================== Circular Inheritance Tests ====================

    #[tokio::test]
    async fn test_update_role_circular_inheritance_self_reference() {
        let mut mock = MockRbacRepository::new();
        let role_id = StringUuid::new_v4();
        let role = Role {
            id: role_id,
            name: "Editor".to_string(),
            parent_role_id: None,
            ..Default::default()
        };
        let role_clone = role.clone();

        mock.expect_find_role_by_id()
            .with(eq(role_id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        let service = RbacService::new(Arc::new(mock), None);

        // Try to set a role as its own parent
        let input = UpdateRoleInput {
            name: None,
            description: None,
            parent_role_id: Some(Some(*role_id)), // Self-reference
        };

        let result = service.update_role(role_id, input).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("own parent"));
        }
    }

    #[tokio::test]
    async fn test_update_role_circular_inheritance_two_level() {
        let mut mock = MockRbacRepository::new();
        let viewer_id = StringUuid::new_v4();
        let editor_id = StringUuid::new_v4();

        // Editor -> Viewer (existing)
        let editor = Role {
            id: editor_id,
            name: "Editor".to_string(),
            parent_role_id: Some(viewer_id),
            ..Default::default()
        };
        let editor_clone = editor.clone();

        // Viewer (root, no parent)
        let viewer = Role {
            id: viewer_id,
            name: "Viewer".to_string(),
            parent_role_id: None,
            ..Default::default()
        };
        let viewer_clone = viewer.clone();

        // First call: get the role being updated (Viewer)
        mock.expect_find_role_by_id()
            .with(eq(viewer_id))
            .times(1)
            .returning(move |_| Ok(Some(viewer_clone.clone())));

        // Second call: get the proposed parent (Editor)
        mock.expect_find_role_by_id()
            .with(eq(editor_id))
            .times(1)
            .returning(move |_| Ok(Some(editor_clone.clone())));

        let service = RbacService::new(Arc::new(mock), None);

        // Try to set Viewer's parent to Editor (would create: Viewer -> Editor -> Viewer)
        let input = UpdateRoleInput {
            name: None,
            description: None,
            parent_role_id: Some(Some(*editor_id)),
        };

        let result = service.update_role(viewer_id, input).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("Circular inheritance"));
        }
    }

    #[tokio::test]
    async fn test_update_role_inheritance_depth_limit_exceeded() {
        let mut mock = MockRbacRepository::new();

        // Create 12 role IDs: target + 11 parents in a chain
        // target -> parent_1 -> parent_2 -> ... -> parent_11
        // depth starts at 1 (target->parent_1), increments each loop iteration
        // At parent_10 (i=10), depth = 11 > MAX_DEPTH(10), triggers error
        let role_ids: Vec<StringUuid> = (0..12).map(|_| StringUuid::new_v4()).collect();
        let target_role_id = role_ids[0];

        // First call: get the role being updated (target) — used by update_role to check existence
        let target_role = Role {
            id: target_role_id,
            name: "Target".to_string(),
            parent_role_id: None,
            ..Default::default()
        };
        let target_clone = target_role.clone();
        mock.expect_find_role_by_id()
            .with(eq(target_role_id))
            .times(1)
            .returning(move |_| Ok(Some(target_clone.clone())));

        // Set up chain: parent_1 -> parent_2 -> ... -> parent_11 (no parent)
        // The check traverses from parent_1 upward. It should hit depth limit
        // before reaching parent_11.
        for i in 1..12 {
            let parent = if i < 11 { Some(role_ids[i + 1]) } else { None };
            let role = Role {
                id: role_ids[i],
                name: format!("Role_{}", i),
                parent_role_id: parent,
                ..Default::default()
            };
            let role_clone = role.clone();
            let rid = role_ids[i];
            mock.expect_find_role_by_id()
                .with(eq(rid))
                .times(..=1) // May be called 0 or 1 times (depth limit may stop early)
                .returning(move |_| Ok(Some(role_clone.clone())));
        }

        let service = RbacService::new(Arc::new(mock), None);

        let input = UpdateRoleInput {
            name: None,
            description: None,
            parent_role_id: Some(Some(*role_ids[1])),
        };

        let result = service.update_role(target_role_id, input).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("depth exceeds maximum"));
        }
    }

    // ==================== Cross-Service Permission Assignment Tests ====================

    #[tokio::test]
    async fn test_assign_permission_cross_service_rejected() {
        let mut mock = MockRbacRepository::new();
        let service_a = StringUuid::new_v4();
        let service_b = StringUuid::new_v4();

        let role = Role {
            service_id: service_a,
            name: "Admin".to_string(),
            ..Default::default()
        };
        let permission = Permission {
            service_id: service_b, // Different service!
            code: "user:read".to_string(),
            ..Default::default()
        };
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

        let service = RbacService::new(Arc::new(mock), None);

        let result = service
            .assign_permission_to_role(role_id, permission_id)
            .await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("Cannot assign permission"));
        }
    }

    #[tokio::test]
    async fn test_assign_permission_same_service_success() {
        let mut mock = MockRbacRepository::new();
        let service_id = StringUuid::new_v4();

        let role = Role {
            service_id,
            name: "Admin".to_string(),
            ..Default::default()
        };
        let permission = Permission {
            service_id, // Same service
            code: "user:read".to_string(),
            ..Default::default()
        };
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

    // ==================== Duplicate Permission Error Message Tests ====================

    #[tokio::test]
    async fn test_create_permission_duplicate_code_friendly_error() {
        let mut mock = MockRbacRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_create_permission().returning(|_| {
            Err(AppError::Database(sqlx::Error::Database(Box::new(
                TestDbError("Duplicate entry 'user:read' for key 'permissions.idx_permissions_service_code'".to_string()),
            ))))
        });

        let service = RbacService::new(Arc::new(mock), None);

        let input = CreatePermissionInput {
            service_id,
            code: "user:read".to_string(),
            name: "Read Users".to_string(),
            description: None,
        };

        let result = service.create_permission(input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
        if let Err(AppError::Conflict(msg)) = result {
            assert!(msg.contains("already exists"));
        }
    }

    // Helper struct for database error testing
    #[derive(Debug)]
    struct TestDbError(String);

    impl std::fmt::Display for TestDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestDbError {}

    impl sqlx::error::DatabaseError for TestDbError {
        fn message(&self) -> &str {
            &self.0
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::UniqueViolation
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
    }
}
