//! Action domain models for Auth9 Actions system

use crate::domain::StringUuid;
use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;
use validator::Validate;

/// Action trigger types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionTrigger {
    /// Triggered after successful login
    PostLogin,
    /// Triggered before user registration
    PreUserRegistration,
    /// Triggered after user registration
    PostUserRegistration,
    /// Triggered after password change
    PostChangePassword,
    /// Triggered after email verification
    PostEmailVerification,
    /// Triggered before token refresh
    PreTokenRefresh,
}

impl ActionTrigger {
    /// Get all available triggers
    pub fn all() -> Vec<ActionTrigger> {
        vec![
            ActionTrigger::PostLogin,
            ActionTrigger::PreUserRegistration,
            ActionTrigger::PostUserRegistration,
            ActionTrigger::PostChangePassword,
            ActionTrigger::PostEmailVerification,
            ActionTrigger::PreTokenRefresh,
        ]
    }

    /// Convert to string ID
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionTrigger::PostLogin => "post-login",
            ActionTrigger::PreUserRegistration => "pre-user-registration",
            ActionTrigger::PostUserRegistration => "post-user-registration",
            ActionTrigger::PostChangePassword => "post-change-password",
            ActionTrigger::PostEmailVerification => "post-email-verification",
            ActionTrigger::PreTokenRefresh => "pre-token-refresh",
        }
    }

    /// Parse from string ID
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "post-login" => Ok(ActionTrigger::PostLogin),
            "pre-user-registration" => Ok(ActionTrigger::PreUserRegistration),
            "post-user-registration" => Ok(ActionTrigger::PostUserRegistration),
            "post-change-password" => Ok(ActionTrigger::PostChangePassword),
            "post-email-verification" => Ok(ActionTrigger::PostEmailVerification),
            "pre-token-refresh" => Ok(ActionTrigger::PreTokenRefresh),
            _ => Err(AppError::BadRequest(format!("Invalid trigger: {}", s))),
        }
    }
}

impl fmt::Display for ActionTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Action entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Action {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub name: String,
    pub description: Option<String>,
    pub trigger_id: String,
    pub script: String,
    pub enabled: bool,
    pub execution_order: i32,
    pub timeout_ms: i32,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub execution_count: i64,
    pub error_count: i64,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Action {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            name: String::new(),
            description: None,
            trigger_id: String::new(),
            script: String::new(),
            enabled: true,
            execution_order: 0,
            timeout_ms: 3000,
            last_executed_at: None,
            execution_count: 0,
            error_count: 0,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for creating an action
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateActionInput {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub trigger_id: String,
    #[validate(length(min = 1))]
    pub script: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub execution_order: i32,
    #[validate(range(min = 1, max = 30000))]
    #[serde(default = "default_timeout")]
    pub timeout_ms: i32,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> i32 {
    3000
}

/// Input for updating an action
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
pub struct UpdateActionInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub script: Option<String>,
    pub enabled: Option<bool>,
    pub execution_order: Option<i32>,
    #[validate(range(min = 1, max = 30000))]
    pub timeout_ms: Option<i32>,
}

/// Context passed to action scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub user: ActionContextUser,
    pub tenant: ActionContextTenant,
    pub request: ActionContextRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claims: Option<HashMap<String, serde_json::Value>>,
}

/// User information in action context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContextUser {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub mfa_enabled: bool,
}

/// Tenant information in action context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContextTenant {
    pub id: String,
    pub slug: String,
    pub name: String,
}

/// Request information in action context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContextRequest {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionExecution {
    pub id: StringUuid,
    pub action_id: StringUuid,
    pub tenant_id: StringUuid,
    pub trigger_id: String,
    pub user_id: Option<StringUuid>,
    pub success: bool,
    pub duration_ms: i32,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
}

/// Input for batch upsert (create or update)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpsertActionInput {
    pub id: Option<StringUuid>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub trigger_id: String,
    #[validate(length(min = 1))]
    pub script: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub execution_order: i32,
    #[validate(range(min = 1, max = 30000))]
    #[serde(default = "default_timeout")]
    pub timeout_ms: i32,
}

/// Batch upsert response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpsertResponse {
    pub created: Vec<Action>,
    pub updated: Vec<Action>,
    pub errors: Vec<BatchError>,
}

/// Batch operation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    pub input_index: usize,
    pub name: String,
    pub error: String,
}

/// Action test response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestActionResponse {
    pub success: bool,
    pub duration_ms: i32,
    pub modified_context: Option<ActionContext>,
    pub error_message: Option<String>,
    pub console_logs: Vec<String>,
}

/// Log query filter
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LogQueryFilter {
    pub action_id: Option<StringUuid>,
    pub user_id: Option<StringUuid>,
    pub success: Option<bool>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Action statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStats {
    pub execution_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    pub last_24h_count: i64,
}
