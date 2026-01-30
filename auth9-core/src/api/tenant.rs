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
    use crate::api::{MessageResponse, PaginatedResponse, SuccessResponse};
    use crate::domain::{CreateTenantInput, Tenant, TenantSettings, TenantStatus, UpdateTenantInput};

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
        assert_eq!(
            input.logo_url,
            Some("https://example.com/logo.png".to_string())
        );
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
    fn test_create_tenant_input_missing_name() {
        let json = r#"{
            "slug": "test-tenant"
        }"#;
        let result: serde_json::Result<CreateTenantInput> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tenant_input_missing_slug() {
        let json = r#"{
            "name": "Test Tenant"
        }"#;
        let result: serde_json::Result<CreateTenantInput> = serde_json::from_str(json);
        assert!(result.is_err());
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
    fn test_create_tenant_input_with_full_settings() {
        let json = r#"{
            "name": "Full Settings Corp",
            "slug": "full-settings",
            "logo_url": "https://example.com/logo.png",
            "settings": {
                "require_mfa": true,
                "session_timeout_secs": 3600,
                "allowed_auth_methods": ["password", "sso", "magic_link"]
            }
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert!(input.settings.is_some());
        let settings = input.settings.unwrap();
        assert_eq!(settings.allowed_auth_methods.len(), 3);
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
    fn test_update_tenant_input_empty() {
        let json = r#"{}"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
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
    fn test_update_tenant_input_status_suspended() {
        let json = r#"{"status": "suspended"}"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.status, Some(TenantStatus::Suspended));
    }

    #[test]
    fn test_update_tenant_input_invalid_status() {
        let json = r#"{"status": "invalid_status"}"#;
        let result: serde_json::Result<UpdateTenantInput> = serde_json::from_str(json);
        assert!(result.is_err());
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
        assert_eq!(
            input.logo_url,
            Some("https://new.example.com/logo.png".to_string())
        );
        assert_eq!(input.status, Some(TenantStatus::Active));
    }

    #[test]
    fn test_update_tenant_input_logo_only() {
        let json = r#"{"logo_url": "https://cdn.example.com/new-logo.png"}"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
        assert_eq!(
            input.logo_url,
            Some("https://cdn.example.com/new-logo.png".to_string())
        );
    }

    #[test]
    fn test_update_tenant_input_settings_only() {
        let json = r#"{
            "settings": {
                "require_mfa": true,
                "session_timeout_secs": 1800
            }
        }"#;
        let input: UpdateTenantInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
        assert!(input.settings.is_some());
        let settings = input.settings.unwrap();
        assert!(settings.require_mfa);
    }

    #[test]
    fn test_tenant_status_serialization() {
        assert_eq!(
            serde_json::to_string(&TenantStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&TenantStatus::Inactive).unwrap(),
            "\"inactive\""
        );
        assert_eq!(
            serde_json::to_string(&TenantStatus::Suspended).unwrap(),
            "\"suspended\""
        );
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
        assert_eq!(
            settings.allowed_auth_methods,
            vec!["password".to_string(), "sso".to_string()]
        );
    }

    #[test]
    fn test_tenant_settings_partial() {
        let json = r#"{
            "require_mfa": true
        }"#;
        let settings: TenantSettings = serde_json::from_str(json).unwrap();
        assert!(settings.require_mfa);
        // Uses default for unspecified fields
        assert_eq!(settings.session_timeout_secs, 3600);
    }

    #[test]
    fn test_tenant_settings_empty_auth_methods() {
        let json = r#"{
            "require_mfa": false,
            "session_timeout_secs": 3600,
            "allowed_auth_methods": []
        }"#;
        let settings: TenantSettings = serde_json::from_str(json).unwrap();
        assert!(settings.allowed_auth_methods.is_empty());
    }

    #[test]
    fn test_success_response_with_tenant() {
        let tenant = Tenant::default();
        let response = SuccessResponse::new(tenant);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("slug"));
    }

    #[test]
    fn test_paginated_response_with_tenants() {
        let tenants = vec![Tenant::default(), Tenant::default()];
        let response = PaginatedResponse::new(tenants, 1, 10, 2);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("pagination"));
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"total_pages\":1"));
    }

    #[test]
    fn test_paginated_response_empty() {
        let tenants: Vec<Tenant> = vec![];
        let response = PaginatedResponse::new(tenants, 1, 10, 0);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn test_message_response_tenant_disabled() {
        let response = MessageResponse::new("Tenant disabled successfully");
        assert_eq!(response.message, "Tenant disabled successfully");
    }

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse::new("Operation completed");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("Operation completed"));
    }

    #[test]
    fn test_create_tenant_input_special_chars_in_name() {
        let json = r#"{
            "name": "Acme & Co.",
            "slug": "acme-co"
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Acme & Co.");
    }

    #[test]
    fn test_create_tenant_input_unicode_name() {
        let json = r#"{
            "name": "日本企業",
            "slug": "japan-corp"
        }"#;
        let input: CreateTenantInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "日本企業");
    }
}
