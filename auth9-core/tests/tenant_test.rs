use auth9_core::domain::{CreateTenantInput, TenantStatus, UpdateTenantInput};
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::TenantRepository;
use sqlx::mysql::MySqlPoolOptions;

mod common;

#[tokio::test]
async fn test_create_and_list_tenants() {
    let config = common::TestApp::test_config();
    let pool = match MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url)
        .await
    {
        Ok(pool) => pool,
        Err(_) => return,
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
    let config = common::TestApp::test_config();
    let pool = match MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&config.database.url)
        .await
    {
        Ok(pool) => pool,
        Err(_) => return,
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
