//! Password management domain models

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

/// Password reset token stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PasswordResetToken {
    pub id: StringUuid,
    pub user_id: StringUuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Default for PasswordResetToken {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            user_id: StringUuid::new_v4(),
            token_hash: String::new(),
            expires_at: now + chrono::Duration::hours(1),
            used_at: None,
            created_at: now,
        }
    }
}

/// Password policy configuration for a tenant
///
/// Defaults define sensible password policy constraints for the tenant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PasswordPolicy {
    /// Minimum password length (default: 12)
    #[serde(default = "default_min_length")]
    pub min_length: u32,
    /// Require at least one uppercase letter
    #[serde(default = "default_true")]
    pub require_uppercase: bool,
    /// Require at least one lowercase letter
    #[serde(default = "default_true")]
    pub require_lowercase: bool,
    /// Require at least one number
    #[serde(default = "default_true")]
    pub require_numbers: bool,
    /// Require at least one symbol
    #[serde(default = "default_true")]
    pub require_symbols: bool,
    /// Maximum password age in days (0 = no expiry)
    #[serde(default)]
    pub max_age_days: u32,
    /// Number of previous passwords to remember (0 = disabled)
    #[serde(default = "default_history_count")]
    pub history_count: u32,
    /// Number of failed attempts before lockout (0 = disabled)
    #[serde(default = "default_lockout_threshold")]
    pub lockout_threshold: u32,
    /// Lockout duration in minutes
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration_mins: u32,
    /// Breached password check mode: "block" (default), "warn", or "disabled"
    #[serde(default = "default_breach_check_mode")]
    pub breach_check_mode: String,
    /// Minimum breach count threshold before blocking/warning (default 1)
    #[serde(default = "default_min_breach_count")]
    pub min_breach_count: u64,
    /// Check password against HIBP on login (async, default true)
    #[serde(default = "default_true")]
    pub breach_check_on_login: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            max_age_days: 0,
            history_count: 5,
            lockout_threshold: 5,
            lockout_duration_mins: 15,
            breach_check_mode: "block".to_string(),
            min_breach_count: 1,
            breach_check_on_login: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_min_length() -> u32 {
    12
}

fn default_history_count() -> u32 {
    5
}

fn default_lockout_threshold() -> u32 {
    5
}

fn default_lockout_duration() -> u32 {
    15
}

fn default_breach_check_mode() -> String {
    "block".to_string()
}

fn default_min_breach_count() -> u64 {
    1
}

/// Input for requesting a password reset
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordInput {
    #[validate(email)]
    pub email: String,
}

/// Input for resetting password with token
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordInput {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// Input for changing password (authenticated user)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordInput {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// Input for force-updating password (no current password required, used by required-action flow)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ForceChangePasswordInput {
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

/// Input for creating a password reset token
#[derive(Debug, Clone)]
pub struct CreatePasswordResetTokenInput {
    pub user_id: StringUuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

/// Valid breach check modes
const VALID_BREACH_CHECK_MODES: &[&str] = &["block", "warn", "disabled"];

/// Input for updating password policy
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
#[validate(schema(function = "validate_breach_check_mode"))]
pub struct UpdatePasswordPolicyInput {
    #[validate(range(min = 6, max = 128))]
    pub min_length: Option<u32>,
    pub require_uppercase: Option<bool>,
    pub require_lowercase: Option<bool>,
    pub require_numbers: Option<bool>,
    pub require_symbols: Option<bool>,
    #[validate(range(max = 365))]
    pub max_age_days: Option<u32>,
    #[validate(range(max = 24))]
    pub history_count: Option<u32>,
    #[validate(range(max = 100))]
    pub lockout_threshold: Option<u32>,
    #[validate(range(min = 1, max = 1440))]
    pub lockout_duration_mins: Option<u32>,
    /// Breached password check mode: "block", "warn", or "disabled"
    pub breach_check_mode: Option<String>,
    /// Minimum breach count threshold
    pub min_breach_count: Option<u64>,
    /// Check password on login (async)
    pub breach_check_on_login: Option<bool>,
}

fn validate_breach_check_mode(input: &UpdatePasswordPolicyInput) -> Result<(), validator::ValidationError> {
    if let Some(ref mode) = input.breach_check_mode {
        if !VALID_BREACH_CHECK_MODES.contains(&mode.as_str()) {
            let mut err = validator::ValidationError::new("invalid_breach_check_mode");
            err.message = Some(
                format!(
                    "breach_check_mode must be one of: {}",
                    VALID_BREACH_CHECK_MODES.join(", ")
                )
                .into(),
            );
            return Err(err);
        }
    }
    Ok(())
}

impl PasswordPolicy {
    /// Validate a password against this policy
    pub fn validate_password(&self, password: &str) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if password.len() < self.min_length as usize {
            errors.push(format!(
                "Password must be at least {} characters",
                self.min_length
            ));
        }

        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if self.require_numbers && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one number".to_string());
        }

        if self.require_symbols
            && !password
                .chars()
                .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
        {
            errors.push("Password must contain at least one symbol".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_password_reset_token_default() {
        let token = PasswordResetToken::default();
        assert!(!token.id.is_nil());
        assert!(!token.user_id.is_nil());
        assert!(token.used_at.is_none());
        assert!(token.expires_at > token.created_at);
    }

    #[test]
    fn test_password_policy_default() {
        let policy = PasswordPolicy::default();
        assert_eq!(policy.min_length, 12);
        assert!(policy.require_uppercase);
        assert!(policy.require_lowercase);
        assert!(policy.require_numbers);
        assert!(policy.require_symbols);
        assert_eq!(policy.max_age_days, 0);
        assert_eq!(policy.history_count, 5);
        assert_eq!(policy.lockout_threshold, 5);
        assert_eq!(policy.lockout_duration_mins, 15);
        assert_eq!(policy.breach_check_mode, "block");
        assert_eq!(policy.min_breach_count, 1);
        assert!(policy.breach_check_on_login);
    }

    #[test]
    fn test_password_policy_validate_min_length() {
        let policy = PasswordPolicy {
            min_length: 10,
            require_symbols: false,
            history_count: 0,
            ..Default::default()
        };

        assert!(policy.validate_password("short").is_err());
        assert!(policy.validate_password("Longenough1").is_ok());
    }

    #[test]
    fn test_password_policy_validate_uppercase() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: true,
            require_symbols: false,
            history_count: 0,
            ..Default::default()
        };

        assert!(policy.validate_password("lowercase1").is_err());
        assert!(policy.validate_password("Uppercase1").is_ok());
    }

    #[test]
    fn test_password_policy_validate_lowercase() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_lowercase: true,
            require_symbols: false,
            history_count: 0,
            ..Default::default()
        };

        assert!(policy.validate_password("UPPERCASE1").is_err());
        assert!(policy.validate_password("LOWERcase1").is_ok());
    }

    #[test]
    fn test_password_policy_validate_numbers() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_numbers: true,
            require_symbols: false,
            history_count: 0,
            ..Default::default()
        };

        assert!(policy.validate_password("NoNumbers").is_err());
        assert!(policy.validate_password("HasNumber1").is_ok());
    }

    #[test]
    fn test_password_policy_validate_symbols() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_symbols: true,
            history_count: 0,
            ..Default::default()
        };

        assert!(policy.validate_password("NoSymbols1").is_err());
        assert!(policy.validate_password("HasSymbol1!").is_ok());
    }

    #[test]
    fn test_password_policy_validate_all_requirements() {
        let policy = PasswordPolicy {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            ..Default::default()
        };

        assert!(policy.validate_password("weak").is_err());
        assert!(policy.validate_password("Weak1!").is_err()); // Too short
        assert!(policy.validate_password("StrongPass1!").is_ok());
    }

    #[test]
    fn test_forgot_password_input_valid() {
        let input = ForgotPasswordInput {
            email: "test@example.com".to_string(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_forgot_password_input_invalid_email() {
        let input = ForgotPasswordInput {
            email: "invalid-email".to_string(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_reset_password_input_valid() {
        let input = ResetPasswordInput {
            token: "abc123".to_string(),
            new_password: "newpassword123".to_string(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_reset_password_input_short_password() {
        let input = ResetPasswordInput {
            token: "abc123".to_string(),
            new_password: "short".to_string(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_change_password_input_valid() {
        let input = ChangePasswordInput {
            current_password: "oldpass".to_string(),
            new_password: "newpassword123".to_string(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_password_policy_input_valid() {
        let input = UpdatePasswordPolicyInput {
            min_length: Some(12),
            require_uppercase: Some(true),
            require_lowercase: Some(true),
            require_numbers: Some(true),
            require_symbols: Some(false),
            max_age_days: Some(90),
            history_count: Some(5),
            lockout_threshold: Some(5),
            lockout_duration_mins: Some(30),
            breach_check_mode: None,
            min_breach_count: None,
            breach_check_on_login: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_password_policy_input_invalid_min_length() {
        let input = UpdatePasswordPolicyInput {
            min_length: Some(3), // Too short
            require_uppercase: None,
            require_lowercase: None,
            require_numbers: None,
            require_symbols: None,
            max_age_days: None,
            history_count: None,
            lockout_threshold: None,
            lockout_duration_mins: None,
            breach_check_mode: None,
            min_breach_count: None,
            breach_check_on_login: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_password_policy_invalid_breach_check_mode() {
        let input = UpdatePasswordPolicyInput {
            min_length: None,
            require_uppercase: None,
            require_lowercase: None,
            require_numbers: None,
            require_symbols: None,
            max_age_days: None,
            history_count: None,
            lockout_threshold: None,
            lockout_duration_mins: None,
            breach_check_mode: Some("invalid_mode".to_string()),
            min_breach_count: None,
            breach_check_on_login: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_password_policy_valid_breach_check_modes() {
        for mode in &["block", "warn", "disabled"] {
            let input = UpdatePasswordPolicyInput {
                min_length: None,
                require_uppercase: None,
                require_lowercase: None,
                require_numbers: None,
                require_symbols: None,
                max_age_days: None,
                history_count: None,
                lockout_threshold: None,
                lockout_duration_mins: None,
                breach_check_mode: Some(mode.to_string()),
                min_breach_count: None,
                breach_check_on_login: None,
            };
            assert!(input.validate().is_ok(), "Expected '{}' to be valid", mode);
        }
    }

    #[test]
    fn test_password_policy_serialization() {
        let policy = PasswordPolicy {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            max_age_days: 90,
            history_count: 5,
            lockout_threshold: 5,
            lockout_duration_mins: 30,
            breach_check_mode: "block".to_string(),
            min_breach_count: 1,
            breach_check_on_login: true,
        };

        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: PasswordPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.min_length, policy.min_length);
        assert_eq!(deserialized.require_uppercase, policy.require_uppercase);
        assert_eq!(deserialized.max_age_days, policy.max_age_days);
    }

    #[test]
    fn test_password_policy_deserialization_defaults() {
        let json = r#"{}"#;
        let policy: PasswordPolicy = serde_json::from_str(json).unwrap();

        assert_eq!(policy.min_length, 12);
        assert!(policy.require_uppercase);
        assert!(policy.require_lowercase);
        assert!(policy.require_numbers);
        assert!(policy.require_symbols);
        assert_eq!(policy.history_count, 5);
        assert_eq!(policy.lockout_duration_mins, 15);
    }

    #[test]
    fn test_password_policy_deserialization_explicit_false_preserved() {
        let json =
            r#"{"require_uppercase": false, "require_lowercase": false, "require_numbers": false}"#;
        let policy: PasswordPolicy = serde_json::from_str(json).unwrap();

        assert!(!policy.require_uppercase);
        assert!(!policy.require_lowercase);
        assert!(!policy.require_numbers);
    }
}
