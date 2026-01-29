use crate::common::TestApp;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use auth9_core::api::auth::OpenIdConfiguration;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_openid_configuration() {
    let app = TestApp::spawn().await;

    let response = app.http_client()
        .get(&app.api_url("/.well-known/openid-configuration"))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
    let config: OpenIdConfiguration = response.json().await.unwrap();
    assert_eq!(config.issuer, app.config.jwt.issuer);
    assert_eq!(config.authorization_endpoint, format!("{}/api/v1/auth/authorize", app.config.jwt.issuer));
}

#[tokio::test]
async fn test_authorize_redirects() {
    let app = TestApp::spawn().await;

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
            format!("{}/admin/realms/test/clients/mock-client-uuid", app.mock_server.uri())
        ))
        .mount(&app.mock_server)
        .await;

    // Mock Get Client Secret
    Mock::given(method("GET"))
        .and(path("/admin/realms/test/clients/mock-client-uuid/client-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
             "value": "mock-client-secret"
        })))
        .mount(&app.mock_server)
        .await;

    // Create a service (which creates Keycloak client)
    let client = app.http_client();
    let service_res = client.post(&app.api_url("/api/v1/services"))
        .json(&json!({
            "name": "Test App",
            "client_id": "test-client-id",
            "redirect_uris": ["http://localhost/callback"]
        }))
        .send()
        .await
        .expect("Failed to create service");
    
    if !service_res.status().is_success() {
        let status = service_res.status();
        let body = service_res.text().await.unwrap_or_default();
        panic!("Failed to create service: {} - {}", status, body);
    }

    // Test authorize request
    // We expect a redirect to Keycloak (mocked)
    // The redirect URL should point to Keycloak's auth endpoint (mock_server)
    
    // We disable redirect following to check the 307
    let client_no_redirect = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client_no_redirect
        .get(&app.api_url("/api/v1/auth/authorize"))
        .query(&[
            ("response_type", "code"),
            ("client_id", "test-client-id"), // Use the one we created
            ("redirect_uri", "http://localhost/callback"),
            ("scope", "openid"),
            ("state", "xyz123")
        ])
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(), 307);
    
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    // Verify it redirects to Keycloak auth endpoint
    // Config points Keycloak to mock_server
    println!("Location: {}", location);
    assert!(location.starts_with(&app.mock_server.uri()));
    assert!(location.contains("response_type=code"));
    assert!(location.contains("client_id=test-client-id"));
}
