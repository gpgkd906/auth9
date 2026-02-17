//! Tenant API integration tests
//!
//! Tests for tenant-related operations using mock repositories.
//! These tests verify the service layer behavior that API handlers depend on.

use crate::support::*;
use auth9_core::domain::{
    CreateTenantInput, TenantBranding, TenantSettings, TenantStatus, UpdateTenantInput,
};
use auth9_core::error::AppError;

// ============================================================================
// List Tenants Tests
// ============================================================================

#[tokio::test]
async fn test_list_tenants_success() {
    let builder = TestServicesBuilder::new();

    // Add test tenants
    for i in 1..=3 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        builder.tenant_repo.add_tenant(tenant).await;
    }

    let service = builder.build_tenant_service();
    let (tenants, total) = service.list(1, 10).await.unwrap();

    assert_eq!(tenants.len(), 3);
    assert_eq!(total, 3);
}

#[tokio::test]
async fn test_list_tenants_pagination_page_1() {
    let builder = TestServicesBuilder::new();

    // Add 5 tenants
    for i in 1..=5 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        builder.tenant_repo.add_tenant(tenant).await;
    }

    let service = builder.build_tenant_service();

    // Get page 1 with 2 items per page
    let (tenants, total) = service.list(1, 2).await.unwrap();
    assert_eq!(tenants.len(), 2);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_tenants_pagination_page_2() {
    let builder = TestServicesBuilder::new();

    // Add 5 tenants
    for i in 1..=5 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        builder.tenant_repo.add_tenant(tenant).await;
    }

    let service = builder.build_tenant_service();

    // Get page 2 with 2 items per page
    let (tenants, total) = service.list(2, 2).await.unwrap();
    assert_eq!(tenants.len(), 2);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_tenants_pagination_last_page() {
    let builder = TestServicesBuilder::new();

    // Add 5 tenants
    for i in 1..=5 {
        let mut tenant = create_test_tenant(None);
        tenant.name = format!("Tenant {}", i);
        tenant.slug = format!("tenant-{}", i);
        builder.tenant_repo.add_tenant(tenant).await;
    }

    let service = builder.build_tenant_service();

    // Get page 3 with 2 items per page (should return 1 item)
    let (tenants, total) = service.list(3, 2).await.unwrap();
    assert_eq!(tenants.len(), 1);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn test_list_tenants_empty() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let (tenants, total) = service.list(1, 10).await.unwrap();
    assert!(tenants.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn test_list_tenants_beyond_last_page() {
    let builder = TestServicesBuilder::new();

    // Add 2 tenants
    for i in 1..=2 {
        let mut tenant = create_test_tenant(None);
        tenant.slug = format!("tenant-{}", i);
        builder.tenant_repo.add_tenant(tenant).await;
    }

    let service = builder.build_tenant_service();

    // Request page 10 (way beyond available data)
    let (tenants, total) = service.list(10, 10).await.unwrap();
    assert!(tenants.is_empty());
    assert_eq!(total, 2);
}

// ============================================================================
// Get Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_get_tenant_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();
    let result = service.get(tenant_id).await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert_eq!(found.id, tenant_id);
    assert_eq!(found.name, "Test Tenant");
}

#[tokio::test]
async fn test_get_tenant_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let result = service.get(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_get_tenant_by_slug_success() {
    let builder = TestServicesBuilder::new();

    let mut tenant = create_test_tenant(None);
    tenant.slug = "acme-corp".to_string();
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();
    let result = service.get_by_slug("acme-corp").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().slug, "acme-corp");
}

#[tokio::test]
async fn test_get_tenant_by_slug_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let result = service.get_by_slug("nonexistent").await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Create Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_create_tenant_success() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Acme Corporation".to_string(),
        slug: "acme-corp".to_string(),
        logo_url: Some("https://example.com/logo.png".to_string()),
        settings: None,
    };

    let result = service.create(input).await;
    assert!(result.is_ok());

    let tenant = result.unwrap();
    assert_eq!(tenant.name, "Acme Corporation");
    assert_eq!(tenant.slug, "acme-corp");
    assert_eq!(tenant.status, TenantStatus::Active);
}

#[tokio::test]
async fn test_create_tenant_with_settings() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let settings = TenantSettings {
        require_mfa: true,
        session_timeout_secs: 7200,
        allowed_auth_methods: vec!["password".to_string(), "sso".to_string()],
        branding: TenantBranding::default(),
    };

    let input = CreateTenantInput {
        name: "Enterprise Corp".to_string(),
        slug: "enterprise".to_string(),
        logo_url: None,
        settings: Some(settings),
    };

    let result = service.create(input).await;
    assert!(result.is_ok());

    let tenant = result.unwrap();
    assert!(tenant.settings.require_mfa);
    assert_eq!(tenant.settings.session_timeout_secs, 7200);
}

#[tokio::test]
async fn test_create_tenant_duplicate_slug() {
    let builder = TestServicesBuilder::new();

    // Add existing tenant with same slug
    let mut existing = create_test_tenant(None);
    existing.slug = "existing-slug".to_string();
    builder.tenant_repo.add_tenant(existing).await;

    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "New Tenant".to_string(),
        slug: "existing-slug".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn test_create_tenant_invalid_slug_with_spaces() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Test".to_string(),
        slug: "invalid slug".to_string(), // Contains space
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_tenant_invalid_slug_uppercase() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Test".to_string(),
        slug: "UPPERCASE".to_string(), // Uppercase
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_tenant_empty_name() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "".to_string(),
        slug: "valid-slug".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn test_create_tenant_empty_slug() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Valid Name".to_string(),
        slug: "".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

// ============================================================================
// Update Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_update_tenant_name_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: Some("Updated Name".to_string()),
        logo_url: None,
        settings: None,
        status: None,
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Updated Name");
}

#[tokio::test]
async fn test_update_tenant_logo_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: None,
        logo_url: Some("https://new-logo.com/logo.png".to_string()),
        settings: None,
        status: None,
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().logo_url,
        Some("https://new-logo.com/logo.png".to_string())
    );
}

#[tokio::test]
async fn test_update_tenant_status_to_inactive() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: None,
        logo_url: None,
        settings: None,
        status: Some(TenantStatus::Inactive),
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, TenantStatus::Inactive);
}

#[tokio::test]
async fn test_update_tenant_status_to_suspended() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: None,
        logo_url: None,
        settings: None,
        status: Some(TenantStatus::Suspended),
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, TenantStatus::Suspended);
}

#[tokio::test]
async fn test_update_tenant_settings() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let new_settings = TenantSettings {
        require_mfa: true,
        session_timeout_secs: 1800,
        allowed_auth_methods: vec!["password".to_string()],
        branding: TenantBranding::default(),
    };

    let input = UpdateTenantInput {
        name: None,
        logo_url: None,
        settings: Some(new_settings),
        status: None,
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());
    let updated = result.unwrap();
    assert!(updated.settings.require_mfa);
    assert_eq!(updated.settings.session_timeout_secs, 1800);
}

#[tokio::test]
async fn test_update_tenant_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: Some("New Name".to_string()),
        logo_url: None,
        settings: None,
        status: None,
    };

    let result = service.update(StringUuid::new_v4(), input).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_update_tenant_invalid_empty_name() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: Some("".to_string()), // Empty name
        logo_url: None,
        settings: None,
        status: None,
    };

    let result = service.update(StringUuid::new_v4(), input).await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

// ============================================================================
// Delete/Disable Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_disable_tenant_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();
    let result = service.disable(tenant_id).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, TenantStatus::Inactive);
}

#[tokio::test]
async fn test_disable_tenant_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let result = service.disable(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_tenant_success() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    // Delete
    let result = service.delete(tenant_id).await;
    assert!(result.is_ok());

    // Verify deleted
    let get_result = service.get(tenant_id).await;
    assert!(matches!(get_result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_tenant_not_found() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let result = service.delete(StringUuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_create_tenant_with_unicode_name() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "日本企業株式会社".to_string(),
        slug: "japan-corp".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "日本企業株式会社");
}

#[tokio::test]
async fn test_create_tenant_with_special_chars_name() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Acme & Co. (Inc.)".to_string(),
        slug: "acme-co".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Acme & Co. (Inc.)");
}

#[tokio::test]
async fn test_create_tenant_slug_with_numbers() {
    let builder = TestServicesBuilder::new();
    let service = builder.build_tenant_service();

    let input = CreateTenantInput {
        name: "Company 123".to_string(),
        slug: "company-123".to_string(),
        logo_url: None,
        settings: None,
    };

    let result = service.create(input).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().slug, "company-123");
}

#[tokio::test]
async fn test_update_multiple_fields_at_once() {
    let builder = TestServicesBuilder::new();

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    builder.tenant_repo.add_tenant(tenant).await;

    let service = builder.build_tenant_service();

    let input = UpdateTenantInput {
        name: Some("Completely New Name".to_string()),
        logo_url: Some("https://new.com/logo.png".to_string()),
        settings: Some(TenantSettings {
            require_mfa: true,
            session_timeout_secs: 7200,
            allowed_auth_methods: vec!["sso".to_string()],
            branding: TenantBranding::default(),
        }),
        status: Some(TenantStatus::Inactive),
    };

    let result = service.update(tenant_id, input).await;
    assert!(result.is_ok());

    let updated = result.unwrap();
    assert_eq!(updated.name, "Completely New Name");
    assert_eq!(
        updated.logo_url,
        Some("https://new.com/logo.png".to_string())
    );
    assert!(updated.settings.require_mfa);
    assert_eq!(updated.status, TenantStatus::Inactive);
}
