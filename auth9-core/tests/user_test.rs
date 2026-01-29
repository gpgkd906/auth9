//! User repository integration tests

use auth9_core::domain::{AddUserToTenantInput, CreateTenantInput, CreateUserInput, UpdateUserInput};
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::user::UserRepositoryImpl;
use auth9_core::repository::{TenantRepository, UserRepository};
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_create_and_find_user() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = UserRepositoryImpl::new(pool.clone());

    // Create user
    let user = repo
        .create(
            "keycloak-test-id-001",
            &CreateUserInput {
                email: "test@example.com".to_string(),
                display_name: Some("Test User".to_string()),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.display_name, Some("Test User".to_string()));

    // Find by ID
    let found = repo.find_by_id(user.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().email, "test@example.com");

    // Find by email
    let found_by_email = repo.find_by_email("test@example.com").await.unwrap();
    assert!(found_by_email.is_some());
    assert_eq!(found_by_email.unwrap().id, user.id);

    // Find by keycloak ID
    let found_by_keycloak = repo.find_by_keycloak_id("keycloak-test-id-001").await.unwrap();
    assert!(found_by_keycloak.is_some());
    assert_eq!(found_by_keycloak.unwrap().id, user.id);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_list_and_count_users() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = UserRepositoryImpl::new(pool.clone());

    // Create multiple users
    repo.create(
        "keycloak-id-1",
        &CreateUserInput {
            email: "user1@example.com".to_string(),
            display_name: Some("User One".to_string()),
            avatar_url: None,
        },
    )
    .await
    .unwrap();

    repo.create(
        "keycloak-id-2",
        &CreateUserInput {
            email: "user2@example.com".to_string(),
            display_name: Some("User Two".to_string()),
            avatar_url: None,
        },
    )
    .await
    .unwrap();

    repo.create(
        "keycloak-id-3",
        &CreateUserInput {
            email: "user3@example.com".to_string(),
            display_name: Some("User Three".to_string()),
            avatar_url: None,
        },
    )
    .await
    .unwrap();

    // Count
    let count = repo.count().await.unwrap();
    assert_eq!(count, 3);

    // List with pagination
    let users = repo.list(0, 2).await.unwrap();
    assert_eq!(users.len(), 2);

    let users_page2 = repo.list(2, 2).await.unwrap();
    assert_eq!(users_page2.len(), 1);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_user() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = UserRepositoryImpl::new(pool.clone());

    let user = repo
        .create(
            "keycloak-update-test",
            &CreateUserInput {
                email: "update@example.com".to_string(),
                display_name: Some("Original Name".to_string()),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

    // Update name
    let updated = repo
        .update(
            user.id,
            &UpdateUserInput {
                display_name: Some("Updated Name".to_string()),
                avatar_url: Some("https://example.com/avatar.png".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.display_name, Some("Updated Name".to_string()));
    assert_eq!(
        updated.avatar_url,
        Some("https://example.com/avatar.png".to_string())
    );

    // Update MFA
    let mfa_updated = repo.update_mfa_enabled(user.id, true).await.unwrap();
    assert!(mfa_updated.mfa_enabled);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_user() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = UserRepositoryImpl::new(pool.clone());

    let user = repo
        .create(
            "keycloak-delete-test",
            &CreateUserInput {
                email: "delete@example.com".to_string(),
                display_name: None,
                avatar_url: None,
            },
        )
        .await
        .unwrap();

    // Delete
    repo.delete(user.id).await.unwrap();

    // Verify deleted
    let found = repo.find_by_id(user.id).await.unwrap();
    assert!(found.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_add_and_remove_user_from_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let user_repo = UserRepositoryImpl::new(pool.clone());
    let tenant_repo = TenantRepositoryImpl::new(pool.clone());

    // Create tenant
    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    // Create user
    let user = user_repo
        .create(
            "keycloak-tenant-test",
            &CreateUserInput {
                email: "tenant-user@example.com".to_string(),
                display_name: Some("Tenant User".to_string()),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

    // Add user to tenant - use Uuid type
    let tenant_user = user_repo
        .add_to_tenant(&AddUserToTenantInput {
            user_id: Uuid::parse_str(&user.id.to_string()).unwrap(),
            tenant_id: Uuid::parse_str(&tenant.id.to_string()).unwrap(),
            role_in_tenant: "member".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(tenant_user.user_id, user.id);
    assert_eq!(tenant_user.tenant_id, tenant.id);
    assert_eq!(tenant_user.role_in_tenant, "member");

    // Find tenant users
    let tenant_users = user_repo.find_tenant_users(tenant.id, 0, 10).await.unwrap();
    assert_eq!(tenant_users.len(), 1);
    assert_eq!(tenant_users[0].id, user.id);

    // Find user tenants
    let user_tenants = user_repo.find_user_tenants(user.id).await.unwrap();
    assert_eq!(user_tenants.len(), 1);
    assert_eq!(user_tenants[0].tenant_id, tenant.id);

    // Remove from tenant
    user_repo.remove_from_tenant(user.id, tenant.id).await.unwrap();

    // Verify removed
    let tenant_users_after = user_repo.find_tenant_users(tenant.id, 0, 10).await.unwrap();
    assert_eq!(tenant_users_after.len(), 0);

    common::cleanup_database(&pool).await.unwrap();
}
