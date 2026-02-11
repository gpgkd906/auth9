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
    // Option<Option<Uuid>> allows distinguishing between:
    // - None: not provided, keep existing value
    // - Some(None): explicitly set to null/None
    // - Some(Some(id)): set to specific parent role
    #[serde(default)]
    pub parent_role_id: Option<Option<Uuid>>,
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
    use validator::Validate;

    #[test]
    fn test_permission_default() {
        let perm = Permission::default();
        assert!(!perm.id.is_nil());
        assert!(perm.service_id.is_nil());
        assert!(perm.code.is_empty());
        assert!(perm.name.is_empty());
        assert!(perm.description.is_none());
    }

    #[test]
    fn test_permission_with_values() {
        let perm = Permission {
            id: StringUuid::new_v4(),
            service_id: StringUuid::new_v4(),
            code: "user:read".to_string(),
            name: "Read Users".to_string(),
            description: Some("Can read user data".to_string()),
        };

        assert!(!perm.id.is_nil());
        assert!(!perm.service_id.is_nil());
        assert_eq!(perm.code, "user:read");
        assert_eq!(perm.name, "Read Users");
        assert!(perm.description.is_some());
    }

    #[test]
    fn test_role_default() {
        let role = Role::default();
        assert!(!role.id.is_nil());
        assert!(role.service_id.is_nil());
        assert!(role.name.is_empty());
        assert!(role.description.is_none());
        assert!(role.parent_role_id.is_none());
    }

    #[test]
    fn test_role_with_parent() {
        let parent_id = StringUuid::new_v4();
        let role = Role {
            parent_role_id: Some(parent_id),
            ..Default::default()
        };

        assert_eq!(role.parent_role_id, Some(parent_id));
    }

    #[test]
    fn test_permission_code_regex() {
        // Valid codes
        assert!(PERMISSION_CODE_REGEX.is_match("user:read"));
        assert!(PERMISSION_CODE_REGEX.is_match("report:export:pdf"));
        assert!(PERMISSION_CODE_REGEX.is_match("a:b"));
        assert!(PERMISSION_CODE_REGEX.is_match("user1:action2"));

        // Invalid codes
        assert!(!PERMISSION_CODE_REGEX.is_match("invalid"));
        assert!(!PERMISSION_CODE_REGEX.is_match("User:Read"));
        assert!(!PERMISSION_CODE_REGEX.is_match("user_read"));
        assert!(!PERMISSION_CODE_REGEX.is_match(":read"));
        assert!(!PERMISSION_CODE_REGEX.is_match("user:"));
        assert!(!PERMISSION_CODE_REGEX.is_match("1user:read"));
        assert!(!PERMISSION_CODE_REGEX.is_match(""));
    }

    #[test]
    fn test_validate_permission_code_valid() {
        let result = validate_permission_code("user:read");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_permission_code_invalid() {
        let result = validate_permission_code("invalid");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code.as_ref(), "invalid_permission_code");
    }

    #[test]
    fn test_create_permission_input_valid() {
        let input = CreatePermissionInput {
            service_id: Uuid::new_v4(),
            code: "user:read".to_string(),
            name: "Read Users".to_string(),
            description: Some("Can read user data".to_string()),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_permission_input_invalid_code() {
        let input = CreatePermissionInput {
            service_id: Uuid::new_v4(),
            code: "invalid".to_string(),
            name: "Read Users".to_string(),
            description: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_permission_input_empty_code() {
        let input = CreatePermissionInput {
            service_id: Uuid::new_v4(),
            code: "".to_string(),
            name: "Read Users".to_string(),
            description: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_permission_input_empty_name() {
        let input = CreatePermissionInput {
            service_id: Uuid::new_v4(),
            code: "user:read".to_string(),
            name: "".to_string(),
            description: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_role_input_valid() {
        let input = CreateRoleInput {
            service_id: Uuid::new_v4(),
            name: "Admin".to_string(),
            description: Some("Administrator role".to_string()),
            parent_role_id: None,
            permission_ids: Some(vec![Uuid::new_v4()]),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_role_input_minimal() {
        let input = CreateRoleInput {
            service_id: Uuid::new_v4(),
            name: "User".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_role_input_empty_name() {
        let input = CreateRoleInput {
            service_id: Uuid::new_v4(),
            name: "".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_role_input_valid() {
        let input = UpdateRoleInput {
            name: Some("Updated Role".to_string()),
            description: Some("Updated description".to_string()),
            parent_role_id: Some(Some(Uuid::new_v4())),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_role_input_partial() {
        let input = UpdateRoleInput {
            name: None,
            description: Some("Only description".to_string()),
            parent_role_id: None,
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_role_input_empty_name() {
        let input = UpdateRoleInput {
            name: Some("".to_string()),
            description: None,
            parent_role_id: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_assign_roles_input_valid() {
        let input = AssignRolesInput {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_assign_roles_input_empty_roles() {
        let input = AssignRolesInput {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role_ids: vec![],
        };

        // Empty roles is valid at validation level
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_role_permission_structure() {
        let role_perm = RolePermission {
            role_id: StringUuid::new_v4(),
            permission_id: StringUuid::new_v4(),
        };

        assert!(!role_perm.role_id.is_nil());
        assert!(!role_perm.permission_id.is_nil());
    }

    #[test]
    fn test_user_tenant_role_structure() {
        let utr = UserTenantRole {
            id: StringUuid::new_v4(),
            tenant_user_id: StringUuid::new_v4(),
            role_id: StringUuid::new_v4(),
            granted_at: Utc::now(),
            granted_by: Some(StringUuid::new_v4()),
        };

        assert!(!utr.id.is_nil());
        assert!(utr.granted_by.is_some());
    }

    #[test]
    fn test_user_tenant_role_without_granter() {
        let utr = UserTenantRole {
            id: StringUuid::new_v4(),
            tenant_user_id: StringUuid::new_v4(),
            role_id: StringUuid::new_v4(),
            granted_at: Utc::now(),
            granted_by: None,
        };

        assert!(utr.granted_by.is_none());
    }

    #[test]
    fn test_role_with_permissions_structure() {
        let role = Role::default();
        let permissions = vec![Permission::default()];

        let rwp = RoleWithPermissions {
            role: role.clone(),
            permissions,
        };

        assert_eq!(rwp.role.id, role.id);
        assert_eq!(rwp.permissions.len(), 1);
    }

    #[test]
    fn test_role_with_permissions_serialization() {
        let rwp = RoleWithPermissions {
            role: Role {
                name: "Admin".to_string(),
                ..Default::default()
            },
            permissions: vec![],
        };

        let json = serde_json::to_string(&rwp).unwrap();
        assert!(json.contains("Admin"));
        assert!(json.contains("permissions"));
    }

    #[test]
    fn test_user_roles_in_tenant_structure() {
        let urit = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec!["user:read".to_string(), "user:write".to_string()],
        };

        assert_eq!(urit.roles.len(), 2);
        assert_eq!(urit.permissions.len(), 2);
    }

    #[test]
    fn test_user_roles_in_tenant_serialization() {
        let urit = UserRolesInTenant {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            roles: vec!["admin".to_string()],
            permissions: vec!["user:read".to_string()],
        };

        let json = serde_json::to_string(&urit).unwrap();
        let deserialized: UserRolesInTenant = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.roles, urit.roles);
        assert_eq!(deserialized.permissions, urit.permissions);
    }
}
