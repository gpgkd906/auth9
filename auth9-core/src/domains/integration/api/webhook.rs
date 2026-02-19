//! Webhook API handlers

use crate::api::{write_audit_log_generic, MessageResponse, SuccessResponse};
use crate::domain::{CreateWebhookInput, StringUuid, UpdateWebhookInput, Webhook};
use crate::domains::integration::service::WebhookTestResult;
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasServices, HasWebhooks};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

/// List webhooks for a tenant
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/webhooks",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list_webhooks<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<Vec<Webhook>>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    let webhooks = state.webhook_service().list_by_tenant(tenant_id).await?;
    Ok(Json(SuccessResponse::new(webhooks)))
}

/// Get a webhook by ID
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("webhook_id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_webhook<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    let webhook = state.webhook_service().get(webhook_id).await?;

    // Verify the webhook belongs to the tenant
    if webhook.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    Ok(Json(SuccessResponse::new(webhook)))
}

/// Create a new webhook
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/webhooks",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn create_webhook<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
    Json(input): Json<CreateWebhookInput>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    let webhook = state.webhook_service().create(tenant_id, input).await?;
    Ok(Json(SuccessResponse::new(webhook)))
}

/// Update a webhook
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("webhook_id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_webhook<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
    Json(input): Json<UpdateWebhookInput>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    let webhook = state.webhook_service().update(webhook_id, input).await?;
    Ok(Json(SuccessResponse::new(webhook)))
}

/// Delete a webhook
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("webhook_id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn delete_webhook<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<MessageResponse>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    state.webhook_service().delete(webhook_id).await?;
    Ok(Json(MessageResponse::new("Webhook deleted successfully.")))
}

/// Regenerate a webhook's secret
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/regenerate-secret",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("webhook_id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn regenerate_webhook_secret<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<Webhook>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify the webhook belongs to the tenant
    let existing = state.webhook_service().get(webhook_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Webhook not found".to_string()));
    }

    let webhook = state
        .webhook_service()
        .regenerate_secret(webhook_id)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "webhook.regenerate_secret",
        "webhook",
        Some(*webhook_id),
        serde_json::to_value(existing.id.to_string()).ok(),
        None,
    )
    .await;

    Ok(Json(SuccessResponse::new(webhook)))
}

/// Test a webhook by sending a test event
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/test",
    tag = "Integration",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("webhook_id" = String, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn test_webhook<S: HasWebhooks + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, webhook_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<WebhookTestResult>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

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
