//! Tenant API handlers

use crate::api::{
    write_audit_log, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{CreateTenantInput, StringUuid, UpdateTenantInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

/// List tenants
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let (tenants, total) = state
        .tenant_service
        .list(pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        tenants,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Get tenant by ID
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.get(StringUuid::from(id)).await?;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Create tenant
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateTenantInput>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.create(input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.create",
        "tenant",
        Some(*tenant.id),
        None,
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant))))
}

/// Update tenant
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);
    let before = state.tenant_service.get(id).await?;
    let tenant = state.tenant_service.update(id, input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.update",
        "tenant",
        Some(*tenant.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Delete tenant
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);
    let before = state.tenant_service.get(id).await?;
    let tenant = state.tenant_service.disable(id).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.disable",
        "tenant",
        Some(*id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok(Json(MessageResponse::new("Tenant disabled successfully")))
}

#[cfg(test)]
mod tests {
    use crate::domain::{CreateTenantInput, UpdateTenantInput, TenantStatus, TenantSettings};

    #[test]
    fn test_create_tenant_input_deserialization() {
        let json = r#"{
            "name": "Acme Corp",
            "slug": "acme-corp",
            "logo_url": "https://example.com/logo.png"
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Acme Corp");
        assert_eq!(input.slug, "acme-corp");
        assert_eq!(input.logo_url, Some("https://example.com/logo.png".to_string()));
    }

    #[test]
    fn test_create_tenant_input_minimal() {
        let json = r#"{
            "name": "Test Tenant",
            "slug": "test-tenant"
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Test Tenant");
        assert_eq!(input.slug, "test-tenant");
        assert!(input.logo_url.is_none());
        assert!(input.settings.is_none());
    }

    #[test]
    fn test_create_tenant_input_with_settings() {
        let json = r#"{
            "name": "Enterprise",
            "slug": "enterprise",
            "settings": {
                "require_mfa": true,
                "session_timeout_secs": 1800
            }
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert!(input.settings.is_some());
        let settings = input.settings.unwrap();
        assert!(settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 1800);
    }

    #[test]
    fn test_update_tenant_input_partial() {
        let json = r#"{"name": "New Name"}"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("New Name".to_string()));
        assert!(input.logo_url.is_none());
        assert!(input.settings.is_none());
        assert!(input.status.is_none());
    }

    #[test]
    fn test_update_tenant_input_status_change() {
        let json = r#"{"status": "inactive"}"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.status, Some(TenantStatus::Inactive));
    }

    #[test]
    fn test_update_tenant_input_full() {
        let json = r#"{
            "name": "Updated Corp",
            "logo_url": "https://new.example.com/logo.png",
            "status": "active",
            "settings": {
                "require_mfa": false,
                "session_timeout_secs": 7200
            }
        }"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Updated Corp".to_string()));
        assert_eq!(input.logo_url, Some("https://new.example.com/logo.png".to_string()));
        assert_eq!(input.status, Some(TenantStatus::Active));
    }

    #[test]
    fn test_tenant_status_serialization() {
        assert_eq!(serde_json::to_string(&TenantStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&TenantStatus::Inactive).unwrap(), "\"inactive\"");
        assert_eq!(serde_json::to_string(&TenantStatus::Suspended).unwrap(), "\"suspended\"");
    }

    #[test]
    fn test_tenant_status_deserialization() {
        let active: TenantStatus = serde_json::from_str("\"active\"").unwrap();
        let inactive: TenantStatus = serde_json::from_str("\"inactive\"").unwrap();
        let suspended: TenantStatus = serde_json::from_str("\"suspended\"").unwrap();
        
        assert_eq!(active, TenantStatus::Active);
        assert_eq!(inactive, TenantStatus::Inactive);
        assert_eq!(suspended, TenantStatus::Suspended);
    }

    #[test]
    fn test_tenant_settings_defaults() {
        let settings = TenantSettings::default();
        assert!(!settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 3600);
        assert!(settings.allowed_auth_methods.is_empty());
    }

    #[test]
    fn test_tenant_settings_full() {
        let json = r#"{
            "require_mfa": true,
            "session_timeout_secs": 7200,
            "allowed_auth_methods": ["password", "sso"]
        }"#;
        let settings: TenantSettings = serde_json::from_str(json).unwrap();
        assert!(settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 7200);
        assert_eq!(settings.allowed_auth_methods, vec!["password".to_string(), "sso".to_string()]);
    }
}
