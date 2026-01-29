//! User domain model

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: StringUuid,
    pub keycloak_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub mfa_enabled: bool,
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
            created_at: now,
            updated_at: now,
        }
    }
}

/// User-Tenant relationship
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantUser {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub user_id: StringUuid,
    /// Role within the tenant (e.g., "admin", "member")
    pub role_in_tenant: String,
    pub joined_at: DateTime<Utc>,
}

/// Input for creating a new user
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateUserInput {
    #[validate(email)]
    pub email: String,
    #[validate(length(max = 255))]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Input for updating a user
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateUserInput {
    #[validate(length(max = 255))]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Input for adding user to tenant
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AddUserToTenantInput {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    #[validate(length(min = 1, max = 50))]
    pub role_in_tenant: String,
}

/// User with tenant information (for API responses)
#[derive(Debug, Clone, Serialize)]
pub struct UserWithTenants {
    #[serde(flatten)]
    pub user: User,
    pub tenants: Vec<UserTenantInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTenantInfo {
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub role_in_tenant: String,
    pub joined_at: DateTime<Utc>,
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
}
