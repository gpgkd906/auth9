//! SCIM Group-Role mapping repository

use crate::domain::{ScimGroupRoleMapping, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ScimGroupRoleMappingRepository: Send + Sync {
    async fn find_by_scim_group(
        &self,
        connector_id: StringUuid,
        scim_group_id: &str,
    ) -> Result<Option<ScimGroupRoleMapping>>;
    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
    ) -> Result<Vec<ScimGroupRoleMapping>>;
    async fn upsert(&self, mapping: &ScimGroupRoleMapping) -> Result<ScimGroupRoleMapping>;
    async fn update_display_name(&self, id: StringUuid, display_name: &str) -> Result<()>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64>;
}

pub struct ScimGroupRoleMappingRepositoryImpl {
    pool: MySqlPool,
}

impl ScimGroupRoleMappingRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScimGroupRoleMappingRepository for ScimGroupRoleMappingRepositoryImpl {
    async fn find_by_scim_group(
        &self,
        connector_id: StringUuid,
        scim_group_id: &str,
    ) -> Result<Option<ScimGroupRoleMapping>> {
        let mapping = sqlx::query_as::<_, ScimGroupRoleMapping>(
            r#"
            SELECT id, tenant_id, connector_id, scim_group_id, scim_group_display_name,
                   role_id, created_at, updated_at
            FROM scim_group_role_mappings
            WHERE connector_id = ? AND scim_group_id = ?
            "#,
        )
        .bind(connector_id)
        .bind(scim_group_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(mapping)
    }

    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
    ) -> Result<Vec<ScimGroupRoleMapping>> {
        let mappings = sqlx::query_as::<_, ScimGroupRoleMapping>(
            r#"
            SELECT id, tenant_id, connector_id, scim_group_id, scim_group_display_name,
                   role_id, created_at, updated_at
            FROM scim_group_role_mappings
            WHERE connector_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(connector_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(mappings)
    }

    async fn upsert(&self, mapping: &ScimGroupRoleMapping) -> Result<ScimGroupRoleMapping> {
        sqlx::query(
            r#"
            INSERT INTO scim_group_role_mappings (id, tenant_id, connector_id, scim_group_id, scim_group_display_name, role_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, NOW(), NOW())
            ON DUPLICATE KEY UPDATE
                scim_group_display_name = VALUES(scim_group_display_name),
                role_id = VALUES(role_id),
                updated_at = NOW()
            "#,
        )
        .bind(mapping.id)
        .bind(mapping.tenant_id)
        .bind(mapping.connector_id)
        .bind(&mapping.scim_group_id)
        .bind(&mapping.scim_group_display_name)
        .bind(mapping.role_id)
        .execute(&self.pool)
        .await?;

        self.find_by_scim_group(mapping.connector_id, &mapping.scim_group_id)
            .await?
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("Failed to upsert SCIM group mapping"))
            })
    }

    async fn update_display_name(&self, id: StringUuid, display_name: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE scim_group_role_mappings SET scim_group_display_name = ?, updated_at = NOW() WHERE id = ?",
        )
        .bind(display_name)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "SCIM group mapping {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM scim_group_role_mappings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "SCIM group mapping {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM scim_group_role_mappings WHERE connector_id = ?")
            .bind(connector_id)
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
    async fn test_mock_find_by_scim_group_not_found() {
        let mut mock = MockScimGroupRoleMappingRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_find_by_scim_group()
            .with(eq(cid), eq("nonexistent"))
            .returning(|_, _| Ok(None));

        let result = mock.find_by_scim_group(cid, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_list_by_connector() {
        let mut mock = MockScimGroupRoleMappingRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_list_by_connector()
            .with(eq(cid))
            .returning(|_| Ok(vec![]));

        let result = mock.list_by_connector(cid).await.unwrap();
        assert!(result.is_empty());
    }
}
