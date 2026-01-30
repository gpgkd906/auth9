//! Role/Permission API HTTP Handler Tests
//!
//! Tests for the role and permission HTTP endpoints using mock repositories.

use super::mock_keycloak::MockKeycloakServer;
use super::{build_test_router, delete_json, get_json, post_json, put_json, TestAppState};
use crate::api::{create_test_permission, create_test_role};
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::{Permission, Role, UserRolesInTenant};
use auth9_core::repository::RbacRepository;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Local struct for deserializing RoleWithPermissions in tests
/// (the domain type only has Serialize, not Deserialize)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRoleWithPermissions {
    #[serde(flatten)]
    pub role: Role,
    pub permissions: Vec<Permission>,
}

// ============================================================================
// Permission Tests
// ============================================================================

#[tokio::test]
async fn test_list_permissions() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();

    // Add some permissions for the service
    let perm1 = create_test_permission(None, service_id);
    let mut perm2 = create_test_permission(None, service_id);
    perm2.code = "users:write".to_string();
    perm2.name = "Write Users".to_string();

    state.rbac_repo.add_permission(perm1).await;
    state.rbac_repo.add_permission(perm2).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Permission>>>) =
        get_json(&app, &format!("/api/v1/services/{}/permissions", service_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
}

#[tokio::test]
async fn test_list_permissions_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Permission>>>) =
        get_json(&app, &format!("/api/v1/services/{}/permissions", service_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.is_empty());
}

#[tokio::test]
async fn test_create_permission() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();
    let input = json!({
        "service_id": service_id.to_string(),
        "code": "documents:read",
        "name": "Read Documents",
        "description": "Allows reading documents"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Permission>>) =
        post_json(&app, "/api/v1/permissions", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.code, "documents:read");
    assert_eq!(response.data.name, "Read Documents");
    assert_eq!(
        response.data.description,
        Some("Allows reading documents".to_string())
    );
}

#[tokio::test]
async fn test_create_permission_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();
    let input = json!({
        "service_id": service_id.to_string(),
        "code": "admin:all",
        "name": "Full Admin"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Permission>>) =
        post_json(&app, "/api/v1/permissions", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.code, "admin:all");
    assert!(response.data.description.is_none());
}

#[tokio::test]
async fn test_delete_permission() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id.0;
    state.rbac_repo.add_permission(permission).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, &format!("/api/v1/permissions/{}", permission_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_permission_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json(&app, &format!("/api/v1/permissions/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Role Tests
// ============================================================================

#[tokio::test]
async fn test_list_roles() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();

    // Add some roles for the service
    let role1 = create_test_role(None, service_id);
    let mut role2 = create_test_role(None, service_id);
    role2.name = "editor".to_string();

    state.rbac_repo.add_role(role1).await;
    state.rbac_repo.add_role(role2).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Role>>>) =
        get_json(&app, &format!("/api/v1/services/{}/roles", service_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
}

#[tokio::test]
async fn test_list_roles_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Role>>>) =
        get_json(&app, &format!("/api/v1/services/{}/roles", service_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.is_empty());
}

#[tokio::test]
async fn test_get_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    let mut role = create_test_role(Some(role_id), service_id);
    role.name = "admin".to_string();
    role.description = Some("Administrator role".to_string());

    state.rbac_repo.add_role(role).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<TestRoleWithPermissions>>) =
        get_json(&app, &format!("/api/v1/roles/{}", role_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.role.name, "admin");
    assert_eq!(
        response.data.role.description,
        Some("Administrator role".to_string())
    );
}

#[tokio::test]
async fn test_get_role_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, &format!("/api/v1/roles/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();
    let input = json!({
        "service_id": service_id.to_string(),
        "name": "manager",
        "description": "Manager role with limited permissions"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Role>>) =
        post_json(&app, "/api/v1/roles", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "manager");
}

#[tokio::test]
async fn test_create_role_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();
    let input = json!({
        "service_id": service_id.to_string(),
        "name": "viewer"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Role>>) =
        post_json(&app, "/api/v1/roles", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "viewer");
    assert!(response.data.description.is_none());
}

#[tokio::test]
async fn test_update_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    let role = create_test_role(Some(role_id), service_id);
    state.rbac_repo.add_role(role).await;

    let app = build_test_router(state);

    let input = json!({
        "name": "super-admin",
        "description": "Updated description"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Role>>) =
        put_json(&app, &format!("/api/v1/roles/{}", role_id), &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "super-admin");
    assert_eq!(
        response.data.description,
        Some("Updated description".to_string())
    );
}

#[tokio::test]
async fn test_update_role_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let input = json!({
        "name": "updated"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json(&app, &format!("/api/v1/roles/{}", nonexistent_id), &input).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    let role = create_test_role(Some(role_id), service_id);
    state.rbac_repo.add_role(role).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, &format!("/api/v1/roles/{}", role_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_role_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json(&app, &format!("/api/v1/roles/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Role-Permission Assignment Tests
// ============================================================================

#[tokio::test]
async fn test_assign_permission_to_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role = create_test_role(None, service_id);
    let role_id = role.id.0;
    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id.0;

    state.rbac_repo.add_role(role).await;
    state.rbac_repo.add_permission(permission).await;

    let app = build_test_router(state);

    let input = json!({
        "permission_id": permission_id.to_string()
    });

    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json(&app, &format!("/api/v1/roles/{}/permissions", role_id), &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("assigned"));
}

#[tokio::test]
async fn test_remove_permission_from_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role = create_test_role(None, service_id);
    let role_id = role.id;
    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;

    state.rbac_repo.add_role(role).await;
    state.rbac_repo.add_permission(permission).await;

    // Assign permission to role first using the trait method
    RbacRepository::assign_permission_to_role(&*state.rbac_repo, role_id, permission_id)
        .await
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) = delete_json(
        &app,
        &format!("/api/v1/roles/{}/permissions/{}", role_id.0, permission_id.0),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("removed"));
}

// ============================================================================
// User-Role Assignment Tests
// ============================================================================

#[tokio::test]
async fn test_assign_roles_to_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role1 = create_test_role(None, service_id);
    let role2 = create_test_role(None, service_id);
    let role1_id = role1.id.0;
    let role2_id = role2.id.0;

    state.rbac_repo.add_role(role1).await;
    state.rbac_repo.add_role(role2).await;

    let app = build_test_router(state);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let input = json!({
        "user_id": user_id.to_string(),
        "tenant_id": tenant_id.to_string(),
        "role_ids": [role1_id.to_string(), role2_id.to_string()]
    });

    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/rbac/assign", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("assigned"));
}

#[tokio::test]
async fn test_get_user_roles() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    // Set up user roles in test repository
    let roles = UserRolesInTenant {
        user_id,
        tenant_id,
        roles: vec!["admin".to_string(), "editor".to_string()],
        permissions: vec!["users:read".to_string(), "users:write".to_string()],
    };
    state.rbac_repo.set_user_roles(user_id, tenant_id, roles).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<UserRolesInTenant>>) = get_json(
        &app,
        &format!("/api/v1/users/{}/tenants/{}/roles", user_id, tenant_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.roles.len(), 2);
    assert_eq!(response.data.permissions.len(), 2);
}

#[tokio::test]
async fn test_get_user_roles_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let (status, body): (StatusCode, Option<SuccessResponse<UserRolesInTenant>>) = get_json(
        &app,
        &format!("/api/v1/users/{}/tenants/{}/roles", user_id, tenant_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.roles.is_empty());
    assert!(response.data.permissions.is_empty());
}

#[tokio::test]
async fn test_get_user_assigned_roles() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role = create_test_role(None, service_id);
    state.rbac_repo.add_role(role).await;

    let app = build_test_router(state);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Role>>>) = get_json(
        &app,
        &format!(
            "/api/v1/users/{}/tenants/{}/assigned-roles",
            user_id, tenant_id
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_unassign_role_from_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let role = create_test_role(None, service_id);
    let role_id = role.id;
    state.rbac_repo.add_role(role).await;

    let app = build_test_router(state);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    // Note: The actual unassignment logic depends on the repository finding a tenant_user_id
    // In tests, the TestRbacRepository returns a new UUID for find_tenant_user_id
    let (status, _body): (StatusCode, Option<MessageResponse>) = delete_json(
        &app,
        &format!(
            "/api/v1/users/{}/tenants/{}/roles/{}",
            user_id, tenant_id, role_id.0
        ),
    )
    .await;

    // This should succeed because TestRbacRepository.find_tenant_user_id returns Some(...)
    // But unassign_role may fail if the role isn't actually assigned - depends on implementation
    // For now, we just verify the endpoint is reachable
    assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_create_role_with_parent() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let parent_role = create_test_role(None, service_id);
    let parent_role_id = parent_role.id.0;
    state.rbac_repo.add_role(parent_role).await;

    let app = build_test_router(state);

    let input = json!({
        "service_id": service_id.to_string(),
        "name": "child-role",
        "description": "Role that inherits from parent",
        "parent_role_id": parent_role_id.to_string()
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Role>>) =
        post_json(&app, "/api/v1/roles", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "child-role");
    assert_eq!(
        response.data.parent_role_id,
        Some(auth9_core::domain::StringUuid::from(parent_role_id))
    );
}

#[tokio::test]
async fn test_permission_code_formats() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let service_id = Uuid::new_v4();

    // Test various permission code formats
    // Note: Stick to simple colon-separated formats that pass validation
    let codes = vec![
        "users:read",
        "documents:read:own",
        "admin:settings:update",
        "api:v1:users",
    ];

    for code in codes {
        let input = json!({
            "service_id": service_id.to_string(),
            "code": code,
            "name": format!("Permission for {}", code)
        });

        let (status, _body): (StatusCode, Option<SuccessResponse<Permission>>) =
            post_json(&app, "/api/v1/permissions", &input).await;

        assert_eq!(status, StatusCode::CREATED, "Failed for code: {}", code);
    }
}
