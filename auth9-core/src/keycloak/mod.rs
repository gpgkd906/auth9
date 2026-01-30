//! Keycloak Admin API client

use crate::config::KeycloakConfig;
use crate::error::{AppError, Result};
use anyhow::Context;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Keycloak Admin API client
#[derive(Clone)]
pub struct KeycloakClient {
    config: KeycloakConfig,
    http_client: Client,
    token: Arc<RwLock<Option<AdminToken>>>,
}

#[derive(Debug, Clone)]
struct AdminToken {
    access_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Keycloak user representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUser {
    pub id: Option<String>,
    pub username: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUserUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_actions: Option<Vec<String>>,
}

/// Input for creating a user in Keycloak
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeycloakUserInput {
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<KeycloakCredential>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakCredential {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub value: String,
    pub temporary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUserCredential {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
}

/// Keycloak client (OIDC) representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakOidcClient {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_url: Option<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub web_origins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
    pub public_client: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

impl KeycloakClient {
    pub fn new(config: KeycloakConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get admin access token (with caching)
    async fn get_admin_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let token = self.token.read().await;
            if let Some(ref t) = *token {
                if t.expires_at > chrono::Utc::now() + chrono::Duration::seconds(30) {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Fetch new token using password grant (admin-cli is a public client)
        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        // Get admin credentials from environment
        let admin_username =
            std::env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
        let admin_password =
            std::env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        let params = [
            ("grant_type", "password"),
            ("client_id", &self.config.admin_client_id),
            ("username", &admin_username),
            ("password", &admin_password),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get admin token: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get admin token: {} - {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: i64,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))?;

        let admin_token = AdminToken {
            access_token: token_response.access_token.clone(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in),
        };

        // Cache the token
        {
            let mut token = self.token.write().await;
            *token = Some(admin_token);
        }

        Ok(token_response.access_token)
    }

    /// Create a user in Keycloak
    pub async fn create_user(&self, input: &CreateKeycloakUserInput) -> Result<String> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users",
            self.config.url, self.config.realm
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(input)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to create user: {}", e)))?;

        if response.status() == StatusCode::CONFLICT {
            return Err(AppError::Conflict(
                "User already exists in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to create user: {} - {}",
                status, body
            )));
        }

        // Get user ID from Location header
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Keycloak("Missing location header".to_string()))?;

        let user_id = location
            .split('/')
            .next_back()
            .ok_or_else(|| AppError::Keycloak("Invalid location header".to_string()))?;

        Ok(user_id.to_string())
    }

    /// Get a user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<KeycloakUser> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get user: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get user: {} - {}",
                status, body
            )));
        }

        let user: KeycloakUser = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse user: {}", e)))?;

        Ok(user)
    }

    pub async fn list_user_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<KeycloakUserCredential>> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/credentials",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to list user credentials: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to list user credentials: {} - {}",
                status, body
            )));
        }

        let credentials: Vec<KeycloakUserCredential> = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse credentials: {}", e)))?;

        Ok(credentials)
    }

    pub async fn delete_user_credential(&self, user_id: &str, credential_id: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/credentials/{}",
            self.config.url, self.config.realm, user_id, credential_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to delete credential: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Credential not found in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to delete credential: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    pub async fn remove_totp_credentials(&self, user_id: &str) -> Result<()> {
        let credentials = self.list_user_credentials(user_id).await?;
        for credential in credentials {
            let kind = credential.credential_type.to_lowercase();
            if kind.contains("otp") || kind.contains("totp") {
                self.delete_user_credential(user_id, &credential.id).await?;
            }
        }
        Ok(())
    }

    pub async fn update_user(&self, user_id: &str, input: &KeycloakUserUpdate) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(&token)
            .json(input)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to update user: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to update user: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to delete user: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to delete user: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Search users by email
    pub async fn search_users_by_email(&self, email: &str) -> Result<Vec<KeycloakUser>> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users?email={}&exact=true",
            self.config.url, self.config.realm, email
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to search users: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to search users: {} - {}",
                status, body
            )));
        }

        let users: Vec<KeycloakUser> = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse users: {}", e)))?;

        Ok(users)
    }

    /// Create an OIDC client
    pub async fn create_oidc_client(&self, client: &KeycloakOidcClient) -> Result<String> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients",
            self.config.url, self.config.realm
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(client)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to create client: {}", e)))?;

        if response.status() == StatusCode::CONFLICT {
            return Err(AppError::Conflict(
                "Client already exists in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to create client: {} - {}",
                status, body
            )));
        }

        // Get client ID from Location header
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Keycloak("Missing location header".to_string()))?;

        let client_uuid = location
            .split('/')
            .next_back()
            .ok_or_else(|| AppError::Keycloak("Invalid location header".to_string()))?;

        Ok(client_uuid.to_string())
    }

    /// Get client secret
    pub async fn get_client_secret(&self, client_uuid: &str) -> Result<String> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients/{}/client-secret",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get client secret: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get client secret: {} - {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct SecretResponse {
            value: String,
        }

        let secret: SecretResponse = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse secret: {}", e)))?;

        Ok(secret.value)
    }

    pub async fn regenerate_client_secret(&self, client_uuid: &str) -> Result<String> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients/{}/client-secret",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to regenerate client secret: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to regenerate client secret: {} - {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct SecretResponse {
            value: String,
        }

        let secret: SecretResponse = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse secret: {}", e)))?;

        Ok(secret.value)
    }

    /// Get client UUID by client_id
    pub async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients?clientId={}",
            self.config.url, self.config.realm, client_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to query client: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to query client: {} - {}",
                status, body
            )));
        }

        let clients: Vec<KeycloakOidcClient> = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse client list: {}", e)))?;

        let client = clients
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound("Client not found in Keycloak".to_string()))?;

        client
            .id
            .ok_or_else(|| AppError::Keycloak("Client id missing in Keycloak response".to_string()))
    }

    pub async fn update_oidc_client(
        &self,
        client_uuid: &str,
        client: &KeycloakOidcClient,
    ) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients/{}",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(&token)
            .json(client)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to update client: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Client not found in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to update client: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    pub async fn delete_oidc_client(&self, client_uuid: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/clients/{}",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to delete client: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Client not found in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to delete client: {} - {}",
                status, body
            )));
        }

        Ok(())
    }
}

// ============================================================================
// Keycloak Seeder - For initialization and default data seeding
// ============================================================================

/// Default admin user configuration
const DEFAULT_ADMIN_EMAIL: &str = "admin@auth9.local";
const DEFAULT_ADMIN_PASSWORD: &str = "Admin123!";
const DEFAULT_ADMIN_FIRST_NAME: &str = "Admin";
const DEFAULT_ADMIN_LAST_NAME: &str = "User";

/// Default portal client configuration
const DEFAULT_PORTAL_CLIENT_ID: &str = "auth9-portal";
const DEFAULT_PORTAL_CLIENT_NAME: &str = "Auth9 Admin Portal";

/// Keycloak realm representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakRealm {
    pub realm: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_password_allowed: Option<bool>,
    /// SSL requirement: "none", "external", or "all"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_required: Option<String>,
}

/// Keycloak Seeder for initialization
pub struct KeycloakSeeder {
    config: KeycloakConfig,
    http_client: Client,
}

impl KeycloakSeeder {
    pub fn new(config: &KeycloakConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: config.clone(),
            http_client,
        }
    }

    /// Get admin token using master realm password grant
    /// Uses KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD environment variables
    async fn get_master_admin_token(&self) -> anyhow::Result<String> {
        let admin_username = env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
        let admin_password =
            env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        let params = [
            ("grant_type", "password"),
            ("client_id", "admin-cli"),
            ("username", &admin_username),
            ("password", &admin_password),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .context("Failed to connect to Keycloak")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to get admin token from Keycloak: {} - {}",
                status,
                body
            );
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response.access_token)
    }

    /// Check if a realm exists
    async fn realm_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check realm")?;

        Ok(response.status().is_success())
    }

    /// Create a new realm
    async fn create_realm(&self, token: &str) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms", self.config.url);

        let realm = KeycloakRealm {
            realm: self.config.realm.clone(),
            enabled: true,
            display_name: Some("Auth9".to_string()),
            registration_allowed: Some(true),
            reset_password_allowed: Some(true),
            ssl_required: Some(self.config.ssl_required.clone()),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&realm)
            .send()
            .await
            .context("Failed to create realm")?;

        if response.status() == StatusCode::CONFLICT {
            info!("Realm '{}' already exists", self.config.realm);
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create realm: {} - {}", status, body);
        }

        info!("Created realm '{}'", self.config.realm);
        Ok(())
    }

    /// Update realm SSL settings based on configuration
    async fn update_realm_ssl(&self, token: &str) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        let update = serde_json::json!({
            "sslRequired": self.config.ssl_required
        });

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(token)
            .json(&update)
            .send()
            .await
            .context("Failed to update realm SSL settings")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update realm SSL: {} - {}", status, body);
        }

        info!(
            "Updated realm '{}' SSL requirement to '{}'",
            self.config.realm, self.config.ssl_required
        );
        Ok(())
    }

    /// Ensure realm exists (create if not) and configure SSL
    pub async fn ensure_realm_exists(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.realm_exists(&token).await? {
            info!("Realm '{}' already exists", self.config.realm);
            // Update SSL settings for existing realm
            self.update_realm_ssl(&token).await?;
            return Ok(());
        }

        self.create_realm(&token).await?;
        Ok(())
    }

    /// Check if admin user exists by email
    async fn admin_user_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!(
            "{}/admin/realms/{}/users?email={}&exact=true",
            self.config.url, self.config.realm, DEFAULT_ADMIN_EMAIL
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check admin user")?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let users: Vec<serde_json::Value> = response.json().await.unwrap_or_default();

        Ok(!users.is_empty())
    }

    /// Create default admin user
    async fn create_admin_user(&self, token: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/admin/realms/{}/users",
            self.config.url, self.config.realm
        );

        let user = CreateKeycloakUserInput {
            username: DEFAULT_ADMIN_EMAIL.to_string(),
            email: DEFAULT_ADMIN_EMAIL.to_string(),
            first_name: Some(DEFAULT_ADMIN_FIRST_NAME.to_string()),
            last_name: Some(DEFAULT_ADMIN_LAST_NAME.to_string()),
            enabled: true,
            email_verified: true,
            credentials: Some(vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: DEFAULT_ADMIN_PASSWORD.to_string(),
                temporary: false,
            }]),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&user)
            .send()
            .await
            .context("Failed to create admin user")?;

        if response.status() == StatusCode::CONFLICT {
            info!("Admin user '{}' already exists", DEFAULT_ADMIN_EMAIL);
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create admin user: {} - {}", status, body);
        }

        info!(
            "Created admin user '{}' with password '{}'",
            DEFAULT_ADMIN_EMAIL, DEFAULT_ADMIN_PASSWORD
        );
        Ok(())
    }

    /// Seed default admin user (idempotent)
    pub async fn seed_admin_user(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.admin_user_exists(&token).await? {
            info!(
                "Admin user '{}' already exists, skipping",
                DEFAULT_ADMIN_EMAIL
            );
            return Ok(());
        }

        self.create_admin_user(&token).await?;

        warn!(
            "Default admin credentials - Email: {}, Password: {}",
            DEFAULT_ADMIN_EMAIL, DEFAULT_ADMIN_PASSWORD
        );
        warn!("Please change the default admin password after first login!");

        Ok(())
    }

    /// Check if portal client exists
    async fn portal_client_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!(
            "{}/admin/realms/{}/clients?clientId={}",
            self.config.url, self.config.realm, DEFAULT_PORTAL_CLIENT_ID
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check portal client")?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let clients: Vec<serde_json::Value> = response.json().await.unwrap_or_default();

        Ok(!clients.is_empty())
    }

    /// Create portal client
    async fn create_portal_client(&self, token: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/admin/realms/{}/clients",
            self.config.url, self.config.realm
        );

        let client = KeycloakOidcClient {
            id: None,
            client_id: DEFAULT_PORTAL_CLIENT_ID.to_string(),
            name: Some(DEFAULT_PORTAL_CLIENT_NAME.to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            root_url: Some("http://localhost:3000".to_string()),
            admin_url: Some("http://localhost:3000".to_string()),
            redirect_uris: vec![
                // Auth9-core callback URL (Keycloak redirects here after login)
                "http://localhost:8080/api/v1/auth/callback".to_string(),
                "http://127.0.0.1:8080/api/v1/auth/callback".to_string(),
                // Portal URLs for direct OIDC if needed
                "http://localhost:3000/*".to_string(),
                "http://127.0.0.1:3000/*".to_string(),
            ],
            web_origins: vec![
                "http://localhost:8080".to_string(),
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
                "http://127.0.0.1:3000".to_string(),
            ],
            attributes: None,
            public_client: false,  // Confidential client - Keycloak will generate a secret
            secret: None,
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&client)
            .send()
            .await
            .context("Failed to create portal client")?;

        if response.status() == StatusCode::CONFLICT {
            info!(
                "Portal client '{}' already exists",
                DEFAULT_PORTAL_CLIENT_ID
            );
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create portal client: {} - {}", status, body);
        }

        info!("Created portal client '{}'", DEFAULT_PORTAL_CLIENT_ID);
        Ok(())
    }

    /// Seed portal client (idempotent)
    pub async fn seed_portal_client(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.portal_client_exists(&token).await? {
            info!(
                "Portal client '{}' already exists, skipping",
                DEFAULT_PORTAL_CLIENT_ID
            );
            return Ok(());
        }

        self.create_portal_client(&token).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keycloak_user_serialization() {
        let user = KeycloakUser {
            id: Some("123".to_string()),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            enabled: true,
            email_verified: true,
            attributes: std::collections::HashMap::new(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_create_user_input_serialization() {
        let input = CreateKeycloakUserInput {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
            enabled: true,
            email_verified: false,
            credentials: Some(vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: "secret123".to_string(),
                temporary: true,
            }]),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("newuser"));
        assert!(json.contains("password"));
    }

    #[test]
    fn test_keycloak_user_deserialization() {
        let json = r#"{
            "id": "abc-123",
            "username": "john",
            "email": "john@example.com",
            "firstName": "John",
            "lastName": "Doe",
            "enabled": true,
            "emailVerified": true,
            "attributes": {}
        }"#;

        let user: KeycloakUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, Some("abc-123".to_string()));
        assert_eq!(user.username, "john");
        assert_eq!(user.email, Some("john@example.com".to_string()));
        assert_eq!(user.first_name, Some("John".to_string()));
        assert_eq!(user.last_name, Some("Doe".to_string()));
        assert!(user.enabled);
        assert!(user.email_verified);
    }

    #[test]
    fn test_keycloak_user_deserialization_minimal() {
        let json = r#"{
            "username": "minimal",
            "enabled": false
        }"#;

        let user: KeycloakUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.username, "minimal");
        assert!(user.id.is_none());
        assert!(user.email.is_none());
        assert!(!user.enabled);
        assert!(!user.email_verified); // default
    }

    #[test]
    fn test_keycloak_user_update_serialization_partial() {
        let update = KeycloakUserUpdate {
            username: None,
            email: Some("newemail@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("newemail@example.com"));
        assert!(!json.contains("username"));
        assert!(!json.contains("firstName"));
    }

    #[test]
    fn test_keycloak_user_update_with_required_actions() {
        let update = KeycloakUserUpdate {
            username: None,
            email: None,
            first_name: None,
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: Some(vec!["CONFIGURE_TOTP".to_string(), "UPDATE_PASSWORD".to_string()]),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("requiredActions"));
        assert!(json.contains("CONFIGURE_TOTP"));
        assert!(json.contains("UPDATE_PASSWORD"));
    }

    #[test]
    fn test_keycloak_credential_serialization() {
        let cred = KeycloakCredential {
            credential_type: "password".to_string(),
            value: "secret123".to_string(),
            temporary: false,
        };

        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains("\"type\":\"password\""));
        assert!(json.contains("\"value\":\"secret123\""));
        assert!(json.contains("\"temporary\":false"));
    }

    #[test]
    fn test_keycloak_user_credential_deserialization() {
        let json = r#"{
            "id": "cred-123",
            "type": "otp"
        }"#;

        let cred: KeycloakUserCredential = serde_json::from_str(json).unwrap();
        assert_eq!(cred.id, "cred-123");
        assert_eq!(cred.credential_type, "otp");
    }

    #[test]
    fn test_keycloak_oidc_client_serialization() {
        let client = KeycloakOidcClient {
            id: Some("client-uuid".to_string()),
            client_id: "my-app".to_string(),
            name: Some("My Application".to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some("https://myapp.example.com".to_string()),
            root_url: None,
            admin_url: None,
            redirect_uris: vec!["https://myapp.example.com/callback".to_string()],
            web_origins: vec!["https://myapp.example.com".to_string()],
            attributes: None,
            public_client: false,
            secret: Some("client-secret".to_string()),
        };

        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("\"clientId\":\"my-app\""));
        assert!(json.contains("\"protocol\":\"openid-connect\""));
        assert!(json.contains("\"redirectUris\""));
    }

    #[test]
    fn test_keycloak_oidc_client_deserialization() {
        let json = r#"{
            "id": "uuid-123",
            "clientId": "test-client",
            "name": "Test Client",
            "enabled": true,
            "protocol": "openid-connect",
            "redirectUris": ["http://localhost:3000/callback"],
            "webOrigins": ["http://localhost:3000"],
            "publicClient": true
        }"#;

        let client: KeycloakOidcClient = serde_json::from_str(json).unwrap();
        assert_eq!(client.id, Some("uuid-123".to_string()));
        assert_eq!(client.client_id, "test-client");
        assert_eq!(client.protocol, "openid-connect");
        assert!(client.public_client);
        assert_eq!(client.redirect_uris.len(), 1);
    }

    #[test]
    fn test_keycloak_oidc_client_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("post.logout.redirect.uris".to_string(), "https://app.com/logout".to_string());

        let client = KeycloakOidcClient {
            id: None,
            client_id: "attr-client".to_string(),
            name: None,
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: None,
            root_url: None,
            admin_url: None,
            redirect_uris: vec![],
            web_origins: vec![],
            attributes: Some(attrs),
            public_client: false,
            secret: None,
        };

        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("post.logout.redirect.uris"));
        assert!(json.contains("https://app.com/logout"));
    }

    #[test]
    fn test_keycloak_user_with_attributes() {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("department".to_string(), vec!["Engineering".to_string()]);
        attrs.insert("phone".to_string(), vec!["+1234567890".to_string()]);

        let user = KeycloakUser {
            id: Some("user-123".to_string()),
            username: "attruser".to_string(),
            email: Some("attr@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            attributes: attrs,
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("department"));
        assert!(json.contains("Engineering"));
    }

    #[test]
    fn test_create_user_input_without_credentials() {
        let input = CreateKeycloakUserInput {
            username: "nopassword".to_string(),
            email: "nopass@example.com".to_string(),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(!json.contains("credentials"));
    }

    #[test]
    fn test_keycloak_user_clone() {
        let user = KeycloakUser {
            id: Some("123".to_string()),
            username: "cloneuser".to_string(),
            email: Some("clone@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: true,
            attributes: std::collections::HashMap::new(),
        };

        let cloned = user.clone();
        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
    }

    #[test]
    fn test_keycloak_oidc_client_clone() {
        let client = KeycloakOidcClient {
            id: Some("id".to_string()),
            client_id: "client".to_string(),
            name: None,
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: None,
            root_url: None,
            admin_url: None,
            redirect_uris: vec!["http://localhost".to_string()],
            web_origins: vec![],
            attributes: None,
            public_client: false,
            secret: None,
        };

        let cloned = client.clone();
        assert_eq!(client.client_id, cloned.client_id);
        assert_eq!(client.redirect_uris, cloned.redirect_uris);
    }

    #[test]
    fn test_keycloak_user_debug() {
        let user = KeycloakUser {
            id: Some("debug-123".to_string()),
            username: "debuguser".to_string(),
            email: None,
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            attributes: std::collections::HashMap::new(),
        };

        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("KeycloakUser"));
        assert!(debug_str.contains("debuguser"));
    }

    #[test]
    fn test_keycloak_user_update_debug() {
        let update = KeycloakUserUpdate {
            username: Some("newname".to_string()),
            email: None,
            first_name: None,
            last_name: None,
            enabled: Some(true),
            email_verified: None,
            required_actions: None,
        };

        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("KeycloakUserUpdate"));
        assert!(debug_str.contains("newname"));
    }
}
