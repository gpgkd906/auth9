//! Tenant API HTTP Handler Tests
//!
//! Tests for the tenant HTTP endpoints using mock repositories.

use super::{build_test_router, delete_json, get_json, post_json, put_json, TestAppState};
use crate::api::{create_test_tenant, MockKeycloakServer};
use auth9_core::api::{MessageResponse, PaginatedResponse, SuccessResponse};
use auth9_core::domain::{Tenant, TenantStatus};
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// List Tenants Tests
// ============================================================================

#[tokio::test]
async fn test_list_tenants_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add some test tenants
    for i in 1..=3 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        state.tenant_repo.add_tenant(tenant).await;
    }

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json(&app, "/api/v1/tenants").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 3);
    assert_eq!(response.pagination.total, 3);
    assert_eq!(response.pagination.page, 1);
}

#[tokio::test]
async fn test_list_tenants_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add 25 tenants
    for i in 1..=25 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        state.tenant_repo.add_tenant(tenant).await;
    }

    let app = build_test_router(state);

    // Request page 2 with per_page=10
    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json(&app, "/api/v1/tenants?page=2&per_page=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.total, 25);
    assert_eq!(response.pagination.page, 2);
    assert_eq!(response.pagination.per_page, 10);
    assert_eq!(response.pagination.total_pages, 3);
}

#[tokio::test]
async fn test_list_tenants_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json(&app, "/api/v1/tenants").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.is_empty());
    assert_eq!(response.pagination.total, 0);
}

// ============================================================================
// Get Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_get_tenant_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let mut tenant = create_test_tenant(Some(tenant_id));
    tenant.name = "Acme Corp".to_string();
    tenant.slug = "acme-corp".to_string();
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        get_json(&app, &format!("/api/v1/tenants/{}", tenant_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Acme Corp");
    assert_eq!(response.data.slug, "acme-corp");
}

#[tokio::test]
async fn test_get_tenant_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, &format!("/api/v1/tenants/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_create_tenant_returns_201() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "name": "New Tenant",
        "slug": "new-tenant",
        "logo_url": "https://example.com/logo.png"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "New Tenant");
    assert_eq!(response.data.slug, "new-tenant");
    assert_eq!(
        response.data.logo_url,
        Some("https://example.com/logo.png".to_string())
    );
    assert_eq!(response.data.status, TenantStatus::Active);
}

#[tokio::test]
async fn test_create_tenant_duplicate_slug() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add existing tenant with slug "existing-tenant"
    let mut existing = create_test_tenant(None);
    existing.slug = "existing-tenant".to_string();
    state.tenant_repo.add_tenant(existing).await;

    let app = build_test_router(state);

    let input = json!({
        "name": "Another Tenant",
        "slug": "existing-tenant"  // duplicate slug
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    // Should return 409 Conflict
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_tenant_validation_error_empty_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "name": "",  // empty name should fail validation
        "slug": "valid-slug"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    // Should return 422 Unprocessable Entity for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_tenant_validation_error_empty_slug() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "name": "Valid Name",
        "slug": ""  // empty slug should fail validation
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_tenant_with_settings() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = serde_json::json!({
        "name": "Enterprise Tenant",
        "slug": "enterprise",
        "settings": {
            "require_mfa": true,
            "session_timeout_secs": 1800,
            "allowed_auth_methods": ["password", "sso"]
        }
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.settings.require_mfa);
    assert_eq!(response.data.settings.session_timeout_secs, 1800);
}

// ============================================================================
// Update Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_update_tenant_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let mut tenant = create_test_tenant(Some(tenant_id));
    tenant.name = "Old Name".to_string();
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "name": "New Name"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        put_json(&app, &format!("/api/v1/tenants/{}", tenant_id), &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "New Name");
}

#[tokio::test]
async fn test_update_tenant_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let input = json!({
        "name": "New Name"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json(&app, &format!("/api/v1/tenants/{}", nonexistent_id), &input).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_tenant_status() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "status": "inactive"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        put_json(&app, &format!("/api/v1/tenants/{}", tenant_id), &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, TenantStatus::Inactive);
}

#[tokio::test]
async fn test_update_tenant_logo_url() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "logo_url": "https://new-cdn.example.com/logo.png"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        put_json(&app, &format!("/api/v1/tenants/{}", tenant_id), &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(
        response.data.logo_url,
        Some("https://new-cdn.example.com/logo.png".to_string())
    );
}

// ============================================================================
// Delete Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_delete_tenant_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state.clone());

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, &format!("/api/v1/tenants/{}", tenant_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));

    // Verify the tenant is physically deleted (not just disabled)
    let service = state.tenant_service.clone();
    let result = service
        .get(auth9_core::domain::StringUuid::from(tenant_id))
        .await;
    assert!(result.is_err(), "Tenant should be physically deleted");
}

#[tokio::test]
async fn test_delete_tenant_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json(&app, &format!("/api/v1/tenants/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Edge Cases and Special Scenarios
// ============================================================================

#[tokio::test]
async fn test_tenant_with_unicode_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "name": "日本企業株式会社",
        "slug": "japan-corp"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "日本企業株式会社");
}

#[tokio::test]
async fn test_tenant_with_special_chars_in_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "name": "Acme & Co. (Holdings)",
        "slug": "acme-co"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json(&app, "/api/v1/tenants", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Acme & Co. (Holdings)");
}

#[tokio::test]
async fn test_list_tenants_default_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add 5 tenants
    for i in 1..=5 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        state.tenant_repo.add_tenant(tenant).await;
    }

    let app = build_test_router(state);

    // No pagination params - should use defaults (page=1, per_page=20)
    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json(&app, "/api/v1/tenants").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.page, 1);
    assert_eq!(response.pagination.per_page, 20);
}
