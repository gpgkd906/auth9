//! Shared HTTP support types and helpers.

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

/// Require platform admin with Identity token for platform-level mutations.
/// TenantAccess tokens with platform admin email are rejected to prevent
/// tenant isolation bypass via token exchange.
pub(crate) async fn require_platform_admin_identity<S: HasServices>(
    state: &S,
    auth: &AuthUser,
) -> Result<()> {
    policy::require_platform_admin_identity(state, auth)
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

    if let Ok(claims) = state
        .jwt_manager()
        .verify_tenant_access_token_any_audience(token)
    {
        return Uuid::parse_str(&claims.sub).ok();
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
}
