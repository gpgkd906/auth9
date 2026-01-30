//! Tenant repository integration tests

use auth9_core::domain::{CreateTenantInput, TenantStatus, UpdateTenantInput};
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::TenantRepository;

mod common;

#[tokio::test]
async fn test_create_and_list_tenants() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    let tenant_a = repo
        .create(&CreateTenantInput {
            name: "Tenant A".to_string(),
            slug: "tenant-a".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let tenant_b = repo
        .create(&CreateTenantInput {
            name: "Tenant B".to_string(),
            slug: "tenant-b".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let total = repo.count().await.unwrap();
    let tenants = repo.list(0, 10).await.unwrap();

    assert_eq!(total, 2);
    assert_eq!(tenants.len(), 2);
    assert!(tenants.iter().any(|t| t.id == tenant_a.id));
    assert!(tenants.iter().any(|t| t.id == tenant_b.id));

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_tenant_status() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    let tenant = repo
        .create(&CreateTenantInput {
            name: "Tenant C".to_string(),
            slug: "tenant-c".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let updated = repo
        .update(
            tenant.id,
            &UpdateTenantInput {
                name: None,
                logo_url: None,
                settings: None,
                status: Some(TenantStatus::Suspended),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.status, TenantStatus::Suspended);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_find_tenant_by_slug() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    // Create a tenant with a specific slug
    let tenant = repo
        .create(&CreateTenantInput {
            name: "Find By Slug Tenant".to_string(),
            slug: "find-by-slug-test".to_string(),
            logo_url: Some("https://example.com/logo.png".to_string()),
            settings: None,
        })
        .await
        .unwrap();

    // Find by slug - should succeed
    let found = repo.find_by_slug("find-by-slug-test").await.unwrap();
    assert!(found.is_some());
    let found_tenant = found.unwrap();
    assert_eq!(found_tenant.id, tenant.id);
    assert_eq!(found_tenant.name, "Find By Slug Tenant");
    assert_eq!(found_tenant.slug, "find-by-slug-test");
    assert_eq!(found_tenant.logo_url, Some("https://example.com/logo.png".to_string()));

    // Find by non-existent slug - should return None
    let not_found = repo.find_by_slug("non-existent-slug").await.unwrap();
    assert!(not_found.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    // Create a tenant
    let tenant = repo
        .create(&CreateTenantInput {
            name: "Tenant To Delete".to_string(),
            slug: "tenant-to-delete".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    // Verify tenant exists
    let found = repo.find_by_id(tenant.id).await.unwrap();
    assert!(found.is_some());

    // Delete the tenant
    let delete_result = repo.delete(tenant.id).await;
    assert!(delete_result.is_ok());

    // Verify tenant no longer exists
    let after_delete = repo.find_by_id(tenant.id).await.unwrap();
    assert!(after_delete.is_none());

    // Verify count decreased
    let count = repo.count().await.unwrap();
    assert_eq!(count, 0);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_tenant_full() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    // Create a tenant
    let tenant = repo
        .create(&CreateTenantInput {
            name: "Original Name".to_string(),
            slug: "original-slug".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    // Update all fields
    let settings = auth9_core::domain::TenantSettings {
        require_mfa: true,
        session_timeout_secs: 7200,
        allowed_auth_methods: vec!["password".to_string(), "sso".to_string()],
        branding: auth9_core::domain::TenantBranding {
            primary_color: Some("#FF5733".to_string()),
            logo_url: Some("https://brand.example.com/logo.png".to_string()),
        },
    };

    let updated = repo
        .update(
            tenant.id,
            &UpdateTenantInput {
                name: Some("Updated Name".to_string()),
                logo_url: Some("https://example.com/new-logo.png".to_string()),
                settings: Some(settings),
                status: Some(TenantStatus::Inactive),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.logo_url, Some("https://example.com/new-logo.png".to_string()));
    assert_eq!(updated.status, TenantStatus::Inactive);
    assert!(updated.settings.require_mfa);
    assert_eq!(updated.settings.session_timeout_secs, 7200);
    assert_eq!(updated.settings.allowed_auth_methods.len(), 2);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_nonexistent_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    // Try to update a tenant that doesn't exist
    let non_existent_id = auth9_core::domain::StringUuid::new_v4();
    let result = repo
        .update(
            non_existent_id,
            &UpdateTenantInput {
                name: Some("New Name".to_string()),
                logo_url: None,
                settings: None,
                status: None,
            },
        )
        .await;

    // Should return NotFound error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, auth9_core::error::AppError::NotFound(_)));

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_nonexistent_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = TenantRepositoryImpl::new(pool.clone());

    // Try to delete a tenant that doesn't exist
    let non_existent_id = auth9_core::domain::StringUuid::new_v4();
    let result = repo.delete(non_existent_id).await;

    // Should return NotFound error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, auth9_core::error::AppError::NotFound(_)));

    common::cleanup_database(&pool).await.unwrap();
}

