//! Keycloak Admin API client

use crate::config::KeycloakConfig;
use crate::error::{AppError, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// Keycloak client (OIDC) representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakOidcClient {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub protocol: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub web_origins: Vec<String>,
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

        // Fetch new token
        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.admin_client_id),
            ("client_secret", &self.config.admin_client_secret),
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
}
