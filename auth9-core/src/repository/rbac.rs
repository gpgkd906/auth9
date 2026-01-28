//! RBAC repository

use crate::domain::{
    CreatePermissionInput, CreateRoleInput, Permission, Role, RolePermission,
    UpdateRoleInput, UserTenantRole, AssignRolesInput, UserRolesInTenant,
};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RbacRepository: Send + Sync {
    // Permissions
    async fn create_permission(&self, input: &CreatePermissionInput) -> Result<Permission>;
    async fn find_permission_by_id(&self, id: Uuid) -> Result<Option<Permission>>;
    async fn find_permissions_by_service(&self, service_id: Uuid) -> Result<Vec<Permission>>;
    async fn delete_permission(&self, id: Uuid) -> Result<()>;

    // Roles
    async fn create_role(&self, input: &CreateRoleInput) -> Result<Role>;
    async fn find_role_by_id(&self, id: Uuid) -> Result<Option<Role>>;
    async fn find_roles_by_service(&self, service_id: Uuid) -> Result<Vec<Role>>;
    async fn update_role(&self, id: Uuid, input: &UpdateRoleInput) -> Result<Role>;
    async fn delete_role(&self, id: Uuid) -> Result<()>;

    // Role-Permission mapping
    async fn assign_permission_to_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()>;
    async fn remove_permission_from_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()>;
    async fn find_role_permissions(&self, role_id: Uuid) -> Result<Vec<Permission>>;

    // User-Tenant-Role
    async fn assign_roles_to_user(&self, input: &AssignRolesInput, granted_by: Option<Uuid>) -> Result<()>;
    async fn remove_role_from_user(&self, tenant_user_id: Uuid, role_id: Uuid) -> Result<()>;
    async fn find_user_roles_in_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<UserRolesInTenant>;
}

pub struct RbacRepositoryImpl {
    pool: MySqlPool,
}

impl RbacRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RbacRepository for RbacRepositoryImpl {
    async fn create_permission(&self, input: &CreatePermissionInput) -> Result<Permission> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO permissions (id, service_id, code, name, description)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(input.service_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.description)
        .execute(&self.pool)
        .await?;

        self.find_permission_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create permission")))
    }

    async fn find_permission_by_id(&self, id: Uuid) -> Result<Option<Permission>> {
        let permission = sqlx::query_as::<_, Permission>(
            "SELECT id, service_id, code, name, description FROM permissions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(permission)
    }

    async fn find_permissions_by_service(&self, service_id: Uuid) -> Result<Vec<Permission>> {
        let permissions = sqlx::query_as::<_, Permission>(
            "SELECT id, service_id, code, name, description FROM permissions WHERE service_id = ?",
        )
        .bind(service_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(permissions)
    }

    async fn delete_permission(&self, id: Uuid) -> Result<()> {
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
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO roles (id, service_id, name, description, parent_role_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(input.service_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.parent_role_id)
        .execute(&self.pool)
        .await?;

        // Assign permissions if provided
        if let Some(permission_ids) = &input.permission_ids {
            for perm_id in permission_ids {
                self.assign_permission_to_role(id, *perm_id).await?;
            }
        }

        self.find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create role")))
    }

    async fn find_role_by_id(&self, id: Uuid) -> Result<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT id, service_id, name, description, parent_role_id, created_at, updated_at FROM roles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(role)
    }

    async fn find_roles_by_service(&self, service_id: Uuid) -> Result<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT id, service_id, name, description, parent_role_id, created_at, updated_at FROM roles WHERE service_id = ?",
        )
        .bind(service_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(roles)
    }

    async fn update_role(&self, id: Uuid, input: &UpdateRoleInput) -> Result<Role> {
        let existing = self
            .find_role_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Role {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let description = input.description.as_ref().or(existing.description.as_ref());
        let parent_role_id = input.parent_role_id.or(existing.parent_role_id);

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

    async fn delete_role(&self, id: Uuid) -> Result<()> {
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

    async fn assign_permission_to_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove_permission_from_role(&self, role_id: Uuid, permission_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ?")
            .bind(role_id)
            .bind(permission_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_role_permissions(&self, role_id: Uuid) -> Result<Vec<Permission>> {
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

    async fn assign_roles_to_user(&self, input: &AssignRolesInput, granted_by: Option<Uuid>) -> Result<()> {
        // First find the tenant_user record
        let tenant_user_id: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM tenant_users WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(input.user_id)
        .bind(input.tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        let tenant_user_id = tenant_user_id
            .ok_or_else(|| AppError::NotFound("User not in tenant".to_string()))?
            .0;

        for role_id in &input.role_ids {
            let id = Uuid::new_v4();
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

    async fn remove_role_from_user(&self, tenant_user_id: Uuid, role_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_tenant_roles WHERE tenant_user_id = ? AND role_id = ?")
            .bind(tenant_user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_user_roles_in_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<UserRolesInTenant> {
        // Get roles
        let roles: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT r.name
            FROM roles r
            INNER JOIN user_tenant_roles utr ON r.id = utr.role_id
            INNER JOIN tenant_users tu ON utr.tenant_user_id = tu.id
            WHERE tu.user_id = ? AND tu.tenant_id = ?
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        // Get permissions (from all assigned roles)
        let permissions: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT p.code
            FROM permissions p
            INNER JOIN role_permissions rp ON p.id = rp.permission_id
            INNER JOIN user_tenant_roles utr ON rp.role_id = utr.role_id
            INNER JOIN tenant_users tu ON utr.tenant_user_id = tu.id
            WHERE tu.user_id = ? AND tu.tenant_id = ?
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(UserRolesInTenant {
            user_id,
            tenant_id,
            roles: roles.into_iter().map(|(r,)| r).collect(),
            permissions: permissions.into_iter().map(|(p,)| p).collect(),
        })
    }
}
