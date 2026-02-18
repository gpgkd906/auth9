//! User domain model

use super::common::{validate_url_no_ssrf_strict, StringUuid};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: StringUuid,
    pub keycloak_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub mfa_enabled: bool,
    pub password_changed_at: Option<DateTime<Utc>>,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for User {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            keycloak_id: String::new(),
            email: String::new(),
            display_name: None,
            avatar_url: None,
            mfa_enabled: false,
            password_changed_at: None,
            locked_until: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for admin setting a user's password
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct AdminSetPasswordInput {
    #[validate(length(min = 1, max = 128))]
    pub password: String,
    /// If true, user must change password on next login
    #[serde(default)]
    pub temporary: bool,
}

/// User-Tenant relationship
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TenantUser {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub user_id: StringUuid,
    /// Role within the tenant (e.g., "admin", "member")
    pub role_in_tenant: String,
    pub joined_at: DateTime<Utc>,
}

/// Input for creating a new user
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateUserInput {
    #[validate(email)]
    pub email: String,
    #[validate(length(max = 255), custom(function = "validate_no_html"))]
    pub display_name: Option<String>,
    #[validate(length(max = 2048), custom(function = "validate_avatar_url"))]
    pub avatar_url: Option<String>,
}

/// Validate that a string does not contain HTML tags (prevent stored XSS)
fn validate_no_html(value: &str) -> Result<(), validator::ValidationError> {
    if value.contains('<') || value.contains('>') {
        let mut err = validator::ValidationError::new("contains_html");
        err.message = Some("Value must not contain HTML tags".into());
        Err(err)
    } else {
        Ok(())
    }
}

/// Validate avatar URL with SSRF protection.
/// Blocks private IPs, loopback addresses, cloud metadata endpoints, and requires HTTPS for external URLs.
fn validate_avatar_url(url: &str) -> Result<(), validator::ValidationError> {
    if url.is_empty() {
        return Ok(());
    }
    validate_url_no_ssrf_strict(url)
}

/// Input for updating a user
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateUserInput {
    #[validate(length(max = 255), custom(function = "validate_no_html"))]
    pub display_name: Option<String>,
    #[validate(length(max = 2048), custom(function = "validate_avatar_url"))]
    pub avatar_url: Option<String>,
}

/// Input for adding user to tenant
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct AddUserToTenantInput {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    #[validate(length(min = 1, max = 50))]
    pub role_in_tenant: String,
}

/// User with tenant information (for API responses)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserWithTenants {
    #[serde(flatten)]
    pub user: User,
    pub tenants: Vec<UserTenantInfo>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserTenantInfo {
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub role_in_tenant: String,
    pub joined_at: DateTime<Utc>,
}

/// TenantUser with embedded Tenant data (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantUserWithTenant {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub user_id: StringUuid,
    pub role_in_tenant: String,
    pub joined_at: DateTime<Utc>,
    pub tenant: TenantInfo,
}

/// Lightweight Tenant info for embedding in responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantInfo {
    pub id: StringUuid,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_default() {
        let user = User::default();
        assert!(!user.id.is_nil());
        assert!(!user.mfa_enabled);
    }

    #[test]
    fn test_create_user_input_validation() {
        let input = CreateUserInput {
            email: "invalid-email".to_string(),
            display_name: None,
            avatar_url: None,
        };
        assert!(input.validate().is_err());

        let valid_input = CreateUserInput {
            email: "user@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
        };
        assert!(valid_input.validate().is_ok());
    }

    #[test]
    fn test_display_name_rejects_html() {
        let input = UpdateUserInput {
            display_name: Some("<script>alert(1)</script>".to_string()),
            avatar_url: None,
        };
        assert!(input.validate().is_err());

        let input = UpdateUserInput {
            display_name: Some("<img src=x onerror=alert(1)>".to_string()),
            avatar_url: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_display_name_accepts_valid_text() {
        let input = UpdateUserInput {
            display_name: Some("John Doe".to_string()),
            avatar_url: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_user_display_name_rejects_html() {
        let input = CreateUserInput {
            email: "user@example.com".to_string(),
            display_name: Some("<script>alert(1)</script>".to_string()),
            avatar_url: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_avatar_url_rejects_path_traversal() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: Some("../../etc/passwd".to_string()),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_avatar_url_rejects_encoded_path_traversal() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: Some("..%2F..%2Fetc%2Fpasswd".to_string()),
        };
        assert!(input.validate().is_err()); // no http(s):// scheme
    }

    #[test]
    fn test_avatar_url_rejects_dotdot_in_url() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: Some("https://example.com/../../etc/passwd".to_string()),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_avatar_url_accepts_valid_https() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: Some("https://cdn.example.com/avatars/user123.png".to_string()),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_avatar_url_accepts_none() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_avatar_url_rejects_null_byte() {
        let input = UpdateUserInput {
            display_name: None,
            avatar_url: Some("https://example.com/avatar\0.png".to_string()),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_user_avatar_url_validation() {
        let input = CreateUserInput {
            email: "user@example.com".to_string(),
            display_name: None,
            avatar_url: Some("../../etc/passwd".to_string()),
        };
        assert!(input.validate().is_err());

        let valid = CreateUserInput {
            email: "user@example.com".to_string(),
            display_name: None,
            avatar_url: Some("https://cdn.example.com/avatar.png".to_string()),
        };
        assert!(valid.validate().is_ok());
    }
}
