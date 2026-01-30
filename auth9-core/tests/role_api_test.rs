//! Role API integration tests

use crate::common::TestApp;
use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, ResponseTemplate};

mod common;

#[tokio::test]
async fn test_role_crud_flow() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token - use a more permissive matcher
    Mock::given(method("POST"))
        .and(path_regex("/realms/master/protocol/openid-connect/token.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,  // Long expiry to avoid refresh
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .named("keycloak_admin_token")
        .mount(&app.mock_server)
        .await;

    // Mock Create OIDC Client in Keycloak
    let mock_client_uuid = "keycloak-client-uuid-123";
    Mock::given(method("POST"))
        .and(path_regex("/admin/realms/.*/clients"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
        ))
        .named("create_oidc_client")
        .mount(&app.mock_server)
        .await;

    // Mock Get Client Secret (for retrieving client secret after creation)
    Mock::given(method("GET"))
        .and(path_regex("/admin/realms/.*/clients/.*/client-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "secret",
            "value": "mock-client-secret"
        })))
        .named("get_client_secret")
        .mount(&app.mock_server)
        .await;

    // 1. Create a tenant first
    let tenant_res = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "Role Test Tenant",
            "slug": format!("role-test-{}", uuid::Uuid::new_v4())
        }))
        .send()
        .await
        .expect("Failed to create tenant");
    assert!(tenant_res.status().is_success());
    let tenant_body: serde_json::Value = tenant_res.json().await.unwrap();
    let tenant_id = tenant_body["data"]["id"].as_str().unwrap();

    // 2. Create a service
    let service_res = client
        .post(&app.api_url("/api/v1/services"))
        .json(&json!({
            "tenant_id": tenant_id,
            "name": "Role Test Service",
            "client_id": format!("role-svc-{}", uuid::Uuid::new_v4()),
            "redirect_uris": []
        }))
        .send()
        .await
        .expect("Failed to create service");

    assert!(service_res.status().is_success());
    let service_body: serde_json::Value = service_res.json().await.unwrap();
    let service_id = service_body["data"]["id"].as_str().unwrap();

    // 3. Create a permission
    let perm_res = client
        .post(&app.api_url("/api/v1/permissions"))
        .json(&json!({
            "service_id": service_id,
            "code": "documents:read",
            "name": "Read Documents",
            "description": "Permission to read documents"
        }))
        .send()
        .await
        .expect("Failed to create permission");
    assert!(perm_res.status().is_success());
    let perm_body: serde_json::Value = perm_res.json().await.unwrap();
    let permission_id = perm_body["data"]["id"].as_str().unwrap();

    // 4. Create a role with permission
    let role_res = client
        .post(&app.api_url("/api/v1/roles"))
        .json(&json!({
            "service_id": service_id,
            "name": "Document Reader",
            "description": "Can read documents",
            "permission_ids": [permission_id]
        }))
        .send()
        .await
        .expect("Failed to create role");
    assert!(role_res.status().is_success());
    let role_body: serde_json::Value = role_res.json().await.unwrap();
    let role_id = role_body["data"]["id"].as_str().unwrap();
    assert_eq!(role_body["data"]["name"], "Document Reader");

    // 5. Get role by ID
    let get_res = client
        .get(&app.api_url(&format!("/api/v1/roles/{}", role_id)))
        .send()
        .await
        .expect("Failed to get role");
    assert!(get_res.status().is_success());
    let get_body: serde_json::Value = get_res.json().await.unwrap();
    assert_eq!(get_body["data"]["name"], "Document Reader");

    // 6. Update role
    let update_res = client
        .put(&app.api_url(&format!("/api/v1/roles/{}", role_id)))
        .json(&json!({
            "name": "Document Editor",
            "description": "Can read and edit documents"
        }))
        .send()
        .await
        .expect("Failed to update role");
    assert!(update_res.status().is_success());
    let update_body: serde_json::Value = update_res.json().await.unwrap();
    assert_eq!(update_body["data"]["name"], "Document Editor");

    // 7. Delete role
    let delete_res = client
        .delete(&app.api_url(&format!("/api/v1/roles/{}", role_id)))
        .send()
        .await
        .expect("Failed to delete role");
    assert!(delete_res.status().is_success());

    // 8. Verify role is deleted
    let verify_res = client
        .get(&app.api_url(&format!("/api/v1/roles/{}", role_id)))
        .send()
        .await
        .expect("Failed to verify role deletion");
    assert_eq!(verify_res.status().as_u16(), 404);
}

#[tokio::test]
async fn test_list_roles_by_service() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Mock Keycloak Admin Token
    Mock::given(method("POST"))
        .and(path_regex("/realms/master/protocol/openid-connect/token.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-admin-token",
            "expires_in": 36000,  // Long expiry to avoid refresh
            "refresh_token": "mock-refresh-token",
            "token_type": "bearer"
        })))
        .named("keycloak_admin_token")
        .mount(&app.mock_server)
        .await;

    // Mock Create OIDC Client in Keycloak
    let mock_client_uuid = "keycloak-client-uuid-456";
    Mock::given(method("POST"))
        .and(path_regex("/admin/realms/.*/clients"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
        ))
        .named("create_oidc_client")
        .mount(&app.mock_server)
        .await;

    // Mock Get Client Secret
    Mock::given(method("GET"))
        .and(path_regex("/admin/realms/.*/clients/.*/client-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "secret",
            "value": "mock-client-secret"
        })))
        .named("get_client_secret")
        .mount(&app.mock_server)
        .await;

    // Create tenant
    let tenant_res = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "List Roles Tenant",
            "slug": format!("list-roles-{}", uuid::Uuid::new_v4())
        }))
        .send()
        .await
        .unwrap();
    let tenant_body: serde_json::Value = tenant_res.json().await.unwrap();
    let tenant_id = tenant_body["data"]["id"].as_str().unwrap();

    // Create service
    let service_res = client
        .post(&app.api_url("/api/v1/services"))
        .json(&json!({
            "tenant_id": tenant_id,
            "name": "List Roles Service",
            "client_id": format!("list-roles-svc-{}", uuid::Uuid::new_v4()),
            "redirect_uris": []
        }))
        .send()
        .await
        .unwrap();

    assert!(service_res.status().is_success());
    let service_body: serde_json::Value = service_res.json().await.unwrap();
    let service_id = service_body["data"]["id"].as_str().unwrap();

    // Create multiple roles
    for i in 1..=3 {
        client
            .post(&app.api_url("/api/v1/roles"))
            .json(&json!({
                "service_id": service_id,
                "name": format!("Role {}", i),
                "description": format!("Test role {}", i)
            }))
            .send()
            .await
            .unwrap();
    }

    // List roles by service
    let list_res = client
        .get(&app.api_url(&format!("/api/v1/services/{}/roles", service_id)))
        .send()
        .await
        .expect("Failed to list roles");
    assert!(list_res.status().is_success());
    let list_body: serde_json::Value = list_res.json().await.unwrap();
    let roles = list_body["data"].as_array().unwrap();
    assert_eq!(roles.len(), 3);
}
