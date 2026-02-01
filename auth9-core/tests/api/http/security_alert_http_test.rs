//! Security Alert HTTP API handler tests
//!
//! Tests for security alert listing, retrieval, and resolution endpoints.

use super::{get_json, get_json_with_auth, MockKeycloakServer, TestAppState};
use crate::api::{create_test_jwt_manager, create_test_user};
use auth9_core::api::security_alert::UnresolvedCountResponse;
use auth9_core::api::{PaginatedResponse, SuccessResponse};
use auth9_core::domain::{AlertSeverity, SecurityAlert, SecurityAlertType, StringUuid};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use chrono::Utc;
use tower::ServiceExt;

// ============================================================================
// List Alerts Tests
// ============================================================================

#[tokio::test]
async fn test_list_alerts_default() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add some test alerts
    add_test_security_alerts(&state, 5).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<SecurityAlert>>) =
        get_json(&app, "/api/v1/security/alerts").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 5);
}

#[tokio::test]
async fn test_list_alerts_with_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_security_alerts(&state, 25).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<SecurityAlert>>) =
        get_json(&app, "/api/v1/security/alerts?page=1&per_page=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.total, 25);
    assert_eq!(response.pagination.page, 1);
    assert_eq!(response.pagination.per_page, 10);
}

#[tokio::test]
async fn test_list_alerts_unresolved_only() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add mix of resolved and unresolved alerts
    add_test_security_alerts(&state, 5).await;
    add_resolved_security_alerts(&state, 3).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<SecurityAlert>>) =
        get_json(&app, "/api/v1/security/alerts?unresolved_only=true").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5); // Only unresolved
    assert_eq!(response.pagination.total, 5);
}

#[tokio::test]
async fn test_list_alerts_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<SecurityAlert>>) =
        get_json(&app, "/api/v1/security/alerts").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 0);
    assert_eq!(response.pagination.total, 0);
}

// ============================================================================
// Get Alert Tests
// ============================================================================

#[tokio::test]
async fn test_get_alert_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let alert_id = StringUuid::new_v4();
    let alert = SecurityAlert {
        id: alert_id,
        user_id: None,
        tenant_id: None,
        alert_type: SecurityAlertType::BruteForce,
        severity: AlertSeverity::High,
        details: Some(serde_json::json!({"ip": "192.168.1.1"})),
        resolved_at: None,
        resolved_by: None,
        created_at: Utc::now(),
    };
    state.security_alert_repo.add_alert(alert).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<SecurityAlert>>) =
        get_json(&app, &format!("/api/v1/security/alerts/{}", alert_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.id, alert_id);
    assert_eq!(response.alert_type, SecurityAlertType::BruteForce);
    assert_eq!(response.severity, AlertSeverity::High);
}

#[tokio::test]
async fn test_get_alert_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_security_alert_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _body): (StatusCode, Option<SuccessResponse<SecurityAlert>>) =
        get_json(&app, &format!("/api/v1/security/alerts/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Resolve Alert Tests
// ============================================================================

#[tokio::test]
async fn test_resolve_alert_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a user for authorization
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create an alert
    let alert_id = StringUuid::new_v4();
    let alert = SecurityAlert {
        id: alert_id,
        user_id: None,
        tenant_id: None,
        alert_type: SecurityAlertType::BruteForce,
        severity: AlertSeverity::High,
        details: None,
        resolved_at: None,
        resolved_by: None,
        created_at: Utc::now(),
    };
    state.security_alert_repo.add_alert(alert).await;

    // Generate a token for the user
    let jwt_manager = create_test_jwt_manager();
    let token = jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<SecurityAlert>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/security/alerts/{}/resolve", alert_id),
            &(),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert!(response.resolved_at.is_some());
    assert_eq!(response.resolved_by, Some(user_id));
}

#[tokio::test]
async fn test_resolve_alert_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let alert_id = StringUuid::new_v4();
    let alert = SecurityAlert {
        id: alert_id,
        user_id: None,
        tenant_id: None,
        alert_type: SecurityAlertType::BruteForce,
        severity: AlertSeverity::High,
        details: None,
        resolved_at: None,
        resolved_by: None,
        created_at: Utc::now(),
    };
    state.security_alert_repo.add_alert(alert).await;

    let app = build_security_alert_test_router(state);

    // Try to resolve without authorization
    let request = Request::builder()
        .method(Method::POST)
        .uri(&format!("/api/v1/security/alerts/{}/resolve", alert_id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Unresolved Count Tests
// ============================================================================

#[tokio::test]
async fn test_get_unresolved_count() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add mix of resolved and unresolved alerts
    add_test_security_alerts(&state, 7).await;
    add_resolved_security_alerts(&state, 3).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<UnresolvedCountResponse>>) =
        get_json(&app, "/api/v1/security/alerts/unresolved-count").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.unresolved_count, 7);
}

#[tokio::test]
async fn test_get_unresolved_count_zero() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add only resolved alerts
    add_resolved_security_alerts(&state, 5).await;

    let app = build_security_alert_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<UnresolvedCountResponse>>) =
        get_json(&app, "/api/v1/security/alerts/unresolved-count").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.unresolved_count, 0);
}

// ============================================================================
// Test Helpers
// ============================================================================

async fn add_test_security_alerts(state: &TestAppState, count: usize) {
    for i in 0..count {
        let alert = SecurityAlert {
            id: StringUuid::new_v4(),
            user_id: None,
            tenant_id: None,
            alert_type: match i % 3 {
                0 => SecurityAlertType::BruteForce,
                1 => SecurityAlertType::NewDevice,
                _ => SecurityAlertType::ImpossibleTravel,
            },
            severity: match i % 4 {
                0 => AlertSeverity::Critical,
                1 => AlertSeverity::High,
                2 => AlertSeverity::Medium,
                _ => AlertSeverity::Low,
            },
            details: Some(serde_json::json!({"index": i})),
            resolved_at: None,
            resolved_by: None,
            created_at: Utc::now(),
        };
        state.security_alert_repo.add_alert(alert).await;
    }
}

async fn add_resolved_security_alerts(state: &TestAppState, count: usize) {
    for _i in 0..count {
        let alert = SecurityAlert {
            id: StringUuid::new_v4(),
            user_id: None,
            tenant_id: None,
            alert_type: SecurityAlertType::BruteForce,
            severity: AlertSeverity::Medium,
            details: None,
            resolved_at: Some(Utc::now()),
            resolved_by: Some(StringUuid::new_v4()),
            created_at: Utc::now(),
        };
        state.security_alert_repo.add_alert(alert).await;
    }
}

async fn post_json_with_auth<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    app: &axum::Router,
    path: &str,
    body: &T,
    token: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    if body_bytes.is_empty() {
        return (status, None);
    }

    match serde_json::from_slice(&body_bytes) {
        Ok(data) => (status, Some(data)),
        Err(_) => (status, None),
    }
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_security_alert_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::security_alert;
    use axum::routing::{get, post};

    axum::Router::new()
        .route(
            "/api/v1/security/alerts",
            get(security_alert::list_alerts::<TestAppState>),
        )
        .route(
            "/api/v1/security/alerts/unresolved-count",
            get(security_alert::get_unresolved_count::<TestAppState>),
        )
        .route(
            "/api/v1/security/alerts/{alert_id}",
            get(security_alert::get_alert::<TestAppState>),
        )
        .route(
            "/api/v1/security/alerts/{alert_id}/resolve",
            post(security_alert::resolve_alert::<TestAppState>),
        )
        .with_state(state)
}
