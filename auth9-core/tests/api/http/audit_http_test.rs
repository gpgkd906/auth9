//! Audit HTTP API handler tests

use super::{get_json_with_auth, MockKeycloakServer, TestAppState};
use auth9_core::api::PaginatedResponse;
use auth9_core::repository::audit::{AuditLogWithActor, CreateAuditLogInput};
use auth9_core::repository::AuditRepository;
use axum::http::StatusCode;
use uuid::Uuid;

// ============================================================================
// List Audit Logs Tests
// ============================================================================

#[tokio::test]
async fn test_list_audit_logs_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, "/api/v1/audit-logs", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 0);
    assert_eq!(response.pagination.total, 0);
}

#[tokio::test]
async fn test_list_audit_logs_with_data() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    // Create audit logs using the repo directly
    for i in 0..5 {
        state
            .audit_repo
            .create(&CreateAuditLogInput {
                actor_id: Some(Uuid::new_v4()),
                action: format!("action_{}", i),
                resource_type: "tenant".to_string(),
                resource_id: Some(Uuid::new_v4()),
                old_value: None,
                new_value: None,
                ip_address: Some("192.168.1.1".to_string()),
            })
            .await
            .unwrap();
    }

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, "/api/v1/audit-logs", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 5);
}

#[tokio::test]
async fn test_list_audit_logs_with_resource_type_filter() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    // Create audit logs with different resource types
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: "create".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: "update".to_string(),
            resource_type: "user".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, "/api/v1/audit-logs?resource_type=tenant", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].resource_type, "tenant");
}

#[tokio::test]
async fn test_list_audit_logs_with_action_filter() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: "create".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: "delete".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, "/api/v1/audit-logs?action=create", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].action, "create");
}

#[tokio::test]
async fn test_list_audit_logs_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    // Create 15 audit log entries
    for i in 0..15 {
        state
            .audit_repo
            .create(&CreateAuditLogInput {
                actor_id: None,
                action: format!("action_{}", i),
                resource_type: "tenant".to_string(),
                resource_id: None,
                old_value: None,
                new_value: None,
                ip_address: None,
            })
            .await
            .unwrap();
    }

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, "/api/v1/audit-logs?limit=5", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 15);
}

#[tokio::test]
async fn test_list_audit_logs_with_actor_filter() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(Uuid::new_v4(), "admin@auth9.local", Some("Platform Admin"))
        .unwrap();

    let actor_id = Uuid::new_v4();
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: Some(actor_id),
            action: "create".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: Some(Uuid::new_v4()), // Different actor
            action: "create".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();

    let app = build_audit_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<AuditLogWithActor>>) =
        get_json_with_auth(&app, &format!("/api/v1/audit-logs?actor_id={}", actor_id), &token)
            .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_audit_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::audit;
    use axum::routing::get;

    axum::Router::new()
        .route("/api/v1/audit-logs", get(audit::list::<TestAppState>))
        .with_state(state)
}
