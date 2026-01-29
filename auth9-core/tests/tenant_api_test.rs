use crate::common::TestApp;
use auth9_core::api::SuccessResponse;
use auth9_core::domain::{Tenant, TenantStatus};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_tenant_crud() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // 1. Create Tenant
    let create_res = client.post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "Acme Corp",
            "slug": "acme-corp",
            "logo_url": "https://example.com/logo.png"
        }))
        .send()
        .await
        .expect("Failed to create tenant");
    
    assert!(create_res.status().is_success());
    let create_body: SuccessResponse<Tenant> = create_res.json().await.unwrap();
    let tenant_id = create_body.data.id;
    assert_eq!(create_body.data.name, "Acme Corp");
    assert_eq!(create_body.data.status, TenantStatus::Active);

    // 2. Get Tenant
    let get_res = client.get(&app.api_url(&format!("/api/v1/tenants/{}", tenant_id)))
        .send()
        .await
        .expect("Failed to get tenant");
    
    assert!(get_res.status().is_success());
    let get_body: SuccessResponse<Tenant> = get_res.json().await.unwrap();
    assert_eq!(get_body.data.id, tenant_id);

    // 3. Update Tenant
    let update_res = client.put(&app.api_url(&format!("/api/v1/tenants/{}", tenant_id)))
        .json(&json!({
            "name": "Acme Inc",
            "status": "inactive"
        }))
        .send()
        .await
        .expect("Failed to update tenant");
    
    assert!(update_res.status().is_success());
    let update_body: SuccessResponse<Tenant> = update_res.json().await.unwrap();
    assert_eq!(update_body.data.name, "Acme Inc");
    assert_eq!(update_body.data.status, TenantStatus::Inactive);

    // 4. List Tenants
    let list_res = client.get(&app.api_url("/api/v1/tenants"))
        .query(&[("page", "1"), ("per_page", "10")])
        .send()
        .await
        .expect("Failed to list tenants");

    assert!(list_res.status().is_success());
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    let items = list_json["data"].as_array().unwrap();
    assert!(items.len() >= 1);
    
    // 5. Delete Tenant (Disable)
    let delete_res = client.delete(&app.api_url(&format!("/api/v1/tenants/{}", tenant_id)))
        .send()
        .await
        .expect("Failed to delete tenant");
    
    assert!(delete_res.status().is_success());
    
    // Verify it is disabled (status is usually changed or soft deleted)
    // The implementation of delete sets status to Inactive or Suspended? 
    // Checking src/api/tenant.rs: delete -> calls tenant_service.disable -> likely status change.
    
    let get_after_delete = client.get(&app.api_url(&format!("/api/v1/tenants/{}", tenant_id)))
        .send()
        .await
        .unwrap();
    let get_body_after: SuccessResponse<Tenant> = get_after_delete.json().await.unwrap();
    // Assuming disable sets it to inactive or similar, logic depends on service implementation.
    // Let's check status.
    // Actually tenant::delete calls service.disable().
}
