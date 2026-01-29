//! User repository

use crate::domain::{AddUserToTenantInput, CreateUserInput, TenantUser, UpdateUserInput, User};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, keycloak_id: &str, input: &CreateUserInput) -> Result<User>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn find_by_keycloak_id(&self, keycloak_id: &str) -> Result<Option<User>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<User>>;
    async fn count(&self) -> Result<i64>;
    async fn update(&self, id: Uuid, input: &UpdateUserInput) -> Result<User>;
    async fn update_mfa_enabled(&self, id: Uuid, enabled: bool) -> Result<User>;
    async fn delete(&self, id: Uuid) -> Result<()>;

    // Tenant-User relations
    async fn add_to_tenant(&self, input: &AddUserToTenantInput) -> Result<TenantUser>;
    async fn remove_from_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<()>;
    async fn find_tenant_users(
        &self,
        tenant_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>>;
    async fn find_user_tenants(&self, user_id: Uuid) -> Result<Vec<TenantUser>>;
}

pub struct UserRepositoryImpl {
    pool: MySqlPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, keycloak_id: &str, input: &CreateUserInput) -> Result<User> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO users (id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, false, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(keycloak_id)
        .bind(&input.email)
        .bind(&input.display_name)
        .bind(&input.avatar_url)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create user")))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at
            FROM users
            WHERE email = ?
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn find_by_keycloak_id(&self, keycloak_id: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at
            FROM users
            WHERE keycloak_id = ?
            "#,
        )
        .bind(keycloak_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn update(&self, id: Uuid, input: &UpdateUserInput) -> Result<User> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

        let display_name = input
            .display_name
            .as_ref()
            .or(existing.display_name.as_ref());
        let avatar_url = input.avatar_url.as_ref().or(existing.avatar_url.as_ref());

        sqlx::query(
            r#"
            UPDATE users
            SET display_name = ?, avatar_url = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(display_name)
        .bind(avatar_url)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update user")))
    }

    async fn update_mfa_enabled(&self, id: Uuid, enabled: bool) -> Result<User> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET mfa_enabled = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User {} not found", id)));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update user")))
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User {} not found", id)));
        }

        Ok(())
    }

    async fn add_to_tenant(&self, input: &AddUserToTenantInput) -> Result<TenantUser> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)
            VALUES (?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.tenant_id)
        .bind(input.user_id)
        .bind(&input.role_in_tenant)
        .execute(&self.pool)
        .await?;

        let tenant_user = sqlx::query_as::<_, TenantUser>(
            r#"
            SELECT id, tenant_id, user_id, role_in_tenant, joined_at
            FROM tenant_users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(tenant_user)
    }

    async fn remove_from_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM tenant_users WHERE user_id = ? AND tenant_id = ?")
            .bind(user_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "User-tenant relationship not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn find_tenant_users(
        &self,
        tenant_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.keycloak_id, u.email, u.display_name, u.avatar_url, u.mfa_enabled, u.created_at, u.updated_at
            FROM users u
            INNER JOIN tenant_users tu ON u.id = tu.user_id
            WHERE tu.tenant_id = ?
            ORDER BY tu.joined_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn find_user_tenants(&self, user_id: Uuid) -> Result<Vec<TenantUser>> {
        let tenant_users = sqlx::query_as::<_, TenantUser>(
            r#"
            SELECT id, tenant_id, user_id, role_in_tenant, joined_at
            FROM tenant_users
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tenant_users)
    }
}
