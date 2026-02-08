//! Keycloak Admin API client
//!
//! This module provides a client for interacting with the Keycloak Admin REST API.
//! It handles authentication, token caching, and all CRUD operations for users,
//! clients, sessions, identity providers, and realm configuration.

use crate::config::KeycloakConfig;
use crate::error::{AppError, Result};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::*;

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

impl KeycloakClient {
    /// Create a new Keycloak client
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

    /// Get the public (browser-facing) Keycloak URL
    pub fn public_url(&self) -> &str {
        &self.config.public_url
    }

    /// Get the realm name
    pub fn realm(&self) -> &str {
        &self.config.realm
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

    // ============================================================================
    // User Management
    // ============================================================================

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

    /// List user credentials
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

    /// Delete a user credential
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

    /// Remove all TOTP credentials from a user
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

    /// Update a user
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

    /// Delete a user
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

    // ============================================================================
    // OIDC Client Management
    // ============================================================================

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

    /// Regenerate client secret
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

    /// Update an OIDC client
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

    /// Delete an OIDC client
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

    // ============================================================================
    // Password Management
    // ============================================================================

    /// Reset a user's password
    pub async fn reset_user_password(
        &self,
        user_id: &str,
        password: &str,
        temporary: bool,
    ) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/reset-password",
            self.config.url, self.config.realm, user_id
        );

        let credential = serde_json::json!({
            "type": "password",
            "value": password,
            "temporary": temporary
        });

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(&token)
            .json(&credential)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to reset password: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to reset password: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Validate a user's password by attempting to get a token
    pub async fn validate_user_password(&self, user_id: &str, password: &str) -> Result<bool> {
        // Get the user to get their username
        let user = self.get_user(user_id).await?;

        // Try to authenticate with the password
        let token_url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.config.url, self.config.realm
        );

        let response = self
            .http_client
            .post(&token_url)
            .form(&[
                ("grant_type", "password"),
                ("client_id", &self.config.admin_client_id),
                ("client_secret", &self.config.admin_client_secret),
                ("username", &user.username),
                ("password", password),
            ])
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to validate password: {}", e)))?;

        Ok(response.status().is_success())
    }

    // ============================================================================
    // Session Management
    // ============================================================================

    /// Get all sessions for a user
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<KeycloakSession>> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/sessions",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get user sessions: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get user sessions: {} - {}",
                status, body
            )));
        }

        let sessions: Vec<KeycloakSession> = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse sessions: {}", e)))?;

        Ok(sessions)
    }

    /// Delete a specific session
    pub async fn delete_user_session(&self, session_id: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/sessions/{}",
            self.config.url, self.config.realm, session_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to delete session: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Session not found in Keycloak".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to delete session: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Logout user from all sessions
    pub async fn logout_user(&self, user_id: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/logout",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to logout user: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to logout user: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    // ============================================================================
    // Identity Provider Management
    // ============================================================================

    /// List all identity providers
    pub async fn list_identity_providers(&self) -> Result<Vec<KeycloakIdentityProvider>> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/identity-provider/instances",
            self.config.url, self.config.realm
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to list identity providers: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to list identity providers: {} - {}",
                status, body
            )));
        }

        let providers: Vec<KeycloakIdentityProvider> = response.json().await.map_err(|e| {
            AppError::Keycloak(format!("Failed to parse identity providers: {}", e))
        })?;

        Ok(providers)
    }

    /// Get an identity provider by alias
    pub async fn get_identity_provider(&self, alias: &str) -> Result<KeycloakIdentityProvider> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/identity-provider/instances/{}",
            self.config.url, self.config.realm, alias
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get identity provider: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "Identity provider '{}' not found",
                alias
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get identity provider: {} - {}",
                status, body
            )));
        }

        let provider: KeycloakIdentityProvider = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse identity provider: {}", e)))?;

        Ok(provider)
    }

    /// Create an identity provider
    pub async fn create_identity_provider(
        &self,
        provider: &KeycloakIdentityProvider,
    ) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/identity-provider/instances",
            self.config.url, self.config.realm
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(provider)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to create identity provider: {}", e))
            })?;

        if response.status() == StatusCode::CONFLICT {
            return Err(AppError::Conflict(format!(
                "Identity provider '{}' already exists",
                provider.alias
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to create identity provider: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Update an identity provider
    pub async fn update_identity_provider(
        &self,
        alias: &str,
        provider: &KeycloakIdentityProvider,
    ) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/identity-provider/instances/{}",
            self.config.url, self.config.realm, alias
        );

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(&token)
            .json(provider)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to update identity provider: {}", e))
            })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "Identity provider '{}' not found",
                alias
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to update identity provider: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Delete an identity provider
    pub async fn delete_identity_provider(&self, alias: &str) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/identity-provider/instances/{}",
            self.config.url, self.config.realm, alias
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to delete identity provider: {}", e))
            })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "Identity provider '{}' not found",
                alias
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to delete identity provider: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Get federated identities for a user
    pub async fn get_user_federated_identities(
        &self,
        user_id: &str,
    ) -> Result<Vec<KeycloakFederatedIdentity>> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/federated-identity",
            self.config.url, self.config.realm, user_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to get federated identities: {}", e))
            })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound("User not found in Keycloak".to_string()));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get federated identities: {} - {}",
                status, body
            )));
        }

        let identities: Vec<KeycloakFederatedIdentity> = response.json().await.map_err(|e| {
            AppError::Keycloak(format!("Failed to parse federated identities: {}", e))
        })?;

        Ok(identities)
    }

    /// Remove a federated identity from a user
    pub async fn remove_user_federated_identity(
        &self,
        user_id: &str,
        provider_alias: &str,
    ) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/federated-identity/{}",
            self.config.url, self.config.realm, user_id, provider_alias
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                AppError::Keycloak(format!("Failed to remove federated identity: {}", e))
            })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(
                "Federated identity not found".to_string(),
            ));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to remove federated identity: {} - {}",
                status, body
            )));
        }

        Ok(())
    }

    /// List WebAuthn credentials for a user
    pub async fn list_webauthn_credentials(
        &self,
        user_id: &str,
    ) -> Result<Vec<KeycloakCredentialRepresentation>> {
        let all_creds = self.list_user_credentials(user_id).await?;

        let webauthn_creds: Vec<KeycloakCredentialRepresentation> = all_creds
            .into_iter()
            .filter(|c| {
                let ct = c.credential_type.to_lowercase();
                ct.contains("webauthn")
            })
            .map(|c| KeycloakCredentialRepresentation {
                id: c.id,
                credential_type: c.credential_type,
                user_label: None,
                created_date: None,
            })
            .collect();

        Ok(webauthn_creds)
    }

    // ============================================================================
    // Realm Management
    // ============================================================================

    /// Get current realm configuration
    pub async fn get_realm(&self) -> Result<KeycloakRealm> {
        let token = self.get_admin_token().await?;
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to get realm: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "Realm '{}' not found",
                self.config.realm
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to get realm: {} - {}",
                status, body
            )));
        }

        let realm: KeycloakRealm = response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse realm: {}", e)))?;

        Ok(realm)
    }

    /// Update realm configuration
    pub async fn update_realm(&self, update: &RealmUpdate) -> Result<()> {
        let token = self.get_admin_token().await?;
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(&token)
            .json(update)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to update realm: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "Realm '{}' not found",
                self.config.realm
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to update realm: {} - {}",
                status, body
            )));
        }

        Ok(())
    }
}
