//! SAML Application repository

use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::models::saml_application::{SamlApplication, UpdateSamlApplicationInput};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SamlApplicationRepository: Send + Sync {
    async fn create(&self, app: &SamlApplication) -> Result<SamlApplication>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<SamlApplication>>;
    async fn find_by_tenant_and_entity_id(
        &self,
        tenant_id: StringUuid,
        entity_id: &str,
    ) -> Result<Option<SamlApplication>>;
    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<SamlApplication>>;
    async fn update(
        &self,
        id: StringUuid,
        input: &UpdateSamlApplicationInput,
    ) -> Result<SamlApplication>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
}

pub struct SamlApplicationRepositoryImpl {
    pool: MySqlPool,
}

impl SamlApplicationRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SamlApplicationRepository for SamlApplicationRepositoryImpl {
    async fn create(&self, app: &SamlApplication) -> Result<SamlApplication> {
        let mappings_json = serde_json::to_string(&app.attribute_mappings)
            .map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO saml_applications (
                id, tenant_id, name, entity_id, acs_url, slo_url,
                name_id_format, sign_assertions, sign_responses, encrypt_assertions,
                sp_certificate, attribute_mappings, backend_client_id, enabled,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(app.id)
        .bind(app.tenant_id)
        .bind(&app.name)
        .bind(&app.entity_id)
        .bind(&app.acs_url)
        .bind(&app.slo_url)
        .bind(&app.name_id_format)
        .bind(app.sign_assertions)
        .bind(app.sign_responses)
        .bind(app.encrypt_assertions)
        .bind(&app.sp_certificate)
        .bind(&mappings_json)
        .bind(&app.backend_client_id)
        .bind(app.enabled)
        .execute(&self.pool)
        .await?;

        self.find_by_id(app.id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create SAML application")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<SamlApplication>> {
        let app = sqlx::query_as::<_, SamlApplication>(
            r#"
            SELECT id, tenant_id, name, entity_id, acs_url, slo_url,
                   name_id_format, sign_assertions, sign_responses, encrypt_assertions,
                   sp_certificate, attribute_mappings, backend_client_id, enabled,
                   created_at, updated_at
            FROM saml_applications
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(app)
    }

    async fn find_by_tenant_and_entity_id(
        &self,
        tenant_id: StringUuid,
        entity_id: &str,
    ) -> Result<Option<SamlApplication>> {
        let app = sqlx::query_as::<_, SamlApplication>(
            r#"
            SELECT id, tenant_id, name, entity_id, acs_url, slo_url,
                   name_id_format, sign_assertions, sign_responses, encrypt_assertions,
                   sp_certificate, attribute_mappings, backend_client_id, enabled,
                   created_at, updated_at
            FROM saml_applications
            WHERE tenant_id = ? AND entity_id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(app)
    }

    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<SamlApplication>> {
        let apps = sqlx::query_as::<_, SamlApplication>(
            r#"
            SELECT id, tenant_id, name, entity_id, acs_url, slo_url,
                   name_id_format, sign_assertions, sign_responses, encrypt_assertions,
                   sp_certificate, attribute_mappings, backend_client_id, enabled,
                   created_at, updated_at
            FROM saml_applications
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(apps)
    }

    async fn update(
        &self,
        id: StringUuid,
        input: &UpdateSamlApplicationInput,
    ) -> Result<SamlApplication> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("SAML application {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let acs_url = input.acs_url.as_ref().unwrap_or(&existing.acs_url);
        let slo_url = input.slo_url.as_ref().or(existing.slo_url.as_ref());
        let name_id_format = input
            .name_id_format
            .as_ref()
            .map(|f| f.to_urn().to_string())
            .unwrap_or_else(|| existing.name_id_format.clone());
        let sign_assertions = input.sign_assertions.unwrap_or(existing.sign_assertions);
        let sign_responses = input.sign_responses.unwrap_or(existing.sign_responses);
        let encrypt_assertions = input
            .encrypt_assertions
            .unwrap_or(existing.encrypt_assertions);
        let sp_certificate = input
            .sp_certificate
            .as_ref()
            .or(existing.sp_certificate.as_ref());
        let attribute_mappings = input
            .attribute_mappings
            .as_ref()
            .unwrap_or(&existing.attribute_mappings);
        let enabled = input.enabled.unwrap_or(existing.enabled);

        let mappings_json =
            serde_json::to_string(attribute_mappings).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            UPDATE saml_applications
            SET name = ?, acs_url = ?, slo_url = ?, name_id_format = ?,
                sign_assertions = ?, sign_responses = ?, encrypt_assertions = ?,
                sp_certificate = ?, attribute_mappings = ?, enabled = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(acs_url)
        .bind(slo_url)
        .bind(&name_id_format)
        .bind(sign_assertions)
        .bind(sign_responses)
        .bind(encrypt_assertions)
        .bind(sp_certificate)
        .bind(&mappings_json)
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update SAML application")))
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM saml_applications WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("SAML application not found".to_string()));
        }

        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM saml_applications WHERE tenant_id = ?")
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
    async fn test_mock_list_by_tenant() {
        let mut mock = MockSamlApplicationRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| Ok(vec![]));

        let apps = mock.list_by_tenant(tenant_id).await.unwrap();
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn test_mock_find_by_tenant_and_entity_id() {
        let mut mock = MockSamlApplicationRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_find_by_tenant_and_entity_id()
            .with(eq(tenant_id), eq("https://sp.example.com"))
            .returning(|_, _| Ok(None));

        let result = mock
            .find_by_tenant_and_entity_id(tenant_id, "https://sp.example.com")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mut mock = MockSamlApplicationRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let result = mock.delete(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_delete_by_tenant() {
        let mut mock = MockSamlApplicationRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_delete_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| Ok(3));

        let count = mock.delete_by_tenant(tenant_id).await.unwrap();
        assert_eq!(count, 3);
    }
}
