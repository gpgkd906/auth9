//! RBAC (Role/Permission) API integration tests
//!
//! Tests for RBAC-related operations using mock repositories.

use super::*;
use auth9_core::domain::{CreatePermissionInput, CreateRoleInput, UpdateRoleInput};
use auth9_core::error::AppError;
use uuid::Uuid;

// ============================================================================
// Permission Tests
// ============================================================================

#[tokio::test]
async fn test_create_permission_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreatePermissionInput {
        service_id,
        code: "user:read".to_string(),
        name: "Read Users".to_string(),
        description: Some("Permission to read user data".to_string()),
    };

    let result = service.create_permission(input).await;
    assert!(result.is_ok());

    let permission = result.unwrap();
    assert_eq!(permission.code, "user:read");
    assert_eq!(permission.name, "Read Users");
}

#[tokio::test]
async fn test_create_permission_complex_code() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreatePermissionInput {
        service_id,
        code: "report:export:pdf".to_string(),
        name: "Export PDF Reports".to_string(),
        description: None,
    };

    let result = service.create_permission(input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().code, "report:export:pdf");
}

#[tokio::test]
async fn test_create_permission_invalid_code() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreatePermissionInput {
        service_id,
        code: "invalid code with spaces".to_string(),
        name: "Invalid".to_string(),
        description: None,
    };

    let result = service.create_permission(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_permission_empty_code() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreatePermissionInput {
        service_id,
        code: "".to_string(),
        name: "Empty Code".to_string(),
        description: None,
    };

    let result = service.create_permission(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_get_permission_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    let service = builder.build_rbac_service();
    let result = service.get_permission(permission_id).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, permission_id);
}

#[tokio::test]
async fn test_get_permission_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let result = service.get_permission(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_list_permissions_by_service() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    // Add permissions
    for code in ["user:read", "user:write", "user:delete"] {
        let mut permission = create_test_permission(None, service_id);
        permission.code = code.to_string();
        builder.rbac_repo.add_permission(permission).await;
    }

    let service = builder.build_rbac_service();
    let permissions = service
        .list_permissions(StringUuid::from(service_id))
        .await
        .unwrap();

    assert_eq!(permissions.len(), 3);
}

#[tokio::test]
async fn test_list_permissions_empty() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let permissions = service
        .list_permissions(StringUuid::new_v4())
        .await
        .unwrap();
    assert!(permissions.is_empty());
}

#[tokio::test]
async fn test_delete_permission_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    let service = builder.build_rbac_service();

    // Delete
    let result = service.delete_permission(permission_id).await;
    assert!(result.is_ok());

    // Verify deleted
    let get_result = service.get_permission(permission_id).await;
    assert!(matches!(get_result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_permission_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let result = service.delete_permission(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Role Tests
// ============================================================================

#[tokio::test]
async fn test_create_role_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreateRoleInput {
        service_id,
        name: "admin".to_string(),
        description: Some("Administrator role".to_string()),
        parent_role_id: None,
        permission_ids: None,
    };

    let result = service.create_role(input).await;
    assert!(result.is_ok());

    let role = result.unwrap();
    assert_eq!(role.name, "admin");
    assert_eq!(role.description, Some("Administrator role".to_string()));
}

#[tokio::test]
async fn test_create_role_with_parent() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    // Create parent role
    let parent_role = create_test_role(None, service_id);
    let parent_id = *parent_role.id;
    builder.rbac_repo.add_role(parent_role).await;

    let service = builder.build_rbac_service();

    let input = CreateRoleInput {
        service_id,
        name: "child-role".to_string(),
        description: Some("Child role".to_string()),
        parent_role_id: Some(parent_id),
        permission_ids: None,
    };

    let result = service.create_role(input).await;
    assert!(result.is_ok());

    let role = result.unwrap();
    assert_eq!(role.parent_role_id, Some(StringUuid::from(parent_id)));
}

#[tokio::test]
async fn test_create_role_empty_name() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreateRoleInput {
        service_id,
        name: "".to_string(),
        description: None,
        parent_role_id: None,
        permission_ids: None,
    };

    let result = service.create_role(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_get_role_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let service = builder.build_rbac_service();
    let result = service.get_role(role_id).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, role_id);
}

#[tokio::test]
async fn test_get_role_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let result = service.get_role(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_role_with_permissions() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    // Create role and permission
    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    // Assign permission to role
    builder
        .rbac_repo
        .assign_permission_to_role(role_id, permission_id)
        .await
        .unwrap();

    let service = builder.build_rbac_service();
    let result = service.get_role_with_permissions(role_id).await;

    assert!(result.is_ok());
    let role_with_perms = result.unwrap();
    assert_eq!(role_with_perms.permissions.len(), 1);
}

#[tokio::test]
async fn test_list_roles_by_service() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    // Add roles
    for name in ["admin", "editor", "viewer"] {
        let mut role = create_test_role(None, service_id);
        role.name = name.to_string();
        builder.rbac_repo.add_role(role).await;
    }

    let service = builder.build_rbac_service();
    let roles = service
        .list_roles(StringUuid::from(service_id))
        .await
        .unwrap();

    assert_eq!(roles.len(), 3);
}

#[tokio::test]
async fn test_list_roles_empty() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let roles = service.list_roles(StringUuid::new_v4()).await.unwrap();
    assert!(roles.is_empty());
}

#[tokio::test]
async fn test_update_role_name() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let service = builder.build_rbac_service();

    let input = UpdateRoleInput {
        name: Some("updated-role".to_string()),
        description: None,
        parent_role_id: None,
    };

    let result = service.update_role(role_id, input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "updated-role");
}

#[tokio::test]
async fn test_update_role_description() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let service = builder.build_rbac_service();

    let input = UpdateRoleInput {
        name: None,
        description: Some("New description".to_string()),
        parent_role_id: None,
    };

    let result = service.update_role(role_id, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().description,
        Some("New description".to_string())
    );
}

#[tokio::test]
async fn test_update_role_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let input = UpdateRoleInput {
        name: Some("new-name".to_string()),
        description: None,
        parent_role_id: None,
    };

    let result = service.update_role(StringUuid::new_v4(), input).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_role_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let service = builder.build_rbac_service();

    // Delete
    let result = service.delete_role(role_id).await;
    assert!(result.is_ok());

    // Verify deleted
    let get_result = service.get_role(role_id).await;
    assert!(matches!(get_result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_role_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let result = service.delete_role(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Role-Permission Association Tests
// ============================================================================

#[tokio::test]
async fn test_assign_permission_to_role_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    let service = builder.build_rbac_service();

    let result = service
        .assign_permission_to_role(role_id, permission_id)
        .await;
    assert!(result.is_ok());

    // Verify assignment
    let role_with_perms = service.get_role_with_permissions(role_id).await.unwrap();
    assert_eq!(role_with_perms.permissions.len(), 1);
}

#[tokio::test]
async fn test_assign_permission_to_role_role_not_found() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    let service = builder.build_rbac_service();

    let result = service
        .assign_permission_to_role(StringUuid::new_v4(), permission_id)
        .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_assign_permission_to_role_permission_not_found() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let service = builder.build_rbac_service();

    let result = service
        .assign_permission_to_role(role_id, StringUuid::new_v4())
        .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_remove_permission_from_role_success() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let role = create_test_role(None, service_id);
    let role_id = role.id;
    builder.rbac_repo.add_role(role).await;

    let permission = create_test_permission(None, service_id);
    let permission_id = permission.id;
    builder.rbac_repo.add_permission(permission).await;

    // Assign first
    builder
        .rbac_repo
        .assign_permission_to_role(role_id, permission_id)
        .await
        .unwrap();

    let service = builder.build_rbac_service();

    // Remove
    let result = service
        .remove_permission_from_role(role_id, permission_id)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_permission_from_role_not_assigned() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_rbac_service();

    let result = service
        .remove_permission_from_role(StringUuid::new_v4(), StringUuid::new_v4())
        .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_create_role_with_special_chars_name() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    let input = CreateRoleInput {
        service_id,
        name: "role-with-dashes_and_underscores".to_string(),
        description: None,
        parent_role_id: None,
        permission_ids: None,
    };

    let result = service.create_role(input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_multiple_permissions_same_service() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    // Create multiple permissions
    let codes = ["users:read", "users:write", "users:delete", "reports:view"];

    for code in codes {
        let input = CreatePermissionInput {
            service_id,
            code: code.to_string(),
            name: format!("Permission: {}", code),
            description: None,
        };
        let result = service.create_permission(input).await;
        assert!(result.is_ok());
    }

    // Verify all created
    let permissions = service
        .list_permissions(StringUuid::from(service_id))
        .await
        .unwrap();
    assert_eq!(permissions.len(), 4);
}

#[tokio::test]
async fn test_role_hierarchy() {
    let builder = TestServicesBuilder::new();
    let service_id = Uuid::new_v4();

    let service = builder.build_rbac_service();

    // Create admin role (root)
    let admin_input = CreateRoleInput {
        service_id,
        name: "admin".to_string(),
        description: Some("Administrator".to_string()),
        parent_role_id: None,
        permission_ids: None,
    };
    let admin_role = service.create_role(admin_input).await.unwrap();

    // Create editor role (child of admin)
    let editor_input = CreateRoleInput {
        service_id,
        name: "editor".to_string(),
        description: Some("Editor".to_string()),
        parent_role_id: Some(*admin_role.id),
        permission_ids: None,
    };
    let editor_role = service.create_role(editor_input).await.unwrap();

    // Create viewer role (child of editor)
    let viewer_input = CreateRoleInput {
        service_id,
        name: "viewer".to_string(),
        description: Some("Viewer".to_string()),
        parent_role_id: Some(*editor_role.id),
        permission_ids: None,
    };
    let viewer_role = service.create_role(viewer_input).await.unwrap();

    // Verify hierarchy
    assert!(admin_role.parent_role_id.is_none());
    assert_eq!(editor_role.parent_role_id, Some(admin_role.id));
    assert_eq!(viewer_role.parent_role_id, Some(editor_role.id));
}
