//! SCIM Provisioning Log repository

use crate::domain::{CreateScimLogInput, ScimProvisioningLog, StringUuid};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ScimProvisioningLogRepository: Send + Sync {
    async fn create(&self, input: &CreateScimLogInput) -> Result<ScimProvisioningLog>;
    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ScimProvisioningLog>>;
    async fn count_by_connector(&self, connector_id: StringUuid) -> Result<i64>;
}

pub struct ScimProvisioningLogRepositoryImpl {
    pool: MySqlPool,
}

impl ScimProvisioningLogRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScimProvisioningLogRepository for ScimProvisioningLogRepositoryImpl {
    async fn create(&self, input: &CreateScimLogInput) -> Result<ScimProvisioningLog> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO scim_provisioning_logs
                (id, tenant_id, connector_id, operation, resource_type,
                 scim_resource_id, auth9_resource_id, status, error_detail, response_status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.tenant_id)
        .bind(input.connector_id)
        .bind(&input.operation)
        .bind(&input.resource_type)
        .bind(&input.scim_resource_id)
        .bind(input.auth9_resource_id)
        .bind(&input.status)
        .bind(&input.error_detail)
        .bind(input.response_status)
        .execute(&self.pool)
        .await?;

        let log = sqlx::query_as::<_, ScimProvisioningLog>(
            r#"
            SELECT id, tenant_id, connector_id, operation, resource_type,
                   scim_resource_id, auth9_resource_id, status, error_detail, response_status, created_at
            FROM scim_provisioning_logs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(log)
    }

    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ScimProvisioningLog>> {
        let logs = sqlx::query_as::<_, ScimProvisioningLog>(
            r#"
            SELECT id, tenant_id, connector_id, operation, resource_type,
                   scim_resource_id, auth9_resource_id, status, error_detail, response_status, created_at
            FROM scim_provisioning_logs
            WHERE connector_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(connector_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    async fn count_by_connector(&self, connector_id: StringUuid) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scim_provisioning_logs WHERE connector_id = ?")
                .bind(connector_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_count_by_connector() {
        let mut mock = MockScimProvisioningLogRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_count_by_connector()
            .with(eq(cid))
            .returning(|_| Ok(5));

        let result = mock.count_by_connector(cid).await.unwrap();
        assert_eq!(result, 5);
    }

    #[tokio::test]
    async fn test_mock_list_by_connector() {
        let mut mock = MockScimProvisioningLogRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_list_by_connector()
            .with(eq(cid), eq(0), eq(20))
            .returning(|_, _, _| Ok(vec![]));

        let result = mock.list_by_connector(cid, 0, 20).await.unwrap();
        assert!(result.is_empty());
    }
}
