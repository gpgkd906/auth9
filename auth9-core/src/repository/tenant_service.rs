//! Tenant-Service association repository

use crate::domain::{ServiceWithStatus, StringUuid};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantServiceRepository: Send + Sync {
    /// List all services with their enabled status for a tenant
    async fn list_services_for_tenant(&self, tenant_id: StringUuid) -> Result<Vec<ServiceWithStatus>>;

    /// Enable or disable a service for a tenant
    async fn toggle_service(
        &self,
        tenant_id: StringUuid,
        service_id: StringUuid,
        enabled: bool,
    ) -> Result<()>;

    /// Get enabled services for a tenant (for invitation)
    async fn get_enabled_services(&self, tenant_id: StringUuid) -> Result<Vec<ServiceWithStatus>>;

    /// Check if a service is enabled for a tenant
    async fn is_service_enabled(&self, tenant_id: StringUuid, service_id: StringUuid) -> Result<bool>;
}

pub struct TenantServiceRepositoryImpl {
    pool: MySqlPool,
}

impl TenantServiceRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantServiceRepository for TenantServiceRepositoryImpl {
    async fn list_services_for_tenant(&self, tenant_id: StringUuid) -> Result<Vec<ServiceWithStatus>> {
        // List all global services (tenant_id IS NULL) with their enabled status for this tenant
        let services = sqlx::query_as::<_, ServiceWithStatus>(
            r#"
            SELECT
                s.id,
                s.name,
                s.base_url,
                s.status,
                COALESCE(ts.enabled, FALSE) as enabled
            FROM services s
            LEFT JOIN tenant_services ts ON ts.service_id = s.id AND ts.tenant_id = ?
            WHERE s.tenant_id IS NULL
            ORDER BY s.name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(services)
    }

    async fn toggle_service(
        &self,
        tenant_id: StringUuid,
        service_id: StringUuid,
        enabled: bool,
    ) -> Result<()> {
        // Use INSERT ... ON DUPLICATE KEY UPDATE for upsert
        sqlx::query(
            r#"
            INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
            VALUES (?, ?, ?, NOW(), NOW())
            ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), updated_at = NOW()
            "#,
        )
        .bind(tenant_id)
        .bind(service_id)
        .bind(enabled)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_enabled_services(&self, tenant_id: StringUuid) -> Result<Vec<ServiceWithStatus>> {
        let services = sqlx::query_as::<_, ServiceWithStatus>(
            r#"
            SELECT
                s.id,
                s.name,
                s.base_url,
                s.status,
                TRUE as enabled
            FROM services s
            INNER JOIN tenant_services ts ON ts.service_id = s.id
            WHERE ts.tenant_id = ? AND ts.enabled = TRUE AND s.tenant_id IS NULL
            ORDER BY s.name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(services)
    }

    async fn is_service_enabled(&self, tenant_id: StringUuid, service_id: StringUuid) -> Result<bool> {
        let row: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT enabled FROM tenant_services
            WHERE tenant_id = ? AND service_id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(service_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(enabled,)| enabled).unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_tenant_service_repository() {
        let mut mock = MockTenantServiceRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_services_for_tenant()
            .with(eq(tenant_id))
            .returning(|_| Ok(vec![]));

        let result = mock.list_services_for_tenant(tenant_id).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_toggle_service() {
        let mut mock = MockTenantServiceRepository::new();
        let tenant_id = StringUuid::new_v4();
        let service_id = StringUuid::new_v4();

        mock.expect_toggle_service()
            .with(eq(tenant_id), eq(service_id), eq(true))
            .returning(|_, _, _| Ok(()));

        let result = mock.toggle_service(tenant_id, service_id, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_is_service_enabled() {
        let mut mock = MockTenantServiceRepository::new();
        let tenant_id = StringUuid::new_v4();
        let service_id = StringUuid::new_v4();

        mock.expect_is_service_enabled()
            .with(eq(tenant_id), eq(service_id))
            .returning(|_, _| Ok(true));

        let result = mock.is_service_enabled(tenant_id, service_id).await.unwrap();
        assert!(result);
    }
}
