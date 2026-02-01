//! API Key authentication for gRPC services
//!
//! Validates requests using an API key passed in the `x-api-key` header.

use super::auth::{AuthContext, GrpcAuthenticator};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tonic::Status;

/// API Key header name
const API_KEY_HEADER: &str = "x-api-key";

/// API Key authenticator for gRPC services
///
/// Validates requests by checking the `x-api-key` header against a list of
/// configured API keys. Keys are stored as SHA-256 hashes for security.
#[derive(Clone)]
pub struct ApiKeyAuthenticator {
    /// Set of valid API key hashes
    valid_key_hashes: HashSet<String>,
}

impl ApiKeyAuthenticator {
    /// Create a new API key authenticator from a list of plain-text keys
    ///
    /// Keys are hashed before storage for security.
    pub fn new(api_keys: Vec<String>) -> Self {
        let valid_key_hashes = api_keys
            .into_iter()
            .filter(|k| !k.is_empty())
            .map(|k| Self::hash_key(&k))
            .collect();

        Self { valid_key_hashes }
    }

    /// Hash an API key using SHA-256
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Validate an API key
    fn validate_key(&self, key: &str) -> bool {
        let hash = Self::hash_key(key);
        self.valid_key_hashes.contains(&hash)
    }

    /// Check if any API keys are configured
    pub fn has_keys(&self) -> bool {
        !self.valid_key_hashes.is_empty()
    }
}

impl GrpcAuthenticator for ApiKeyAuthenticator {
    fn authenticate(&self, metadata: &tonic::metadata::MetadataMap) -> Result<AuthContext, Status> {
        // Get the API key from the request header
        let api_key = metadata
            .get(API_KEY_HEADER)
            .ok_or_else(|| Status::unauthenticated("Missing API key. Provide 'x-api-key' header."))?
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid API key format"))?;

        // Validate the API key
        if !self.validate_key(api_key) {
            return Err(Status::unauthenticated("Invalid API key"));
        }

        // Create auth context
        // Use first 8 chars of the key hash as client ID (for logging)
        let key_hash = Self::hash_key(api_key);
        let client_id = format!("apikey:{}", &key_hash[..8]);

        Ok(AuthContext::api_key(client_id))
    }

    fn name(&self) -> &'static str {
        "api_key"
    }
}

// Re-export hex encoding for key hashing
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut hex = String::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            hex.push(HEX_CHARS[(byte >> 4) as usize] as char);
            hex.push(HEX_CHARS[(byte & 0xf) as usize] as char);
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_authenticator_new() {
        let auth = ApiKeyAuthenticator::new(vec!["key1".to_string(), "key2".to_string()]);

        assert!(auth.has_keys());
        assert_eq!(auth.valid_key_hashes.len(), 2);
    }

    #[test]
    fn test_api_key_authenticator_empty() {
        let auth = ApiKeyAuthenticator::new(vec![]);
        assert!(!auth.has_keys());
    }

    #[test]
    fn test_api_key_authenticator_filters_empty_keys() {
        let auth =
            ApiKeyAuthenticator::new(vec!["key1".to_string(), "".to_string(), "key2".to_string()]);

        assert_eq!(auth.valid_key_hashes.len(), 2);
    }

    #[test]
    fn test_validate_key_success() {
        let auth = ApiKeyAuthenticator::new(vec!["test-api-key".to_string()]);
        assert!(auth.validate_key("test-api-key"));
    }

    #[test]
    fn test_validate_key_failure() {
        let auth = ApiKeyAuthenticator::new(vec!["test-api-key".to_string()]);
        assert!(!auth.validate_key("wrong-key"));
    }

    #[test]
    fn test_hash_key_deterministic() {
        let hash1 = ApiKeyAuthenticator::hash_key("test");
        let hash2 = ApiKeyAuthenticator::hash_key("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_key_different_inputs() {
        let hash1 = ApiKeyAuthenticator::hash_key("key1");
        let hash2 = ApiKeyAuthenticator::hash_key("key2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_authenticate_success() {
        let auth = ApiKeyAuthenticator::new(vec!["valid-key".to_string()]);

        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert(API_KEY_HEADER, "valid-key".parse().unwrap());

        let result = auth.authenticate(&metadata);
        assert!(result.is_ok());

        let ctx = result.unwrap();
        assert!(ctx.client_id.starts_with("apikey:"));
    }

    #[test]
    fn test_authenticate_missing_header() {
        let auth = ApiKeyAuthenticator::new(vec!["valid-key".to_string()]);
        let metadata = tonic::metadata::MetadataMap::new();

        let result = auth.authenticate(&metadata);
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert!(status.message().contains("Missing API key"));
    }

    #[test]
    fn test_authenticate_invalid_key() {
        let auth = ApiKeyAuthenticator::new(vec!["valid-key".to_string()]);

        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert(API_KEY_HEADER, "invalid-key".parse().unwrap());

        let result = auth.authenticate(&metadata);
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unauthenticated);
        assert!(status.message().contains("Invalid API key"));
    }

    #[test]
    fn test_authenticator_name() {
        let auth = ApiKeyAuthenticator::new(vec![]);
        assert_eq!(auth.name(), "api_key");
    }

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex::encode([0x00]), "00");
        assert_eq!(hex::encode([0xff]), "ff");
        assert_eq!(hex::encode([0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn test_multiple_valid_keys() {
        let auth = ApiKeyAuthenticator::new(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);

        assert!(auth.validate_key("key1"));
        assert!(auth.validate_key("key2"));
        assert!(auth.validate_key("key3"));
        assert!(!auth.validate_key("key4"));
    }
}
