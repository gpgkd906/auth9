//! Audit Log API integration tests

use crate::common::TestApp;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_list_audit_logs() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // First, perform some actions that generate audit logs
    // Create a tenant (this should create an audit log)
    let tenant_res = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "Audit Test Tenant",
            "slug": format!("audit-test-{}", uuid::Uuid::new_v4())
        }))
        .send()
        .await
        .expect("Failed to create tenant");
    assert!(tenant_res.status().is_success());

    // Now query audit logs using offset/limit parameters
    let audit_res = client
        .get(&app.api_url("/api/v1/audit-logs"))
        .query(&[("limit", "10"), ("offset", "0")])
        .send()
        .await
        .expect("Failed to list audit logs");

    let status = audit_res.status();
    let audit_body: serde_json::Value = audit_res.json().await.unwrap();
    
    if !status.is_success() {
        panic!("Expected success but got: {} - body: {}", status, audit_body);
    }
    
    // Should have at least one audit log entry
    let logs = audit_body["data"].as_array().unwrap();
    assert!(!logs.is_empty());
}

#[tokio::test]
async fn test_list_audit_logs_with_filters() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Create a tenant to generate audit log
    let tenant_res = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({
            "name": "Audit Filter Tenant",
            "slug": format!("audit-filter-{}", uuid::Uuid::new_v4())
        }))
        .send()
        .await
        .unwrap();
    assert!(tenant_res.status().is_success());

    // Filter by resource_type using correct parameters
    let filter_res = client
        .get(&app.api_url("/api/v1/audit-logs"))
        .query(&[
            ("resource_type", "tenant"),
            ("limit", "100"),
            ("offset", "0"),
        ])
        .send()
        .await
        .expect("Failed to filter audit logs");

    assert!(filter_res.status().is_success(), "Expected success but got: {}", filter_res.status());
    let filter_body: serde_json::Value = filter_res.json().await.unwrap();
    let logs = filter_body["data"].as_array().unwrap();
    
    // All returned logs should be about tenants
    for log in logs {
        assert_eq!(log["resource_type"], "tenant");
    }
}

#[tokio::test]
async fn test_audit_log_pagination() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Create multiple tenants to generate multiple audit logs
    for i in 1..=5 {
        client
            .post(&app.api_url("/api/v1/tenants"))
            .json(&json!({
                "name": format!("Pagination Tenant {}", i),
                "slug": format!("pagination-{}-{}", i, uuid::Uuid::new_v4())
            }))
            .send()
            .await
            .unwrap();
    }

    // Get first page (offset=0, limit=2)
    let page1_res = client
        .get(&app.api_url("/api/v1/audit-logs"))
        .query(&[("limit", "2"), ("offset", "0")])
        .send()
        .await
        .expect("Failed to get page 1");

    assert!(page1_res.status().is_success(), "Expected success but got: {}", page1_res.status());
    let page1_body: serde_json::Value = page1_res.json().await.unwrap();
    let page1_logs = page1_body["data"].as_array().unwrap();
    assert_eq!(page1_logs.len(), 2);

    // Get second page (offset=2, limit=2)
    let page2_res = client
        .get(&app.api_url("/api/v1/audit-logs"))
        .query(&[("limit", "2"), ("offset", "2")])
        .send()
        .await
        .expect("Failed to get page 2");

    assert!(page2_res.status().is_success(), "Expected success but got: {}", page2_res.status());
    let page2_body: serde_json::Value = page2_res.json().await.unwrap();
    let page2_logs = page2_body["data"].as_array().unwrap();
    assert_eq!(page2_logs.len(), 2);

    // Verify different pages have different entries
    let page1_ids: Vec<&str> = page1_logs
        .iter()
        .map(|l| l["id"].as_i64().unwrap())
        .map(|_| "id") // Just verify they exist
        .collect();
    let page2_ids: Vec<i64> = page2_logs
        .iter()
        .map(|l| l["id"].as_i64().unwrap())
        .collect();
    let page1_id_nums: Vec<i64> = page1_logs
        .iter()
        .map(|l| l["id"].as_i64().unwrap())
        .collect();
    
    // Ensure page 2 IDs are different from page 1 IDs
    for id in &page2_ids {
        assert!(!page1_id_nums.contains(id), "Page 2 should have different entries than page 1");
    }
}
