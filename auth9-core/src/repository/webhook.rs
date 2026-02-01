//! Webhook repository

use crate::domain::{CreateWebhookInput, StringUuid, UpdateWebhookInput, Webhook};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebhookRepository: Send + Sync {
    async fn create(&self, tenant_id: StringUuid, input: &CreateWebhookInput) -> Result<Webhook>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Webhook>>;
    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Webhook>>;
    async fn list_enabled_for_event(&self, event: &str) -> Result<Vec<Webhook>>;
    async fn update(&self, id: StringUuid, input: &UpdateWebhookInput) -> Result<Webhook>;
    async fn update_triggered(&self, id: StringUuid, success: bool) -> Result<()>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
}

pub struct WebhookRepositoryImpl {
    pool: MySqlPool,
}

impl WebhookRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookRepository for WebhookRepositoryImpl {
    async fn create(&self, tenant_id: StringUuid, input: &CreateWebhookInput) -> Result<Webhook> {
        let id = StringUuid::new_v4();
        let events_json =
            serde_json::to_string(&input.events).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO webhooks (id, tenant_id, name, url, secret, events, enabled,
                                  created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&input.name)
        .bind(&input.url)
        .bind(&input.secret)
        .bind(&events_json)
        .bind(input.enabled)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create webhook")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Webhook>> {
        let webhook = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT id, tenant_id, name, url, secret, events, enabled,
                   last_triggered_at, failure_count, created_at, updated_at
            FROM webhooks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(webhook)
    }

    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Webhook>> {
        let webhooks = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT id, tenant_id, name, url, secret, events, enabled,
                   last_triggered_at, failure_count, created_at, updated_at
            FROM webhooks
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(webhooks)
    }

    async fn list_enabled_for_event(&self, event: &str) -> Result<Vec<Webhook>> {
        // Use JSON_CONTAINS to find webhooks that have this event in their events array
        let webhooks = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT id, tenant_id, name, url, secret, events, enabled,
                   last_triggered_at, failure_count, created_at, updated_at
            FROM webhooks
            WHERE enabled = true AND JSON_CONTAINS(events, ?)
            "#,
        )
        .bind(format!("\"{}\"", event))
        .fetch_all(&self.pool)
        .await?;

        Ok(webhooks)
    }

    async fn update(&self, id: StringUuid, input: &UpdateWebhookInput) -> Result<Webhook> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Webhook {} not found", id)))?;

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let url = input.url.as_ref().unwrap_or(&existing.url);
        let secret = input.secret.as_ref().or(existing.secret.as_ref());
        let events = input.events.as_ref().unwrap_or(&existing.events);
        let enabled = input.enabled.unwrap_or(existing.enabled);

        let events_json =
            serde_json::to_string(&events).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            UPDATE webhooks
            SET name = ?, url = ?, secret = ?, events = ?, enabled = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(url)
        .bind(secret)
        .bind(&events_json)
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update webhook")))
    }

    async fn update_triggered(&self, id: StringUuid, success: bool) -> Result<()> {
        if success {
            sqlx::query(
                r#"
                UPDATE webhooks
                SET last_triggered_at = NOW(), failure_count = 0
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE webhooks
                SET last_triggered_at = NOW(), failure_count = failure_count + 1
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM webhooks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Webhook not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_webhook_repository() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| {
                Ok(vec![Webhook {
                    name: "Test Webhook".to_string(),
                    url: "https://example.com/webhook".to_string(),
                    events: vec!["login.success".to_string()],
                    ..Default::default()
                }])
            });

        let webhooks = mock.list_by_tenant(tenant_id).await.unwrap();
        assert_eq!(webhooks.len(), 1);
        assert_eq!(webhooks[0].name, "Test Webhook");
    }

    #[tokio::test]
    async fn test_mock_list_enabled_for_event() {
        let mut mock = MockWebhookRepository::new();

        mock.expect_list_enabled_for_event()
            .with(eq("login.success"))
            .returning(|_| {
                Ok(vec![
                    Webhook {
                        enabled: true,
                        events: vec!["login.success".to_string()],
                        ..Default::default()
                    },
                ])
            });

        let webhooks = mock.list_enabled_for_event("login.success").await.unwrap();
        assert_eq!(webhooks.len(), 1);
        assert!(webhooks[0].enabled);
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_create()
            .returning(|tenant_id, input| {
                Ok(Webhook {
                    tenant_id,
                    name: input.name.clone(),
                    url: input.url.clone(),
                    secret: input.secret.clone(),
                    events: input.events.clone(),
                    enabled: input.enabled,
                    ..Default::default()
                })
            });

        let input = CreateWebhookInput {
            name: "New Webhook".to_string(),
            url: "https://example.com/hook".to_string(),
            secret: Some("secret123".to_string()),
            events: vec!["user.created".to_string()],
            enabled: true,
        };

        let webhook = mock.create(tenant_id, &input).await.unwrap();
        assert_eq!(webhook.name, "New Webhook");
        assert_eq!(webhook.tenant_id, tenant_id);
    }

    #[tokio::test]
    async fn test_mock_update_triggered() {
        let mut mock = MockWebhookRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_update_triggered()
            .with(eq(id), eq(true))
            .returning(|_, _| Ok(()));

        let result = mock.update_triggered(id, true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mut mock = MockWebhookRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        let result = mock.delete(id).await;
        assert!(result.is_ok());
    }
}
