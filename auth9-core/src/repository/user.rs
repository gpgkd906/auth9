//! User repository

use crate::domain::{
    AddUserToTenantInput, CreateUserInput, StringUuid, TenantInfo, TenantUser,
    TenantUserWithTenant, UpdateUserInput, User,
};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, keycloak_id: &str, input: &CreateUserInput) -> Result<User>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn find_by_keycloak_id(&self, keycloak_id: &str) -> Result<Option<User>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<User>>;
    async fn count(&self) -> Result<i64>;
    async fn search(&self, query: &str, offset: i64, limit: i64) -> Result<Vec<User>>;
    async fn search_count(&self, query: &str) -> Result<i64>;
    async fn update(&self, id: StringUuid, input: &UpdateUserInput) -> Result<User>;
    async fn update_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User>;
    async fn delete(&self, id: StringUuid) -> Result<()>;

    // Tenant-User relations
    async fn add_to_tenant(&self, input: &AddUserToTenantInput) -> Result<TenantUser>;
    async fn update_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role: &str,
    ) -> Result<TenantUser>;
    async fn remove_from_tenant(&self, user_id: StringUuid, tenant_id: StringUuid) -> Result<()>;
    async fn find_tenant_users(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>>;
    async fn find_user_tenants(&self, user_id: StringUuid) -> Result<Vec<TenantUser>>;

    /// Find user's tenants with tenant data (for API responses)
    async fn find_user_tenants_with_tenant(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<TenantUserWithTenant>>;

    /// Delete all tenant memberships for a user (tenant_users records)
    async fn delete_all_tenant_memberships(&self, user_id: StringUuid) -> Result<u64>;

    /// List all tenant_user IDs for a user (for cascade delete)
    async fn list_tenant_user_ids(&self, user_id: StringUuid) -> Result<Vec<StringUuid>>;

    /// List all tenant_user IDs for a tenant (for cascade delete)
    async fn list_tenant_user_ids_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Vec<StringUuid>>;

    /// Delete all tenant memberships for a tenant
    async fn delete_tenant_memberships_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
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
        let id = StringUuid::new_v4();

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

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<User>> {
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

    async fn search(&self, query: &str, offset: i64, limit: i64) -> Result<Vec<User>> {
        let pattern = format!("%{}%", query);
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at
            FROM users
            WHERE email LIKE ? OR display_name LIKE ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    async fn search_count(&self, query: &str) -> Result<i64> {
        let pattern = format!("%{}%", query);
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE email LIKE ? OR display_name LIKE ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn update(&self, id: StringUuid, input: &UpdateUserInput) -> Result<User> {
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

    async fn update_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User> {
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

    async fn delete(&self, id: StringUuid) -> Result<()> {
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
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)
            VALUES (?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(StringUuid::from(input.tenant_id))
        .bind(StringUuid::from(input.user_id))
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

    async fn update_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role: &str,
    ) -> Result<TenantUser> {
        let result = sqlx::query(
            r#"
            UPDATE tenant_users
            SET role_in_tenant = ?
            WHERE user_id = ? AND tenant_id = ?
            "#,
        )
        .bind(role)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "User-tenant relationship not found".to_string(),
            ));
        }

        let tenant_user = sqlx::query_as::<_, TenantUser>(
            r#"
            SELECT id, tenant_id, user_id, role_in_tenant, joined_at
            FROM tenant_users
            WHERE user_id = ? AND tenant_id = ?
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(tenant_user)
    }

    async fn remove_from_tenant(&self, user_id: StringUuid, tenant_id: StringUuid) -> Result<()> {
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
        tenant_id: StringUuid,
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

    async fn find_user_tenants(&self, user_id: StringUuid) -> Result<Vec<TenantUser>> {
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

    async fn find_user_tenants_with_tenant(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<TenantUserWithTenant>> {
        // Query tenant_users with tenant data joined
        let rows: Vec<(
            StringUuid,
            StringUuid,
            StringUuid,
            String,
            chrono::DateTime<chrono::Utc>,
            StringUuid,
            String,
            String,
            Option<String>,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT
                tu.id, tu.tenant_id, tu.user_id, tu.role_in_tenant, tu.joined_at,
                t.id as tenant_real_id, t.name as tenant_name, t.slug as tenant_slug, t.logo_url as tenant_logo_url, t.status as tenant_status
            FROM tenant_users tu
            INNER JOIN tenants t ON tu.tenant_id = t.id
            WHERE tu.user_id = ?
            ORDER BY tu.joined_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let result = rows
            .into_iter()
            .map(
                |(
                    id,
                    tenant_id,
                    user_id,
                    role_in_tenant,
                    joined_at,
                    tenant_real_id,
                    name,
                    slug,
                    logo_url,
                    status,
                )| {
                    TenantUserWithTenant {
                        id,
                        tenant_id,
                        user_id,
                        role_in_tenant,
                        joined_at,
                        tenant: TenantInfo {
                            id: tenant_real_id,
                            name,
                            slug,
                            logo_url,
                            status,
                        },
                    }
                },
            )
            .collect();

        Ok(result)
    }

    async fn delete_all_tenant_memberships(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM tenant_users WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn list_tenant_user_ids(&self, user_id: StringUuid) -> Result<Vec<StringUuid>> {
        let ids: Vec<(StringUuid,)> =
            sqlx::query_as("SELECT id FROM tenant_users WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    async fn list_tenant_user_ids_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Vec<StringUuid>> {
        let ids: Vec<(StringUuid,)> =
            sqlx::query_as("SELECT id FROM tenant_users WHERE tenant_id = ?")
                .bind(tenant_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    async fn delete_tenant_memberships_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM tenant_users WHERE tenant_id = ?")
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_user_repository_find_by_id() {
        let mut mock = MockUserRepository::new();
        let user = User::default();
        let user_clone = user.clone();
        let id = user.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        let result = mock.find_by_id(id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_mock_user_repository_find_by_id_not_found() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let result = mock.find_by_id(id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_user_repository_find_by_email() {
        let mut mock = MockUserRepository::new();
        let user = User {
            email: "test@example.com".to_string(),
            ..Default::default()
        };
        let user_clone = user.clone();

        mock.expect_find_by_email()
            .with(eq("test@example.com"))
            .returning(move |_| Ok(Some(user_clone.clone())));

        let result = mock.find_by_email("test@example.com").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_mock_user_repository_create() {
        let mut mock = MockUserRepository::new();
        let keycloak_id = "kc-123";
        let input = CreateUserInput {
            email: "new@example.com".to_string(),
            display_name: Some("New User".to_string()),
            avatar_url: None,
        };

        mock.expect_create().returning(|_, input| {
            Ok(User {
                email: input.email.clone(),
                display_name: input.display_name.clone(),
                ..Default::default()
            })
        });

        let result = mock.create(keycloak_id, &input).await.unwrap();
        assert_eq!(result.email, "new@example.com");
        assert_eq!(result.display_name, Some("New User".to_string()));
    }

    #[tokio::test]
    async fn test_mock_user_repository_list() {
        let mut mock = MockUserRepository::new();

        mock.expect_list().with(eq(0), eq(10)).returning(|_, _| {
            Ok(vec![
                User {
                    email: "user1@example.com".to_string(),
                    ..Default::default()
                },
                User {
                    email: "user2@example.com".to_string(),
                    ..Default::default()
                },
            ])
        });

        let result = mock.list(0, 10).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_user_repository_count() {
        let mut mock = MockUserRepository::new();

        mock.expect_count().returning(|| Ok(42));

        let result = mock.count().await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_mock_user_repository_delete() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let result = mock.delete(id).await;
        assert!(result.is_ok());
    }
}
