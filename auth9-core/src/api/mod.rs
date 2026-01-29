//! REST API handlers

pub mod audit;
pub mod auth;
pub mod health;
pub mod role;
pub mod service;
pub mod tenant;
pub mod user;

use crate::error::Result;
use crate::repository::audit::CreateAuditLogInput;
use crate::repository::AuditRepository;
use crate::server::AppState;
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            data,
            pagination: PaginationMeta {
                page,
                per_page,
                total,
                total_pages,
            },
        }
    }
}

/// Success response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Message response (for delete, etc.)
#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

impl MessageResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub async fn write_audit_log(
    state: &AppState,
    headers: &HeaderMap,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    old_value: Option<serde_json::Value>,
    new_value: Option<serde_json::Value>,
) -> Result<()> {
    let actor_id = extract_actor_id(state, headers);
    let ip_address = extract_ip(headers);
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id,
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id,
            old_value,
            new_value,
            ip_address,
        })
        .await
}

fn extract_actor_id(state: &AppState, headers: &HeaderMap) -> Option<Uuid> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    let token = auth_str.strip_prefix("Bearer ")?;

    if let Ok(claims) = state.jwt_manager.verify_identity_token(token) {
        return Uuid::parse_str(&claims.sub).ok();
    }

    if let Ok(claims) = state.jwt_manager.verify_tenant_access_token(token, None) {
        return Uuid::parse_str(&claims.sub).ok();
    }

    None
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers.get("x-forwarded-for") {
        if let Ok(forwarded) = value.to_str() {
            if let Some(first) = forwarded.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    if let Some(value) = headers.get("x-real-ip") {
        if let Ok(real_ip) = value.to_str() {
            if !real_ip.trim().is_empty() {
                return Some(real_ip.trim().to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_query_defaults() {
        let query: PaginationQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 20);
    }

    #[test]
    fn test_pagination_query_custom_values() {
        let query: PaginationQuery = serde_json::from_str(r#"{"page": 5, "per_page": 50}"#).unwrap();
        assert_eq!(query.page, 5);
        assert_eq!(query.per_page, 50);
    }

    #[test]
    fn test_paginated_response_calculation() {
        let data = vec!["a", "b", "c"];
        let response = PaginatedResponse::new(data, 1, 10, 100);
        
        assert_eq!(response.pagination.page, 1);
        assert_eq!(response.pagination.per_page, 10);
        assert_eq!(response.pagination.total, 100);
        assert_eq!(response.pagination.total_pages, 10);
        assert_eq!(response.data.len(), 3);
    }

    #[test]
    fn test_paginated_response_partial_last_page() {
        let data: Vec<String> = vec![];
        let response = PaginatedResponse::new(data, 3, 10, 25);
        
        assert_eq!(response.pagination.total_pages, 3); // ceil(25/10) = 3
    }

    #[test]
    fn test_success_response() {
        let data = "test data";
        let response = SuccessResponse::new(data);
        assert_eq!(response.data, "test data");
    }

    #[test]
    fn test_message_response() {
        let response = MessageResponse::new("Operation successful");
        assert_eq!(response.message, "Operation successful");
    }

    #[test]
    fn test_message_response_from_string() {
        let response = MessageResponse::new(String::from("Dynamic message"));
        assert_eq!(response.message, "Dynamic message");
    }

    #[test]
    fn test_extract_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.1, 10.0.0.1".parse().unwrap());
        
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_ip_from_x_forwarded_for_single() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50".parse().unwrap());
        
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("203.0.113.50".to_string()));
    }

    #[test]
    fn test_extract_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "10.20.30.40".parse().unwrap());
        
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("10.20.30.40".to_string()));
    }

    #[test]
    fn test_extract_ip_prefers_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        headers.insert("x-real-ip", "5.6.7.8".parse().unwrap());
        
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn test_extract_ip_empty_headers() {
        let headers = HeaderMap::new();
        let ip = extract_ip(&headers);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_extract_ip_empty_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "".parse().unwrap());
        headers.insert("x-real-ip", "1.2.3.4".parse().unwrap());
        
        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("1.2.3.4".to_string()));
    }
}
