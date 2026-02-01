//! Webhook service for event notifications

use crate::domain::{CreateWebhookInput, StringUuid, UpdateWebhookInput, Webhook, WebhookEvent};
use crate::error::{AppError, Result};
use crate::repository::WebhookRepository;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use validator::Validate;

type HmacSha256 = Hmac<Sha256>;

/// Webhook service for managing and triggering webhooks
pub struct WebhookService<W: WebhookRepository> {
    webhook_repo: Arc<W>,
    http_client: reqwest::Client,
}

impl<W: WebhookRepository + 'static> WebhookService<W> {
    pub fn new(webhook_repo: Arc<W>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            webhook_repo,
            http_client,
        }
    }

    /// Create a new webhook
    pub async fn create(
        &self,
        tenant_id: StringUuid,
        input: CreateWebhookInput,
    ) -> Result<Webhook> {
        input.validate()?;
        self.webhook_repo.create(tenant_id, &input).await
    }

    /// Get a webhook by ID
    pub async fn get(&self, id: StringUuid) -> Result<Webhook> {
        self.webhook_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Webhook {} not found", id)))
    }

    /// List webhooks for a tenant
    pub async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Webhook>> {
        self.webhook_repo.list_by_tenant(tenant_id).await
    }

    /// Update a webhook
    pub async fn update(&self, id: StringUuid, input: UpdateWebhookInput) -> Result<Webhook> {
        input.validate()?;
        self.webhook_repo.update(id, &input).await
    }

    /// Delete a webhook
    pub async fn delete(&self, id: StringUuid) -> Result<()> {
        self.webhook_repo.delete(id).await
    }

    /// Trigger webhooks for an event
    ///
    /// This method finds all enabled webhooks subscribed to the event
    /// and sends the payload to each one asynchronously.
    pub async fn trigger_event(&self, event: WebhookEvent) -> Result<()> {
        let webhooks = self
            .webhook_repo
            .list_enabled_for_event(&event.event_type)
            .await?;

        for webhook in webhooks {
            // Clone what we need for the spawned task
            let http_client = self.http_client.clone();
            let webhook_repo = self.webhook_repo.clone();
            let event_clone = event.clone();
            let webhook_clone = webhook.clone();

            // Fire and forget - don't block on webhook delivery
            tokio::spawn(async move {
                let result =
                    deliver_webhook(&http_client, &webhook_clone, &event_clone).await;

                // Update the webhook status
                let success = result.is_ok();
                let _ = webhook_repo.update_triggered(webhook_clone.id, success).await;

                if let Err(e) = result {
                    tracing::warn!(
                        "Webhook delivery failed for {}: {}",
                        webhook_clone.id,
                        e
                    );
                }
            });
        }

        Ok(())
    }

    /// Test a webhook by sending a test event
    pub async fn test(&self, id: StringUuid) -> Result<WebhookTestResult> {
        let webhook = self.get(id).await?;

        let test_event = WebhookEvent {
            event_type: "test".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({
                "message": "This is a test webhook event",
                "webhook_id": id.to_string(),
            }),
        };

        match deliver_webhook(&self.http_client, &webhook, &test_event).await {
            Ok(response) => Ok(WebhookTestResult {
                success: true,
                status_code: Some(response.status_code),
                response_body: response.body,
                error: None,
            }),
            Err(e) => Ok(WebhookTestResult {
                success: false,
                status_code: None,
                response_body: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

/// Result of a webhook test
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebhookTestResult {
    pub success: bool,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub error: Option<String>,
}

/// Response from a webhook delivery
struct WebhookResponse {
    status_code: u16,
    body: Option<String>,
}

/// Deliver a webhook event to a specific webhook endpoint
async fn deliver_webhook(
    client: &reqwest::Client,
    webhook: &Webhook,
    event: &WebhookEvent,
) -> Result<WebhookResponse> {
    let payload = serde_json::to_string(event)
        .map_err(|e| AppError::Internal(e.into()))?;

    let mut request = client
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", &event.event_type)
        .header("X-Webhook-Timestamp", event.timestamp.to_rfc3339());

    // Add signature if secret is configured
    if let Some(secret) = &webhook.secret {
        let signature = compute_signature(&payload, secret)?;
        request = request.header("X-Webhook-Signature", signature);
    }

    // Send the request with a timeout
    let response = timeout(Duration::from_secs(30), request.body(payload).send())
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Webhook request timed out")))?
        .map_err(|e| AppError::Internal(e.into()))?;

    let status_code = response.status().as_u16();

    // Consider 2xx status codes as success
    if !response.status().is_success() {
        let body = response.text().await.ok();
        return Err(AppError::Internal(anyhow::anyhow!(
            "Webhook returned error status {}: {:?}",
            status_code,
            body
        )));
    }

    let body = response.text().await.ok();

    Ok(WebhookResponse { status_code, body })
}

/// Compute HMAC-SHA256 signature for webhook payload
fn compute_signature(payload: &str, secret: &str) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid HMAC key: {}", e)))?;

    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());

    Ok(format!("sha256={}", signature))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::webhook::MockWebhookRepository;
    use mockall::predicate::*;

    #[test]
    fn test_compute_signature() {
        let payload = r#"{"event_type":"test","data":{}}"#;
        let secret = "my-secret-key";

        let signature = compute_signature(payload, secret).unwrap();

        // Signature should start with sha256=
        assert!(signature.starts_with("sha256="));

        // Same payload and secret should produce same signature
        let signature2 = compute_signature(payload, secret).unwrap();
        assert_eq!(signature, signature2);

        // Different secret should produce different signature
        let signature3 = compute_signature(payload, "different-secret").unwrap();
        assert_ne!(signature, signature3);
    }

    #[tokio::test]
    async fn test_create_webhook() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_create()
            .returning(|tenant_id, input| {
                Ok(Webhook {
                    id: StringUuid::new_v4(),
                    tenant_id,
                    name: input.name.clone(),
                    url: input.url.clone(),
                    events: input.events.clone(),
                    enabled: input.enabled,
                    ..Default::default()
                })
            });

        let service = WebhookService::new(Arc::new(mock));

        let input = CreateWebhookInput {
            name: "Test Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: None,
            events: vec!["login.success".to_string()],
            enabled: true,
        };

        let webhook = service.create(tenant_id, input).await.unwrap();
        assert_eq!(webhook.name, "Test Webhook");
        assert_eq!(webhook.tenant_id, tenant_id);
    }

    #[tokio::test]
    async fn test_list_by_tenant() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| {
                Ok(vec![
                    Webhook {
                        name: "Webhook 1".to_string(),
                        ..Default::default()
                    },
                    Webhook {
                        name: "Webhook 2".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let service = WebhookService::new(Arc::new(mock));
        let webhooks = service.list_by_tenant(tenant_id).await.unwrap();

        assert_eq!(webhooks.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let mut mock = MockWebhookRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        let service = WebhookService::new(Arc::new(mock));
        let result = service.delete(id).await;

        assert!(result.is_ok());
    }
}
