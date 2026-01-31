//! System settings service

use crate::crypto::{decrypt, encrypt, EncryptionKey};
use crate::domain::{
    EmailProviderConfig, SettingCategory, SettingKey, SystemSettingResponse, SystemSettingRow,
    UpsertSystemSettingInput,
};
use crate::error::{AppError, Result};
use crate::repository::SystemSettingsRepository;
use std::sync::Arc;

/// Service for managing system-wide settings
pub struct SystemSettingsService<R: SystemSettingsRepository> {
    repo: Arc<R>,
    encryption_key: Option<EncryptionKey>,
}

impl<R: SystemSettingsRepository> SystemSettingsService<R> {
    pub fn new(repo: Arc<R>, encryption_key: Option<EncryptionKey>) -> Self {
        Self {
            repo,
            encryption_key,
        }
    }

    /// Get email provider configuration
    pub async fn get_email_config(&self) -> Result<EmailProviderConfig> {
        let row = self
            .repo
            .get(
                SettingKey::EmailProvider.category().as_str(),
                SettingKey::EmailProvider.as_str(),
            )
            .await?;

        match row {
            Some(row) => self.parse_email_config(&row),
            None => Ok(EmailProviderConfig::None),
        }
    }

    /// Get email provider configuration for API response (with sensitive data masked)
    pub async fn get_email_config_masked(&self) -> Result<SystemSettingResponse> {
        let row = self
            .repo
            .get(
                SettingKey::EmailProvider.category().as_str(),
                SettingKey::EmailProvider.as_str(),
            )
            .await?;

        match row {
            Some(row) => Ok(self.mask_sensitive_fields(row)),
            None => Ok(SystemSettingResponse {
                category: SettingCategory::Email.as_str().to_string(),
                setting_key: SettingKey::EmailProvider.as_str().to_string(),
                value: serde_json::json!({"type": "none"}),
                description: Some("Email provider configuration".to_string()),
                updated_at: chrono::Utc::now(),
            }),
        }
    }

    /// Update email provider configuration
    pub async fn update_email_config(&self, config: EmailProviderConfig) -> Result<()> {
        // Encrypt sensitive fields if we have an encryption key
        let (value, encrypted) = self.prepare_email_config_for_storage(&config)?;

        let input = UpsertSystemSettingInput {
            category: SettingCategory::Email.as_str().to_string(),
            setting_key: SettingKey::EmailProvider.as_str().to_string(),
            value,
            encrypted,
            description: Some("Email provider configuration".to_string()),
        };

        self.repo.upsert(&input).await?;
        Ok(())
    }

    /// Test if email configuration is valid
    pub fn validate_email_config(&self, config: &EmailProviderConfig) -> Result<()> {
        use validator::Validate;

        match config {
            EmailProviderConfig::None => Ok(()),
            EmailProviderConfig::Smtp(cfg) => cfg
                .validate()
                .map_err(|e| AppError::Validation(e.to_string())),
            EmailProviderConfig::Ses(cfg) => cfg
                .validate()
                .map_err(|e| AppError::Validation(e.to_string())),
            EmailProviderConfig::Oracle(cfg) => cfg
                .validate()
                .map_err(|e| AppError::Validation(e.to_string())),
        }
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    fn parse_email_config(&self, row: &SystemSettingRow) -> Result<EmailProviderConfig> {
        let mut value = row.value.clone();

        // Decrypt sensitive fields if needed
        if row.encrypted {
            if let Some(key) = &self.encryption_key {
                value = self.decrypt_sensitive_fields(&value, key)?;
            } else {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "Encrypted settings but no encryption key configured"
                )));
            }
        }

        serde_json::from_value(value).map_err(|e| AppError::Internal(e.into()))
    }

    fn prepare_email_config_for_storage(
        &self,
        config: &EmailProviderConfig,
    ) -> Result<(serde_json::Value, bool)> {
        let mut value = serde_json::to_value(config).map_err(|e| AppError::Internal(e.into()))?;

        if let Some(key) = &self.encryption_key {
            // Encrypt sensitive fields
            value = self.encrypt_sensitive_fields(&value, key)?;
            Ok((value, true))
        } else {
            Ok((value, false))
        }
    }

    fn encrypt_sensitive_fields(
        &self,
        value: &serde_json::Value,
        key: &EncryptionKey,
    ) -> Result<serde_json::Value> {
        let mut value = value.clone();

        // Encrypt password fields based on provider type
        if let Some(obj) = value.as_object_mut() {
            if let Some(password) = obj.get("password").and_then(|v| v.as_str()) {
                let encrypted = encrypt(key, password)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Encryption failed: {}", e)))?;
                obj.insert("password".to_string(), serde_json::json!(encrypted));
            }

            if let Some(secret) = obj.get("secret_access_key").and_then(|v| v.as_str()) {
                let encrypted = encrypt(key, secret)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Encryption failed: {}", e)))?;
                obj.insert(
                    "secret_access_key".to_string(),
                    serde_json::json!(encrypted),
                );
            }
        }

        Ok(value)
    }

    fn decrypt_sensitive_fields(
        &self,
        value: &serde_json::Value,
        key: &EncryptionKey,
    ) -> Result<serde_json::Value> {
        let mut value = value.clone();

        if let Some(obj) = value.as_object_mut() {
            if let Some(password) = obj.get("password").and_then(|v| v.as_str()) {
                if password.contains(':') {
                    // Looks like encrypted data
                    let decrypted = decrypt(key, password).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Decryption failed: {}", e))
                    })?;
                    obj.insert("password".to_string(), serde_json::json!(decrypted));
                }
            }

            if let Some(secret) = obj.get("secret_access_key").and_then(|v| v.as_str()) {
                if secret.contains(':') {
                    let decrypted = decrypt(key, secret).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Decryption failed: {}", e))
                    })?;
                    obj.insert(
                        "secret_access_key".to_string(),
                        serde_json::json!(decrypted),
                    );
                }
            }
        }

        Ok(value)
    }

    fn mask_sensitive_fields(&self, row: SystemSettingRow) -> SystemSettingResponse {
        let mut value = row.value.clone();

        // Mask password and secret fields
        if let Some(obj) = value.as_object_mut() {
            if obj.contains_key("password") {
                obj.insert("password".to_string(), serde_json::json!("***"));
            }
            if obj.contains_key("secret_access_key") {
                obj.insert("secret_access_key".to_string(), serde_json::json!("***"));
            }
        }

        SystemSettingResponse {
            category: row.category,
            setting_key: row.setting_key,
            value,
            description: row.description,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SmtpConfig;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use mockall::predicate::*;

    fn test_key() -> EncryptionKey {
        EncryptionKey::new([0x42u8; 32])
    }

    #[tokio::test]
    async fn test_get_email_config_none() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| Ok(None));

        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = service.get_email_config().await.unwrap();
        assert!(matches!(config, EmailProviderConfig::None));
    }

    #[tokio::test]
    async fn test_get_email_config_smtp() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({
                        "type": "smtp",
                        "host": "smtp.example.com",
                        "port": 587,
                        "username": "user",
                        "password": "pass",
                        "use_tls": true,
                        "from_email": "test@example.com",
                        "from_name": "Test"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = service.get_email_config().await.unwrap();
        assert!(matches!(config, EmailProviderConfig::Smtp(_)));

        if let EmailProviderConfig::Smtp(smtp) = config {
            assert_eq!(smtp.host, "smtp.example.com");
            assert_eq!(smtp.port, 587);
        }
    }

    #[tokio::test]
    async fn test_get_email_config_masked() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({
                        "type": "smtp",
                        "host": "smtp.example.com",
                        "port": 587,
                        "password": "secret-password",
                        "from_email": "test@example.com"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = SystemSettingsService::new(Arc::new(mock), None);

        let response = service.get_email_config_masked().await.unwrap();

        // Password should be masked
        let password = response.value.get("password").unwrap().as_str().unwrap();
        assert_eq!(password, "***");

        // Other fields should be preserved
        let host = response.value.get("host").unwrap().as_str().unwrap();
        assert_eq!(host, "smtp.example.com");
    }

    #[tokio::test]
    async fn test_update_email_config() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_upsert().returning(|input| {
            Ok(SystemSettingRow {
                id: 1,
                category: input.category.clone(),
                setting_key: input.setting_key.clone(),
                value: input.value.clone(),
                encrypted: input.encrypted,
                description: input.description.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        });

        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "test@example.com".to_string(),
            from_name: None,
        });

        let result = service.update_email_config(config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_config_valid() {
        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "valid@example.com".to_string(),
            from_name: None,
        });

        assert!(service.validate_email_config(&config).is_ok());
    }

    #[test]
    fn test_validate_email_config_invalid() {
        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "not-an-email".to_string(), // Invalid
            from_name: None,
        });

        assert!(service.validate_email_config(&config).is_err());
    }

    #[test]
    fn test_mask_sensitive_fields() {
        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let row = SystemSettingRow {
            id: 1,
            category: "email".to_string(),
            setting_key: "provider".to_string(),
            value: serde_json::json!({
                "type": "ses",
                "region": "us-east-1",
                "access_key_id": "AKIA...",
                "secret_access_key": "super-secret",
                "from_email": "test@example.com"
            }),
            encrypted: false,
            description: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let masked = service.mask_sensitive_fields(row);

        assert_eq!(
            masked
                .value
                .get("secret_access_key")
                .unwrap()
                .as_str()
                .unwrap(),
            "***"
        );
        assert_eq!(
            masked.value.get("access_key_id").unwrap().as_str().unwrap(),
            "AKIA..."
        );
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mock = MockSystemSettingsRepository::new();
        let key = test_key();
        let service = SystemSettingsService::new(Arc::new(mock), Some(key.clone()));

        let value = serde_json::json!({
            "type": "smtp",
            "password": "secret123"
        });

        let encrypted = service.encrypt_sensitive_fields(&value, &key).unwrap();

        // Password should be encrypted (contains :)
        let enc_password = encrypted.get("password").unwrap().as_str().unwrap();
        assert!(enc_password.contains(':'));
        assert_ne!(enc_password, "secret123");

        // Decrypt should restore original
        let decrypted = service.decrypt_sensitive_fields(&encrypted, &key).unwrap();
        assert_eq!(
            decrypted.get("password").unwrap().as_str().unwrap(),
            "secret123"
        );
    }

    #[tokio::test]
    async fn test_get_email_config_masked_no_config() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| Ok(None));

        let service = SystemSettingsService::new(Arc::new(mock), None);

        let response = service.get_email_config_masked().await.unwrap();
        assert_eq!(response.category, "email");
        assert_eq!(response.setting_key, "provider");
        assert_eq!(
            response.value.get("type").unwrap().as_str().unwrap(),
            "none"
        );
    }

    #[test]
    fn test_validate_none_config() {
        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::None;
        assert!(service.validate_email_config(&config).is_ok());
    }

    #[test]
    fn test_validate_ses_config_valid() {
        use crate::domain::SesConfig;

        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIA...".to_string()),
            secret_access_key: Some("secret".to_string()),
            from_email: "valid@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        });

        assert!(service.validate_email_config(&config).is_ok());
    }

    #[test]
    fn test_validate_oracle_config_valid() {
        use crate::domain::OracleEmailConfig;

        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Oracle(OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user...".to_string(),
            password: "password".to_string(),
            from_email: "valid@example.com".to_string(),
            from_name: Some("Auth9".to_string()),
        });

        assert!(service.validate_email_config(&config).is_ok());
    }

    #[test]
    fn test_validate_oracle_config_invalid_email() {
        use crate::domain::OracleEmailConfig;

        let mock = MockSystemSettingsRepository::new();
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let config = EmailProviderConfig::Oracle(OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user...".to_string(),
            password: "password".to_string(),
            from_email: "not-an-email".to_string(),
            from_name: None,
        });

        assert!(service.validate_email_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_update_email_config_with_encryption() {
        let mut mock = MockSystemSettingsRepository::new();
        let key = test_key();

        mock.expect_upsert().returning(|input| {
            // Verify that password is encrypted
            let password = input.value.get("password").and_then(|v| v.as_str());
            if let Some(p) = password {
                assert!(p.contains(':'), "Password should be encrypted");
            }
            assert!(input.encrypted, "Should be marked as encrypted");

            Ok(SystemSettingRow {
                id: 1,
                category: input.category.clone(),
                setting_key: input.setting_key.clone(),
                value: input.value.clone(),
                encrypted: input.encrypted,
                description: input.description.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        });

        let service = SystemSettingsService::new(Arc::new(mock), Some(key));

        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: Some("user".to_string()),
            password: Some("secret-password".to_string()),
            use_tls: true,
            from_email: "test@example.com".to_string(),
            from_name: None,
        });

        let result = service.update_email_config(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_encrypted_config_without_key_fails() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({
                        "type": "smtp",
                        "host": "smtp.example.com",
                        "password": "encrypted:data"
                    }),
                    encrypted: true, // Marked as encrypted
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        // No encryption key provided
        let service = SystemSettingsService::new(Arc::new(mock), None);

        let result = service.get_email_config().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_secret_access_key() {
        let mock = MockSystemSettingsRepository::new();
        let key = test_key();
        let service = SystemSettingsService::new(Arc::new(mock), Some(key.clone()));

        let value = serde_json::json!({
            "type": "ses",
            "secret_access_key": "my-secret-key"
        });

        let encrypted = service.encrypt_sensitive_fields(&value, &key).unwrap();

        let enc_secret = encrypted
            .get("secret_access_key")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(enc_secret.contains(':'));
        assert_ne!(enc_secret, "my-secret-key");

        // Decrypt should restore original
        let decrypted = service.decrypt_sensitive_fields(&encrypted, &key).unwrap();
        assert_eq!(
            decrypted
                .get("secret_access_key")
                .unwrap()
                .as_str()
                .unwrap(),
            "my-secret-key"
        );
    }

    #[test]
    fn test_decrypt_non_encrypted_password() {
        let mock = MockSystemSettingsRepository::new();
        let key = test_key();
        let service = SystemSettingsService::new(Arc::new(mock), Some(key.clone()));

        // Password without ':' is not encrypted
        let value = serde_json::json!({
            "type": "smtp",
            "password": "plain-password"
        });

        let result = service.decrypt_sensitive_fields(&value, &key).unwrap();
        // Should remain unchanged
        assert_eq!(
            result.get("password").unwrap().as_str().unwrap(),
            "plain-password"
        );
    }
}
