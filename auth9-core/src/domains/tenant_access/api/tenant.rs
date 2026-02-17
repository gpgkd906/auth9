//! Tenant API handlers

use crate::api::{
    deserialize_page, deserialize_per_page, extract_actor_id_generic, extract_ip,
    is_platform_admin_with_db, require_platform_admin_with_db, write_audit_log_generic,
    MessageResponse, PaginatedResponse, SuccessResponse,
};
use crate::config::Config;
use crate::domain::{CreateTenantInput, StringUuid, UpdateTenantInput};
use crate::error::{AppError, Result};
use crate::middleware::auth::{AuthUser, TokenType};
use crate::repository::audit::CreateAuditLogInput;
use crate::repository::AuditRepository;
use crate::state::HasServices;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

/// Check if user has access to a specific tenant, logging access_denied events to audit log
async fn check_tenant_access<S: HasServices>(
    state: &S,
    headers: &HeaderMap,
    auth: &AuthUser,
    tenant_id: Uuid,
) -> Result<()> {
    let result = check_tenant_access_inner(state.config(), auth, tenant_id);
    if let Err(AppError::Forbidden(ref reason)) = result {
        let actor_id = extract_actor_id_generic(state, headers);
        let ip_address = extract_ip(headers);
        let _ = state
            .audit_repo()
            .create(&CreateAuditLogInput {
                actor_id,
                action: "access_denied".to_string(),
                resource_type: "tenant".to_string(),
                resource_id: Some(tenant_id),
                old_value: None,
                new_value: serde_json::to_value(serde_json::json!({
                    "reason": reason,
                    "actor_email": &auth.email,
                }))
                .ok(),
                ip_address,
            })
            .await;
    }
    result
}

fn check_tenant_access_inner(config: &Config, auth: &AuthUser, tenant_id: Uuid) -> Result<()> {
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Tenant-scoped token required (exchange identity token first)".to_string(),
                ))
            }
        }
        TokenType::TenantAccess => {
            // Tenant access tokens must match the tenant_id
            if auth.tenant_id == Some(tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: you don't have permission to access this tenant".to_string(),
                ))
            }
        }
        TokenType::ServiceClient => {
            // Service client tokens can only access their own tenant
            if auth.tenant_id == Some(tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Service client tokens can only access their own tenant".to_string(),
                ))
            }
        }
    }
}

/// Query parameters for tenant list endpoint with search
#[derive(Debug, Deserialize)]
pub struct TenantListQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
    pub search: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// List tenants
/// - Platform admin (Identity token with config email or platform tenant admin): can list all tenants
/// - Non-admin Identity token: can see tenants they belong to
/// - Tenant user (TenantAccess token): can only see their own tenant
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(query): Query<TenantListQuery>,
) -> Result<impl IntoResponse> {
    match auth.token_type {
        TokenType::Identity => {
            let is_platform_admin = is_platform_admin_with_db(&state, &auth).await;

            if is_platform_admin {
                // Platform admin: can list all tenants
                let (tenants, total) = if let Some(ref search) = query.search {
                    state
                        .tenant_service()
                        .search(search, query.page, query.per_page)
                        .await?
                } else {
                    state
                        .tenant_service()
                        .list(query.page, query.per_page)
                        .await?
                };

                Ok(Json(PaginatedResponse::new(
                    tenants,
                    query.page,
                    query.per_page,
                    total,
                )))
            } else {
                // Non-admin Identity token: show tenants they belong to
                let user_tenants = state
                    .user_service()
                    .get_user_tenants(StringUuid::from(auth.user_id))
                    .await?;
                let mut tenants = Vec::new();
                for tu in &user_tenants {
                    if let Ok(tenant) = state.tenant_service().get(tu.tenant_id).await {
                        tenants.push(tenant);
                    }
                }
                let total = tenants.len() as i64;
                Ok(Json(PaginatedResponse::new(tenants, 1, total, total)))
            }
        }
        TokenType::TenantAccess | TokenType::ServiceClient => {
            // Tenant user / service client: can only see their own tenant
            let tenant_id = auth
                .tenant_id
                .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
            let tenant = state
                .tenant_service()
                .get(StringUuid::from(tenant_id))
                .await?;
            Ok(Json(PaginatedResponse::new(vec![tenant], 1, 1, 1)))
        }
    }
}

/// Get tenant by ID
/// Verifies the user has access to this tenant
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check tenant access before returning data
    check_tenant_access(&state, &headers, &auth, id).await?;

    let tenant = state.tenant_service().get(StringUuid::from(id)).await?;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Create tenant
/// Only platform admins (Identity token holders) can create tenants
pub async fn create<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<CreateTenantInput>,
) -> Result<impl IntoResponse> {
    // Only platform admins can create tenants
    require_platform_admin_with_db(&state, &auth).await?;

    let tenant = state.tenant_service().create(input).await?;
    let _ = write_audit_log_generic(
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
/// Verifies the user has access to this tenant
pub async fn update<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<impl IntoResponse> {
    // Check tenant access before updating
    check_tenant_access(&state, &headers, &auth, id).await?;

    let id = StringUuid::from(id);
    let before = state.tenant_service().get(id).await?;
    let tenant = state.tenant_service().update(id, input).await?;
    let _ = write_audit_log_generic(
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
/// Only platform admins can delete tenants
/// Requires `X-Confirm-Destructive: true` header to prevent accidental deletion
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Only platform admins can delete tenants
    require_platform_admin_with_db(&state, &auth).await?;

    // Require explicit confirmation header for destructive operation
    let confirmed = headers
        .get("X-Confirm-Destructive")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !confirmed {
        return Err(crate::error::AppError::Validation(
            "Destructive operation requires X-Confirm-Destructive: true header".to_string(),
        ));
    }

    let id = StringUuid::from(id);
    let before = state.tenant_service().get(id).await?;

    // Perform physical delete with cascade cleanup
    state.tenant_service().delete(id).await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.delete",
        "tenant",
        Some(*id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Tenant deleted successfully")))
}

#[cfg(test)]
mod tests {
    use crate::api::{MessageResponse, PaginatedResponse, SuccessResponse};
    use crate::domain::{
        CreateTenantInput, Tenant, TenantSettings, TenantStatus, UpdateTenantInput,
    };

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
