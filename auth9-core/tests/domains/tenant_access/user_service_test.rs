//! User API integration tests
//!
//! Tests for user-related operations using mock repositories.

use crate::support::*;
use auth9_core::domain::{AddUserToTenantInput, CreateUserInput, UpdateUserInput};
use auth9_core::error::AppError;

// ============================================================================
// List Users Tests
// ============================================================================

#[tokio::test]
async fn test_list_users_success() {
    let builder = TestServicesBuilder::new();

    // Add test users
    for i in 1..=3 {
        let mut user = create_test_user(None);
        user.email = format!("user{}@example.com", i);
        builder.user_repo.add_user(user).await;
    }

    let service = builder.build_user_service();
    let (users, total) = service.list(1, 10).await.unwrap();

    assert_eq!(users.len(), 3);
    assert_eq!(total, 3);
}

#[tokio::test]
async fn test_list_users_pagination() {
    let builder = TestServicesBuilder::new();

    // Add 5 users
    for i in 1..=5 {
        let mut user = create_test_user(None);
        user.email = format!("user{}@example.com", i);
        builder.user_repo.add_user(user).await;
    }

    let service = builder.build_user_service();

    // Get page 1
    let (users, total) = service.list(1, 2).await.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(total, 5);

    // Get page 2
    let (users, _) = service.list(2, 2).await.unwrap();
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_list_users_empty() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let (users, total) = service.list(1, 10).await.unwrap();
    assert!(users.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn test_list_tenant_users_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    // Add users to tenant
    for i in 1..=2 {
        let mut user = create_test_user(None);
        user.email = format!("tenant-user{}@example.com", i);
        let user_id = user.id;
        builder.user_repo.add_user(user).await;

        let tenant_user = TenantUser {
            id: StringUuid::new_v4(),
            user_id,
            tenant_id,
            role_in_tenant: "member".to_string(),
            joined_at: chrono::Utc::now(),
        };
        builder.user_repo.add_tenant_user(tenant_user).await;
    }

    let service = builder.build_user_service();
    let users = service.list_tenant_users(tenant_id, 1, 10).await.unwrap();

    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_list_tenant_users_empty() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let users = service
        .list_tenant_users(StringUuid::new_v4(), 1, 10)
        .await
        .unwrap();
    assert!(users.is_empty());
}

// ============================================================================
// Get User Tests
// ============================================================================

#[tokio::test]
async fn test_get_user_success() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let result = service.get(user_id).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, user_id);
}

#[tokio::test]
async fn test_get_user_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service.get(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_user_by_email_success() {
    let builder = TestServicesBuilder::new();

    let mut user = create_test_user(None);
    user.email = "specific@example.com".to_string();
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let result = service.get_by_email("specific@example.com").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().email, "specific@example.com");
}

#[tokio::test]
async fn test_get_user_by_email_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service.get_by_email("nonexistent@example.com").await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_user_by_keycloak_id_success() {
    let builder = TestServicesBuilder::new();

    let mut user = create_test_user(None);
    user.keycloak_id = "kc-specific-123".to_string();
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let result = service.get_by_keycloak_id("kc-specific-123").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().keycloak_id, "kc-specific-123");
}

#[tokio::test]
async fn test_get_user_by_keycloak_id_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service.get_by_keycloak_id("nonexistent-kc-id").await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Create User Tests
// ============================================================================

#[tokio::test]
async fn test_create_user_success() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "new-user@example.com".to_string(),
        display_name: Some("New User".to_string()),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
    };

    let result = service.create("kc-new-user", input).await;
    assert!(result.is_ok());

    let user = result.unwrap();
    assert_eq!(user.email, "new-user@example.com");
    assert_eq!(user.display_name, Some("New User".to_string()));
    assert_eq!(user.keycloak_id, "kc-new-user");
}

#[tokio::test]
async fn test_create_user_minimal() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "minimal@example.com".to_string(),
        display_name: None,
        avatar_url: None,
    };

    let result = service.create("kc-minimal", input).await;
    assert!(result.is_ok());

    let user = result.unwrap();
    assert_eq!(user.email, "minimal@example.com");
    assert!(user.display_name.is_none());
}

#[tokio::test]
async fn test_create_user_duplicate_keycloak_id() {
    let builder = TestServicesBuilder::new();

    // Add existing user with keycloak_id "kc-existing"
    let mut existing = create_test_user(None);
    existing.keycloak_id = "kc-existing".to_string();
    existing.email = "existing@example.com".to_string();
    builder.user_repo.add_user(existing).await;

    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "another@example.com".to_string(),
        display_name: None,
        avatar_url: None,
    };

    let result = service.create("kc-existing", input).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn test_create_user_invalid_email() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "not-a-valid-email".to_string(),
        display_name: None,
        avatar_url: None,
    };

    let result = service.create("kc-invalid", input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_user_empty_email() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "".to_string(),
        display_name: None,
        avatar_url: None,
    };

    let result = service.create("kc-empty", input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

// ============================================================================
// Update User Tests
// ============================================================================

#[tokio::test]
async fn test_update_user_display_name() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    let input = UpdateUserInput {
        display_name: Some("Updated Name".to_string()),
        avatar_url: None,
    };

    let result = service.update(user_id, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().display_name,
        Some("Updated Name".to_string())
    );
}

#[tokio::test]
async fn test_update_user_avatar() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    let input = UpdateUserInput {
        display_name: None,
        avatar_url: Some("https://new-avatar.com/img.png".to_string()),
    };

    let result = service.update(user_id, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().avatar_url,
        Some("https://new-avatar.com/img.png".to_string())
    );
}

#[tokio::test]
async fn test_update_user_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = UpdateUserInput {
        display_name: Some("New Name".to_string()),
        avatar_url: None,
    };

    let result = service.update(StringUuid::new_v4(), input).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Delete User Tests
// ============================================================================

#[tokio::test]
async fn test_delete_user_success() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    // Delete
    let result = service.delete(user_id).await;
    assert!(result.is_ok());

    // Verify deleted
    let get_result = service.get(user_id).await;
    assert!(matches!(get_result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_user_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service.delete(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// MFA Tests
// ============================================================================

#[tokio::test]
async fn test_enable_mfa() {
    let builder = TestServicesBuilder::new();

    let mut user = create_test_user(None);
    user.mfa_enabled = false;
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let result = service.set_mfa_enabled(user_id, true).await;

    assert!(result.is_ok());
    assert!(result.unwrap().mfa_enabled);
}

#[tokio::test]
async fn test_disable_mfa() {
    let builder = TestServicesBuilder::new();

    let mut user = create_test_user(None);
    user.mfa_enabled = true;
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let result = service.set_mfa_enabled(user_id, false).await;

    assert!(result.is_ok());
    assert!(!result.unwrap().mfa_enabled);
}

#[tokio::test]
async fn test_set_mfa_user_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service.set_mfa_enabled(StringUuid::new_v4(), true).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// User-Tenant Association Tests
// ============================================================================

#[tokio::test]
async fn test_add_user_to_tenant_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let user = create_test_user(None);
    let tenant_id = *tenant.id;
    let user_id = *user.id;

    builder.tenant_repo.add_tenant(tenant).await;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    let input = AddUserToTenantInput {
        user_id,
        tenant_id,
        role_in_tenant: "admin".to_string(),
    };

    let result = service.add_to_tenant(input).await;
    assert!(result.is_ok());

    let tenant_user = result.unwrap();
    assert_eq!(tenant_user.role_in_tenant, "admin");
}

#[tokio::test]
async fn test_add_user_to_tenant_as_member() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let user = create_test_user(None);
    let tenant_id = *tenant.id;
    let user_id = *user.id;

    builder.tenant_repo.add_tenant(tenant).await;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    let input = AddUserToTenantInput {
        user_id,
        tenant_id,
        role_in_tenant: "member".to_string(),
    };

    let result = service.add_to_tenant(input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().role_in_tenant, "member");
}

#[tokio::test]
async fn test_remove_user_from_tenant_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let user = create_test_user(None);
    let tenant_id = tenant.id;
    let user_id = user.id;

    builder.tenant_repo.add_tenant(tenant).await;
    builder.user_repo.add_user(user).await;

    // Add user to tenant
    let tenant_user = TenantUser {
        id: StringUuid::new_v4(),
        user_id,
        tenant_id,
        role_in_tenant: "member".to_string(),
        joined_at: chrono::Utc::now(),
    };
    builder.user_repo.add_tenant_user(tenant_user).await;

    let service = builder.build_user_service();

    // Remove
    let result = service.remove_from_tenant(user_id, tenant_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_user_from_tenant_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let result = service
        .remove_from_tenant(StringUuid::new_v4(), StringUuid::new_v4())
        .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_user_tenants_success() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    // Add user to multiple tenants
    for i in 1..=2 {
        let mut tenant = create_test_tenant(None);
        tenant.slug = format!("tenant-{}", i);
        let tenant_id = tenant.id;
        builder.tenant_repo.add_tenant(tenant).await;

        let tenant_user = TenantUser {
            id: StringUuid::new_v4(),
            user_id,
            tenant_id,
            role_in_tenant: "member".to_string(),
            joined_at: chrono::Utc::now(),
        };
        builder.user_repo.add_tenant_user(tenant_user).await;
    }

    let service = builder.build_user_service();
    let tenants = service.get_user_tenants(user_id).await.unwrap();

    assert_eq!(tenants.len(), 2);
}

#[tokio::test]
async fn test_get_user_tenants_empty() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();
    let tenants = service.get_user_tenants(user_id).await.unwrap();

    assert!(tenants.is_empty());
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_create_user_with_unicode_display_name() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let input = CreateUserInput {
        email: "unicode@example.com".to_string(),
        display_name: Some("田中太郎".to_string()),
        avatar_url: None,
    };

    let result = service.create("kc-unicode", input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().display_name, Some("田中太郎".to_string()));
}

#[tokio::test]
async fn test_create_user_with_long_email() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_user_service();

    let long_local = "a".repeat(64);
    let long_email = format!("{}@example.com", long_local);

    let input = CreateUserInput {
        email: long_email.clone(),
        display_name: None,
        avatar_url: None,
    };

    let result = service.create("kc-long", input).await;
    // Should succeed as long email is valid
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_user_multiple_fields() {
    let builder = TestServicesBuilder::new();

    let user = create_test_user(None);
    let user_id = user.id;
    builder.user_repo.add_user(user).await;

    let service = builder.build_user_service();

    let input = UpdateUserInput {
        display_name: Some("New Name".to_string()),
        avatar_url: Some("https://new.com/avatar.png".to_string()),
    };

    let result = service.update(user_id, input).await;
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.display_name, Some("New Name".to_string()));
    assert_eq!(
        updated.avatar_url,
        Some("https://new.com/avatar.png".to_string())
    );
}
