//! Tenant-Service toggle API handlers

use crate::api::SuccessResponse;
use crate::domain::{ServiceWithStatus, StringUuid, ToggleServiceInput};
use crate::error::{AppError, Result};
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasDbPool, HasServices};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/services",
    tag = "Authorization",
    responses(
        (status = 200, description = "List of services with status")
    )
)]
/// List all global services with their enabled status for a tenant
pub async fn list_services<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);

    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::TenantServiceRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify tenant exists
    state.tenant_service().get(tenant_id).await?;

    // Get db pool from state
    let pool = state.db_pool();

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
    .fetch_all(pool)
    .await?;

    Ok(Json(SuccessResponse::new(services)))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/services",
    tag = "Authorization",
    responses(
        (status = 200, description = "Service toggled")
    )
)]
/// Toggle a service for a tenant (enable/disable)
pub async fn toggle_service<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<ToggleServiceInput>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);
    let service_id = StringUuid::from(input.service_id);

    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::TenantServiceWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify tenant exists
    state.tenant_service().get(tenant_id).await?;

    // Get db pool from state
    let pool = state.db_pool();

    // Verify service exists and is global (tenant_id IS NULL)
    let service_exists: Option<(bool,)> =
        sqlx::query_as("SELECT TRUE FROM services WHERE id = ? AND tenant_id IS NULL")
            .bind(service_id)
            .fetch_optional(pool)
            .await?;

    if service_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Global service {} not found",
            input.service_id
        )));
    }

    // Toggle the service
    sqlx::query(
        r#"
        INSERT INTO tenant_services (tenant_id, service_id, enabled, created_at, updated_at)
        VALUES (?, ?, ?, NOW(), NOW())
        ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), updated_at = NOW()
        "#,
    )
    .bind(tenant_id)
    .bind(service_id)
    .bind(input.enabled)
    .execute(pool)
    .await?;

    // Return updated list
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
    .fetch_all(pool)
    .await?;

    Ok(Json(SuccessResponse::new(services)))
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/services/enabled",
    tag = "Authorization",
    responses(
        (status = 200, description = "Enabled services for tenant")
    )
)]
/// Get enabled services for a tenant (used by invitation)
pub async fn get_enabled_services<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);

    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::TenantServiceRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    // Verify tenant exists
    state.tenant_service().get(tenant_id).await?;

    // Get db pool from state
    let pool = state.db_pool();

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
    .fetch_all(pool)
    .await?;

    Ok(Json(SuccessResponse::new(services)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_service_input_deserialize() {
        let json = r#"{"service_id": "550e8400-e29b-41d4-a716-446655440000", "enabled": true}"#;
        let input: ToggleServiceInput = serde_json::from_str(json).unwrap();
        assert!(input.enabled);
    }

    #[test]
    fn test_toggle_service_input_disable() {
        let json = r#"{"service_id": "550e8400-e29b-41d4-a716-446655440000", "enabled": false}"#;
        let input: ToggleServiceInput = serde_json::from_str(json).unwrap();
        assert!(!input.enabled);
    }
}
