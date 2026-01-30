//! Service repository

use crate::domain::{CreateServiceInput, Service, ServiceStatus, UpdateServiceInput};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ServiceRepository: Send + Sync {
    async fn create(&self, input: &CreateServiceInput) -> Result<Service>;
    async fn create_client(
        &self,
        service_id: Uuid,
        client_id: &str,
        secret_hash: &str,
        name: Option<String>,
    ) -> Result<crate::domain::Client>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>>;
    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Service>>;
    async fn find_client_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::domain::Client>>;
    async fn list(&self, tenant_id: Option<Uuid>, offset: i64, limit: i64) -> Result<Vec<Service>>;
    async fn list_clients(&self, service_id: Uuid) -> Result<Vec<crate::domain::Client>>;
    async fn count(&self, tenant_id: Option<Uuid>) -> Result<i64>;
    async fn update(&self, id: Uuid, input: &UpdateServiceInput) -> Result<Service>;
    async fn delete(&self, id: Uuid) -> Result<()>;
    async fn delete_client(&self, service_id: Uuid, client_id: &str) -> Result<()>;
    async fn update_client_secret_hash(&self, client_id: &str, new_secret_hash: &str)
        -> Result<()>;
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
    async fn create(&self, input: &CreateServiceInput) -> Result<Service> {
        let id = Uuid::new_v4();
        let redirect_uris = serde_json::to_string(&input.redirect_uris)
            .map_err(|e| AppError::Internal(e.into()))?;
        let logout_uris = serde_json::to_string(&input.logout_uris.clone().unwrap_or_default())
            .map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'active', NOW(), NOW())
            "#,
        )
        // UUID must be converted to string for CHAR(36) columns
        .bind(id.to_string())
        .bind(input.tenant_id.map(|id| id.to_string()))
        .bind(&input.name)
        .bind(&input.base_url)
        .bind(&redirect_uris)
        .bind(&logout_uris)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create service")))
    }

    async fn create_client(
        &self,
        service_id: Uuid,
        client_id: &str,
        secret_hash: &str,
        name: Option<String>,
    ) -> Result<crate::domain::Client> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
            VALUES (?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id.to_string())
        .bind(service_id.to_string())
        .bind(client_id)
        .bind(secret_hash)
        .bind(name)
        .execute(&self.pool)
        .await?;

        let client =
            sqlx::query_as::<_, crate::domain::Client>("SELECT * FROM clients WHERE id = ?")
                .bind(id.to_string())
                .fetch_one(&self.pool)
                .await?;

        Ok(client)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>> {
        let service = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at
            FROM services
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(service)
    }

    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Service>> {
        let service = sqlx::query_as::<_, Service>(
            r#"
            SELECT s.id, s.tenant_id, s.name, s.base_url, s.redirect_uris, s.logout_uris, s.status, s.created_at, s.updated_at
            FROM services s
            JOIN clients c ON s.id = c.service_id
            WHERE c.client_id = ?
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(service)
    }

    async fn find_client_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<Option<crate::domain::Client>> {
        let client =
            sqlx::query_as::<_, crate::domain::Client>("SELECT * FROM clients WHERE client_id = ?")
                .bind(client_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(client)
    }

    async fn list(&self, tenant_id: Option<Uuid>, offset: i64, limit: i64) -> Result<Vec<Service>> {
        let services = if let Some(tid) = tenant_id {
            sqlx::query_as::<_, Service>(
                r#"
                SELECT id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at
                FROM services
                WHERE tenant_id = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tid.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Service>(
                r#"
                SELECT id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at
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

    async fn list_clients(&self, service_id: Uuid) -> Result<Vec<crate::domain::Client>> {
        let clients = sqlx::query_as::<_, crate::domain::Client>(
            "SELECT * FROM clients WHERE service_id = ? ORDER BY created_at DESC",
        )
        .bind(service_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        Ok(clients)
    }

    async fn count(&self, tenant_id: Option<Uuid>) -> Result<i64> {
        let row: (i64,) = if let Some(tid) = tenant_id {
            sqlx::query_as("SELECT COUNT(*) FROM services WHERE tenant_id = ?")
                .bind(tid.to_string())
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
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update service")))
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM services WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Service {} not found", id)));
        }

        Ok(())
    }

    async fn delete_client(&self, service_id: Uuid, client_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM clients WHERE service_id = ? AND client_id = ?")
            .bind(service_id.to_string())
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Client {} not found",
                client_id
            )));
        }
        Ok(())
    }

    async fn update_client_secret_hash(
        &self,
        client_id: &str,
        new_secret_hash: &str,
    ) -> Result<()> {
        let result = sqlx::query("UPDATE clients SET client_secret_hash = ? WHERE client_id = ?")
            .bind(new_secret_hash)
            .bind(client_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Client {} not found",
                client_id
            )));
        }
        Ok(())
    }
}
