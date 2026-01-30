//! Health API integration tests

use crate::common::TestApp;

mod common;

#[tokio::test]
async fn test_health_check() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    let response = client
        .get(&app.api_url("/health"))
        .send()
        .await
        .expect("Failed to call health endpoint");

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_readiness_check() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Test /health/ready endpoint if it exists
    let response = client
        .get(&app.api_url("/health/ready"))
        .send()
        .await
        .expect("Failed to call readiness endpoint");

    // Accept 200 or 404 (if endpoint doesn't exist)
    assert!(response.status().is_success() || response.status().as_u16() == 404);
}
