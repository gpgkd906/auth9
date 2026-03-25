//! LDAP/Active Directory connector models.

use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

// ── LDAP Config (parsed from enterprise_sso_connectors.config JSON) ──

#[derive(Debug, Clone)]
pub struct LdapConfig {
    pub server_url: String,
    pub use_tls: bool,
    pub tls_skip_verify: bool,
    pub tls_ca_cert: Option<String>,
    pub connection_timeout_secs: u64,

    pub bind_dn: String,
    pub bind_password: String, // pragma: allowlist secret

    pub base_dn: String,
    pub user_search_filter: String,
    pub user_search_scope: SearchScope,
    pub group_search_base: Option<String>,
    pub group_search_filter: Option<String>,

    pub attr_username: String,
    pub attr_email: String,
    pub attr_first_name: String,
    pub attr_last_name: String,
    pub attr_display_name: Option<String>,
    pub attr_phone: Option<String>,
    pub attr_groups: Option<String>,

    pub is_active_directory: bool,
    pub ad_domain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Sub,
    One,
    Base,
}

impl SearchScope {
    pub fn to_ldap3_scope(&self) -> ldap3::Scope {
        match self {
            SearchScope::Sub => ldap3::Scope::Subtree,
            SearchScope::One => ldap3::Scope::OneLevel,
            SearchScope::Base => ldap3::Scope::Base,
        }
    }
}

pub fn parse_ldap_config(config: &HashMap<String, String>) -> Result<LdapConfig> {
    let server_url = config
        .get("serverUrl")
        .filter(|s| !s.is_empty())
        .cloned()
        .ok_or_else(|| AppError::Validation("Missing required LDAP config: serverUrl".into()))?;

    let bind_dn = config
        .get("bindDn")
        .filter(|s| !s.is_empty())
        .cloned()
        .ok_or_else(|| AppError::Validation("Missing required LDAP config: bindDn".into()))?;

    let bind_password = config // pragma: allowlist secret
        .get("bindPassword")
        .filter(|s| !s.is_empty())
        .cloned()
        .ok_or_else(|| AppError::Validation("Missing required LDAP config: bindPassword".into()))?; // pragma: allowlist secret

    let base_dn = config
        .get("baseDn")
        .filter(|s| !s.is_empty())
        .cloned()
        .ok_or_else(|| AppError::Validation("Missing required LDAP config: baseDn".into()))?;

    let is_active_directory = config
        .get("isActiveDirectory")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let default_user_filter = if is_active_directory {
        "(sAMAccountName={username})"
    } else {
        "(uid={username})"
    };

    let scope = match config.get("userSearchScope").map(|s| s.as_str()) {
        Some("one") => SearchScope::One,
        Some("base") => SearchScope::Base,
        _ => SearchScope::Sub,
    };

    let default_attr_username = if is_active_directory {
        "sAMAccountName"
    } else {
        "uid"
    };

    Ok(LdapConfig {
        server_url,
        use_tls: config
            .get("useTls")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true),
        tls_skip_verify: config
            .get("tlsSkipVerify")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false),
        tls_ca_cert: config.get("tlsCaCert").filter(|s| !s.is_empty()).cloned(),
        connection_timeout_secs: config
            .get("connectionTimeoutSecs")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),
        bind_dn,
        bind_password, // pragma: allowlist secret
        base_dn,
        user_search_filter: config
            .get("userSearchFilter")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| default_user_filter.to_string()),
        user_search_scope: scope,
        group_search_base: config
            .get("groupSearchBase")
            .filter(|s| !s.is_empty())
            .cloned(),
        group_search_filter: config
            .get("groupSearchFilter")
            .filter(|s| !s.is_empty())
            .cloned(),
        attr_username: config
            .get("attrUsername")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| default_attr_username.to_string()),
        attr_email: config
            .get("attrEmail")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "mail".to_string()),
        attr_first_name: config
            .get("attrFirstName")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "givenName".to_string()),
        attr_last_name: config
            .get("attrLastName")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "sn".to_string()),
        attr_display_name: config
            .get("attrDisplayName")
            .filter(|s| !s.is_empty())
            .cloned(),
        attr_phone: config
            .get("attrPhone")
            .filter(|s| !s.is_empty())
            .cloned(),
        attr_groups: config
            .get("attrGroups")
            .filter(|s| !s.is_empty())
            .cloned()
            .or_else(|| {
                if is_active_directory {
                    Some("memberOf".to_string())
                } else {
                    None
                }
            }),
        is_active_directory,
        ad_domain: config
            .get("adDomain")
            .filter(|s| !s.is_empty())
            .cloned(),
    })
}

// ── LDAP User Profile (authentication result) ──

#[derive(Debug, Clone)]
pub struct LdapUserProfile {
    pub dn: String,
    pub username: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub phone: Option<String>,
    pub groups: Vec<String>,
}

// ── LDAP Group-Role Mapping (DB entity) ──

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LdapGroupRoleMapping {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub connector_id: StringUuid,
    pub ldap_group_dn: String,
    pub ldap_group_display_name: Option<String>,
    pub role_id: StringUuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateLdapGroupRoleMappingInput {
    #[validate(length(min = 1, max = 1024))]
    pub ldap_group_dn: String,
    pub ldap_group_display_name: Option<String>,
    pub role_id: StringUuid,
}

// ── LDAP Search Filter Escaping (RFC 4515) ──

/// Escape special characters in LDAP search filter values per RFC 4515.
/// This MUST be called on any user-supplied value before interpolation into search filters
/// to prevent LDAP injection attacks.
pub fn escape_ldap_search_filter(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len() + 8);
    for c in input.chars() {
        match c {
            '\\' => escaped.push_str("\\5c"),
            '*' => escaped.push_str("\\2a"),
            '(' => escaped.push_str("\\28"),
            ')' => escaped.push_str("\\29"),
            '\0' => escaped.push_str("\\00"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── escape_ldap_search_filter ──

    #[test]
    fn escape_filter_no_special_chars() {
        assert_eq!(escape_ldap_search_filter("john.doe"), "john.doe");
    }

    #[test]
    fn escape_filter_asterisk() {
        assert_eq!(escape_ldap_search_filter("user*"), "user\\2a");
    }

    #[test]
    fn escape_filter_parentheses() {
        assert_eq!(escape_ldap_search_filter("(admin)"), "\\28admin\\29");
    }

    #[test]
    fn escape_filter_backslash() {
        assert_eq!(escape_ldap_search_filter("user\\name"), "user\\5cname");
    }

    #[test]
    fn escape_filter_null_byte() {
        assert_eq!(escape_ldap_search_filter("user\0name"), "user\\00name");
    }

    #[test]
    fn escape_filter_all_special_chars() {
        assert_eq!(
            escape_ldap_search_filter("a*b(c)d\\e\0f"),
            "a\\2ab\\28c\\29d\\5ce\\00f"
        );
    }

    #[test]
    fn escape_filter_injection_attempt() {
        // Attempt: )(cn=*) -- should be fully escaped
        assert_eq!(
            escape_ldap_search_filter(")(cn=*)"),
            "\\29\\28cn=\\2a\\29"
        );
    }

    #[test]
    fn escape_filter_empty_string() {
        assert_eq!(escape_ldap_search_filter(""), "");
    }

    // ── parse_ldap_config ──

    fn minimal_config() -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert("serverUrl".into(), "ldaps://ldap.example.com:636".into());
        config.insert("bindDn".into(), "cn=auth9,ou=svc,dc=example,dc=com".into());
        config.insert("bindPassword".into(), "secret123".into()); // pragma: allowlist secret
        config.insert("baseDn".into(), "ou=users,dc=example,dc=com".into());
        config
    }

    #[test]
    fn parse_config_minimal() {
        let config = parse_ldap_config(&minimal_config()).unwrap();
        assert_eq!(config.server_url, "ldaps://ldap.example.com:636");
        assert_eq!(config.bind_dn, "cn=auth9,ou=svc,dc=example,dc=com");
        assert_eq!(config.base_dn, "ou=users,dc=example,dc=com");
        assert!(config.use_tls);
        assert!(!config.tls_skip_verify);
        assert!(!config.is_active_directory);
        assert_eq!(config.user_search_filter, "(uid={username})");
        assert_eq!(config.attr_username, "uid");
        assert_eq!(config.attr_email, "mail");
    }

    #[test]
    fn parse_config_active_directory_defaults() {
        let mut config = minimal_config();
        config.insert("isActiveDirectory".into(), "true".into());
        let parsed = parse_ldap_config(&config).unwrap();
        assert!(parsed.is_active_directory);
        assert_eq!(parsed.user_search_filter, "(sAMAccountName={username})");
        assert_eq!(parsed.attr_username, "sAMAccountName");
        assert_eq!(parsed.attr_groups.as_deref(), Some("memberOf"));
    }

    #[test]
    fn parse_config_missing_server_url() {
        let mut config = minimal_config();
        config.remove("serverUrl");
        assert!(parse_ldap_config(&config).is_err());
    }

    #[test]
    fn parse_config_missing_bind_dn() {
        let mut config = minimal_config();
        config.remove("bindDn");
        assert!(parse_ldap_config(&config).is_err());
    }

    #[test]
    fn parse_config_missing_bind_password() {
        let mut config = minimal_config();
        config.remove("bindPassword"); // pragma: allowlist secret
        assert!(parse_ldap_config(&config).is_err());
    }

    #[test]
    fn parse_config_missing_base_dn() {
        let mut config = minimal_config();
        config.remove("baseDn");
        assert!(parse_ldap_config(&config).is_err());
    }

    #[test]
    fn parse_config_custom_attributes() {
        let mut config = minimal_config();
        config.insert("attrUsername".into(), "cn".into());
        config.insert("attrEmail".into(), "email".into());
        config.insert("attrFirstName".into(), "firstName".into());
        config.insert("attrLastName".into(), "lastName".into());
        config.insert("attrDisplayName".into(), "fullName".into());
        config.insert("attrPhone".into(), "mobile".into());
        config.insert("attrGroups".into(), "groupMembership".into());
        let parsed = parse_ldap_config(&config).unwrap();
        assert_eq!(parsed.attr_username, "cn");
        assert_eq!(parsed.attr_email, "email");
        assert_eq!(parsed.attr_first_name, "firstName");
        assert_eq!(parsed.attr_last_name, "lastName");
        assert_eq!(parsed.attr_display_name.as_deref(), Some("fullName"));
        assert_eq!(parsed.attr_phone.as_deref(), Some("mobile"));
        assert_eq!(parsed.attr_groups.as_deref(), Some("groupMembership"));
    }

    #[test]
    fn parse_config_search_scope_one() {
        let mut config = minimal_config();
        config.insert("userSearchScope".into(), "one".into());
        let parsed = parse_ldap_config(&config).unwrap();
        assert_eq!(parsed.user_search_scope, SearchScope::One);
    }

    #[test]
    fn parse_config_tls_options() {
        let mut config = minimal_config();
        config.insert("useTls".into(), "false".into());
        config.insert("tlsSkipVerify".into(), "true".into());
        config.insert("connectionTimeoutSecs".into(), "30".into());
        let parsed = parse_ldap_config(&config).unwrap();
        assert!(!parsed.use_tls);
        assert!(parsed.tls_skip_verify);
        assert_eq!(parsed.connection_timeout_secs, 30);
    }
}
