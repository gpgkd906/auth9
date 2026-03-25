//! Action claims sanitization: denylist filtering and namespace prefixing.

use std::collections::HashMap;

/// JWT standard and Auth9 reserved claim keys that actions must never override.
const RESERVED_CLAIMS: &[&str] = &[
    // JWT / OIDC standard
    "sub",
    "sid",
    "email",
    "name",
    "iss",
    "aud",
    "token_type",
    "iat",
    "exp",
    "nbf",
    "jti",
    "nonce",
    "at_hash",
    "c_hash",
    "auth_time",
    "acr",
    "amr",
    "azp",
    // Auth9 TenantAccessClaims-specific
    "tenant_id",
    "roles",
    "permissions",
];

/// Namespace prefix applied to all action-produced claim keys.
pub const NAMESPACE_PREFIX: &str = "https://auth9.dev/";

/// Sanitize action claims by filtering reserved keys and applying namespace prefix.
///
/// Returns `None` if the result is empty (all keys were reserved).
pub fn sanitize_action_claims(
    raw: HashMap<String, serde_json::Value>,
) -> Option<HashMap<String, serde_json::Value>> {
    let mut result = HashMap::with_capacity(raw.len());
    for (key, value) in raw {
        if RESERVED_CLAIMS.contains(&key.as_str()) {
            tracing::warn!(
                key = %key,
                "Filtered reserved claim key from action output"
            );
            continue;
        }
        let namespaced = if key.starts_with(NAMESPACE_PREFIX) {
            key
        } else {
            format!("{NAMESPACE_PREFIX}{key}")
        };
        result.insert(namespaced, value);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filters_reserved_claims() {
        let mut raw = HashMap::new();
        raw.insert("sub".to_string(), json!("evil-user-id"));
        raw.insert("iss".to_string(), json!("evil-issuer"));
        raw.insert("exp".to_string(), json!(9999999999i64));
        raw.insert("department".to_string(), json!("engineering"));

        let result = sanitize_action_claims(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("https://auth9.dev/department"),
            Some(&json!("engineering"))
        );
        assert!(!result.contains_key("sub"));
        assert!(!result.contains_key("iss"));
        assert!(!result.contains_key("exp"));
    }

    #[test]
    fn test_namespace_prefix_applied() {
        let mut raw = HashMap::new();
        raw.insert("role".to_string(), json!("admin"));
        raw.insert("org_id".to_string(), json!("org-123"));

        let result = sanitize_action_claims(raw).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("https://auth9.dev/role"),
            Some(&json!("admin"))
        );
        assert_eq!(
            result.get("https://auth9.dev/org_id"),
            Some(&json!("org-123"))
        );
    }

    #[test]
    fn test_already_prefixed_not_double_prefixed() {
        let mut raw = HashMap::new();
        raw.insert(
            "https://auth9.dev/foo".to_string(),
            json!("bar"),
        );

        let result = sanitize_action_claims(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("https://auth9.dev/foo"),
            Some(&json!("bar"))
        );
        // Ensure no double-prefix key exists
        assert!(!result.contains_key("https://auth9.dev/https://auth9.dev/foo"));
    }

    #[test]
    fn test_all_reserved_returns_none() {
        let mut raw = HashMap::new();
        raw.insert("sub".to_string(), json!("evil"));
        raw.insert("aud".to_string(), json!("evil"));
        raw.insert("token_type".to_string(), json!("evil"));

        assert!(sanitize_action_claims(raw).is_none());
    }

    #[test]
    fn test_tenant_access_reserved_keys_filtered() {
        let mut raw = HashMap::new();
        raw.insert("tenant_id".to_string(), json!("hijack-tenant"));
        raw.insert("roles".to_string(), json!(["super-admin"]));
        raw.insert("permissions".to_string(), json!(["*"]));
        raw.insert("custom_field".to_string(), json!("safe"));

        let result = sanitize_action_claims(raw).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("https://auth9.dev/custom_field"),
            Some(&json!("safe"))
        );
        assert!(!result.contains_key("tenant_id"));
        assert!(!result.contains_key("roles"));
        assert!(!result.contains_key("permissions"));
    }
}
