//! AES-256-GCM encryption for sensitive configuration data
//!
//! This module provides encryption/decryption for sensitive settings
//! like SMTP passwords, API keys, and other secrets stored in the database.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;
use thiserror::Error;

/// Encryption key for AES-256-GCM
#[derive(Clone)]
pub struct EncryptionKey {
    key: [u8; 32],
}

/// Encryption error types
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Invalid key: must be exactly 32 bytes (256 bits)")]
    InvalidKeyLength,

    #[error("Invalid base64 encoding: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed: invalid ciphertext or wrong key")]
    DecryptionFailed,

    #[error("Invalid ciphertext format")]
    InvalidCiphertextFormat,
}

impl EncryptionKey {
    /// Create a new encryption key from a 32-byte array
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Create encryption key from a base64-encoded string
    pub fn from_base64(encoded: &str) -> Result<Self, EncryptionError> {
        let bytes = BASE64.decode(encoded)?;
        if bytes.len() != 32 {
            return Err(EncryptionError::InvalidKeyLength);
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self { key })
    }

    /// Create encryption key from environment variable
    pub fn from_env() -> Result<Self, EncryptionError> {
        let encoded = std::env::var("SETTINGS_ENCRYPTION_KEY")
            .map_err(|_| EncryptionError::InvalidKeyLength)?;
        Self::from_base64(&encoded)
    }

    /// Get the raw key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

/// Encrypt plaintext using AES-256-GCM
///
/// Returns base64-encoded ciphertext in format: nonce:ciphertext
/// The nonce is 12 bytes (96 bits) as required by GCM
pub fn encrypt(key: &EncryptionKey, plaintext: &str) -> Result<String, EncryptionError> {
    let cipher =
        Aes256Gcm::new_from_slice(&key.key).map_err(|_| EncryptionError::EncryptionFailed)?;

    // Generate random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| EncryptionError::EncryptionFailed)?;

    // Encode as base64: nonce:ciphertext
    let nonce_b64 = BASE64.encode(nonce_bytes);
    let ciphertext_b64 = BASE64.encode(&ciphertext);

    Ok(format!("{}:{}", nonce_b64, ciphertext_b64))
}

/// Decrypt ciphertext that was encrypted with [encrypt]
///
/// Expects base64-encoded input in format: nonce:ciphertext
pub fn decrypt(key: &EncryptionKey, encrypted: &str) -> Result<String, EncryptionError> {
    let parts: Vec<&str> = encrypted.split(':').collect();
    if parts.len() != 2 {
        return Err(EncryptionError::InvalidCiphertextFormat);
    }

    let nonce_bytes = BASE64.decode(parts[0])?;
    if nonce_bytes.len() != 12 {
        return Err(EncryptionError::InvalidCiphertextFormat);
    }

    let ciphertext = BASE64.decode(parts[1])?;

    let cipher =
        Aes256Gcm::new_from_slice(&key.key).map_err(|_| EncryptionError::DecryptionFailed)?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| EncryptionError::DecryptionFailed)?;

    String::from_utf8(plaintext).map_err(|_| EncryptionError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> EncryptionKey {
        // Test key: 32 bytes
        EncryptionKey::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ])
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = "my-secret-password";

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let key = test_key();
        let plaintext = "test-password";

        let encrypted1 = encrypt(&key, plaintext).unwrap();
        let encrypted2 = encrypt(&key, plaintext).unwrap();

        // Due to random nonce, encryptions should be different
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(decrypt(&key, &encrypted1).unwrap(), plaintext);
        assert_eq!(decrypt(&key, &encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = test_key();
        let key2 = EncryptionKey::new([0xffu8; 32]);

        let plaintext = "secret";
        let encrypted = encrypt(&key1, plaintext).unwrap();

        let result = decrypt(&key2, &encrypted);
        assert!(matches!(result, Err(EncryptionError::DecryptionFailed)));
    }

    #[test]
    fn test_decrypt_invalid_format() {
        let key = test_key();

        // Missing colon separator
        let result = decrypt(&key, "invalid");
        assert!(matches!(
            result,
            Err(EncryptionError::InvalidCiphertextFormat)
        ));

        // Too many parts
        let result = decrypt(&key, "a:b:c");
        assert!(matches!(
            result,
            Err(EncryptionError::InvalidCiphertextFormat)
        ));
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let key = test_key();

        let result = decrypt(&key, "!!!:valid");
        assert!(matches!(result, Err(EncryptionError::Base64Error(_))));
    }

    #[test]
    fn test_key_from_base64() {
        // Generate a valid 32-byte key in base64
        let key_bytes = [0x42u8; 32];
        let encoded = BASE64.encode(key_bytes);

        let key = EncryptionKey::from_base64(&encoded).unwrap();
        assert_eq!(key.as_bytes(), &key_bytes);
    }

    #[test]
    fn test_key_from_base64_wrong_length() {
        let short_key = BASE64.encode([0x42u8; 16]); // Only 16 bytes
        let result = EncryptionKey::from_base64(&short_key);
        assert!(matches!(result, Err(EncryptionError::InvalidKeyLength)));
    }

    #[test]
    fn test_key_from_base64_invalid_encoding() {
        let result = EncryptionKey::from_base64("not-valid-base64!!!");
        assert!(matches!(result, Err(EncryptionError::Base64Error(_))));
    }

    #[test]
    fn test_encrypt_empty_string() {
        let key = test_key();
        let encrypted = encrypt(&key, "").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_encrypt_unicode() {
        let key = test_key();
        let plaintext = "Hello, World!";

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_long_text() {
        let key = test_key();
        let plaintext = "a".repeat(10000);

        let encrypted = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_format() {
        let key = test_key();
        let encrypted = encrypt(&key, "test").unwrap();

        // Should be in format nonce:ciphertext
        let parts: Vec<&str> = encrypted.split(':').collect();
        assert_eq!(parts.len(), 2);

        // Nonce should be 12 bytes = 16 base64 chars
        let nonce = BASE64.decode(parts[0]).unwrap();
        assert_eq!(nonce.len(), 12);

        // Ciphertext should be non-empty
        assert!(!parts[1].is_empty());
    }

    #[test]
    fn test_key_clone() {
        let key1 = test_key();
        let key2 = key1.clone();

        let plaintext = "test";
        let encrypted = encrypt(&key1, plaintext).unwrap();
        let decrypted = decrypt(&key2, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_error_display() {
        let errors = vec![
            EncryptionError::InvalidKeyLength,
            EncryptionError::EncryptionFailed,
            EncryptionError::DecryptionFailed,
            EncryptionError::InvalidCiphertextFormat,
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }
}
