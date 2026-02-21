//! Service branding repository

use crate::domain::{BrandingConfig, ServiceBranding, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ServiceBrandingRepository: Send + Sync {
    async fn get_by_service_id(&self, service_id: StringUuid) -> Result<Option<ServiceBranding>>;
    async fn upsert(
        &self,
        service_id: StringUuid,
        config: &BrandingConfig,
    ) -> Result<ServiceBranding>;
    async fn delete_by_service_id(&self, service_id: StringUuid) -> Result<()>;
}

pub struct ServiceBrandingRepositoryImpl {
    pool: MySqlPool,
}

impl ServiceBrandingRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceBrandingRepository for ServiceBrandingRepositoryImpl {
    async fn get_by_service_id(&self, service_id: StringUuid) -> Result<Option<ServiceBranding>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT id, service_id, CAST(config AS CHAR) as config, created_at, updated_at
            FROM service_branding
            WHERE service_id = ?
            "#,
        )
        .bind(service_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((id, svc_id, config_json, created_at, updated_at)) => {
                let config: BrandingConfig =
                    serde_json::from_str(&config_json).map_err(|e| AppError::Internal(e.into()))?;
                Ok(Some(ServiceBranding {
                    id,
                    service_id: svc_id,
                    config,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn upsert(
        &self,
        service_id: StringUuid,
        config: &BrandingConfig,
    ) -> Result<ServiceBranding> {
        let id = StringUuid::new_v4();
        let config_json =
            serde_json::to_string(config).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO service_branding (id, service_id, config, created_at, updated_at)
            VALUES (?, ?, ?, NOW(), NOW())
            ON DUPLICATE KEY UPDATE config = VALUES(config), updated_at = NOW()
            "#,
        )
        .bind(id)
        .bind(service_id)
        .bind(&config_json)
        .execute(&self.pool)
        .await?;

        self.get_by_service_id(service_id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to upsert service branding")))
    }

    async fn delete_by_service_id(&self, service_id: StringUuid) -> Result<()> {
        sqlx::query("DELETE FROM service_branding WHERE service_id = ?")
            .bind(service_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_branding_repository_trait_is_mockable() {
        let _mock = MockServiceBrandingRepository::new();
    }
}
