//! Tenant repository

use crate::domain::{CreateTenantInput, Tenant, TenantStatus, UpdateTenantInput};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Tenant>>;
    async fn count(&self) -> Result<i64>;
    async fn update(&self, id: Uuid, input: &UpdateTenantInput) -> Result<Tenant>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

pub struct TenantRepositoryImpl {
    pool: MySqlPool,
}

impl TenantRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepository for TenantRepositoryImpl {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant> {
        let id = Uuid::new_v4();
        let settings = input.settings.clone().unwrap_or_default();
        let settings_json =
            serde_json::to_string(&settings).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, slug, logo_url, settings, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'active', NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.slug)
        .bind(&input.logo_url)
        .bind(&settings_json)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create tenant")))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, logo_url, settings, status, created_at, updated_at
            FROM tenants
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, logo_url, settings, status, created_at, updated_at
            FROM tenants
            WHERE slug = ?
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, logo_url, settings, status, created_at, updated_at
            FROM tenants
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(tenants)
    }

    async fn count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn update(&self, id: Uuid, input: &UpdateTenantInput) -> Result<Tenant> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let logo_url = input.logo_url.as_ref().or(existing.logo_url.as_ref());
        let settings = input.settings.as_ref().unwrap_or(&existing.settings);
        let status = input.status.as_ref().unwrap_or(&existing.status);

        let settings_json =
            serde_json::to_string(&settings).map_err(|e| AppError::Internal(e.into()))?;

        let status_str = match status {
            TenantStatus::Active => "active",
            TenantStatus::Inactive => "inactive",
            TenantStatus::Suspended => "suspended",
        };

        sqlx::query(
            r#"
            UPDATE tenants
            SET name = ?, logo_url = ?, settings = ?, status = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(logo_url)
        .bind(&settings_json)
        .bind(status_str)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update tenant")))
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM tenants WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Tenant {} not found", id)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_tenant_repository() {
        let mut mock = MockTenantRepository::new();

        let tenant = Tenant::default();
        let tenant_clone = tenant.clone();

        mock.expect_find_by_id()
            .with(eq(tenant.id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        let result = mock.find_by_id(tenant.id).await.unwrap();
        assert!(result.is_some());
    }
}
