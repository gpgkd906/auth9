//! Analytics and security alert domain models

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use validator::Validate;

/// Login event types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginEventType {
    Success,
    FailedPassword,
    FailedMfa,
    Locked,
    Social,
}

impl std::str::FromStr for LoginEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "success" => Ok(LoginEventType::Success),
            "failed_password" => Ok(LoginEventType::FailedPassword),
            "failed_mfa" => Ok(LoginEventType::FailedMfa),
            "locked" => Ok(LoginEventType::Locked),
            "social" => Ok(LoginEventType::Social),
            _ => Err(format!("Unknown login event type: {}", s)),
        }
    }
}

impl std::fmt::Display for LoginEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginEventType::Success => write!(f, "success"),
            LoginEventType::FailedPassword => write!(f, "failed_password"),
            LoginEventType::FailedMfa => write!(f, "failed_mfa"),
            LoginEventType::Locked => write!(f, "locked"),
            LoginEventType::Social => write!(f, "social"),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for LoginEventType {
    fn decode(
        value: sqlx::mysql::MySqlValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s: String = sqlx::Decode::<'r, sqlx::MySql>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Type<sqlx::MySql> for LoginEventType {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for LoginEventType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.to_string();
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s.as_str(), buf)
    }
}

/// Login event entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LoginEvent {
    pub id: i64,
    pub user_id: Option<StringUuid>,
    pub email: Option<String>,
    pub tenant_id: Option<StringUuid>,
    pub event_type: LoginEventType,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub session_id: Option<StringUuid>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a login event
#[derive(Debug, Clone)]
pub struct CreateLoginEventInput {
    pub user_id: Option<StringUuid>,
    pub email: Option<String>,
    pub tenant_id: Option<StringUuid>,
    pub event_type: LoginEventType,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub session_id: Option<StringUuid>,
    pub failure_reason: Option<String>,
}

/// Security alert types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityAlertType {
    BruteForce,
    NewDevice,
    ImpossibleTravel,
    SuspiciousIp,
}

impl std::str::FromStr for SecurityAlertType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "brute_force" => Ok(SecurityAlertType::BruteForce),
            "new_device" => Ok(SecurityAlertType::NewDevice),
            "impossible_travel" => Ok(SecurityAlertType::ImpossibleTravel),
            "suspicious_ip" => Ok(SecurityAlertType::SuspiciousIp),
            _ => Err(format!("Unknown security alert type: {}", s)),
        }
    }
}

impl std::fmt::Display for SecurityAlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityAlertType::BruteForce => write!(f, "brute_force"),
            SecurityAlertType::NewDevice => write!(f, "new_device"),
            SecurityAlertType::ImpossibleTravel => write!(f, "impossible_travel"),
            SecurityAlertType::SuspiciousIp => write!(f, "suspicious_ip"),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for SecurityAlertType {
    fn decode(
        value: sqlx::mysql::MySqlValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s: String = sqlx::Decode::<'r, sqlx::MySql>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Type<sqlx::MySql> for SecurityAlertType {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for SecurityAlertType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.to_string();
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s.as_str(), buf)
    }
}

/// Security alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::str::FromStr for AlertSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(AlertSeverity::Low),
            "medium" => Ok(AlertSeverity::Medium),
            "high" => Ok(AlertSeverity::High),
            "critical" => Ok(AlertSeverity::Critical),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Low => write!(f, "low"),
            AlertSeverity::Medium => write!(f, "medium"),
            AlertSeverity::High => write!(f, "high"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for AlertSeverity {
    fn decode(
        value: sqlx::mysql::MySqlValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let s: String = sqlx::Decode::<'r, sqlx::MySql>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Type<sqlx::MySql> for AlertSeverity {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for AlertSeverity {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.to_string();
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s.as_str(), buf)
    }
}

/// Security alert entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SecurityAlert {
    pub id: StringUuid,
    pub user_id: Option<StringUuid>,
    pub tenant_id: Option<StringUuid>,
    pub alert_type: SecurityAlertType,
    pub severity: AlertSeverity,
    #[sqlx(json)]
    pub details: Option<serde_json::Value>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<StringUuid>,
    pub created_at: DateTime<Utc>,
}

impl Default for SecurityAlert {
    fn default() -> Self {
        Self {
            id: StringUuid::new_v4(),
            user_id: None,
            tenant_id: None,
            alert_type: SecurityAlertType::BruteForce,
            severity: AlertSeverity::Medium,
            details: None,
            resolved_at: None,
            resolved_by: None,
            created_at: Utc::now(),
        }
    }
}

/// Input for creating a security alert
#[derive(Debug, Clone)]
pub struct CreateSecurityAlertInput {
    pub user_id: Option<StringUuid>,
    pub tenant_id: Option<StringUuid>,
    pub alert_type: SecurityAlertType,
    pub severity: AlertSeverity,
    pub details: Option<serde_json::Value>,
}

/// Webhook entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Webhook {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    #[sqlx(json)]
    pub events: Vec<String>,
    pub enabled: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Webhook {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            name: String::new(),
            url: String::new(),
            secret: None,
            events: Vec::new(),
            enabled: true,
            last_triggered_at: None,
            failure_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for creating a webhook
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateWebhookInput {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(url)]
    pub url: String,
    pub secret: Option<String>,
    #[validate(length(min = 1))]
    pub events: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Input for updating a webhook
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateWebhookInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(url)]
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Webhook event types
pub const WEBHOOK_EVENTS: &[&str] = &[
    "login.success",
    "login.failed",
    "user.created",
    "user.updated",
    "user.deleted",
    "password.changed",
    "mfa.enabled",
    "mfa.disabled",
    "session.revoked",
    "security.alert",
];

/// Webhook event payload sent to webhook endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

/// Login statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginStats {
    pub total_logins: i64,
    pub successful_logins: i64,
    pub failed_logins: i64,
    pub unique_users: i64,
    pub by_event_type: HashMap<String, i64>,
    pub by_device_type: HashMap<String, i64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl Default for LoginStats {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            total_logins: 0,
            successful_logins: 0,
            failed_logins: 0,
            unique_users: 0,
            by_event_type: HashMap::new(),
            by_device_type: HashMap::new(),
            period_start: now,
            period_end: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_login_event_type_display() {
        assert_eq!(format!("{}", LoginEventType::Success), "success");
        assert_eq!(format!("{}", LoginEventType::FailedPassword), "failed_password");
        assert_eq!(format!("{}", LoginEventType::FailedMfa), "failed_mfa");
    }

    #[test]
    fn test_login_event_type_from_str() {
        assert_eq!("success".parse::<LoginEventType>().unwrap(), LoginEventType::Success);
        assert_eq!("failed_password".parse::<LoginEventType>().unwrap(), LoginEventType::FailedPassword);
        assert!("invalid".parse::<LoginEventType>().is_err());
    }

    #[test]
    fn test_security_alert_type_display() {
        assert_eq!(format!("{}", SecurityAlertType::BruteForce), "brute_force");
        assert_eq!(format!("{}", SecurityAlertType::NewDevice), "new_device");
        assert_eq!(format!("{}", SecurityAlertType::ImpossibleTravel), "impossible_travel");
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(format!("{}", AlertSeverity::Low), "low");
        assert_eq!(format!("{}", AlertSeverity::Critical), "critical");
    }

    #[test]
    fn test_security_alert_default() {
        let alert = SecurityAlert::default();
        assert!(!alert.id.is_nil());
        assert!(alert.user_id.is_none());
        assert!(alert.resolved_at.is_none());
    }

    #[test]
    fn test_webhook_default() {
        let webhook = Webhook::default();
        assert!(!webhook.id.is_nil());
        assert!(webhook.enabled);
        assert_eq!(webhook.failure_count, 0);
        assert!(webhook.events.is_empty());
    }

    #[test]
    fn test_create_webhook_input_valid() {
        let input = CreateWebhookInput {
            name: "My Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: Some("secret123".to_string()),
            events: vec!["login.success".to_string()],
            enabled: true,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_webhook_input_invalid_url() {
        let input = CreateWebhookInput {
            name: "My Webhook".to_string(),
            url: "not-a-url".to_string(),
            secret: None,
            events: vec!["login.success".to_string()],
            enabled: true,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_webhook_input_empty_events() {
        let input = CreateWebhookInput {
            name: "My Webhook".to_string(),
            url: "https://example.com/webhook".to_string(),
            secret: None,
            events: vec![],
            enabled: true,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_login_stats_default() {
        let stats = LoginStats::default();
        assert_eq!(stats.total_logins, 0);
        assert_eq!(stats.successful_logins, 0);
        assert!(stats.by_event_type.is_empty());
    }

    #[test]
    fn test_webhook_events_list() {
        assert!(WEBHOOK_EVENTS.contains(&"login.success"));
        assert!(WEBHOOK_EVENTS.contains(&"user.created"));
        assert!(WEBHOOK_EVENTS.contains(&"security.alert"));
    }

    #[test]
    fn test_login_event_serialization() {
        let event = LoginEvent {
            id: 1,
            user_id: Some(StringUuid::new_v4()),
            email: Some("test@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Success,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            device_type: Some("desktop".to_string()),
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("desktop"));
    }
}
