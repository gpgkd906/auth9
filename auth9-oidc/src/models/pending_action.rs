use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Action type for pending / required actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    VerifyEmail,
    UpdatePassword,
    ConfigureTotp,
    ConfigureWebAuthn,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VerifyEmail => "verify_email",
            Self::UpdatePassword => "update_password", // pragma: allowlist secret
            Self::ConfigureTotp => "configure_totp",
            Self::ConfigureWebAuthn => "configure_webauthn",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "verify_email" => Some(Self::VerifyEmail),
            "update_password" => Some(Self::UpdatePassword),
            "configure_totp" => Some(Self::ConfigureTotp),
            "configure_webauthn" => Some(Self::ConfigureWebAuthn),
            _ => None,
        }
    }
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Action status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    Completed,
    Cancelled,
}

impl ActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stored pending action row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub user_id: String,
    pub action_type: ActionType,
    pub status: ActionStatus,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Input for creating a pending action.
#[derive(Debug, Clone)]
pub struct CreatePendingActionInput {
    pub user_id: String,
    pub action_type: ActionType,
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_type_roundtrip() {
        for at in [
            ActionType::VerifyEmail,
            ActionType::UpdatePassword,
            ActionType::ConfigureTotp,
            ActionType::ConfigureWebAuthn,
        ] {
            let s = at.as_str();
            assert_eq!(ActionType::from_str_value(s), Some(at));
            assert_eq!(at.to_string(), s);
        }
    }

    #[test]
    fn action_type_unknown_returns_none() {
        assert_eq!(ActionType::from_str_value("unknown"), None);
    }

    #[test]
    fn action_status_roundtrip() {
        for st in [
            ActionStatus::Pending,
            ActionStatus::Completed,
            ActionStatus::Cancelled,
        ] {
            let s = st.as_str();
            assert_eq!(ActionStatus::from_str_value(s), Some(st));
            assert_eq!(st.to_string(), s);
        }
    }

    #[test]
    fn action_status_unknown_returns_none() {
        assert_eq!(ActionStatus::from_str_value("unknown"), None);
    }

    #[test]
    fn pending_action_construction() {
        let action = PendingAction {
            id: "action-1".to_string(),
            user_id: "user-1".to_string(),
            action_type: ActionType::VerifyEmail,
            status: ActionStatus::Pending,
            metadata: Some(serde_json::json!({"email": "user@test.example.com"})),
            created_at: Utc::now(),
            completed_at: None,
        };
        assert_eq!(action.status, ActionStatus::Pending);
        assert!(action.completed_at.is_none());
    }
}
