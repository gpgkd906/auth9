//! Tenant API HTTP Handler Tests
//!
//! Tests for the tenant HTTP endpoints using mock repositories.

use crate::support::http::{
    build_test_router, delete_json_with_auth, get_json_with_auth, post_json_with_auth,
    put_json_with_auth, TestAppState,
};
use crate::support::{
    create_test_identity_token, create_test_jwt_manager, create_test_tenant,
    create_test_tenant_access_token, create_test_tenant_access_token_for_tenant,
};
use auth9_core::http_support::{MessageResponse, PaginatedResponse, SuccessResponse};
use auth9_core::models::system_settings::TenantMaliciousIpBlacklistEntry;
use auth9_core::models::tenant::{Tenant, TenantStatus};
use auth9_core::repository::MaliciousIpBlacklistRepository;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// List Tenants Tests
// ============================================================================

#[tokio::test]
async fn test_list_tenants_returns_200() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_tenant_access_token(); // Platform admin token

    // Add some test tenants
    for i in 1..=3 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        state.tenant_repo.add_tenant(tenant).await;
    }

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json_with_auth(&app, "/api/v1/tenants", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 3);
    assert_eq!(response.pagination.total, 3);
    assert_eq!(response.pagination.page, 1);
}

#[tokio::test]
async fn test_list_tenants_pagination() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_tenant_access_token();

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
        get_json_with_auth(&app, "/api/v1/tenants?page=2&per_page=10", &token).await;

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
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_tenant_access_token();
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<Tenant>>) =
        get_json_with_auth(&app, "/api/v1/tenants", &token).await;

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
    let state = TestAppState::new("http://localhost:8081");

    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    let mut tenant = create_test_tenant(Some(tenant_id));
    tenant.name = "Acme Corp".to_string();
    tenant.slug = "acme-corp".to_string();
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        get_json_with_auth(&app, &format!("/api/v1/tenants/{}", tenant_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Acme Corp");
    assert_eq!(response.data.slug, "acme-corp");
}

#[tokio::test]
async fn test_get_tenant_returns_404() {
    let state = TestAppState::new("http://localhost:8081");
    let nonexistent_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(nonexistent_id);
    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, &format!("/api/v1/tenants/{}", nonexistent_id), &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_create_tenant_returns_201() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token(); // Platform admin Identity token required
    let app = build_test_router(state);

    let input = json!({
        "name": "New Tenant",
        "slug": "new-tenant",
        "logo_url": "https://example.com/logo.png"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

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
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();

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
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

    // Should return 409 Conflict
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_tenant_validation_error_empty_name() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let input = json!({
        "name": "",  // empty name should fail validation
        "slug": "valid-slug"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

    // Should return 422 Unprocessable Entity for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_tenant_validation_error_empty_slug() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let input = json!({
        "name": "Valid Name",
        "slug": ""  // empty slug should fail validation
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_tenant_with_settings() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
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
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

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
    let state = TestAppState::new("http://localhost:8081");

    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    let mut tenant = create_test_tenant(Some(tenant_id));
    tenant.name = "Old Name".to_string();
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "name": "New Name"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "New Name");
}

#[tokio::test]
async fn test_update_tenant_returns_404() {
    let state = TestAppState::new("http://localhost:8081");
    let nonexistent_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(nonexistent_id);
    let app = build_test_router(state);

    let input = json!({
        "name": "New Name"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}", nonexistent_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_tenant_status() {
    let state = TestAppState::new("http://localhost:8081");

    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "status": "inactive"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, TenantStatus::Inactive);
}

#[tokio::test]
async fn test_update_tenant_logo_url() {
    let state = TestAppState::new("http://localhost:8081");

    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    let input = json!({
        "logo_url": "https://new-cdn.example.com/logo.png"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}", tenant_id),
        &input,
        &token,
    )
    .await;

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
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token(); // Platform admin Identity token required

    let tenant_id = Uuid::new_v4();
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state.clone());

    // Must include X-Confirm-Destructive header for delete
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/tenants/{}", tenant_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Confirm-Destructive", "true")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Option<MessageResponse> = serde_json::from_slice(&body_bytes).ok();

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));

    // Verify the tenant is physically deleted (not just disabled)
    let service = state.tenant_service.clone();
    let result = service
        .get(auth9_core::models::common::StringUuid::from(tenant_id))
        .await;
    assert!(result.is_err(), "Tenant should be physically deleted");
}

#[tokio::test]
async fn test_delete_tenant_returns_404() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    // Must include X-Confirm-Destructive header for delete
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/tenants/{}", nonexistent_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Confirm-Destructive", "true")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_tenant_requires_confirmation_header() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();

    let tenant_id = Uuid::new_v4();
    let tenant = create_test_tenant(Some(tenant_id));
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    // Without X-Confirm-Destructive header, should return 422
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json_with_auth(&app, &format!("/api/v1/tenants/{}", tenant_id), &token).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Edge Cases and Special Scenarios
// ============================================================================

#[tokio::test]
async fn test_tenant_with_unicode_name() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let input = json!({
        "name": "日本企業株式会社",
        "slug": "japan-corp"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "日本企業株式会社");
}

#[tokio::test]
async fn test_tenant_with_special_chars_in_name() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let input = json!({
        "name": "Acme & Co. (Holdings)",
        "slug": "acme-co"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Tenant>>) =
        post_json_with_auth(&app, "/api/v1/tenants", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Acme & Co. (Holdings)");
}

#[tokio::test]
async fn test_list_tenants_default_pagination() {
    let state = TestAppState::new("http://localhost:8081");
    let token = create_test_tenant_access_token();

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
        get_json_with_auth(&app, "/api/v1/tenants", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.page, 1);
    assert_eq!(response.pagination.per_page, 20);
}

#[tokio::test]
async fn test_get_tenant_malicious_ip_blacklist_returns_entries() {
    let state = TestAppState::new("http://localhost:8081");
    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    state
        .tenant_repo
        .add_tenant(create_test_tenant(Some(tenant_id)))
        .await;
    state
        .malicious_ip_blacklist_repo
        .add_tenant_entry(TenantMaliciousIpBlacklistEntry {
            id: auth9_core::models::common::StringUuid::new_v4(),
            tenant_id: tenant_id.into(),
            ip_address: "203.0.113.10".to_string(),
            reason: Some("tenant_only".to_string()),
            created_by: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await;
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(response["data"][0]["ip_address"], "203.0.113.10");
}

#[tokio::test]
async fn test_update_tenant_malicious_ip_blacklist_returns_200() {
    let state = TestAppState::new("http://localhost:8081");
    let tenant_id = Uuid::new_v4();
    let token = create_test_tenant_access_token_for_tenant(tenant_id);
    state
        .tenant_repo
        .add_tenant(create_test_tenant(Some(tenant_id)))
        .await;
    let app = build_test_router(state.clone());

    let input = json!({
        "entries": [
            { "ip_address": "203.0.113.10" },
            { "ip_address": "203.0.113.10" }
        ]
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist"),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(response["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        state
            .malicious_ip_blacklist_repo
            .list_by_tenant(tenant_id.into())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn test_update_tenant_malicious_ip_blacklist_denies_cross_tenant_access() {
    let state = TestAppState::new("http://localhost:8081");
    let tenant_id = Uuid::new_v4();
    let other_tenant_id = Uuid::new_v4();
    let token = create_test_jwt_manager()
        .create_tenant_access_token(
            Uuid::new_v4(),
            "member@test.com",
            other_tenant_id,
            "auth9-test-service",
            vec!["member".to_string()],
            vec![],
        )
        .unwrap();
    state
        .tenant_repo
        .add_tenant(create_test_tenant(Some(tenant_id)))
        .await;
    let app = build_test_router(state);

    let input = json!({
        "entries": [
            { "ip_address": "203.0.113.10" }
        ]
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist"),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
