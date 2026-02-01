//! Webhook API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::{CreateWebhookInput, StringUuid, UpdateWebhookInput, Webhook};
use crate::error::AppError;
use crate::service::WebhookTestResult;
use crate::state::HasWebhooks;
use axum::{
    extract::{Path, State},
    Json,
};

/// List webhooks for a tenant
pub async fn list_webhooks<S: HasWebhooks>(
    State(state): State<S>,
    Path(tenant_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<Vec<Webhook>>>, AppError> {
    let webhooks = state.webhook_service().list_by_tenant(tenant_id).await?;
    Ok(Json(SuccessResponse::new(webhooks)))
}

/// Get a webhook by ID
pub async fn get_webhook<S: HasWebhooks>(
    State(state): State<S>,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    let webhook = state.webhook_service().get(webhook_id).await?;

    // Verify the webhook belongs to the tenant
    if webhook.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    Ok(Json(SuccessResponse::new(webhook)))
}

/// Create a new webhook
pub async fn create_webhook<S: HasWebhooks>(
    State(state): State<S>,
    Path(tenant_id): Path<StringUuid>,
    Json(input): Json<CreateWebhookInput>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    let webhook = state.webhook_service().create(tenant_id, input).await?;
    Ok(Json(SuccessResponse::new(webhook)))
}

/// Update a webhook
pub async fn update_webhook<S: HasWebhooks>(
    State(state): State<S>,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
    Json(input): Json<UpdateWebhookInput>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    let webhook = state.webhook_service().update(webhook_id, input).await?;
    Ok(Json(SuccessResponse::new(webhook)))
}

/// Delete a webhook
pub async fn delete_webhook<S: HasWebhooks>(
    State(state): State<S>,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<MessageResponse>, AppError> {
    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    state.webhook_service().delete(webhook_id).await?;
    Ok(Json(MessageResponse::new("Webhook deleted successfully.")))
}

/// Test a webhook by sending a test event
pub async fn test_webhook<S: HasWebhooks>(
    State(state): State<S>,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<WebhookTestResult>>, AppError> {
    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    let result = state.webhook_service().test(webhook_id).await?;
    Ok(Json(SuccessResponse::new(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_webhook_input_validation() {
        let input = CreateWebhookInput {
            name: "Test Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: Some("secret123".to_string()),
            events: vec!["login.success".to_string()],
            enabled: true,
        };

        assert_eq!(input.name, "Test Webhook");
        assert_eq!(input.events.len(), 1);
    }
}
