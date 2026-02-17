//! Analytics HTTP API handler tests
//!
//! Tests for analytics/stats and login events endpoints.

use crate::support::http::{get_json, MockKeycloakServer, TestAppState};
use crate::support::{create_test_tenant, create_test_user};
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
        get_json(&app, "/api/v1/analytics/login-stats?period=daily").await;

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
        get_json(&app, "/api/v1/analytics/login-stats?period=weekly").await;

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
        get_json(&app, "/api/v1/analytics/login-stats?period=monthly").await;

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
        get_json(&app, "/api/v1/analytics/login-stats?days=14").await;

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
        get_json(&app, "/api/v1/analytics/login-stats").await;

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
        get_json(&app, "/api/v1/analytics/login-stats").await;

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
        get_json(&app, "/api/v1/analytics/login-events?page=1&per_page=10").await;

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
        get_json(&app, "/api/v1/analytics/login-events").await;

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

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) = get_json(
        &app,
        &format!("/api/v1/analytics/tenants/{}/events", tenant_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 7);
    assert_eq!(response.pagination.total, 7);
}

#[tokio::test]
async fn test_get_stats_with_start_end_dates() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 12).await;

    let app = build_analytics_test_router(state);

    // Use current year to capture the test events created with Utc::now()
    let now = Utc::now();
    let start = format!("{}-01-01T00:00:00Z", now.format("%Y"));
    let end = format!("{}-12-31T23:59:59Z", now.format("%Y"));

    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) = get_json(
        &app,
        &format!("/api/v1/analytics/login-stats?start={}&end={}", start, end),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let stats = body.unwrap().data;
    assert_eq!(stats.total_logins, 12);
}

#[tokio::test]
async fn test_get_stats_with_invalid_dates_falls_back() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 5).await;

    let app = build_analytics_test_router(state);

    // Invalid date format - should fallback to default (7 days)
    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) = get_json(
        &app,
        "/api/v1/analytics/login-stats?start=invalid&end=invalid",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_get_stats_period_aliases() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 6).await;

    let app = build_analytics_test_router(state.clone());

    // Test "day" alias
    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/login-stats?period=day").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());

    // Test "week" alias
    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/login-stats?period=week").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());

    // Test "month" alias
    let (status, body): (StatusCode, Option<SuccessResponse<LoginStats>>) =
        get_json(&app, "/api/v1/analytics/login-stats?period=month").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_list_events_filter_by_email() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add events with specific email
    for i in 0..5 {
        let event = LoginEvent {
            id: i as i64 + 1,
            user_id: None,
            email: Some("specific@example.com".to_string()),
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

    // Add events with different emails
    add_test_login_events(&state, 10).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) = get_json(
        &app,
        "/api/v1/analytics/login-events?email=specific@example.com",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 5);
}

#[tokio::test]
async fn test_list_events_second_page() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    add_test_login_events(&state, 30).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) =
        get_json(&app, "/api/v1/analytics/login-events?page=2&per_page=10").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.page, 2);
    assert_eq!(response.pagination.total, 30);
}

#[tokio::test]
async fn test_list_user_events_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add many events for this user
    add_test_login_events_for_user(&state, user_id, 25).await;

    let app = build_analytics_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<LoginEvent>>) = get_json(
        &app,
        &format!(
            "/api/v1/analytics/users/{}/events?page=2&per_page=10",
            user_id
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.page, 2);
    assert_eq!(response.pagination.total, 25);
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
    use auth9_core::domains::security_observability::api::analytics;
    use axum::routing::get;

    axum::Router::new()
        .route(
            "/api/v1/analytics/login-stats",
            get(analytics::get_stats::<TestAppState>),
        )
        .route(
            "/api/v1/analytics/login-events",
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
