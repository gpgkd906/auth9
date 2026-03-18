//! Social provider repository

use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::models::social_provider::{
    CreateSocialProviderInput, SocialProvider, UpdateSocialProviderInput,
};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SocialProviderRepository: Send + Sync {
    async fn create(&self, input: &CreateSocialProviderInput) -> Result<SocialProvider>;
    async fn find_by_alias(&self, alias: &str) -> Result<Option<SocialProvider>>;
    async fn list_all(&self) -> Result<Vec<SocialProvider>>;
    async fn list_enabled(&self) -> Result<Vec<SocialProvider>>;
    async fn update(
        &self,
        alias: &str,
        input: &UpdateSocialProviderInput,
    ) -> Result<SocialProvider>;
    async fn delete_by_alias(&self, alias: &str) -> Result<()>;
}

pub struct SocialProviderRepositoryImpl {
    pool: MySqlPool,
}

impl SocialProviderRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SocialProviderRepository for SocialProviderRepositoryImpl {
    async fn create(&self, input: &CreateSocialProviderInput) -> Result<SocialProvider> {
        let id = StringUuid::new_v4();
        let config_json = serde_json::to_value(&input.config)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize config: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO social_providers (id, alias, display_name, provider_type,
                                          enabled, trust_email, store_token, link_only,
                                          first_login_policy, config)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(&input.alias)
        .bind(&input.display_name)
        .bind(&input.provider_type)
        .bind(input.enabled)
        .bind(input.trust_email)
        .bind(input.store_token)
        .bind(input.link_only)
        .bind(&input.first_login_policy)
        .bind(&config_json)
        .execute(&self.pool)
        .await?;

        self.find_by_alias(&input.alias)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create social provider")))
    }

    async fn find_by_alias(&self, alias: &str) -> Result<Option<SocialProvider>> {
        let provider = sqlx::query_as::<_, SocialProvider>(
            r#"
            SELECT id, alias, display_name, provider_type, enabled,
                   trust_email, store_token, link_only, first_login_policy,
                   config, created_at, updated_at
            FROM social_providers
            WHERE alias = ?
            "#,
        )
        .bind(alias)
        .fetch_optional(&self.pool)
        .await?;

        Ok(provider)
    }

    async fn list_all(&self) -> Result<Vec<SocialProvider>> {
        let providers = sqlx::query_as::<_, SocialProvider>(
            r#"
            SELECT id, alias, display_name, provider_type, enabled,
                   trust_email, store_token, link_only, first_login_policy,
                   config, created_at, updated_at
            FROM social_providers
            ORDER BY alias
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(providers)
    }

    async fn list_enabled(&self) -> Result<Vec<SocialProvider>> {
        let providers = sqlx::query_as::<_, SocialProvider>(
            r#"
            SELECT id, alias, display_name, provider_type, enabled,
                   trust_email, store_token, link_only, first_login_policy,
                   config, created_at, updated_at
            FROM social_providers
            WHERE enabled = TRUE
            ORDER BY alias
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(providers)
    }

    async fn update(
        &self,
        alias: &str,
        input: &UpdateSocialProviderInput,
    ) -> Result<SocialProvider> {
        let existing = self
            .find_by_alias(alias)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Social provider '{}' not found", alias)))?;

        let display_name = input.display_name.as_ref().or(existing.display_name.as_ref());
        let enabled = input.enabled.unwrap_or(existing.enabled);
        let trust_email = input.trust_email.unwrap_or(existing.trust_email);
        let store_token = input.store_token.unwrap_or(existing.store_token);
        let link_only = input.link_only.unwrap_or(existing.link_only);
        let first_login_policy = input
            .first_login_policy
            .as_ref()
            .unwrap_or(&existing.first_login_policy);
        let config = input.config.as_ref().unwrap_or(&existing.config);
        let config_json = serde_json::to_value(config)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize config: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE social_providers
            SET display_name = ?, enabled = ?, trust_email = ?,
                store_token = ?, link_only = ?, first_login_policy = ?,
                config = ?
            WHERE alias = ?
            "#,
        )
        .bind(display_name)
        .bind(enabled)
        .bind(trust_email)
        .bind(store_token)
        .bind(link_only)
        .bind(first_login_policy)
        .bind(&config_json)
        .bind(alias)
        .execute(&self.pool)
        .await?;

        self.find_by_alias(alias)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to update social provider")))
    }

    async fn delete_by_alias(&self, alias: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM social_providers WHERE alias = ?")
            .bind(alias)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Social provider '{}' not found",
                alias
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_mock_list_all() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_list_all().returning(|| {
            Ok(vec![
                SocialProvider {
                    alias: "google".to_string(),
                    provider_type: "google".to_string(),
                    enabled: true,
                    ..Default::default()
                },
                SocialProvider {
                    alias: "github".to_string(),
                    provider_type: "github".to_string(),
                    enabled: true,
                    ..Default::default()
                },
            ])
        });

        let providers = mock.list_all().await.unwrap();
        assert_eq!(providers.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_find_by_alias() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_find_by_alias()
            .with(eq("google"))
            .returning(|alias| {
                Ok(Some(SocialProvider {
                    alias: alias.to_string(),
                    provider_type: "google".to_string(),
                    ..Default::default()
                }))
            });

        let result = mock.find_by_alias("google").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().alias, "google");
    }

    #[tokio::test]
    async fn test_mock_find_by_alias_not_found() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_find_by_alias()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let result = mock.find_by_alias("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_create().returning(|input| {
            Ok(SocialProvider {
                alias: input.alias.clone(),
                provider_type: input.provider_type.clone(),
                config: input.config.clone(),
                ..Default::default()
            })
        });

        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "test-id".to_string());

        let input = CreateSocialProviderInput {
            alias: "google".to_string(),
            display_name: Some("Google".to_string()),
            provider_type: "google".to_string(),
            enabled: true,
            trust_email: true,
            store_token: false,
            link_only: false,
            first_login_policy: "auto_merge".to_string(),
            config,
        };

        let result = mock.create(&input).await.unwrap();
        assert_eq!(result.alias, "google");
        assert_eq!(result.config.get("clientId"), Some(&"test-id".to_string()));
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_delete_by_alias()
            .with(eq("google"))
            .returning(|_| Ok(()));

        let result = mock.delete_by_alias("google").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_list_enabled() {
        let mut mock = MockSocialProviderRepository::new();

        mock.expect_list_enabled().returning(|| {
            Ok(vec![SocialProvider {
                alias: "google".to_string(),
                provider_type: "google".to_string(),
                enabled: true,
                ..Default::default()
            }])
        });

        let providers = mock.list_enabled().await.unwrap();
        assert_eq!(providers.len(), 1);
        assert!(providers[0].enabled);
    }
}
