//! Service repository

use crate::domain::{CreateServiceInput, Service, ServiceStatus, UpdateServiceInput};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ServiceRepository: Send + Sync {
    async fn create(&self, input: &CreateServiceInput, secret_hash: &str) -> Result<Service>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>>;
    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Service>>;
    async fn list(&self, tenant_id: Option<Uuid>, offset: i64, limit: i64) -> Result<Vec<Service>>;
    async fn count(&self, tenant_id: Option<Uuid>) -> Result<i64>;
    async fn update(&self, id: Uuid, input: &UpdateServiceInput) -> Result<Service>;
    async fn update_secret(&self, id: Uuid, secret_hash: &str) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

pub struct ServiceRepositoryImpl {
    pool: MySqlPool,
}

impl ServiceRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceRepository for ServiceRepositoryImpl {
    async fn create(&self, input: &CreateServiceInput, secret_hash: &str) -> Result<Service> {
        let id = Uuid::new_v4();
        let redirect_uris = serde_json::to_string(&input.redirect_uris)
            .map_err(|e| AppError::Internal(e.into()))?;
        let logout_uris = serde_json::to_string(&input.logout_uris.clone().unwrap_or_default())
            .map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO services (id, tenant_id, name, client_id, client_secret_hash, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', NOW(), NOW())
            "#,
        )
        // UUID must be converted to string for CHAR(36) columns
        .bind(id.to_string())
        .bind(input.tenant_id.map(|id| id.to_string()))
        .bind(&input.name)
        .bind(&input.client_id)
        .bind(secret_hash)
        .bind(&input.base_url)
        .bind(&redirect_uris)
        .bind(&logout_uris)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create service")))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>> {
        let service = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, tenant_id, name, client_id, client_secret_hash, base_url, redirect_uris, logout_uris, status, created_at, updated_at
            FROM services
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(service)
    }

    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Service>> {
        let service = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, tenant_id, name, client_id, client_secret_hash, base_url, redirect_uris, logout_uris, status, created_at, updated_at
            FROM services
            WHERE client_id = ?
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(service)
    }

    async fn list(&self, tenant_id: Option<Uuid>, offset: i64, limit: i64) -> Result<Vec<Service>> {
        let services = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, Service>(
                r#"
                SELECT id, tenant_id, name, client_id, client_secret_hash, base_url, redirect_uris, logout_uris, status, created_at, updated_at
                FROM services
                WHERE tenant_id = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tid)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Service>(
                r#"
                SELECT id, tenant_id, name, client_id, client_secret_hash, base_url, redirect_uris, logout_uris, status, created_at, updated_at
                FROM services
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(services)
    }

    async fn count(&self, tenant_id: Option<Uuid>) -> Result<i64> {
        let row: (i64,) = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT COUNT(*) FROM services WHERE tenant_id = ?")
                .bind(tid)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM services")
                .fetch_one(&self.pool)
                .await?
        };
        Ok(row.0)
    }

    async fn update(&self, id: Uuid, input: &UpdateServiceInput) -> Result<Service> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Service {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let base_url = input.base_url.as_ref().or(existing.base_url.as_ref());
        let redirect_uris = input
            .redirect_uris
            .as_ref()
            .unwrap_or(&existing.redirect_uris);
        let logout_uris = input.logout_uris.as_ref().unwrap_or(&existing.logout_uris);
        let status = input.status.as_ref().unwrap_or(&existing.status);

        let redirect_uris_json =
            serde_json::to_string(&redirect_uris).map_err(|e| AppError::Internal(e.into()))?;
        let logout_uris_json =
            serde_json::to_string(&logout_uris).map_err(|e| AppError::Internal(e.into()))?;

        let status_str = match status {
            ServiceStatus::Active => "active",
            ServiceStatus::Inactive => "inactive",
        };

        sqlx::query(
            r#"
            UPDATE services
            SET name = ?, base_url = ?, redirect_uris = ?, logout_uris = ?, status = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(base_url)
        .bind(&redirect_uris_json)
        .bind(&logout_uris_json)
        .bind(status_str)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update service")))
    }

    async fn update_secret(&self, id: Uuid, secret_hash: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE services SET client_secret_hash = ?, updated_at = NOW() WHERE id = ?",
        )
        .bind(secret_hash)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Service {} not found", id)));
        }

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM services WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Service {} not found", id)));
        }

        Ok(())
    }
}
