use crate::common::TestApp;
use auth9_core::api::SuccessResponse;
use auth9_core::domain::User;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_user_crud() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token (used by all operations)
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

    // Mock Create User in Keycloak
    let mock_user_id = "keycloak-user-uuid-123";
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location", 
            format!("{}/admin/realms/test/users/{}", app.mock_server.uri(), mock_user_id)
        ))
        .mount(&app.mock_server)
        .await;

    // Mock Update User in Keycloak
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/users/{}", mock_user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // Mock Delete User in Keycloak
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/users/{}", mock_user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // 1. Create User
    let create_res = client.post(&app.api_url("/api/v1/users"))
        .json(&json!({
            "email": "test@example.com",
            "display_name": "Test User",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to create user");
    
    assert!(create_res.status().is_success());
    let create_body: SuccessResponse<User> = create_res.json().await.unwrap();
    let user_id = create_body.data.id;
    assert_eq!(create_body.data.email, "test@example.com");
    assert_eq!(create_body.data.keycloak_id, mock_user_id);

    // 2. Get User
    let get_res = client.get(&app.api_url(&format!("/api/v1/users/{}", user_id)))
        .send()
        .await
        .expect("Failed to get user");
    
    assert!(get_res.status().is_success());
    let get_body: SuccessResponse<User> = get_res.json().await.unwrap();
    assert_eq!(get_body.data.id, user_id);

    // 3. Update User
    let update_res = client.put(&app.api_url(&format!("/api/v1/users/{}", user_id)))
        .json(&json!({
            "display_name": "Updated Test User"
        }))
        .send()
        .await
        .expect("Failed to update user");
    
    assert!(update_res.status().is_success());
    let update_body: SuccessResponse<User> = update_res.json().await.unwrap();
    assert_eq!(update_body.data.display_name, Some("Updated Test User".to_string()));

    // 4. List Users
    let list_res = client.get(&app.api_url("/api/v1/users"))
        .query(&[("page", "1"), ("per_page", "10")])
        .send()
        .await
        .expect("Failed to list users");

    assert!(list_res.status().is_success());
    let list_json: serde_json::Value = list_res.json().await.unwrap();
    let items = list_json["data"].as_array().unwrap();
    assert!(items.len() >= 1);

    // 5. Delete User
    let delete_res = client.delete(&app.api_url(&format!("/api/v1/users/{}", user_id)))
        .send()
        .await
        .expect("Failed to delete user");
    
    assert!(delete_res.status().is_success());

    // Verify deletion (should return 404)
    let get_after_delete = client.get(&app.api_url(&format!("/api/v1/users/{}", user_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(get_after_delete.status().as_u16(), 404);
}
