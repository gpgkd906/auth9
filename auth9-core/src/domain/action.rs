//! Action domain models for Auth9 Actions system

use crate::domain::StringUuid;
use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;
use validator::Validate;

/// Action trigger types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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
}

impl FromStr for ActionTrigger {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Action {
    pub id: StringUuid,
    pub tenant_id: Option<StringUuid>,
    pub service_id: StringUuid,
    pub name: String,
    pub description: Option<String>,
    pub trigger_id: String,
    pub script: String,
    pub enabled: bool,
    pub strict_mode: bool,
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
            tenant_id: None,
            service_id: StringUuid::new_v4(),
            name: String::new(),
            description: None,
            trigger_id: String::new(),
            script: String::new(),
            enabled: true,
            strict_mode: false,
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
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
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
    pub strict_mode: bool,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateActionInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    #[validate(length(min = 1))]
    pub script: Option<String>,
    pub enabled: Option<bool>,
    pub strict_mode: Option<bool>,
    pub execution_order: Option<i32>,
    #[validate(range(min = 1, max = 30000))]
    pub timeout_ms: Option<i32>,
}

/// Lightweight request metadata for cross-layer IP/UA propagation.
/// Extracted at the API/gRPC boundary and passed down to service layer
/// for use in ActionContext construction and audit logging.
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Context passed to action scripts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionContext {
    pub user: ActionContextUser,
    pub tenant: ActionContextTenant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ActionContextService>,
    pub request: ActionContextRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claims: Option<HashMap<String, serde_json::Value>>,
}

/// Service information in action context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionContextService {
    pub id: String,
    pub name: String,
    pub client_id: Option<String>,
}

/// User information in action context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionContextUser {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub mfa_enabled: bool,
}

/// Tenant information in action context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionContextTenant {
    pub id: String,
    pub slug: String,
    pub name: String,
}

/// Request information in action context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionContextRequest {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Action execution result
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ActionExecution {
    pub id: StringUuid,
    pub action_id: StringUuid,
    pub tenant_id: Option<StringUuid>,
    pub service_id: StringUuid,
    pub trigger_id: String,
    pub user_id: Option<StringUuid>,
    pub success: bool,
    pub duration_ms: i32,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
}

/// Input for batch upsert (create or update)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
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
    pub strict_mode: bool,
    #[serde(default)]
    pub execution_order: i32,
    #[validate(range(min = 1, max = 30000))]
    #[serde(default = "default_timeout")]
    pub timeout_ms: i32,
}

/// Batch upsert response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchUpsertResponse {
    pub created: Vec<Action>,
    pub updated: Vec<Action>,
    pub errors: Vec<BatchError>,
}

/// Batch operation error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchError {
    pub input_index: usize,
    pub name: String,
    pub error: String,
}

/// Action test response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TestActionResponse {
    pub success: bool,
    pub duration_ms: i32,
    pub modified_context: Option<ActionContext>,
    pub error_message: Option<String>,
    pub console_logs: Vec<String>,
}

/// Log query filter
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct LogQueryFilter {
    pub action_id: Option<StringUuid>,
    pub user_id: Option<StringUuid>,
    pub success: Option<bool>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Security configuration for async action execution
#[derive(Debug, Clone)]
pub struct AsyncActionConfig {
    /// Allowed domains for fetch (empty = block all)
    pub allowed_domains: Vec<String>,
    /// Per-request timeout in milliseconds (default: 10s)
    pub request_timeout_ms: u64,
    /// Max response body size in bytes (default: 1MB)
    pub max_response_bytes: usize,
    /// Max HTTP requests per single action execution (default: 5)
    pub max_requests_per_execution: usize,
    /// Allow requests to private/loopback IPs (default: false, set true only for testing)
    pub allow_private_ips: bool,
    /// Max V8 heap size in MB per isolate (default: 64MB)
    pub max_heap_mb: usize,
}

impl Default for AsyncActionConfig {
    fn default() -> Self {
        Self {
            allowed_domains: vec![],
            request_timeout_ms: 10_000,
            max_response_bytes: 1_048_576,
            max_requests_per_execution: 5,
            allow_private_ips: false,
            max_heap_mb: 64,
        }
    }
}

/// Action statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActionStats {
    pub execution_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    pub last_24h_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // 1. ActionTrigger::all() returns all 6 variants
    #[test]
    fn test_action_trigger_all_returns_six_variants() {
        let all = ActionTrigger::all();
        assert_eq!(all.len(), 6);
        assert_eq!(all[0], ActionTrigger::PostLogin);
        assert_eq!(all[1], ActionTrigger::PreUserRegistration);
        assert_eq!(all[2], ActionTrigger::PostUserRegistration);
        assert_eq!(all[3], ActionTrigger::PostChangePassword);
        assert_eq!(all[4], ActionTrigger::PostEmailVerification);
        assert_eq!(all[5], ActionTrigger::PreTokenRefresh);
    }

    // 2. ActionTrigger::as_str() for each variant
    #[test]
    fn test_action_trigger_as_str() {
        assert_eq!(ActionTrigger::PostLogin.as_str(), "post-login");
        assert_eq!(
            ActionTrigger::PreUserRegistration.as_str(),
            "pre-user-registration"
        );
        assert_eq!(
            ActionTrigger::PostUserRegistration.as_str(),
            "post-user-registration"
        );
        assert_eq!(
            ActionTrigger::PostChangePassword.as_str(),
            "post-change-password"
        );
        assert_eq!(
            ActionTrigger::PostEmailVerification.as_str(),
            "post-email-verification"
        );
        assert_eq!(ActionTrigger::PreTokenRefresh.as_str(), "pre-token-refresh");
    }

    // 3. ActionTrigger parsing valid + invalid
    #[test]
    fn test_action_trigger_from_str_valid() {
        assert_eq!(
            "post-login".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PostLogin
        );
        assert_eq!(
            "pre-user-registration".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PreUserRegistration
        );
        assert_eq!(
            "post-user-registration".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PostUserRegistration
        );
        assert_eq!(
            "post-change-password".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PostChangePassword
        );
        assert_eq!(
            "post-email-verification".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PostEmailVerification
        );
        assert_eq!(
            "pre-token-refresh".parse::<ActionTrigger>().unwrap(),
            ActionTrigger::PreTokenRefresh
        );
    }

    #[test]
    fn test_action_trigger_from_str_invalid() {
        let result = "invalid-trigger".parse::<ActionTrigger>();
        assert!(result.is_err());
    }

    // 4. ActionTrigger Display impl
    #[test]
    fn test_action_trigger_display() {
        assert_eq!(format!("{}", ActionTrigger::PostLogin), "post-login");
        assert_eq!(
            format!("{}", ActionTrigger::PreUserRegistration),
            "pre-user-registration"
        );
        assert_eq!(
            format!("{}", ActionTrigger::PostUserRegistration),
            "post-user-registration"
        );
        assert_eq!(
            format!("{}", ActionTrigger::PostChangePassword),
            "post-change-password"
        );
        assert_eq!(
            format!("{}", ActionTrigger::PostEmailVerification),
            "post-email-verification"
        );
        assert_eq!(
            format!("{}", ActionTrigger::PreTokenRefresh),
            "pre-token-refresh"
        );
    }

    // 5. Action::default() field values
    #[test]
    fn test_action_default() {
        let action = Action::default();
        assert_eq!(action.name, "");
        assert!(action.description.is_none());
        assert_eq!(action.trigger_id, "");
        assert_eq!(action.script, "");
        assert!(action.enabled);
        assert!(!action.strict_mode);
        assert_eq!(action.execution_order, 0);
        assert_eq!(action.timeout_ms, 3000);
        assert!(action.last_executed_at.is_none());
        assert_eq!(action.execution_count, 0);
        assert_eq!(action.error_count, 0);
        assert!(action.last_error.is_none());
        // created_at and updated_at should be close to now
        let now = Utc::now();
        assert!((now - action.created_at).num_seconds() < 2);
        assert!((now - action.updated_at).num_seconds() < 2);
    }

    // 6. CreateActionInput serde defaults
    #[test]
    fn test_create_action_input_serde_defaults() {
        let json = r#"{
            "name": "My Action",
            "trigger_id": "post-login",
            "script": "console.log('hello');"
        }"#;
        let input: CreateActionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "My Action");
        assert_eq!(input.trigger_id, "post-login");
        assert_eq!(input.script, "console.log('hello');");
        assert!(input.description.is_none());
        assert!(input.enabled); // default_true
        assert!(!input.strict_mode); // serde default
        assert_eq!(input.execution_order, 0); // serde default
        assert_eq!(input.timeout_ms, 3000); // default_timeout
    }

    #[test]
    fn test_create_action_input_serde_overrides() {
        let json = r#"{
            "name": "My Action",
            "trigger_id": "post-login",
            "script": "console.log('hello');",
            "description": "A description",
            "enabled": false,
            "execution_order": 5,
            "timeout_ms": 5000
        }"#;
        let input: CreateActionInput = serde_json::from_str(json).unwrap();
        assert!(!input.enabled);
        assert_eq!(input.execution_order, 5);
        assert_eq!(input.timeout_ms, 5000);
        assert_eq!(input.description, Some("A description".to_string()));
    }

    // 7. UpdateActionInput::default() all None
    #[test]
    fn test_update_action_input_default() {
        let input = UpdateActionInput::default();
        assert!(input.name.is_none());
        assert!(input.description.is_none());
        assert!(input.script.is_none());
        assert!(input.enabled.is_none());
        assert!(input.strict_mode.is_none());
        assert!(input.execution_order.is_none());
        assert!(input.timeout_ms.is_none());
    }

    // 8. ActionContext serialization/deserialization
    #[test]
    fn test_action_context_serde_roundtrip() {
        let ctx = ActionContext {
            user: ActionContextUser {
                id: "user-1".to_string(),
                email: "test@example.com".to_string(),
                display_name: Some("Test User".to_string()),
                mfa_enabled: false,
            },
            tenant: ActionContextTenant {
                id: "tenant-1".to_string(),
                slug: "my-tenant".to_string(),
                name: "My Tenant".to_string(),
            },
            service: None,
            request: ActionContextRequest {
                ip: Some("127.0.0.1".to_string()),
                user_agent: Some("TestAgent/1.0".to_string()),
                timestamp: Utc::now(),
            },
            claims: None,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: ActionContext = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user.id, "user-1");
        assert_eq!(deserialized.user.email, "test@example.com");
        assert_eq!(
            deserialized.user.display_name,
            Some("Test User".to_string())
        );
        assert!(!deserialized.user.mfa_enabled);
        assert_eq!(deserialized.tenant.id, "tenant-1");
        assert_eq!(deserialized.tenant.slug, "my-tenant");
        assert_eq!(deserialized.tenant.name, "My Tenant");
        assert_eq!(deserialized.request.ip, Some("127.0.0.1".to_string()));
        assert!(deserialized.claims.is_none());
    }

    #[test]
    fn test_action_context_claims_skipped_when_none() {
        let ctx = ActionContext {
            user: ActionContextUser {
                id: "u".to_string(),
                email: "e".to_string(),
                display_name: None,
                mfa_enabled: true,
            },
            tenant: ActionContextTenant {
                id: "t".to_string(),
                slug: "s".to_string(),
                name: "n".to_string(),
            },
            service: None,
            request: ActionContextRequest {
                ip: None,
                user_agent: None,
                timestamp: Utc::now(),
            },
            claims: None,
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(!json.contains("claims"));
    }

    #[test]
    fn test_action_context_with_claims() {
        let mut claims = HashMap::new();
        claims.insert(
            "role".to_string(),
            serde_json::Value::String("admin".to_string()),
        );

        let ctx = ActionContext {
            user: ActionContextUser {
                id: "u".to_string(),
                email: "e".to_string(),
                display_name: None,
                mfa_enabled: false,
            },
            tenant: ActionContextTenant {
                id: "t".to_string(),
                slug: "s".to_string(),
                name: "n".to_string(),
            },
            service: None,
            request: ActionContextRequest {
                ip: None,
                user_agent: None,
                timestamp: Utc::now(),
            },
            claims: Some(claims),
        };

        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("claims"));
        let deserialized: ActionContext = serde_json::from_str(&json).unwrap();
        let c = deserialized.claims.unwrap();
        assert_eq!(c.get("role").unwrap(), "admin");
    }

    // 9. LogQueryFilter::default() all None
    #[test]
    fn test_log_query_filter_default() {
        let filter = LogQueryFilter::default();
        assert!(filter.action_id.is_none());
        assert!(filter.user_id.is_none());
        assert!(filter.success.is_none());
        assert!(filter.from.is_none());
        assert!(filter.to.is_none());
        assert!(filter.limit.is_none());
        assert!(filter.offset.is_none());
    }

    // 10. BatchUpsertResponse, BatchError, TestActionResponse, ActionStats serialization
    #[test]
    fn test_batch_upsert_response_serde() {
        let resp = BatchUpsertResponse {
            created: vec![],
            updated: vec![],
            errors: vec![BatchError {
                input_index: 0,
                name: "bad-action".to_string(),
                error: "invalid script".to_string(),
            }],
        };

        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: BatchUpsertResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.created.is_empty());
        assert!(deserialized.updated.is_empty());
        assert_eq!(deserialized.errors.len(), 1);
        assert_eq!(deserialized.errors[0].input_index, 0);
        assert_eq!(deserialized.errors[0].name, "bad-action");
        assert_eq!(deserialized.errors[0].error, "invalid script");
    }

    #[test]
    fn test_batch_error_serde() {
        let err = BatchError {
            input_index: 2,
            name: "action-x".to_string(),
            error: "timeout".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: BatchError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input_index, 2);
        assert_eq!(deserialized.name, "action-x");
        assert_eq!(deserialized.error, "timeout");
    }

    #[test]
    fn test_test_action_response_serde() {
        let resp = TestActionResponse {
            success: true,
            duration_ms: 42,
            modified_context: None,
            error_message: None,
            console_logs: vec!["log1".to_string(), "log2".to_string()],
        };

        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: TestActionResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.duration_ms, 42);
        assert!(deserialized.modified_context.is_none());
        assert!(deserialized.error_message.is_none());
        assert_eq!(deserialized.console_logs, vec!["log1", "log2"]);
    }

    #[test]
    fn test_action_stats_serde() {
        let stats = ActionStats {
            execution_count: 100,
            error_count: 5,
            avg_duration_ms: 12.5,
            last_24h_count: 42,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: ActionStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_count, 100);
        assert_eq!(deserialized.error_count, 5);
        assert!((deserialized.avg_duration_ms - 12.5).abs() < f64::EPSILON);
        assert_eq!(deserialized.last_24h_count, 42);
    }

    // 11. Serde round-trip for kebab-case ActionTrigger
    #[test]
    fn test_action_trigger_serde_kebab_case() {
        for trigger in ActionTrigger::all() {
            let json = serde_json::to_string(&trigger).unwrap();
            let deserialized: ActionTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(trigger, deserialized);
            // Verify the JSON string matches the kebab-case as_str() value
            assert_eq!(json, format!("\"{}\"", trigger.as_str()));
        }
    }

    #[test]
    fn test_upsert_action_input_serde_defaults() {
        let json = r#"{
            "name": "Upsert Action",
            "trigger_id": "post-login",
            "script": "return;"
        }"#;
        let input: UpsertActionInput = serde_json::from_str(json).unwrap();
        assert!(input.id.is_none());
        assert_eq!(input.name, "Upsert Action");
        assert!(input.enabled); // default_true
        assert!(!input.strict_mode); // serde default
        assert_eq!(input.timeout_ms, 3000); // default_timeout
        assert_eq!(input.execution_order, 0);
    }
}
