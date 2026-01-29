//! RBAC repository

use crate::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, Permission, Role, StringUuid,
    UpdateRoleInput, UserRolesInTenant,
};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

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
    async fn assign_permission_to_role(&self, role_id: StringUuid, permission_id: StringUuid) -> Result<()>;
    async fn remove_permission_from_role(&self, role_id: StringUuid, permission_id: StringUuid) -> Result<()>;
    async fn find_role_permissions(&self, role_id: StringUuid) -> Result<Vec<Permission>>;

    // User-Tenant-Role
    async fn assign_roles_to_user(
        &self,
        input: &AssignRolesInput,
        granted_by: Option<StringUuid>,
    ) -> Result<()>;
    async fn remove_role_from_user(&self, tenant_user_id: StringUuid, role_id: StringUuid) -> Result<()>;
    async fn find_tenant_user_id(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<Option<StringUuid>>;
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
}

pub struct RbacRepositoryImpl {
    pool: MySqlPool,
}

impl RbacRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn resolve_inherited_roles(&self, base_roles: Vec<Role>) -> Result<Vec<Role>> {
        let mut roles_by_id: HashMap<StringUuid, Role> = base_roles
            .iter()
            .cloned()
            .map(|role| (role.id, role))
            .collect();
        let mut queue: VecDeque<StringUuid> = base_roles
            .iter()
            .filter_map(|role| role.parent_role_id)
            .collect();
        let mut visited: HashSet<StringUuid> = roles_by_id.keys().cloned().collect();

        while let Some(role_id) = queue.pop_front() {
            if visited.contains(&role_id) {
                continue;
            }
            visited.insert(role_id);
            if let Some(role) = self.find_role_by_id(role_id).await? {
                if let Some(parent_id) = role.parent_role_id {
                    if !visited.contains(&parent_id) {
                        queue.push_back(parent_id);
                    }
                }
                roles_by_id.insert(role.id, role);
            }
        }

        Ok(roles_by_id.into_values().collect())
    }

    async fn collect_permissions(&self, roles: &[Role]) -> Result<Vec<String>> {
        let mut permissions = HashSet::new();
        for role in roles {
            for permission in self.find_role_permissions(role.id).await? {
                permissions.insert(permission.code);
            }
        }
        Ok(permissions.into_iter().collect())
    }
}

#[async_trait]
impl RbacRepository for RbacRepositoryImpl {
    async fn create_permission(&self, input: &CreatePermissionInput) -> Result<Permission> {
        let id = StringUuid::new_v4();
        let service_id = StringUuid::from(input.service_id);

        sqlx::query(
            r#"
            INSERT INTO permissions (id, service_id, code, name, description)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(service_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.description)
        .execute(&self.pool)
        .await?;

        self.find_permission_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create permission")))
    }

    async fn find_permission_by_id(&self, id: StringUuid) -> Result<Option<Permission>> {
        let permission = sqlx::query_as::<_, Permission>(
            "SELECT id, service_id, code, name, description FROM permissions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(permission)
    }

    async fn find_permissions_by_service(&self, service_id: StringUuid) -> Result<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            "SELECT id, service_id, code, name, description FROM permissions WHERE service_id = ?",
        )
        .bind(service_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(permissions)
    }

    async fn delete_permission(&self, id: StringUuid) -> Result<()> {
        // Delete role-permission mappings first
        sqlx::query("DELETE FROM role_permissions WHERE permission_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let result = sqlx::query("DELETE FROM permissions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Permission {} not found", id)));
        }

        Ok(())
    }

    async fn create_role(&self, input: &CreateRoleInput) -> Result<Role> {
        let id = StringUuid::new_v4();
        let service_id = StringUuid::from(input.service_id);
        let parent_role_id = input.parent_role_id.map(StringUuid::from);

        sqlx::query(
            r#"
            INSERT INTO roles (id, service_id, name, description, parent_role_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(service_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(parent_role_id)
        .execute(&self.pool)
        .await?;

        // Assign permissions if provided
        if let Some(permission_ids) = &input.permission_ids {
            for perm_id in permission_ids {
                self.assign_permission_to_role(id, StringUuid::from(*perm_id)).await?;
            }
        }

        self.find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create role")))
    }

    async fn find_role_by_id(&self, id: StringUuid) -> Result<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT id, service_id, name, description, parent_role_id, created_at, updated_at FROM roles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role)
    }

    async fn find_roles_by_service(&self, service_id: StringUuid) -> Result<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT id, service_id, name, description, parent_role_id, created_at, updated_at FROM roles WHERE service_id = ?",
        )
        .bind(service_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    async fn update_role(&self, id: StringUuid, input: &UpdateRoleInput) -> Result<Role> {
        let existing = self
            .find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let description = input.description.as_ref().or(existing.description.as_ref());
        let parent_role_id = input.parent_role_id.map(StringUuid::from).or(existing.parent_role_id);

        sqlx::query(
            r#"
            UPDATE roles
            SET name = ?, description = ?, parent_role_id = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(parent_role_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update role")))
    }

    async fn delete_role(&self, id: StringUuid) -> Result<()> {
        // Delete role-permission and user-role mappings first
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM user_tenant_roles WHERE role_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let result = sqlx::query("DELETE FROM roles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Role {} not found", id)));
        }

        Ok(())
    }

    async fn assign_permission_to_role(&self, role_id: StringUuid, permission_id: StringUuid) -> Result<()> {
        sqlx::query("INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn remove_permission_from_role(&self, role_id: StringUuid, permission_id: StringUuid) -> Result<()> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_role_permissions(&self, role_id: StringUuid) -> Result<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            r#"
            SELECT p.id, p.service_id, p.code, p.name, p.description
            FROM permissions p
            INNER JOIN role_permissions rp ON p.id = rp.permission_id
            WHERE rp.role_id = ?
            "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(permissions)
    }

    async fn assign_roles_to_user(
        &self,
        input: &AssignRolesInput,
        granted_by: Option<StringUuid>,
    ) -> Result<()> {
        // First find the tenant_user record
        let user_id = StringUuid::from(input.user_id);
        let tenant_id = StringUuid::from(input.tenant_id);
        let tenant_user_id: Option<(StringUuid,)> =
            sqlx::query_as("SELECT id FROM tenant_users WHERE user_id = ? AND tenant_id = ?")
                .bind(user_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        let tenant_user_id = tenant_user_id
            .ok_or_else(|| AppError::NotFound("User not in tenant".to_string()))?
            .0;

        for role_id in &input.role_ids {
            let id = StringUuid::new_v4();
            let role_id = StringUuid::from(*role_id);
            sqlx::query(
                r#"
                INSERT IGNORE INTO user_tenant_roles (id, tenant_user_id, role_id, granted_at, granted_by)
                VALUES (?, ?, ?, NOW(), ?)
                "#,
            )
            .bind(id)
            .bind(tenant_user_id)
            .bind(role_id)
            .bind(granted_by)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn remove_role_from_user(&self, tenant_user_id: StringUuid, role_id: StringUuid) -> Result<()> {
        sqlx::query("DELETE FROM user_tenant_roles WHERE tenant_user_id = ? AND role_id = ?")
            .bind(tenant_user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_tenant_user_id(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<Option<StringUuid>> {
        let result: Option<(StringUuid,)> =
            sqlx::query_as("SELECT id FROM tenant_users WHERE user_id = ? AND tenant_id = ?")
                .bind(user_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(result.map(|(id,)| id))
    }

    async fn find_user_roles_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<UserRolesInTenant> {
        let base_roles = self
            .find_user_role_records_in_tenant(user_id, tenant_id, None)
            .await?;
        let roles = self.resolve_inherited_roles(base_roles).await?;
        let permissions = self.collect_permissions(&roles).await?;
        Ok(UserRolesInTenant {
            user_id: Uuid::from(user_id),
            tenant_id: Uuid::from(tenant_id),
            roles: roles.into_iter().map(|role| role.name).collect(),
            permissions,
        })
    }

    async fn find_user_roles_in_tenant_for_service(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        service_id: StringUuid,
    ) -> Result<UserRolesInTenant> {
        let base_roles = self
            .find_user_role_records_in_tenant(user_id, tenant_id, Some(service_id))
            .await?;
        let roles = self.resolve_inherited_roles(base_roles).await?;
        let permissions = self.collect_permissions(&roles).await?;
        Ok(UserRolesInTenant {
            user_id: Uuid::from(user_id),
            tenant_id: Uuid::from(tenant_id),
            roles: roles.into_iter().map(|role| role.name).collect(),
            permissions,
        })
    }

    async fn find_user_role_records_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        service_id: Option<StringUuid>,
    ) -> Result<Vec<Role>> {
        let mut sql = String::from(
            "SELECT r.id, r.service_id, r.name, r.description, r.parent_role_id, r.created_at, r.updated_at \
             FROM roles r \
             INNER JOIN user_tenant_roles utr ON r.id = utr.role_id \
             INNER JOIN tenant_users tu ON utr.tenant_user_id = tu.id \
             WHERE tu.user_id = ? AND tu.tenant_id = ?",
        );

        if service_id.is_some() {
            sql.push_str(" AND r.service_id = ?");
        }

        let mut query = sqlx::query_as::<_, Role>(&sql)
            .bind(user_id)
            .bind(tenant_id);

        if let Some(service_id) = service_id {
            query = query.bind(service_id);
        }

        let roles = query.fetch_all(&self.pool).await?;
        Ok(roles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_rbac_find_permission_by_id() {
        let mut mock = MockRbacRepository::new();
        let permission = Permission::default();
        let permission_clone = permission.clone();
        let id = permission.id;

        mock.expect_find_permission_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(permission_clone.clone())));

        let result = mock.find_permission_by_id(id).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_mock_rbac_find_permissions_by_service() {
        let mut mock = MockRbacRepository::new();
        let service_id = StringUuid::new_v4();

        mock.expect_find_permissions_by_service()
            .with(eq(service_id))
            .returning(|_| {
                Ok(vec![
                    Permission { code: "read".to_string(), name: "Read".to_string(), ..Default::default() },
                    Permission { code: "write".to_string(), name: "Write".to_string(), ..Default::default() },
                ])
            });

        let result = mock.find_permissions_by_service(service_id).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_rbac_find_role_by_id() {
        let mut mock = MockRbacRepository::new();
        let role = Role::default();
        let role_clone = role.clone();
        let id = role.id;

        mock.expect_find_role_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(role_clone.clone())));

        let result = mock.find_role_by_id(id).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_mock_rbac_find_roles_by_service() {
        let mut mock = MockRbacRepository::new();
        let service_id = StringUuid::new_v4();

        mock.expect_find_roles_by_service()
            .with(eq(service_id))
            .returning(|_| {
                Ok(vec![
                    Role { name: "admin".to_string(), ..Default::default() },
                    Role { name: "viewer".to_string(), ..Default::default() },
                ])
            });

        let result = mock.find_roles_by_service(service_id).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_rbac_find_role_permissions() {
        let mut mock = MockRbacRepository::new();
        let role_id = StringUuid::new_v4();

        mock.expect_find_role_permissions()
            .with(eq(role_id))
            .returning(|_| {
                Ok(vec![
                    Permission { code: "user:read".to_string(), ..Default::default() },
                ])
            });

        let result = mock.find_role_permissions(role_id).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].code, "user:read");
    }

    #[tokio::test]
    async fn test_mock_rbac_find_user_roles_in_tenant() {
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
                    permissions: vec!["read".to_string(), "write".to_string()],
                })
            });

        let result = mock.find_user_roles_in_tenant(user_id, tenant_id).await.unwrap();
        assert_eq!(result.roles, vec!["admin"]);
        assert_eq!(result.permissions.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_rbac_assign_permission_to_role() {
        let mut mock = MockRbacRepository::new();
        let role_id = StringUuid::new_v4();
        let permission_id = StringUuid::new_v4();

        mock.expect_assign_permission_to_role()
            .with(eq(role_id), eq(permission_id))
            .returning(|_, _| Ok(()));

        let result = mock.assign_permission_to_role(role_id, permission_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_rbac_delete_permission() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete_permission()
            .with(eq(id))
            .returning(|_| Ok(()));

        let result = mock.delete_permission(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_rbac_delete_role() {
        let mut mock = MockRbacRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete_role()
            .with(eq(id))
            .returning(|_| Ok(()));

        let result = mock.delete_role(id).await;
        assert!(result.is_ok());
    }
}
