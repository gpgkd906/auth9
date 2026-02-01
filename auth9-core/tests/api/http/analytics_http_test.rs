//! Analytics HTTP API handler tests
//!
//! Tests for analytics/stats and login events endpoints.

use super::{get_json, MockKeycloakServer, TestAppState};
use crate::api::{create_test_tenant, create_test_user};
use auth9_core::api::{PaginatedResponse, SuccessResponse};
use auth9_core::domain::{LoginEvent, LoginEventType, LoginStats, StringUuid};
use axum::http::StatusCode;
use chrono::Utc;

// ============================================================================
// Get Stats Tests
// ============================================================================

#[tokio::test]
async fn test_get_stats_daily_period() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add some login events
    add_test_login_events(&state, 5).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats?period=daily").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 5);
}

#[tokio::test]
async fn test_get_stats_weekly_period() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 10).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats?period=weekly").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 10);
}

#[tokio::test]
async fn test_get_stats_monthly_period() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 15).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats?period=monthly").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 15);
}

#[tokio::test]
async fn test_get_stats_custom_days() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 8).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats?days=14").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 8);
}

#[tokio::test]
async fn test_get_stats_default() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 3).await;

    let app = build_analytics_test_router(state);

    // No period or days specified - defaults to 7 days
    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 3);
}

#[tokio::test]
async fn test_get_stats_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/stats").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 0);
    assert_eq!(stats.successful_logins, 0);
    assert_eq!(stats.failed_logins, 0);
}

// ============================================================================
// List Events Tests
// ============================================================================

#[tokio::test]
async fn test_list_events_with_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 25).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) =
        get_json(&app, "/api/v1/analytics/events?page=1&per_page=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.total, 25);
    assert_eq!(response.pagination.page, 1);
    assert_eq!(response.pagination.per_page, 10);
}

#[tokio::test]
async fn test_list_events_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) =
        get_json(&app, "/api/v1/analytics/events").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 0);
    assert_eq!(response.pagination.total, 0);
}

#[tokio::test]
async fn test_list_user_events() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add events for this user
    add_test_login_events_for_user(&state, user_id, 5).await;
    // Add events for other users
    add_test_login_events(&state, 10).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) =
        get_json(&app, &format!("/api/v1/analytics/users/{}/events", user_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 5);
}

#[tokio::test]
async fn test_list_tenant_events() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Add events for this tenant
    add_test_login_events_for_tenant(&state, tenant_id, 7).await;
    // Add events for other tenants
    add_test_login_events(&state, 5).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) =
        get_json(&app, &format!("/api/v1/analytics/tenants/{}/events", tenant_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 7);
    assert_eq!(response.pagination.total, 7);
}

// ============================================================================
// Test Helpers
// ============================================================================

async fn add_test_login_events(state: &TestAppState, count: usize) {
    for i in 0..count {
        let event = LoginEvent {
            id: i as i64 + 1,
            user_id: None,
            email: Some(format!("user{}@example.com", i)),
            tenant_id: None,
            event_type: if i % 3 == 0 {
                LoginEventType::FailedPassword
            } else {
                LoginEventType::Success
            },
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            device_type: Some("desktop".to_string()),
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };
        state.login_event_repo.add_event(event).await;
    }
}

async fn add_test_login_events_for_user(state: &TestAppState, user_id: StringUuid, count: usize) {
    for i in 0..count {
        let event = LoginEvent {
            id: (i + 100) as i64,
            user_id: Some(user_id),
            email: Some("user@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Success,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            device_type: Some("desktop".to_string()),
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };
        state.login_event_repo.add_event(event).await;
    }
}

async fn add_test_login_events_for_tenant(
    state: &TestAppState,
    tenant_id: StringUuid,
    count: usize,
) {
    for i in 0..count {
        let event = LoginEvent {
            id: (i + 200) as i64,
            user_id: None,
            email: Some(format!("tenant-user{}@example.com", i)),
            tenant_id: Some(tenant_id),
            event_type: LoginEventType::Success,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
            device_type: Some("desktop".to_string()),
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };
        state.login_event_repo.add_event(event).await;
    }
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_analytics_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::analytics;
    use axum::routing::get;

    axum::Router::new()
        .route(
            "/api/v1/analytics/stats",
            get(analytics::get_stats::<TestAppState>),
        )
        .route(
            "/api/v1/analytics/events",
            get(analytics::list_events::<TestAppState>),
        )
        .route(
            "/api/v1/analytics/users/{user_id}/events",
            get(analytics::list_user_events::<TestAppState>),
        )
        .route(
            "/api/v1/analytics/tenants/{tenant_id}/events",
            get(analytics::list_tenant_events::<TestAppState>),
        )
        .with_state(state)
}
