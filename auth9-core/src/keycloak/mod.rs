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

        // Fetch new token using password grant with auth9-admin (Confidential Client)
        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        // Get admin credentials from environment
        let admin_username =
            std::env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
        let admin_password =
            std::env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        let mut params = vec![
            ("grant_type", "password"),
            ("client_id", &self.config.admin_client_id), // auth9-admin (Confidential Client)
            ("username", &admin_username),
            ("password", &admin_password),
        ];

        // Add client_secret for Confidential Client (required for auth9-admin)
        if !self.config.admin_client_secret.is_empty() {
            params.push(("client_secret", &self.config.admin_client_secret));
        }

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
const DEFAULT_ADMIN_USERNAME: &str = "admin";
const DEFAULT_ADMIN_EMAIL: &str = "admin@auth9.local";
const DEFAULT_ADMIN_FIRST_NAME: &str = "Admin";
const DEFAULT_ADMIN_LAST_NAME: &str = "User";

/// Get admin password from env var or generate a secure random one
fn get_admin_password() -> String {
    // Allow override via environment variable (useful for local development)
    if let Ok(password) = env::var("AUTH9_ADMIN_PASSWORD") {
        if !password.is_empty() {
            return password;
        }
    }
    generate_secure_password()
}

/// Generate a cryptographically secure random password
fn generate_secure_password() -> String {
    use rand::Rng;
    const CHARSET_LOWER: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
    const CHARSET_UPPER: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    const CHARSET_DIGIT: &[u8] = b"23456789";
    const CHARSET_SPECIAL: &[u8] = b"!@#$%^&*";

    let mut rng = rand::thread_rng();

    // Ensure at least one of each type
    let mut password: Vec<char> = Vec::with_capacity(16);
    password.push(CHARSET_LOWER[rng.gen_range(0..CHARSET_LOWER.len())] as char);
    password.push(CHARSET_UPPER[rng.gen_range(0..CHARSET_UPPER.len())] as char);
    password.push(CHARSET_DIGIT[rng.gen_range(0..CHARSET_DIGIT.len())] as char);
    password.push(CHARSET_SPECIAL[rng.gen_range(0..CHARSET_SPECIAL.len())] as char);

    // Fill remaining with mixed charset
    let all_chars: Vec<u8> =
        [CHARSET_LOWER, CHARSET_UPPER, CHARSET_DIGIT, CHARSET_SPECIAL].concat();
    for _ in 0..12 {
        password.push(all_chars[rng.gen_range(0..all_chars.len())] as char);
    }

    // Shuffle the password
    use rand::seq::SliceRandom;
    password.shuffle(&mut rng);

    password.into_iter().collect()
}

/// Default portal client configuration
const DEFAULT_PORTAL_CLIENT_ID: &str = "auth9-portal";
const DEFAULT_PORTAL_CLIENT_NAME: &str = "Auth9 Admin Portal";

/// Default admin client configuration
const DEFAULT_ADMIN_CLIENT_ID: &str = "auth9-admin";
const DEFAULT_ADMIN_CLIENT_NAME: &str = "Auth9 Admin Client";

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

/// Build comprehensive redirect URIs list combining localhost and production URLs
fn build_redirect_uris(core_public_url: Option<&str>, portal_url: Option<&str>) -> Vec<String> {
    let mut uris = Vec::new();

    // Always include localhost URLs for local development
    uris.extend([
        "http://localhost:8080/api/v1/auth/callback".to_string(),
        "http://127.0.0.1:8080/api/v1/auth/callback".to_string(),
        "http://localhost:3000/*".to_string(),
        "http://127.0.0.1:3000/*".to_string(),
    ]);

    // Add production URLs if configured
    if let Some(core_url) = core_public_url {
        uris.push(format!("{}/api/v1/auth/callback", core_url));
    }
    if let Some(portal_url_str) = portal_url {
        uris.push(format!("{}/*", portal_url_str));
    }

    uris
}

/// Build comprehensive web origins list
fn build_web_origins(core_public_url: Option<&str>, portal_url: Option<&str>) -> Vec<String> {
    let mut origins = Vec::new();

    // Always include localhost
    origins.extend([
        "http://localhost:8080".to_string(),
        "http://127.0.0.1:8080".to_string(),
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
    ]);

    // Add production URLs
    if let Some(core_url) = core_public_url {
        origins.push(core_url.to_string());
    }
    if let Some(portal_url_str) = portal_url {
        origins.push(portal_url_str.to_string());
    }

    origins
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
    /// Uses admin-cli client (default public client in master realm)
    async fn get_master_admin_token(&self) -> anyhow::Result<String> {
        let admin_username = env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
        let admin_password =
            env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        // Use admin-cli (Keycloak's default public client in master realm)
        // This is necessary during initialization when auth9-admin client doesn't exist yet
        let admin_cli = "admin-cli";

        let params = vec![
            ("grant_type", "password"),
            ("client_id", admin_cli),
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
            "{}/admin/realms/{}/users?username={}&exact=true",
            self.config.url, self.config.realm, DEFAULT_ADMIN_USERNAME
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

    /// Create default admin user with a randomly generated password
    /// Returns the generated password if user was created
    async fn create_admin_user(&self, token: &str) -> anyhow::Result<Option<String>> {
        let url = format!(
            "{}/admin/realms/{}/users",
            self.config.url, self.config.realm
        );

        let password = get_admin_password();

        let user = CreateKeycloakUserInput {
            username: DEFAULT_ADMIN_USERNAME.to_string(),
            email: DEFAULT_ADMIN_EMAIL.to_string(),
            first_name: Some(DEFAULT_ADMIN_FIRST_NAME.to_string()),
            last_name: Some(DEFAULT_ADMIN_LAST_NAME.to_string()),
            enabled: true,
            email_verified: true,
            credentials: Some(vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: password.clone(),
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
            info!("Admin user '{}' already exists", DEFAULT_ADMIN_USERNAME);
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create admin user: {} - {}", status, body);
        }

        info!("Created admin user '{}'", DEFAULT_ADMIN_USERNAME);
        Ok(Some(password))
    }

    /// Seed default admin user (idempotent)
    pub async fn seed_admin_user(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.admin_user_exists(&token).await? {
            info!(
                "Admin user '{}' already exists, skipping",
                DEFAULT_ADMIN_USERNAME
            );
            return Ok(());
        }

        if let Some(password) = self.create_admin_user(&token).await? {
            // Print credentials in machine-parseable format for deploy script
            println!("========================================");
            println!("AUTH9_ADMIN_USERNAME={}", DEFAULT_ADMIN_USERNAME);
            println!("AUTH9_ADMIN_PASSWORD={}", password);
            println!("========================================");

            warn!(
                "Created admin user '{}' with generated password",
                DEFAULT_ADMIN_USERNAME
            );
            warn!("Please save the admin credentials shown above!");
        }

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
            base_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            root_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            admin_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            redirect_uris: build_redirect_uris(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            web_origins: build_web_origins(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            attributes: None,
            public_client: false, // Confidential client - Keycloak will generate a secret
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

    /// Update existing portal client configuration
    async fn update_portal_client(&self, token: &str) -> anyhow::Result<()> {
        // 1. Query existing client to get UUID
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
            .context("Failed to query portal client")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to query portal client for update");
        }

        let clients: Vec<KeycloakOidcClient> = response.json().await?;
        let existing_client = clients
            .into_iter()
            .next()
            .context("Portal client not found for update")?;

        let client_uuid = existing_client.id.context("Portal client UUID missing")?;

        // 2. Build updated client configuration
        let updated_client = KeycloakOidcClient {
            id: Some(client_uuid.clone()),
            client_id: DEFAULT_PORTAL_CLIENT_ID.to_string(),
            name: Some(DEFAULT_PORTAL_CLIENT_NAME.to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            root_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            admin_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            redirect_uris: build_redirect_uris(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            web_origins: build_web_origins(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            attributes: existing_client.attributes,
            public_client: false,
            secret: existing_client.secret,
        };

        // 3. Update client
        let update_url = format!(
            "{}/admin/realms/{}/clients/{}",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .put(&update_url)
            .bearer_auth(token)
            .json(&updated_client)
            .send()
            .await
            .context("Failed to update portal client")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update portal client: {} - {}", status, body);
        }

        info!(
            "Updated portal client '{}' configuration",
            DEFAULT_PORTAL_CLIENT_ID
        );
        Ok(())
    }

    /// Seed portal client (idempotent - creates or updates)
    pub async fn seed_portal_client(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        // Log configuration being used
        match (&self.config.core_public_url, &self.config.portal_url) {
            (Some(core), Some(portal)) => {
                info!(
                    "Configuring portal client with production URLs: core={}, portal={}",
                    core, portal
                );
            }
            _ => {
                warn!(
                    "Production URLs not configured (AUTH9_CORE_PUBLIC_URL, AUTH9_PORTAL_URL). \
                    Using localhost URLs only. This is OK for local development but not for production."
                );
            }
        }

        if self.portal_client_exists(&token).await? {
            info!(
                "Portal client '{}' already exists, updating configuration...",
                DEFAULT_PORTAL_CLIENT_ID
            );

            // Update existing client to ensure production URLs are configured
            self.update_portal_client(&token).await?;

            return Ok(());
        }

        // Create new client
        self.create_portal_client(&token).await?;
        Ok(())
    }

    /// Check if admin client exists in a specific realm
    async fn admin_client_exists_in_realm(&self, token: &str, realm: &str) -> anyhow::Result<bool> {
        let client_id = DEFAULT_ADMIN_CLIENT_ID;
        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .query(&[("clientId", client_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let clients: Vec<serde_json::Value> = response.json().await?;
        Ok(!clients.is_empty())
    }

    /// Create auth9-admin client in a specific realm
    async fn create_admin_client_in_realm(&self, token: &str, realm: &str) -> anyhow::Result<()> {
        info!(
            "Creating auth9-admin client in realm '{}' (Confidential Client)",
            realm
        );

        // Read secret from environment
        let client_secret =
            env::var("KEYCLOAK_ADMIN_CLIENT_SECRET").unwrap_or_else(|_| String::new());

        // Build client configuration
        let mut client = serde_json::json!({
            "clientId": DEFAULT_ADMIN_CLIENT_ID,
            "name": DEFAULT_ADMIN_CLIENT_NAME,
            "enabled": true,
            "protocol": "openid-connect",
            "publicClient": false,  // Confidential client
            "serviceAccountsEnabled": false,  // Use password grant, not client credentials
            "standardFlowEnabled": false,
            "directAccessGrantsEnabled": true,  // Enable password grant
            "redirectUris": [],
            "webOrigins": [],
        });

        // If secret is provided, set it explicitly
        if !client_secret.is_empty() {
            client["secret"] = serde_json::json!(client_secret);
        }

        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&client)
            .send()
            .await?;

        // Handle conflict (client already exists) as success for idempotency
        if response.status() == StatusCode::CONFLICT {
            info!("auth9-admin client already exists in realm '{}'", realm);
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!(
                "Failed to create auth9-admin client in realm '{}': {} - {}",
                realm,
                status,
                body
            );
        }

        // If no secret was provided, retrieve the auto-generated one
        if client_secret.is_empty() {
            info!("Retrieving auto-generated client secret...");
            match self.get_admin_client_secret_in_realm(token, realm).await {
                Ok(secret) => {
                    info!("✅ auth9-admin client created in realm '{}'!", realm);
                    info!("📋 Copy this secret to your secrets configuration:");
                    info!("   KEYCLOAK_ADMIN_CLIENT_SECRET={}", secret);
                }
                Err(e) => {
                    warn!("Client created but failed to retrieve secret: {}", e);
                    info!("You can retrieve it manually from Keycloak Admin Console:");
                    info!("  Clients → auth9-admin → Credentials tab");
                }
            }
        } else {
            info!(
                "✅ auth9-admin client created in realm '{}' with preset secret",
                realm
            );
        }

        Ok(())
    }

    /// Get the client secret for auth9-admin in a specific realm
    async fn get_admin_client_secret_in_realm(
        &self,
        token: &str,
        realm: &str,
    ) -> anyhow::Result<String> {
        // Get client UUID
        let client_uuid = self
            .get_client_uuid_by_client_id_in_realm(token, realm, DEFAULT_ADMIN_CLIENT_ID)
            .await?;

        // Get client secret
        let secret = self
            .get_client_secret_in_realm(token, realm, &client_uuid)
            .await?;
        Ok(secret)
    }

    /// Get client UUID by client ID in a specific realm
    async fn get_client_uuid_by_client_id_in_realm(
        &self,
        token: &str,
        realm: &str,
        client_id: &str,
    ) -> anyhow::Result<String> {
        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .query(&[("clientId", client_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Failed to get client by clientId: {} - {}", status, body);
        }

        let clients: Vec<serde_json::Value> = response.json().await?;
        if clients.is_empty() {
            anyhow::bail!("Client '{}' not found in realm '{}'", client_id, realm);
        }

        let client_uuid = clients[0]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Client UUID not found in response"))?
            .to_string();

        Ok(client_uuid)
    }

    /// Get client secret in a specific realm
    async fn get_client_secret_in_realm(
        &self,
        token: &str,
        realm: &str,
        client_uuid: &str,
    ) -> anyhow::Result<String> {
        let url = format!(
            "{}/admin/realms/{}/clients/{}/client-secret",
            self.config.url, realm, client_uuid
        );

        let response = self.http_client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Failed to get client secret: {} - {}", status, body);
        }

        let secret_response: serde_json::Value = response.json().await?;
        let secret = secret_response["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Client secret value not found in response"))?
            .to_string();

        Ok(secret)
    }

    /// Seed auth9-admin client in master realm (idempotent)
    pub async fn seed_master_admin_client(&self) -> anyhow::Result<()> {
        info!("Seeding auth9-admin client in master realm...");

        // Get admin token using admin-cli
        let token = self.get_master_admin_token().await?;

        // Check if client already exists in master realm
        if self.admin_client_exists_in_realm(&token, "master").await? {
            info!("auth9-admin client already exists in master realm, skipping");
            return Ok(());
        }

        // Create client in master realm
        self.create_admin_client_in_realm(&token, "master").await?;
        Ok(())
    }

    /// Seed auth9-admin client in configured realm (idempotent)
    pub async fn seed_admin_client(&self) -> anyhow::Result<()> {
        info!(
            "Seeding auth9-admin client in realm '{}'...",
            self.config.realm
        );

        // Get admin token using current configuration (admin-cli or auth9-admin)
        let token = self.get_master_admin_token().await?;

        // Check if client already exists
        if self
            .admin_client_exists_in_realm(&token, &self.config.realm)
            .await?
        {
            info!(
                "auth9-admin client already exists in realm '{}', skipping",
                self.config.realm
            );
            return Ok(());
        }

        // Create client in configured realm
        self.create_admin_client_in_realm(&token, &self.config.realm)
            .await?;
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
            required_actions: Some(vec![
                "CONFIGURE_TOTP".to_string(),
                "UPDATE_PASSWORD".to_string(),
            ]),
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
        attrs.insert(
            "post.logout.redirect.uris".to_string(),
            "https://app.com/logout".to_string(),
        );

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
