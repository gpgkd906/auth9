//! Common types and shared validation for domain models

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;
use validator::ValidationError;

/// Wrapper type for UUID stored as CHAR(36) in MySQL/TiDB
/// sqlx's uuid feature expects BINARY(16), but we use CHAR(36)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StringUuid(pub Uuid);

impl StringUuid {
    pub fn new_v4() -> Self {
        StringUuid(Uuid::new_v4())
    }

    pub fn nil() -> Self {
        StringUuid(Uuid::nil())
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Parse a UUID string
    pub fn parse_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(StringUuid(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for StringUuid {
    fn from(uuid: Uuid) -> Self {
        StringUuid(uuid)
    }
}

impl From<StringUuid> for Uuid {
    fn from(s: StringUuid) -> Self {
        s.0
    }
}

impl std::ops::Deref for StringUuid {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for StringUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for StringUuid {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StringUuid(Uuid::parse_str(s)?))
    }
}

impl sqlx::Type<sqlx::MySql> for StringUuid {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for StringUuid {
    fn decode(value: sqlx::mysql::MySqlValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::MySql>>::decode(value)?;
        let uuid = Uuid::parse_str(&s)?;
        Ok(StringUuid(uuid))
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for StringUuid {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&self.0.to_string(), buf)
    }
}

/// Validate a URL against SSRF attacks.
/// Blocks cloud metadata endpoints, private/loopback IPs, and restricts HTTP to local networks.
pub fn validate_url_no_ssrf(url: &str) -> Result<(), ValidationError> {
    let parsed = Url::parse(url).map_err(|_| ValidationError::new("invalid_url"))?;

    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("");

    // Only allow http and https schemes
    if scheme != "http" && scheme != "https" {
        return Err(ValidationError::new("invalid_scheme"));
    }

    // Block cloud metadata endpoints for ALL schemes
    let is_cloud_metadata = host == "169.254.169.254" || host == "metadata.google.internal";
    let is_loopback = host == "127.0.0.1" || host == "::1" || host == "0.0.0.0";
    let is_private = host.starts_with("192.168.")
        || host.starts_with("10.")
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|s| s.parse::<u8>().ok())
                .map(|n| (16..=31).contains(&n))
                .unwrap_or(false));

    if is_cloud_metadata {
        let mut err = ValidationError::new("ssrf_blocked");
        err.message = Some("Cloud metadata endpoints are not allowed".into());
        return Err(err);
    }

    // For HTTPS: allow external URLs, block private/loopback
    if scheme == "https" {
        if is_loopback || is_private {
            let mut err = ValidationError::new("internal_ip_blocked");
            err.message = Some("Internal IP addresses are not allowed".into());
            return Err(err);
        }
        return Ok(());
    }

    // HTTP only allowed for localhost/private networks (dev environment)
    if scheme == "http" {
        let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";
        if is_localhost || is_private {
            return Ok(());
        }

        let mut err = ValidationError::new("http_not_allowed");
        err.message = Some(
            "HTTP URLs are only allowed for localhost or private networks. Use HTTPS for external URLs.".into(),
        );
        return Err(err);
    }

    Err(ValidationError::new("invalid_scheme"))
}

/// Validate an optional URL against SSRF (for use with validator crate on Option<String> fields)
pub fn validate_url_no_ssrf_option(url: &str) -> Result<(), ValidationError> {
    validate_url_no_ssrf(url)
}

/// Strict URL validation that blocks ALL private/loopback IPs regardless of scheme.
/// Use for user-supplied URLs that will be stored and rendered (e.g. branding logos).
pub fn validate_url_no_ssrf_strict(url: &str) -> Result<(), ValidationError> {
    let parsed = Url::parse(url).map_err(|_| ValidationError::new("invalid_url"))?;

    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("");

    // Only allow http and https schemes
    if scheme != "http" && scheme != "https" {
        return Err(ValidationError::new("invalid_scheme"));
    }

    let is_cloud_metadata = host == "169.254.169.254" || host == "metadata.google.internal";
    let is_loopback = host == "127.0.0.1"
        || host == "::1"
        || host == "0.0.0.0"
        || host == "localhost";
    let is_private = host.starts_with("192.168.")
        || host.starts_with("10.")
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|s| s.parse::<u8>().ok())
                .map(|n| (16..=31).contains(&n))
                .unwrap_or(false));

    if is_cloud_metadata {
        let mut err = ValidationError::new("ssrf_blocked");
        err.message = Some("Cloud metadata endpoints are not allowed".into());
        return Err(err);
    }

    if is_loopback || is_private {
        let mut err = ValidationError::new("internal_ip_blocked");
        err.message = Some("Internal IP addresses are not allowed".into());
        return Err(err);
    }

    // Block external HTTP (require HTTPS for external URLs)
    if scheme == "http" {
        let mut err = ValidationError::new("http_not_allowed");
        err.message = Some("Only HTTPS URLs are allowed".into());
        return Err(err);
    }

    Ok(())
}

/// Strict URL validation for optional fields
pub fn validate_url_no_ssrf_strict_option(url: &str) -> Result<(), ValidationError> {
    validate_url_no_ssrf_strict(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_uuid_new() {
        let uuid = StringUuid::new_v4();
        assert!(!uuid.is_nil());
    }

    #[test]
    fn test_string_uuid_nil() {
        let uuid = StringUuid::nil();
        assert!(uuid.is_nil());
        assert_eq!(uuid.0, Uuid::nil());
    }

    #[test]
    fn test_string_uuid_is_nil() {
        let nil_uuid = StringUuid::nil();
        let valid_uuid = StringUuid::new_v4();

        assert!(nil_uuid.is_nil());
        assert!(!valid_uuid.is_nil());
    }

    #[test]
    fn test_string_uuid_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_string_uuid_from_str_invalid() {
        let invalid = "not-a-uuid";
        let result: Result<StringUuid, _> = invalid.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_string_uuid_conversion() {
        let uuid = Uuid::new_v4();
        let string_uuid: StringUuid = uuid.into();
        let back: Uuid = string_uuid.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn test_string_uuid_deref() {
        let uuid = Uuid::new_v4();
        let string_uuid = StringUuid(uuid);

        // Test deref - should be able to call Uuid methods directly
        assert_eq!(*string_uuid, uuid);
        assert_eq!(string_uuid.as_bytes(), uuid.as_bytes());
    }

    #[test]
    fn test_string_uuid_display() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();

        // Test Display trait
        assert_eq!(format!("{}", uuid), uuid_str);
    }

    #[test]
    fn test_string_uuid_equality() {
        let uuid1 = StringUuid::new_v4();
        let uuid2 = uuid1;
        let uuid3 = StringUuid::new_v4();

        assert_eq!(uuid1, uuid2);
        assert_ne!(uuid1, uuid3);
    }

    #[test]
    fn test_string_uuid_hash() {
        use std::collections::HashSet;

        let uuid1 = StringUuid::new_v4();
        let uuid2 = StringUuid::new_v4();

        let mut set = HashSet::new();
        set.insert(uuid1);
        set.insert(uuid2);
        set.insert(uuid1); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_string_uuid_serialization() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid: StringUuid = uuid_str.parse().unwrap();

        // Test serde serialization
        let json = serde_json::to_string(&uuid).unwrap();
        assert_eq!(json, format!("\"{}\"", uuid_str));

        // Test deserialization
        let deserialized: StringUuid = serde_json::from_str(&json).unwrap();
        assert_eq!(uuid, deserialized);
    }

    #[test]
    fn test_string_uuid_copy() {
        let uuid1 = StringUuid::new_v4();
        let uuid2 = uuid1; // Copy

        // Both should be usable (Copy trait)
        assert_eq!(uuid1, uuid2);
        assert!(!uuid1.is_nil());
        assert!(!uuid2.is_nil());
    }

    #[test]
    fn test_string_uuid_parse_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid = StringUuid::parse_str(uuid_str).unwrap();
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_string_uuid_parse_str_invalid() {
        let result = StringUuid::parse_str("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_uuid_encode_by_ref() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let uuid = StringUuid::parse_str(uuid_str).unwrap();
        let mut buf = Vec::new();
        let result = sqlx::Encode::<sqlx::MySql>::encode_by_ref(&uuid, &mut buf);
        assert!(result.is_ok());
        // The encoded buffer should contain the UUID string
        let encoded = String::from_utf8_lossy(&buf);
        assert!(encoded.contains(uuid_str));
    }

    #[test]
    fn test_ssrf_allows_valid_https() {
        assert!(validate_url_no_ssrf("https://example.com/logo.png").is_ok());
    }

    #[test]
    fn test_ssrf_allows_private_ip_http_for_dev() {
        // Non-strict mode allows HTTP to private IPs (dev convenience for webhooks)
        assert!(validate_url_no_ssrf("http://192.168.1.1/logo.png").is_ok());
    }

    #[test]
    fn test_ssrf_allows_localhost_http() {
        assert!(validate_url_no_ssrf("http://localhost:8080/logo.png").is_ok());
    }

    #[test]
    fn test_ssrf_blocks_cloud_metadata() {
        let result = validate_url_no_ssrf("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "ssrf_blocked");
    }

    #[test]
    fn test_ssrf_blocks_gcp_metadata() {
        let result =
            validate_url_no_ssrf("http://metadata.google.internal/computeMetadata/v1/");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "ssrf_blocked");
    }

    #[test]
    fn test_ssrf_blocks_private_https() {
        let result = validate_url_no_ssrf("https://192.168.1.1/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "internal_ip_blocked");
    }

    #[test]
    fn test_ssrf_blocks_loopback_https() {
        let result = validate_url_no_ssrf("https://127.0.0.1/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "internal_ip_blocked");
    }

    #[test]
    fn test_ssrf_blocks_external_http() {
        let result = validate_url_no_ssrf("http://example.com/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "http_not_allowed");
    }

    #[test]
    fn test_ssrf_blocks_invalid_scheme() {
        assert!(validate_url_no_ssrf("ftp://example.com/logo.png").is_err());
    }

    #[test]
    fn test_ssrf_option_delegates() {
        assert!(validate_url_no_ssrf_option("https://example.com/logo.png").is_ok());
        assert!(validate_url_no_ssrf_option("http://example.com/x").is_err());
    }

    // Strict SSRF validation tests (for branding URLs)
    #[test]
    fn test_ssrf_strict_allows_valid_https() {
        assert!(validate_url_no_ssrf_strict("https://example.com/logo.png").is_ok());
    }

    #[test]
    fn test_ssrf_strict_blocks_private_ip_http() {
        let result = validate_url_no_ssrf_strict("http://192.168.1.1/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "internal_ip_blocked");
    }

    #[test]
    fn test_ssrf_strict_blocks_private_ip_https() {
        let result = validate_url_no_ssrf_strict("https://192.168.1.1/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "internal_ip_blocked");
    }

    #[test]
    fn test_ssrf_strict_blocks_localhost() {
        let result = validate_url_no_ssrf_strict("http://localhost:8080/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "internal_ip_blocked");
    }

    #[test]
    fn test_ssrf_strict_blocks_external_http() {
        let result = validate_url_no_ssrf_strict("http://example.com/logo.png");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "http_not_allowed");
    }

    #[test]
    fn test_ssrf_strict_blocks_cloud_metadata() {
        let result = validate_url_no_ssrf_strict("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code.as_ref(), "ssrf_blocked");
    }
}
