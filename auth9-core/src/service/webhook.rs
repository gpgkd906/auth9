//! Webhook service for event notifications

use crate::domain::{CreateWebhookInput, StringUuid, UpdateWebhookInput, Webhook, WebhookEvent};
use crate::error::{AppError, Result};
use crate::repository::WebhookRepository;
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use validator::Validate;

type HmacSha256 = Hmac<Sha256>;

/// Trait for publishing webhook events.
///
/// This trait allows services to trigger webhook events without depending
/// on the concrete WebhookService type.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebhookEventPublisher: Send + Sync {
    /// Trigger webhooks for an event.
    ///
    /// This method finds all enabled webhooks subscribed to the event
    /// and sends the payload to each one asynchronously.
    async fn trigger_event(&self, event: WebhookEvent) -> Result<()>;
}

/// Generate a random webhook secret
fn generate_webhook_secret() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    format!("whsec_{}", hex::encode(bytes))
}

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
    ///
    /// If no secret is provided, a random secret will be auto-generated.
    pub async fn create(
        &self,
        tenant_id: StringUuid,
        mut input: CreateWebhookInput,
    ) -> Result<Webhook> {
        input.validate()?;

        // Auto-generate secret if not provided
        if input.secret.is_none() {
            input.secret = Some(generate_webhook_secret());
        }

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

        let start = Instant::now();
        match deliver_webhook_with_status(&self.http_client, &webhook, &test_event).await {
            Ok(response) => Ok(WebhookTestResult {
                success: true,
                status_code: Some(response.status_code),
                response_body: response.body,
                error: None,
                response_time_ms: Some(start.elapsed().as_millis() as u64),
            }),
            Err((status_code, error_msg)) => Ok(WebhookTestResult {
                success: false,
                status_code,
                response_body: None,
                error: Some(error_msg),
                response_time_ms: Some(start.elapsed().as_millis() as u64),
            }),
        }
    }

    /// Regenerate webhook secret
    pub async fn regenerate_secret(&self, id: StringUuid) -> Result<Webhook> {
        let new_secret = generate_webhook_secret();
        self.webhook_repo
            .update(
                id,
                &UpdateWebhookInput {
                    secret: Some(new_secret),
                    ..Default::default()
                },
            )
            .await
    }
}

#[async_trait]
impl<W: WebhookRepository + 'static> WebhookEventPublisher for WebhookService<W> {
    async fn trigger_event(&self, event: WebhookEvent) -> Result<()> {
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
                let result = deliver_webhook(&http_client, &webhook_clone, &event_clone).await;

                // Update the webhook status
                let success = result.is_ok();
                let _ = webhook_repo
                    .update_triggered(webhook_clone.id, success)
                    .await;

                if let Err(e) = result {
                    tracing::warn!("Webhook delivery failed for {}: {}", webhook_clone.id, e);
                }
            });
        }

        Ok(())
    }
}

/// Result of a webhook test
#[derive(Debug, Clone, serde::Serialize)]
pub struct WebhookTestResult {
    pub success: bool,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub response_time_ms: Option<u64>,
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
    match deliver_webhook_with_status(client, webhook, event).await {
        Ok(response) => Ok(response),
        Err((_, msg)) => Err(AppError::Internal(anyhow::anyhow!("{}", msg))),
    }
}

/// Deliver a webhook and return status code even on error
async fn deliver_webhook_with_status(
    client: &reqwest::Client,
    webhook: &Webhook,
    event: &WebhookEvent,
) -> std::result::Result<WebhookResponse, (Option<u16>, String)> {
    let payload = serde_json::to_string(event).map_err(|e| (None, e.to_string()))?;

    let mut request = client
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", &event.event_type)
        .header("X-Webhook-Timestamp", event.timestamp.to_rfc3339());

    // Add signature if secret is configured
    if let Some(secret) = &webhook.secret {
        let signature = compute_signature(&payload, secret).map_err(|e| (None, e.to_string()))?;
        request = request.header("X-Webhook-Signature", signature);
    }

    // Send the request with a timeout
    let response = timeout(Duration::from_secs(30), request.body(payload).send())
        .await
        .map_err(|_| (None, "Webhook request timed out".to_string()))?
        .map_err(|e| (None, e.to_string()))?;

    let status_code = response.status().as_u16();

    // Consider 2xx status codes as success
    if !response.status().is_success() {
        let body = response.text().await.ok();
        return Err((
            Some(status_code),
            format!("Webhook returned error status {}: {:?}", status_code, body),
        ));
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
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

        mock.expect_create().returning(|tenant_id, input| {
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
    async fn test_create_webhook_validation_error() {
        let mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        let service = WebhookService::new(Arc::new(mock));

        // Invalid URL
        let input = CreateWebhookInput {
            name: "Test Webhook".to_string(),
            url: "not-a-valid-url".to_string(),
            secret: None,
            events: vec!["login.success".to_string()],
            enabled: true,
        };

        let result = service.create(tenant_id, input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_webhook_success() {
        let mut mock = MockWebhookRepository::new();
        let webhook_id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(webhook_id))
            .returning(|id| {
                Ok(Some(Webhook {
                    id,
                    name: "Found Webhook".to_string(),
                    ..Default::default()
                }))
            });

        let service = WebhookService::new(Arc::new(mock));
        let webhook = service.get(webhook_id).await.unwrap();

        assert_eq!(webhook.id, webhook_id);
        assert_eq!(webhook.name, "Found Webhook");
    }

    #[tokio::test]
    async fn test_get_webhook_not_found() {
        let mut mock = MockWebhookRepository::new();
        let webhook_id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(webhook_id))
            .returning(|_| Ok(None));

        let service = WebhookService::new(Arc::new(mock));
        let result = service.get(webhook_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected NotFound error"),
        }
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
    async fn test_list_by_tenant_empty() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| Ok(vec![]));

        let service = WebhookService::new(Arc::new(mock));
        let webhooks = service.list_by_tenant(tenant_id).await.unwrap();

        assert!(webhooks.is_empty());
    }

    #[tokio::test]
    async fn test_update_webhook() {
        let mut mock = MockWebhookRepository::new();
        let webhook_id = StringUuid::new_v4();

        mock.expect_update().returning(|id, input| {
            Ok(Webhook {
                id,
                name: input.name.clone().unwrap_or_default(),
                url: input.url.clone().unwrap_or_default(),
                enabled: input.enabled.unwrap_or(true),
                ..Default::default()
            })
        });

        let service = WebhookService::new(Arc::new(mock));

        let input = UpdateWebhookInput {
            name: Some("Updated Webhook".to_string()),
            url: Some("https://example.com/updated".to_string()),
            secret: None,
            events: None,
            enabled: Some(false),
        };

        let webhook = service.update(webhook_id, input).await.unwrap();
        assert_eq!(webhook.name, "Updated Webhook");
        assert!(!webhook.enabled);
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let mut mock = MockWebhookRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let service = WebhookService::new(Arc::new(mock));
        let result = service.delete(id).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_event_success() {
        // Start a mock HTTP server
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .and(header("Content-Type", "application/json"))
            .and(header("X-Webhook-Event", "login.success"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        let mut mock = MockWebhookRepository::new();
        mock.expect_list_enabled_for_event().returning({
            let url = format!("{}/webhook", mock_server.uri());
            move |_| {
                Ok(vec![Webhook {
                    id: webhook_id,
                    tenant_id,
                    name: "Test Webhook".to_string(),
                    url: url.clone(),
                    events: vec!["login.success".to_string()],
                    enabled: true,
                    secret: None,
                    ..Default::default()
                }])
            }
        });
        mock.expect_update_triggered().returning(|_, _| Ok(()));

        let service = WebhookService::new(Arc::new(mock));

        let event = WebhookEvent {
            event_type: "login.success".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({"user_id": "user-123"}),
        };

        let result = service.trigger_event(event).await;
        assert!(result.is_ok());

        // Wait a bit for the spawned task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_trigger_event_no_webhooks() {
        let mut mock = MockWebhookRepository::new();
        mock.expect_list_enabled_for_event()
            .returning(|_| Ok(vec![]));

        let service = WebhookService::new(Arc::new(mock));

        let event = WebhookEvent {
            event_type: "login.success".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({"user_id": "user-123"}),
        };

        let result = service.trigger_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_trigger_event_with_signature() {
        let mock_server = MockServer::start().await;

        // Verify that signature header is present
        Mock::given(method("POST"))
            .and(path("/webhook"))
            .and(header("Content-Type", "application/json"))
            .and(header("X-Webhook-Event", "user.created"))
            .and(wiremock::matchers::header_exists("X-Webhook-Signature"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        let mut mock = MockWebhookRepository::new();
        mock.expect_list_enabled_for_event().returning({
            let url = format!("{}/webhook", mock_server.uri());
            move |_| {
                Ok(vec![Webhook {
                    id: webhook_id,
                    tenant_id,
                    name: "Signed Webhook".to_string(),
                    url: url.clone(),
                    events: vec!["user.created".to_string()],
                    enabled: true,
                    secret: Some("webhook-secret".to_string()),
                    ..Default::default()
                }])
            }
        });
        mock.expect_update_triggered().returning(|_, _| Ok(()));

        let service = WebhookService::new(Arc::new(mock));

        let event = WebhookEvent {
            event_type: "user.created".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({"user_id": "user-456"}),
        };

        let result = service.trigger_event(event).await;
        assert!(result.is_ok());

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_test_webhook_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .and(header("X-Webhook-Event", "test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Success"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_id = StringUuid::new_v4();

        let mut mock = MockWebhookRepository::new();
        mock.expect_find_by_id().returning({
            let url = format!("{}/webhook", mock_server.uri());
            move |id| {
                Ok(Some(Webhook {
                    id,
                    name: "Test Webhook".to_string(),
                    url: url.clone(),
                    enabled: true,
                    secret: None,
                    ..Default::default()
                }))
            }
        });

        let service = WebhookService::new(Arc::new(mock));
        let result = service.test(webhook_id).await.unwrap();

        assert!(result.success);
        assert_eq!(result.status_code, Some(200));
        assert_eq!(result.response_body, Some("Success".to_string()));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_test_webhook_failure_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let webhook_id = StringUuid::new_v4();

        let mut mock = MockWebhookRepository::new();
        mock.expect_find_by_id().returning({
            let url = format!("{}/webhook", mock_server.uri());
            move |id| {
                Ok(Some(Webhook {
                    id,
                    name: "Test Webhook".to_string(),
                    url: url.clone(),
                    enabled: true,
                    secret: None,
                    ..Default::default()
                }))
            }
        });

        let service = WebhookService::new(Arc::new(mock));
        let result = service.test(webhook_id).await.unwrap();

        assert!(!result.success);
        // Now status_code should be returned even on error
        assert_eq!(result.status_code, Some(500));
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("500"));
        // response_time_ms should be present
        assert!(result.response_time_ms.is_some());
    }

    #[tokio::test]
    async fn test_test_webhook_not_found() {
        let mut mock = MockWebhookRepository::new();
        let webhook_id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(webhook_id))
            .returning(|_| Ok(None));

        let service = WebhookService::new(Arc::new(mock));
        let result = service.test(webhook_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_webhook_test_result_serialization() {
        let result = WebhookTestResult {
            success: true,
            status_code: Some(200),
            response_body: Some("ok".to_string()),
            error: None,
            response_time_ms: Some(42),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"status_code\":200"));
        assert!(json.contains("\"response_time_ms\":42"));
    }

    #[test]
    fn test_generate_webhook_secret() {
        let secret = generate_webhook_secret();
        assert!(secret.starts_with("whsec_"));
        // 32 bytes = 64 hex chars + "whsec_" prefix
        assert_eq!(secret.len(), 6 + 64);

        // Two secrets should be different
        let secret2 = generate_webhook_secret();
        assert_ne!(secret, secret2);
    }

    #[tokio::test]
    async fn test_create_webhook_auto_generates_secret() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_create().returning(|tenant_id, input| {
            // Verify secret was auto-generated
            assert!(input.secret.is_some());
            let secret = input.secret.as_ref().unwrap();
            assert!(secret.starts_with("whsec_"));

            Ok(Webhook {
                id: StringUuid::new_v4(),
                tenant_id,
                name: input.name.clone(),
                url: input.url.clone(),
                secret: input.secret.clone(),
                events: input.events.clone(),
                enabled: input.enabled,
                ..Default::default()
            })
        });

        let service = WebhookService::new(Arc::new(mock));

        let input = CreateWebhookInput {
            name: "Test Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: None, // No secret provided
            events: vec!["login.success".to_string()],
            enabled: true,
        };

        let webhook = service.create(tenant_id, input).await.unwrap();
        assert!(webhook.secret.is_some());
        assert!(webhook.secret.unwrap().starts_with("whsec_"));
    }

    #[tokio::test]
    async fn test_create_webhook_preserves_user_secret() {
        let mut mock = MockWebhookRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_create().returning(|tenant_id, input| {
            Ok(Webhook {
                id: StringUuid::new_v4(),
                tenant_id,
                name: input.name.clone(),
                url: input.url.clone(),
                secret: input.secret.clone(),
                events: input.events.clone(),
                enabled: input.enabled,
                ..Default::default()
            })
        });

        let service = WebhookService::new(Arc::new(mock));

        let input = CreateWebhookInput {
            name: "Test Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: Some("user-provided-secret".to_string()),
            events: vec!["login.success".to_string()],
            enabled: true,
        };

        let webhook = service.create(tenant_id, input).await.unwrap();
        assert_eq!(webhook.secret, Some("user-provided-secret".to_string()));
    }

    #[test]
    fn test_signature_format() {
        let signature = compute_signature("test payload", "secret").unwrap();

        // Verify format: sha256=<hex>
        assert!(signature.starts_with("sha256="));
        let hex_part = &signature[7..];
        // SHA256 produces 32 bytes = 64 hex characters
        assert_eq!(hex_part.len(), 64);
        // All characters should be valid hex
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
