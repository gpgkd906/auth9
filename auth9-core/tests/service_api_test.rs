use crate::common::TestApp;
use auth9_core::api::SuccessResponse;
use auth9_core::domain::Service;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_service_crud() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 300,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    let mock_client_uuid = "mock-client-uuid-123";

    // Mock Create Client
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/clients"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location", 
            format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
        ))
        .mount(&app.mock_server)
        .await;

    // Mock Update Client
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/clients/{}", mock_client_uuid)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // Mock Delete Client
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/clients/{}", mock_client_uuid)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // Mock Get Client Secret
    Mock::given(method("GET"))
        .and(path(format!("/admin/realms/test/clients/{}/client-secret", mock_client_uuid)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
             "value": "mock-client-secret"
        })))
        .mount(&app.mock_server)
        .await;

    // 1. Create Service
    let create_res = client.post(&app.api_url("/api/v1/services"))
        .json(&json!({
            "name": "My Service",
            "client_id": "my-service-client",
            "redirect_uris": ["https://app.example.com/callback"]
        }))
        .send()
        .await
        .expect("Failed to create service");
    
    assert!(create_res.status().is_success());
    let create_body: SuccessResponse<Service> = create_res.json().await.unwrap();
    let service_id = create_body.data.id;
    assert_eq!(create_body.data.name, "My Service");

    // 2. Get Service
    let get_res = client.get(&app.api_url(&format!("/api/v1/services/{}", service_id)))
        .send()
        .await
        .expect("Failed to get service");
    
    assert!(get_res.status().is_success());
    let get_body: SuccessResponse<Service> = get_res.json().await.unwrap();
    assert_eq!(get_body.data.id, service_id);

    // 3. Update Service
    let update_res = client.put(&app.api_url(&format!("/api/v1/services/{}", service_id)))
        .json(&json!({
            "name": "Updated Service"
        }))
        .send()
        .await
        .expect("Failed to update service");
    
    assert!(update_res.status().is_success());
    let update_body: SuccessResponse<Service> = update_res.json().await.unwrap();
    assert_eq!(update_body.data.name, "Updated Service");

    // 4. List Services
    let list_res = client.get(&app.api_url("/api/v1/services"))
        .query(&[("page", "1"), ("per_page", "10")])
        .send()
        .await
        .expect("Failed to list services");
    
    assert!(list_res.status().is_success());
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    let items = list_json["data"].as_array().unwrap();
    assert!(items.len() >= 1);

    // 5. Delete Service
    let delete_res = client.delete(&app.api_url(&format!("/api/v1/services/{}", service_id)))
        .send()
        .await
        .expect("Failed to delete service");
    
    assert!(delete_res.status().is_success());

    // Verify deletion (should return 404)
    let get_after_delete = client.get(&app.api_url(&format!("/api/v1/services/{}", service_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(get_after_delete.status().as_u16(), 404);
}

#[tokio::test]
async fn test_regenerate_secret() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    let mock_client_uuid = "mock-client-uuid-456";

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 300,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    // Mock Create Client
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/clients"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location", 
            format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
        ))
        .mount(&app.mock_server)
        .await;

    // Mock Regenerate Secret
    Mock::given(method("POST"))
        .and(path(format!("/admin/realms/test/clients/{}/client-secret", mock_client_uuid)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
             "value": "new-secret-value"
        })))
        .mount(&app.mock_server)
        .await;

    // Create Service
    let create_res = client.post(&app.api_url("/api/v1/services"))
        .json(&json!({
            "name": "Secret Service",
            "client_id": "secret-client",
            "redirect_uris": ["http://localhost"]
        }))
        .send()
        .await
        .expect("Failed to create service");
    let create_body: SuccessResponse<Service> = create_res.json().await.unwrap();
    let service_id = create_body.data.id;

    // Call regenerate endpoint
    let regen_res = client.post(&app.api_url(&format!("/api/v1/services/{}/regenerate_secret", service_id)))
        .send()
        .await
        .expect("Failed to regenerate secret");

    assert!(regen_res.status().is_success());
    let regen_json: serde_json::Value = regen_res.json().await.unwrap();
    assert_eq!(regen_json["data"]["client_secret"], "new-secret-value");
}
