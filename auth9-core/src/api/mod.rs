//! REST API shared utilities (response types, pagination, audit helpers)

pub mod metrics;

use crate::error::{AppError, Result};
use crate::middleware::auth::AuthUser;
use crate::policy;
use crate::repository::audit::CreateAuditLogInput;
use crate::repository::AuditRepository;
use crate::state::HasServices;
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maximum allowed per_page value for pagination
pub(crate) const MAX_PER_PAGE: i64 = 100;

/// Require platform admin access. Returns Forbidden if the user is not a platform admin.
pub(crate) async fn require_platform_admin_with_db<S: HasServices>(
    state: &S,
    auth: &AuthUser,
) -> Result<()> {
    policy::require_platform_admin_with_db(state, auth)
        .await
        .map_err(|e| match e {
            AppError::Forbidden(_) => AppError::Forbidden("Platform admin required".to_string()),
            other => other,
        })
}

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PaginationQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
}

pub(crate) fn default_page() -> i64 {
    1
}

pub(crate) fn default_per_page() -> i64 {
    20
}

/// Reject page values less than 1
pub(crate) fn deserialize_page<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    if value < 1 {
        return Err(serde::de::Error::custom(
            "page must be a positive integer (>= 1)",
        ));
    }
    Ok(value)
}

/// Reject per_page values less than 1, clamp to MAX_PER_PAGE
pub(crate) fn deserialize_per_page<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    if value < 1 {
        return Err(serde::de::Error::custom(
            "per_page must be a positive integer (>= 1)",
        ));
    }
    Ok(value.min(MAX_PER_PAGE))
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    pub data: T,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Message response (for delete, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

/// Write an audit log entry using the HasServices trait
pub async fn write_audit_log_generic<S: HasServices>(
    state: &S,
    headers: &HeaderMap,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    old_value: Option<serde_json::Value>,
    new_value: Option<serde_json::Value>,
) -> Result<()> {
    let actor_id = extract_actor_id_generic(state, headers);
    let ip_address = extract_ip(headers);
    state
        .audit_repo()
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

/// Extract actor ID from the Authorization header using the HasServices trait
pub(crate) fn extract_actor_id_generic<S: HasServices>(
    state: &S,
    headers: &HeaderMap,
) -> Option<Uuid> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    let token = auth_str.strip_prefix("Bearer ")?;

    if let Ok(claims) = state.jwt_manager().verify_identity_token(token) {
        return Uuid::parse_str(&claims.sub).ok();
    }

    let allowed = &state.config().jwt_tenant_access_allowed_audiences;
    if !allowed.is_empty() {
        if let Ok(claims) = state
            .jwt_manager()
            .verify_tenant_access_token_strict(token, allowed)
        {
            return Uuid::parse_str(&claims.sub).ok();
        }
    } else if !state.config().is_production() {
        if let Ok(claims) = {
            #[allow(deprecated)]
            state.jwt_manager().verify_tenant_access_token(token, None)
        } {
            return Uuid::parse_str(&claims.sub).ok();
        }
    }

    None
}

pub(crate) fn extract_ip(headers: &HeaderMap) -> Option<String> {
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
        let query: PaginationQuery =
            serde_json::from_str(r#"{"page": 5, "per_page": 50}"#).unwrap();
        assert_eq!(query.page, 5);
        assert_eq!(query.per_page, 50);
    }

    #[test]
    fn test_pagination_query_per_page_clamped_to_max() {
        let query: PaginationQuery =
            serde_json::from_str(r#"{"page": 1, "per_page": 1000000}"#).unwrap();
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn test_pagination_query_per_page_negative_rejected() {
        let result = serde_json::from_str::<PaginationQuery>(r#"{"page": 1, "per_page": -5}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_query_per_page_zero_rejected() {
        let result = serde_json::from_str::<PaginationQuery>(r#"{"page": 1, "per_page": 0}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_query_page_zero_rejected() {
        let result = serde_json::from_str::<PaginationQuery>(r#"{"page": 0, "per_page": 20}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_query_page_negative_rejected() {
        let result = serde_json::from_str::<PaginationQuery>(r#"{"page": -1, "per_page": 20}"#);
        assert!(result.is_err());
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

    #[test]
    fn test_extract_ip_whitespace_only() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "   ".parse().unwrap());
        headers.insert("x-real-ip", "  ".parse().unwrap());

        let ip = extract_ip(&headers);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_extract_ip_x_real_ip_trimmed() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "  10.0.0.1  ".parse().unwrap());

        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_paginated_response_single_item() {
        let data = vec!["single"];
        let response = PaginatedResponse::new(data, 1, 10, 1);

        assert_eq!(response.pagination.total_pages, 1);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_paginated_response_empty() {
        let data: Vec<String> = vec![];
        let response = PaginatedResponse::new(data, 1, 10, 0);

        assert_eq!(response.pagination.total, 0);
        assert_eq!(response.pagination.total_pages, 0);
        assert!(response.data.is_empty());
    }

    #[test]
    fn test_paginated_response_exact_multiple() {
        let data = vec!["a", "b"];
        let response = PaginatedResponse::new(data, 1, 2, 10);

        assert_eq!(response.pagination.total_pages, 5); // 10/2 = 5 exact
    }

    #[test]
    fn test_paginated_response_serialization() {
        let data = vec!["test"];
        let response = PaginatedResponse::new(data, 2, 25, 100);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"page\":2"));
        assert!(json.contains("\"per_page\":25"));
        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"total_pages\":4"));
    }

    #[test]
    fn test_pagination_query_serialization() {
        let query = PaginationQuery {
            page: 3,
            per_page: 15,
        };
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("\"page\":3"));
        assert!(json.contains("\"per_page\":15"));
    }

    #[test]
    fn test_success_response_with_complex_data() {
        #[derive(serde::Serialize)]
        struct TestData {
            id: u32,
            name: String,
        }

        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"name\":\"Test\""));
    }

    #[test]
    fn test_extract_ip_multiple_proxies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "192.168.1.100, 10.0.0.1, 172.16.0.1".parse().unwrap(),
        );

        let ip = extract_ip(&headers);
        // Should return the first (client) IP
        assert_eq!(ip, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_extract_ip_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "::1".parse().unwrap());

        let ip = extract_ip(&headers);
        assert_eq!(ip, Some("::1".to_string()));
    }

    #[test]
    fn test_extract_ip_ipv6_full() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-real-ip",
            "2001:0db8:85a3:0000:0000:8a2e:0370:7334".parse().unwrap(),
        );

        let ip = extract_ip(&headers);
        assert_eq!(
            ip,
            Some("2001:0db8:85a3:0000:0000:8a2e:0370:7334".to_string())
        );
    }
}
