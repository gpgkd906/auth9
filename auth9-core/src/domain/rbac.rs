//! RBAC (Role-Based Access Control) domain models

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Permission entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub id: StringUuid,
    pub service_id: StringUuid,
    /// Permission code (e.g., "user:read", "report:export")
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

impl Default for Permission {
    fn default() -> Self {
        Self {
            id: StringUuid::new_v4(),
            service_id: StringUuid::nil(),
            code: String::new(),
            name: String::new(),
            description: None,
        }
    }
}

/// Role entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: StringUuid,
    pub service_id: StringUuid,
    pub name: String,
    pub description: Option<String>,
    /// Parent role for inheritance (optional)
    pub parent_role_id: Option<StringUuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Role {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            service_id: StringUuid::nil(),
            name: String::new(),
            description: None,
            parent_role_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Role-Permission mapping
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RolePermission {
    pub role_id: StringUuid,
    pub permission_id: StringUuid,
}

/// User-Tenant-Role assignment
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserTenantRole {
    pub id: StringUuid,
    pub tenant_user_id: StringUuid,
    pub role_id: StringUuid,
    pub granted_at: DateTime<Utc>,
    pub granted_by: Option<StringUuid>,
}

/// Input for creating a permission
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePermissionInput {
    pub service_id: Uuid,
    #[validate(
        length(min = 1, max = 100),
        custom(function = "validate_permission_code")
    )]
    pub code: String,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
}

/// Validate permission code format (e.g., "user:read", "report:export:pdf")
fn validate_permission_code(code: &str) -> Result<(), validator::ValidationError> {
    if PERMISSION_CODE_REGEX.is_match(code) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_permission_code"))
    }
}

/// Input for creating a role
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRoleInput {
    pub service_id: Uuid,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
    pub permission_ids: Option<Vec<Uuid>>,
}

/// Input for updating a role
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateRoleInput {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
}

/// Input for assigning roles to a user in a tenant
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AssignRolesInput {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role_ids: Vec<Uuid>,
}

/// Role with its permissions (for API responses)
#[derive(Debug, Clone, Serialize)]
pub struct RoleWithPermissions {
    #[serde(flatten)]
    pub role: Role,
    pub permissions: Vec<Permission>,
}

/// User roles in a tenant (for token claims)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRolesInTenant {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

// Regex for permission code validation
lazy_static::lazy_static! {
    pub static ref PERMISSION_CODE_REGEX: regex::Regex =
        regex::Regex::new(r"^[a-z][a-z0-9]*(?::[a-z][a-z0-9]*)+$").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_default() {
        let perm = Permission::default();
        assert!(!perm.id.is_nil());
    }

    #[test]
    fn test_role_default() {
        let role = Role::default();
        assert!(!role.id.is_nil());
        assert!(role.parent_role_id.is_none());
    }

    #[test]
    fn test_permission_code_regex() {
        assert!(PERMISSION_CODE_REGEX.is_match("user:read"));
        assert!(PERMISSION_CODE_REGEX.is_match("report:export:pdf"));
        assert!(!PERMISSION_CODE_REGEX.is_match("invalid"));
        assert!(!PERMISSION_CODE_REGEX.is_match("User:Read"));
        assert!(!PERMISSION_CODE_REGEX.is_match("user_read"));
    }
}
