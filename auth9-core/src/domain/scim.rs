//! SCIM 2.0 domain models (RFC 7643 / 7644)

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

// ============================================================
// Database Entities
// ============================================================

/// SCIM Bearer Token record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScimToken {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub token_hash: String,
    pub token_prefix: String,
    pub description: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SCIM Token API response (excludes hash)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScimTokenResponse {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub token_prefix: String,
    pub description: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ScimToken> for ScimTokenResponse {
    fn from(t: ScimToken) -> Self {
        Self {
            id: t.id,
            tenant_id: t.tenant_id,
            connector_id: t.connector_id,
            token_prefix: t.token_prefix,
            description: t.description,
            expires_at: t.expires_at,
            last_used_at: t.last_used_at,
            revoked_at: t.revoked_at,
            created_at: t.created_at,
        }
    }
}

/// Input for creating a SCIM token
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateScimTokenInput {
    pub description: Option<String>,
    /// Token expiry in days (None = no expiry)
    pub expires_in_days: Option<i64>,
}

/// Response after creating a SCIM token (includes the raw token, shown only once)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateScimTokenResponse {
    pub token: String,
    #[serde(flatten)]
    pub details: ScimTokenResponse,
}

/// SCIM Group → Auth9 Role mapping
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScimGroupRoleMapping {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub scim_group_id: String,
    pub scim_group_display_name: Option<String>,
    pub role_id: StringUuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SCIM provisioning audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScimProvisioningLog {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub operation: String,
    pub resource_type: String,
    pub scim_resource_id: Option<String>,
    pub auth9_resource_id: Option<StringUuid>,
    pub status: String,
    pub error_detail: Option<String>,
    pub response_status: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a SCIM provisioning log
#[derive(Debug, Clone)]
pub struct CreateScimLogInput {
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub operation: String,
    pub resource_type: String,
    pub scim_resource_id: Option<String>,
    pub auth9_resource_id: Option<StringUuid>,
    pub status: String,
    pub error_detail: Option<String>,
    pub response_status: Option<i32>,
}

// ============================================================
// SCIM Protocol Types (RFC 7643 / 7644)
// ============================================================

/// SCIM User resource (RFC 7643 §4.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimUser {
    pub schemas: Vec<String>,
    pub id: Option<String>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimName>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<ScimEmail>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub photos: Vec<ScimPhoto>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimMeta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ScimGroupRef>,
}

impl ScimUser {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:schemas:core:2.0:User";
}

/// SCIM Name sub-attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimName {
    #[serde(rename = "givenName", skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(rename = "familyName", skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
}

/// SCIM Email sub-attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimEmail {
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
    #[serde(default)]
    pub primary: bool,
}

/// SCIM Photo sub-attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimPhoto {
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub photo_type: Option<String>,
}

/// SCIM Group reference (within User resource)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimGroupRef {
    pub value: String,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// SCIM Meta sub-attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimMeta {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub created: Option<String>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    pub location: Option<String>,
}

/// SCIM Group resource (RFC 7643 §4.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimGroup {
    pub schemas: Vec<String>,
    pub id: Option<String>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<ScimMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimMeta>,
}

impl ScimGroup {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:schemas:core:2.0:Group";
}

/// SCIM Group member reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimMember {
    pub value: String,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// SCIM ListResponse (RFC 7644 §3.4.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimListResponse<T: Serialize> {
    pub schemas: Vec<String>,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    #[serde(rename = "Resources")]
    pub resources: Vec<T>,
}

impl<T: Serialize> ScimListResponse<T> {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";

    pub fn new(resources: Vec<T>, total_results: i64, start_index: i64, count: i64) -> Self {
        Self {
            schemas: vec![Self::SCHEMA.to_string()],
            total_results,
            start_index,
            items_per_page: count,
            resources,
        }
    }
}

/// SCIM PatchOp (RFC 7644 §3.5.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimPatchOp {
    pub schemas: Vec<String>,
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
}

impl ScimPatchOp {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
}

/// Individual SCIM patch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimPatchOperation {
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// SCIM Bulk request (RFC 7644 §3.7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimBulkRequest {
    pub schemas: Vec<String>,
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimBulkOperation>,
    #[serde(rename = "failOnErrors", default)]
    pub fail_on_errors: Option<i64>,
}

impl ScimBulkRequest {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:api:messages:2.0:BulkRequest";
}

/// Individual bulk operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimBulkOperation {
    pub method: String,
    pub path: String,
    #[serde(rename = "bulkId", skip_serializing_if = "Option::is_none")]
    pub bulk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// SCIM Bulk response (RFC 7644 §3.7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimBulkResponse {
    pub schemas: Vec<String>,
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimBulkOperationResponse>,
}

impl ScimBulkResponse {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:api:messages:2.0:BulkResponse";
}

/// Bulk operation response entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimBulkOperationResponse {
    pub method: String,
    #[serde(rename = "bulkId", skip_serializing_if = "Option::is_none")]
    pub bulk_id: Option<String>,
    pub location: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

/// SCIM Error response (RFC 7644 §3.12)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimError {
    pub schemas: Vec<String>,
    pub status: String,
    #[serde(rename = "scimType", skip_serializing_if = "Option::is_none")]
    pub scim_type: Option<String>,
    pub detail: Option<String>,
}

impl ScimError {
    pub const SCHEMA: &'static str = "urn:ietf:params:scim:api:messages:2.0:Error";

    pub fn new(status: u16, scim_type: Option<&str>, detail: impl Into<String>) -> Self {
        Self {
            schemas: vec![Self::SCHEMA.to_string()],
            status: status.to_string(),
            scim_type: scim_type.map(|s| s.to_string()),
            detail: Some(detail.into()),
        }
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new(404, None, detail)
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(400, Some("invalidValue"), detail)
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new(409, Some("uniqueness"), detail)
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(500, None, detail)
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self::new(401, None, detail)
    }
}

/// SCIM ServiceProviderConfig (RFC 7643 §5)
#[derive(Debug, Clone, Serialize)]
pub struct ScimServiceProviderConfig {
    pub schemas: Vec<String>,
    pub patch: ScimSupported,
    pub bulk: ScimBulkSupported,
    pub filter: ScimFilterSupported,
    #[serde(rename = "changePassword")]
    pub change_password: ScimSupported,
    pub sort: ScimSupported,
    pub etag: ScimSupported,
    #[serde(rename = "authenticationSchemes")]
    pub authentication_schemes: Vec<ScimAuthScheme>,
}

impl Default for ScimServiceProviderConfig {
    fn default() -> Self {
        Self {
            schemas: vec![
                "urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig".to_string(),
            ],
            patch: ScimSupported { supported: true },
            bulk: ScimBulkSupported {
                supported: true,
                max_operations: 100,
                max_payload_size: 1_048_576,
            },
            filter: ScimFilterSupported {
                supported: true,
                max_results: 200,
            },
            change_password: ScimSupported { supported: false },
            sort: ScimSupported { supported: false },
            etag: ScimSupported { supported: false },
            authentication_schemes: vec![ScimAuthScheme {
                name: "OAuth Bearer Token".to_string(),
                description: "Authentication scheme using the OAuth Bearer Token Standard"
                    .to_string(),
                scheme_type: "oauthbearertoken".to_string(),
                primary: true,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimSupported {
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimBulkSupported {
    pub supported: bool,
    #[serde(rename = "maxOperations")]
    pub max_operations: i64,
    #[serde(rename = "maxPayloadSize")]
    pub max_payload_size: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimFilterSupported {
    pub supported: bool,
    #[serde(rename = "maxResults")]
    pub max_results: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimAuthScheme {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub scheme_type: String,
    pub primary: bool,
}

/// Request context injected after SCIM token authentication
#[derive(Debug, Clone)]
pub struct ScimRequestContext {
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub token_id: StringUuid,
    pub base_url: String,
}

/// SCIM resource schema definition (for /Schemas endpoint)
#[derive(Debug, Clone, Serialize)]
pub struct ScimSchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub attributes: Vec<ScimSchemaAttribute>,
    pub meta: ScimMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScimSchemaAttribute {
    pub name: String,
    #[serde(rename = "type")]
    pub attr_type: String,
    #[serde(rename = "multiValued")]
    pub multi_valued: bool,
    pub required: bool,
    pub mutability: String,
    pub returned: String,
    pub uniqueness: String,
}

/// SCIM ResourceType definition (for /ResourceTypes endpoint)
#[derive(Debug, Clone, Serialize)]
pub struct ScimResourceType {
    pub schemas: Vec<String>,
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub schema: String,
    pub meta: ScimMeta,
}

/// Input for updating group-role mappings
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateGroupRoleMappingsInput {
    pub mappings: Vec<GroupRoleMappingEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct GroupRoleMappingEntry {
    pub scim_group_id: String,
    pub scim_group_display_name: Option<String>,
    pub role_id: StringUuid,
}

// ============================================================
// SCIM Filter AST
// ============================================================

/// Parsed SCIM filter expression
#[derive(Debug, Clone, PartialEq)]
pub enum ScimFilterExpr {
    /// Comparison: attrPath op value
    Compare {
        attr: String,
        op: ScimCompareOp,
        value: String,
    },
    /// Presence: attrPath pr
    Present { attr: String },
    /// Logical AND
    And(Box<ScimFilterExpr>, Box<ScimFilterExpr>),
    /// Logical OR
    Or(Box<ScimFilterExpr>, Box<ScimFilterExpr>),
    /// Logical NOT
    Not(Box<ScimFilterExpr>),
}

/// SCIM comparison operators
#[derive(Debug, Clone, PartialEq)]
pub enum ScimCompareOp {
    Eq,
    Ne,
    Co,
    Sw,
    Ew,
    Gt,
    Ge,
    Lt,
    Le,
}

impl ScimCompareOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "eq" => Some(Self::Eq),
            "ne" => Some(Self::Ne),
            "co" => Some(Self::Co),
            "sw" => Some(Self::Sw),
            "ew" => Some(Self::Ew),
            "gt" => Some(Self::Gt),
            "ge" => Some(Self::Ge),
            "lt" => Some(Self::Lt),
            "le" => Some(Self::Le),
            _ => None,
        }
    }
}

/// Compiled SCIM filter → SQL WHERE clause
#[derive(Debug, Clone)]
pub struct CompiledFilter {
    pub where_clause: String,
    pub bindings: Vec<String>,
}

/// SCIM attribute mapping configuration
#[derive(Debug, Clone)]
pub struct ScimAttributeMapping {
    pub scim_path: String,
    pub auth9_field: String,
}

/// Mapped user fields from SCIM request
#[derive(Debug, Clone, Default)]
pub struct MappedUserFields {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub external_id: Option<String>,
    pub active: Option<bool>,
}

fn default_true() -> bool {
    true
}

// ============================================================
// SCIM filter attribute → SQL column mapping
// ============================================================

/// Map a SCIM attribute path to the corresponding SQL column.
/// Returns None for unsupported attributes.
pub fn scim_attr_to_column(attr: &str) -> Option<&'static str> {
    match attr.to_lowercase().as_str() {
        "username" | "emails.value" | "emails[type eq \"work\"].value" => Some("users.email"),
        "displayname" | "name.formatted" => Some("users.display_name"),
        "externalid" => Some("users.scim_external_id"),
        "active" => Some("users.locked_until"),
        "id" => Some("users.id"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scim_error_not_found() {
        let err = ScimError::not_found("User not found");
        assert_eq!(err.status, "404");
        assert!(err.scim_type.is_none());
        assert_eq!(err.detail.as_deref(), Some("User not found"));
    }

    #[test]
    fn test_scim_error_bad_request() {
        let err = ScimError::bad_request("Invalid filter");
        assert_eq!(err.status, "400");
        assert_eq!(err.scim_type.as_deref(), Some("invalidValue"));
    }

    #[test]
    fn test_scim_error_conflict() {
        let err = ScimError::conflict("Email already exists");
        assert_eq!(err.status, "409");
        assert_eq!(err.scim_type.as_deref(), Some("uniqueness"));
    }

    #[test]
    fn test_scim_error_unauthorized() {
        let err = ScimError::unauthorized("Invalid token");
        assert_eq!(err.status, "401");
    }

    #[test]
    fn test_scim_list_response() {
        let resp = ScimListResponse::new(vec!["a", "b"], 10, 1, 2);
        assert_eq!(resp.total_results, 10);
        assert_eq!(resp.start_index, 1);
        assert_eq!(resp.items_per_page, 2);
        assert_eq!(resp.resources.len(), 2);
    }

    #[test]
    fn test_scim_user_schema() {
        assert_eq!(
            ScimUser::SCHEMA,
            "urn:ietf:params:scim:schemas:core:2.0:User"
        );
    }

    #[test]
    fn test_scim_group_schema() {
        assert_eq!(
            ScimGroup::SCHEMA,
            "urn:ietf:params:scim:schemas:core:2.0:Group"
        );
    }

    #[test]
    fn test_scim_compare_op_from_str() {
        assert_eq!(ScimCompareOp::parse("eq"), Some(ScimCompareOp::Eq));
        assert_eq!(ScimCompareOp::parse("NE"), Some(ScimCompareOp::Ne));
        assert_eq!(ScimCompareOp::parse("co"), Some(ScimCompareOp::Co));
        assert_eq!(ScimCompareOp::parse("SW"), Some(ScimCompareOp::Sw));
        assert_eq!(ScimCompareOp::parse("ew"), Some(ScimCompareOp::Ew));
        assert_eq!(ScimCompareOp::parse("gt"), Some(ScimCompareOp::Gt));
        assert_eq!(ScimCompareOp::parse("GE"), Some(ScimCompareOp::Ge));
        assert_eq!(ScimCompareOp::parse("lt"), Some(ScimCompareOp::Lt));
        assert_eq!(ScimCompareOp::parse("le"), Some(ScimCompareOp::Le));
        assert_eq!(ScimCompareOp::parse("invalid"), None);
    }

    #[test]
    fn test_scim_attr_to_column() {
        assert_eq!(scim_attr_to_column("userName"), Some("users.email"));
        assert_eq!(
            scim_attr_to_column("displayName"),
            Some("users.display_name")
        );
        assert_eq!(
            scim_attr_to_column("externalId"),
            Some("users.scim_external_id")
        );
        assert_eq!(scim_attr_to_column("active"), Some("users.locked_until"));
        assert_eq!(scim_attr_to_column("id"), Some("users.id"));
        assert_eq!(scim_attr_to_column("unknownAttr"), None);
    }

    #[test]
    fn test_scim_service_provider_config_default() {
        let config = ScimServiceProviderConfig::default();
        assert!(config.patch.supported);
        assert!(config.bulk.supported);
        assert!(config.filter.supported);
        assert!(!config.change_password.supported);
        assert!(!config.sort.supported);
        assert!(!config.etag.supported);
        assert_eq!(config.authentication_schemes.len(), 1);
    }

    #[test]
    fn test_scim_user_serialization() {
        let user = ScimUser {
            schemas: vec![ScimUser::SCHEMA.to_string()],
            id: Some("123".to_string()),
            external_id: Some("ext-456".to_string()),
            user_name: "test@example.com".to_string(),
            name: Some(ScimName {
                given_name: Some("Test".to_string()),
                family_name: Some("User".to_string()),
                formatted: None,
            }),
            display_name: Some("Test User".to_string()),
            emails: vec![ScimEmail {
                value: "test@example.com".to_string(),
                email_type: Some("work".to_string()),
                primary: true,
            }],
            photos: vec![],
            active: true,
            meta: None,
            groups: vec![],
        };
        let json = serde_json::to_value(&user).unwrap();
        assert_eq!(json["userName"], "test@example.com");
        assert_eq!(json["displayName"], "Test User");
        assert_eq!(json["externalId"], "ext-456");
        assert_eq!(json["active"], true);
    }

    #[test]
    fn test_scim_group_serialization() {
        let group = ScimGroup {
            schemas: vec![ScimGroup::SCHEMA.to_string()],
            id: Some("grp-1".to_string()),
            external_id: None,
            display_name: "Engineering".to_string(),
            members: vec![ScimMember {
                value: "user-1".to_string(),
                ref_uri: None,
                display: Some("Alice".to_string()),
            }],
            meta: None,
        };
        let json = serde_json::to_value(&group).unwrap();
        assert_eq!(json["displayName"], "Engineering");
        assert_eq!(json["members"][0]["value"], "user-1");
    }

    #[test]
    fn test_scim_patch_op_deserialization() {
        let json = serde_json::json!({
            "schemas": [ScimPatchOp::SCHEMA],
            "Operations": [
                {"op": "replace", "path": "displayName", "value": "New Name"},
                {"op": "add", "path": "emails", "value": [{"value": "new@example.com", "primary": true}]}
            ]
        });
        let patch: ScimPatchOp = serde_json::from_value(json).unwrap();
        assert_eq!(patch.operations.len(), 2);
        assert_eq!(patch.operations[0].op, "replace");
    }

    #[test]
    fn test_scim_token_response_from() {
        let token = ScimToken {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            connector_id: StringUuid::new_v4(),
            token_hash: "hash123".to_string(),
            token_prefix: "scim_abc".to_string(),
            description: Some("Test token".to_string()),
            expires_at: None,
            last_used_at: None,
            revoked_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let resp = ScimTokenResponse::from(token.clone());
        assert_eq!(resp.id, token.id);
        assert_eq!(resp.token_prefix, "scim_abc");
        assert_eq!(resp.description.as_deref(), Some("Test token"));
    }

    #[test]
    fn test_mapped_user_fields_default() {
        let fields = MappedUserFields::default();
        assert!(fields.email.is_none());
        assert!(fields.display_name.is_none());
        assert!(fields.active.is_none());
    }
}
