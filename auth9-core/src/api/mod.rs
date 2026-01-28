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
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    old_value: Option<serde_json::Value>,
    new_value: Option<serde_json::Value>,
) -> Result<()> {
    state
        .audit_repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id,
            old_value,
            new_value,
            ip_address: None,
        })
        .await
}
