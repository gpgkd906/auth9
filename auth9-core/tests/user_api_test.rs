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

#[tokio::test]
async fn test_user_tenant_association() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    // Mock Create User in Keycloak
    let mock_user_id = "keycloak-user-uuid-456";
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            format!("{}/admin/realms/test/users/{}", app.mock_server.uri(), mock_user_id)
        ))
        .mount(&app.mock_server)
        .await;

    // 1. Create a tenant
    let tenant_res = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "User Test Tenant",
            "slug": format!("user-test-{}", uuid::Uuid::new_v4())
        }))
        .send()
        .await
        .expect("Failed to create tenant");

    assert!(tenant_res.status().is_success());
    let tenant_body: serde_json::Value = tenant_res.json().await.unwrap();
    let tenant_id = tenant_body["data"]["id"].as_str().unwrap();

    // 2. Create a user
    let user_res = client
        .post(&app.api_url("/api/v1/users"))
        .json(&json!({
            "email": "tenant-user@example.com",
            "display_name": "Tenant User",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to create user");

    assert!(user_res.status().is_success());
    let user_body: serde_json::Value = user_res.json().await.unwrap();
    let user_id = user_body["data"]["id"].as_str().unwrap();

    // 3. Add user to tenant
    let add_res = client
        .post(&app.api_url(&format!("/api/v1/users/{}/tenants", user_id)))
        .json(&json!({
            "tenant_id": tenant_id,
            "role_in_tenant": "member"
        }))
        .send()
        .await
        .expect("Failed to add user to tenant");

    assert!(add_res.status().is_success());

    // 4. Get user's tenants
    let get_tenants_res = client
        .get(&app.api_url(&format!("/api/v1/users/{}/tenants", user_id)))
        .send()
        .await
        .expect("Failed to get user tenants");

    assert!(get_tenants_res.status().is_success());
    let tenants_json: serde_json::Value = get_tenants_res.json().await.unwrap();
    let tenants = tenants_json["data"].as_array().unwrap();
    assert_eq!(tenants.len(), 1);
    assert_eq!(tenants[0]["tenant_id"], tenant_id);

    // 5. List tenant users
    let list_users_res = client
        .get(&app.api_url(&format!("/api/v1/tenants/{}/users", tenant_id)))
        .query(&[("page", "1"), ("per_page", "10")])
        .send()
        .await
        .expect("Failed to list tenant users");

    assert!(list_users_res.status().is_success());
    let users_json: serde_json::Value = list_users_res.json().await.unwrap();
    let users = users_json["data"].as_array().unwrap();
    assert_eq!(users.len(), 1);

    // 6. Remove user from tenant
    let remove_res = client
        .delete(&app.api_url(&format!(
            "/api/v1/users/{}/tenants/{}",
            user_id, tenant_id
        )))
        .send()
        .await
        .expect("Failed to remove user from tenant");

    assert!(remove_res.status().is_success());

    // Verify removal
    let get_after_remove = client
        .get(&app.api_url(&format!("/api/v1/users/{}/tenants", user_id)))
        .send()
        .await
        .unwrap();

    let tenants_after: serde_json::Value = get_after_remove.json().await.unwrap();
    let tenants_after_arr = tenants_after["data"].as_array().unwrap();
    assert_eq!(tenants_after_arr.len(), 0);
}

#[tokio::test]
async fn test_user_mfa_management() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    // Mock Create User in Keycloak
    let mock_user_id = "keycloak-user-uuid-789";
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            format!("{}/admin/realms/test/users/{}", app.mock_server.uri(), mock_user_id)
        ))
        .mount(&app.mock_server)
        .await;

    // Mock Update User in Keycloak (for MFA enable/disable)
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/users/{}", mock_user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // Mock List User Credentials (for MFA disable - checking for TOTP)
    Mock::given(method("GET"))
        .and(path(format!("/admin/realms/test/users/{}/credentials", mock_user_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "credential-id-123",
                "type": "otp",
                "userLabel": "TOTP",
                "createdDate": 1234567890,
                "credentialData": "{}",
                "credentialType": "totp"
            }
        ])))
        .mount(&app.mock_server)
        .await;

    // Mock Delete User Credential (for MFA disable - removing TOTP)
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/users/{}/credentials/credential-id-123", mock_user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&app.mock_server)
        .await;

    // 1. Create a user
    let user_res = client
        .post(&app.api_url("/api/v1/users"))
        .json(&json!({
            "email": "mfa-user@example.com",
            "display_name": "MFA User",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to create user");

    assert!(user_res.status().is_success());
    let user_body: serde_json::Value = user_res.json().await.unwrap();
    let user_id = user_body["data"]["id"].as_str().unwrap();
    assert_eq!(user_body["data"]["mfa_enabled"], false);

    // 2. Enable MFA
    let enable_res = client
        .post(&app.api_url(&format!("/api/v1/users/{}/mfa", user_id)))
        .send()
        .await
        .expect("Failed to enable MFA");

    assert!(enable_res.status().is_success());
    let enabled_body: serde_json::Value = enable_res.json().await.unwrap();
    assert_eq!(enabled_body["data"]["mfa_enabled"], true);

    // 3. Disable MFA
    let disable_res = client
        .delete(&app.api_url(&format!("/api/v1/users/{}/mfa", user_id)))
        .send()
        .await
        .expect("Failed to disable MFA");

    assert!(disable_res.status().is_success());
    let disabled_body: serde_json::Value = disable_res.json().await.unwrap();
    assert_eq!(disabled_body["data"]["mfa_enabled"], false);
}

#[tokio::test]
async fn test_get_nonexistent_user_returns_404() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    let fake_id = uuid::Uuid::new_v4();
    let response = client
        .get(&app.api_url(&format!("/api/v1/users/{}", fake_id)))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_create_user_with_duplicate_email() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    // Mock first user creation - success
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            format!("{}/admin/realms/test/users/user-1", app.mock_server.uri())
        ))
        .up_to_n_times(1)
        .mount(&app.mock_server)
        .await;

    // Mock second user creation - conflict
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&app.mock_server)
        .await;

    let email = format!("duplicate-{}@example.com", uuid::Uuid::new_v4());

    // Create first user
    let create_res1 = client
        .post(&app.api_url("/api/v1/users"))
        .json(&json!({
            "email": email,
            "display_name": "First User",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Failed to create user");

    assert!(create_res1.status().is_success());

    // Try to create user with same email
    let create_res2 = client
        .post(&app.api_url("/api/v1/users"))
        .json(&json!({
            "email": email,
            "display_name": "Second User",
            "password": "password123"
        }))
        .send()
        .await
        .expect("Request failed");

    // Should return error (409 Conflict)
    assert!(create_res2.status().is_client_error());
}

#[tokio::test]
async fn test_user_list_pagination() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .mount(&app.mock_server)
        .await;

    // Create multiple users with unique Keycloak IDs
    for i in 1..=5 {
        // Mock each user creation with a unique Keycloak ID
        let mock_user_id = format!("keycloak-user-id-{}", uuid::Uuid::new_v4());
        Mock::given(method("POST"))
            .and(path("/admin/realms/test/users"))
            .respond_with(ResponseTemplate::new(201).insert_header(
                "Location",
                format!("{}/admin/realms/test/users/{}", app.mock_server.uri(), mock_user_id)
            ))
            .up_to_n_times(1)  // Each mock is used only once
            .mount(&app.mock_server)
            .await;

        let res = client
            .post(&app.api_url("/api/v1/users"))
            .json(&json!({
                "email": format!("user{}@pagination.test", i),
                "display_name": format!("User {}", i),
                "password": "password123"
            }))
            .send()
            .await
            .expect("Failed to create user");

        assert!(res.status().is_success(), "Failed to create user {}", i);
    }

    // Test pagination
    let page1 = client
        .get(&app.api_url("/api/v1/users"))
        .query(&[("page", "1"), ("per_page", "2")])
        .send()
        .await
        .expect("Failed to list users");

    assert!(page1.status().is_success());
    let page1_json: serde_json::Value = page1.json().await.unwrap();
    let items = page1_json["data"].as_array().unwrap();
    assert!(items.len() <= 2);
    assert!(page1_json["pagination"]["total"].as_i64().unwrap() >= 5);
}
