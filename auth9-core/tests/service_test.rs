//! Service repository integration tests

use auth9_core::domain::{CreateServiceInput, CreateTenantInput, ServiceStatus, UpdateServiceInput};
use auth9_core::repository::service::ServiceRepositoryImpl;
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::{ServiceRepository, TenantRepository};
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_create_and_find_service() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    // Create tenant first
    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Service Test Tenant".to_string(),
            slug: "service-test".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    // Create service
    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "Test Service".to_string(),
            client_id: format!("client-{}", Uuid::new_v4()),
            base_url: Some("https://example.com".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            logout_uris: None,
        })
        .await
        .unwrap();

    assert_eq!(service.name, "Test Service");
    assert_eq!(service.status, ServiceStatus::Active);

    // Find by ID
    let found = service_repo.find_by_id(*service.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Test Service");

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_create_client_for_service() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Client Test Tenant".to_string(),
            slug: "client-test".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "Client Test Service".to_string(),
            client_id: format!("initial-client-{}", Uuid::new_v4()),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        })
        .await
        .unwrap();

    // Create additional client
    let client_id = format!("test-client-{}", Uuid::new_v4());
    let secret_hash = "hashed_secret_value";

    let client = service_repo
        .create_client(*service.id, &client_id, secret_hash, Some("Test Client".to_string()))
        .await
        .unwrap();

    assert_eq!(client.client_id, client_id);
    assert_eq!(client.name, Some("Test Client".to_string()));

    // Find client by client_id
    let found_client = service_repo.find_client_by_client_id(&client_id).await.unwrap();
    assert!(found_client.is_some());

    // Find service by client_id
    let found_service = service_repo.find_by_client_id(&client_id).await.unwrap();
    assert!(found_service.is_some());
    assert_eq!(found_service.unwrap().id, service.id);

    // List clients for service
    let clients = service_repo.list_clients(*service.id).await.unwrap();
    // Should have initial client + the one we just created
    assert!(clients.len() >= 1);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_list_services_with_tenant_filter() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    // Create two tenants
    let tenant_a = tenant_repo
        .create(&CreateTenantInput {
            name: "Tenant A".to_string(),
            slug: "tenant-a-svc".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let tenant_b = tenant_repo
        .create(&CreateTenantInput {
            name: "Tenant B".to_string(),
            slug: "tenant-b-svc".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    // Create services for each tenant
    for i in 1..=3 {
        service_repo
            .create(&CreateServiceInput {
                tenant_id: Some(Uuid::parse_str(&tenant_a.id.to_string()).unwrap()),
                name: format!("Service A-{}", i),
                client_id: format!("client-a-{}-{}", i, Uuid::new_v4()),
                base_url: None,
                redirect_uris: vec![],
                logout_uris: None,
            })
            .await
            .unwrap();
    }

    for i in 1..=2 {
        service_repo
            .create(&CreateServiceInput {
                tenant_id: Some(Uuid::parse_str(&tenant_b.id.to_string()).unwrap()),
                name: format!("Service B-{}", i),
                client_id: format!("client-b-{}-{}", i, Uuid::new_v4()),
                base_url: None,
                redirect_uris: vec![],
                logout_uris: None,
            })
            .await
            .unwrap();
    }

    // List all services
    let all_services = service_repo.list(None, 0, 100).await.unwrap();
    assert_eq!(all_services.len(), 5);

    // List services for tenant A
    let tenant_a_uuid = Uuid::parse_str(&tenant_a.id.to_string()).unwrap();
    let tenant_a_services = service_repo.list(Some(tenant_a_uuid), 0, 100).await.unwrap();
    assert_eq!(tenant_a_services.len(), 3);

    // List services for tenant B
    let tenant_b_uuid = Uuid::parse_str(&tenant_b.id.to_string()).unwrap();
    let tenant_b_services = service_repo.list(Some(tenant_b_uuid), 0, 100).await.unwrap();
    assert_eq!(tenant_b_services.len(), 2);

    // Count services
    let count_all = service_repo.count(None).await.unwrap();
    assert_eq!(count_all, 5);

    let count_a = service_repo.count(Some(tenant_a_uuid)).await.unwrap();
    assert_eq!(count_a, 3);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_service_status() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Update Test Tenant".to_string(),
            slug: "update-svc-test".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "Update Test Service".to_string(),
            client_id: format!("update-client-{}", Uuid::new_v4()),
            base_url: Some("https://old.example.com".to_string()),
            redirect_uris: vec![],
            logout_uris: None,
        })
        .await
        .unwrap();

    // Update service
    let updated = service_repo
        .update(
            *service.id,
            &UpdateServiceInput {
                name: Some("Updated Service Name".to_string()),
                base_url: Some("https://new.example.com".to_string()),
                redirect_uris: Some(vec!["https://new-callback.com".to_string()]),
                logout_uris: None,
                status: Some(ServiceStatus::Inactive),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Service Name");
    assert_eq!(updated.status, ServiceStatus::Inactive);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_service_and_client() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Delete Test Tenant".to_string(),
            slug: "delete-svc-test".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "Delete Test Service".to_string(),
            client_id: format!("delete-initial-{}", Uuid::new_v4()),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        })
        .await
        .unwrap();

    // Create additional client
    let client_id = format!("delete-client-{}", Uuid::new_v4());
    service_repo
        .create_client(*service.id, &client_id, "hash", None)
        .await
        .unwrap();

    // Delete client
    service_repo.delete_client(*service.id, &client_id).await.unwrap();

    // Delete service
    service_repo.delete(*service.id).await.unwrap();

    let found = service_repo.find_by_id(*service.id).await.unwrap();
    assert!(found.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_client_secret_hash() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "Secret Update Tenant".to_string(),
            slug: "secret-update-test".to_string(),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "Secret Update Service".to_string(),
            client_id: format!("secret-initial-{}", Uuid::new_v4()),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        })
        .await
        .unwrap();

    let client_id = format!("secret-client-{}", Uuid::new_v4());
    service_repo
        .create_client(*service.id, &client_id, "original_hash", None)
        .await
        .unwrap();

    // Update secret hash
    service_repo
        .update_client_secret_hash(&client_id, "new_secret_hash")
        .await
        .unwrap();

    // Verify by finding the client (hash is not exposed, but we can confirm no error)
    let client = service_repo.find_client_by_client_id(&client_id).await.unwrap();
    assert!(client.is_some());

    common::cleanup_database(&pool).await.unwrap();
}
