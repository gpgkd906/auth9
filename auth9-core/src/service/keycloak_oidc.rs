//! Keycloak OIDC service for authentication flow
//!
//! This service encapsulates OIDC authentication flows with Keycloak,
//! providing a clean interface for authorization, callback handling,
//! token exchange, and logout operations.

use crate::config::KeycloakConfig;
use crate::domain::CreateUserInput;
use crate::error::{AppError, Result};
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::{ServiceRepository, UserRepository};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

/// OIDC authorization request parameters
#[derive(Debug, Clone)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub nonce: Option<String>,
}

/// OIDC callback result
#[derive(Debug, Clone)]
pub struct CallbackResult {
    pub identity_token: String,
    pub redirect_url: String,
    pub expires_in: i64,
}

/// Token response from OIDC flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

/// Internal callback state for tracking OIDC flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackState {
    pub redirect_uri: String,
    pub client_id: String,
    pub original_state: Option<String>,
}

/// Keycloak token response
#[derive(Debug, Deserialize)]
struct KeycloakTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

/// Keycloak userinfo response
#[derive(Debug, Deserialize)]
struct KeycloakUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
}

/// Keycloak OIDC service
///
/// Encapsulates OIDC authentication flows including:
/// - Authorization URL building
/// - Callback handling with user creation/lookup
/// - Token exchange
/// - Token refresh
/// - Logout URL building
pub struct KeycloakOidcService<U: UserRepository, S: ServiceRepository> {
    keycloak: Arc<KeycloakClient>,
    jwt_manager: Arc<JwtManager>,
    user_repo: Arc<U>,
    service_repo: Arc<S>,
    config: KeycloakConfig,
    issuer: String,
    http_client: reqwest::Client,
}

impl<U: UserRepository, S: ServiceRepository> KeycloakOidcService<U, S> {
    /// Create a new KeycloakOidcService
    pub fn new(
        keycloak: Arc<KeycloakClient>,
        jwt_manager: Arc<JwtManager>,
        user_repo: Arc<U>,
        service_repo: Arc<S>,
        config: KeycloakConfig,
        issuer: String,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            keycloak,
            jwt_manager,
            user_repo,
            service_repo,
            config,
            issuer,
            http_client,
        }
    }

    /// Build the Keycloak authorization URL for login
    pub async fn build_authorize_url(&self, params: &AuthorizeParams) -> Result<String> {
        // Validate client_id exists and get allowed redirect URIs
        let service = self
            .service_repo
            .find_by_client_id(&params.client_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Client '{}' not found", params.client_id))
            })?;

        // Validate redirect_uri is allowed
        if !service.redirect_uris.contains(&params.redirect_uri) {
            return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
        }

        // Build callback URL
        let callback_url = format!("{}/api/v1/auth/callback", self.issuer.trim_end_matches('/'));

        // Create state payload
        let state_payload = CallbackState {
            redirect_uri: params.redirect_uri.clone(),
            client_id: params.client_id.clone(),
            original_state: params.state.clone(),
        };

        let encoded_state = encode_state(&state_payload)?;

        // Build Keycloak auth URL
        let mut auth_url = Url::parse(&format!(
            "{}/realms/{}/protocol/openid-connect/auth",
            self.config.public_url, self.config.realm
        ))
        .map_err(|e| AppError::Internal(e.into()))?;

        {
            let mut pairs = auth_url.query_pairs_mut();
            pairs.append_pair("response_type", &params.response_type);
            pairs.append_pair("client_id", &params.client_id);
            pairs.append_pair("redirect_uri", &callback_url);
            pairs.append_pair("scope", &params.scope);
            pairs.append_pair("state", &encoded_state);
            if let Some(ref n) = params.nonce {
                pairs.append_pair("nonce", n);
            }
        }

        Ok(auth_url.to_string())
    }

    /// Handle the OIDC callback after Keycloak authentication
    pub async fn handle_callback(&self, code: &str, state: Option<&str>) -> Result<CallbackResult> {
        // Decode state
        let state_payload = decode_state(state)?;

        // Exchange code for tokens
        let token_response = self.exchange_code(code, &state_payload).await?;

        // Fetch user info from Keycloak
        let userinfo = self.fetch_userinfo(&token_response.access_token).await?;

        // Find or create user
        let user = match self.user_repo.find_by_keycloak_id(&userinfo.sub).await? {
            Some(existing) => existing,
            None => {
                let input = CreateUserInput {
                    email: userinfo.email.clone(),
                    display_name: userinfo.name.clone(),
                    avatar_url: None,
                };
                self.user_repo.create(&userinfo.sub, &input).await?
            }
        };

        // Create identity token
        let identity_token = self.jwt_manager.create_identity_token(
            *user.id,
            &userinfo.email,
            userinfo.name.as_deref(),
        )?;

        // Build redirect URL with tokens
        let mut redirect_url = Url::parse(&state_payload.redirect_uri)
            .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;

        {
            let mut pairs = redirect_url.query_pairs_mut();
            pairs.append_pair("access_token", &identity_token);
            pairs.append_pair("token_type", "Bearer");
            pairs.append_pair(
                "expires_in",
                &self.jwt_manager.access_token_ttl().to_string(),
            );
            if let Some(original_state) = state_payload.original_state {
                pairs.append_pair("state", &original_state);
            }
        }

        Ok(CallbackResult {
            identity_token,
            redirect_url: redirect_url.to_string(),
            expires_in: self.jwt_manager.access_token_ttl(),
        })
    }

    /// Exchange authorization code for tokens (for token endpoint)
    pub async fn exchange_authorization_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<OidcTokenResponse> {
        let state_payload = CallbackState {
            redirect_uri: redirect_uri.to_string(),
            client_id: client_id.to_string(),
            original_state: None,
        };

        let token_response = self.exchange_code(code, &state_payload).await?;
        let userinfo = self.fetch_userinfo(&token_response.access_token).await?;

        // Find or create user
        let user = match self.user_repo.find_by_keycloak_id(&userinfo.sub).await? {
            Some(existing) => existing,
            None => {
                let input = CreateUserInput {
                    email: userinfo.email.clone(),
                    display_name: userinfo.name.clone(),
                    avatar_url: None,
                };
                self.user_repo.create(&userinfo.sub, &input).await?
            }
        };

        let identity_token = self.jwt_manager.create_identity_token(
            *user.id,
            &userinfo.email,
            userinfo.name.as_deref(),
        )?;

        Ok(OidcTokenResponse {
            access_token: identity_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_manager.access_token_ttl(),
            refresh_token: token_response.refresh_token,
            id_token: token_response.id_token,
        })
    }

    /// Refresh token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        client_id: &str,
    ) -> Result<OidcTokenResponse> {
        let state_payload = CallbackState {
            redirect_uri: String::new(),
            client_id: client_id.to_string(),
            original_state: None,
        };

        let token_response = self
            .exchange_refresh_token(refresh_token, &state_payload)
            .await?;
        let userinfo = self.fetch_userinfo(&token_response.access_token).await?;

        // Find or create user
        let user = match self.user_repo.find_by_keycloak_id(&userinfo.sub).await? {
            Some(existing) => existing,
            None => {
                let input = CreateUserInput {
                    email: userinfo.email.clone(),
                    display_name: userinfo.name.clone(),
                    avatar_url: None,
                };
                self.user_repo.create(&userinfo.sub, &input).await?
            }
        };

        let identity_token = self.jwt_manager.create_identity_token(
            *user.id,
            &userinfo.email,
            userinfo.name.as_deref(),
        )?;

        Ok(OidcTokenResponse {
            access_token: identity_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_manager.access_token_ttl(),
            refresh_token: token_response.refresh_token,
            id_token: token_response.id_token,
        })
    }

    /// Build Keycloak logout URL
    pub fn build_logout_url(
        &self,
        id_token_hint: Option<&str>,
        post_logout_redirect_uri: Option<&str>,
        state: Option<&str>,
    ) -> Result<String> {
        let mut logout_url = Url::parse(&format!(
            "{}/realms/{}/protocol/openid-connect/logout",
            self.config.public_url, self.config.realm
        ))
        .map_err(|e| AppError::Internal(e.into()))?;

        {
            let mut pairs = logout_url.query_pairs_mut();
            if let Some(hint) = id_token_hint {
                pairs.append_pair("id_token_hint", hint);
            }
            if let Some(uri) = post_logout_redirect_uri {
                pairs.append_pair("post_logout_redirect_uri", uri);
            }
            if let Some(s) = state {
                pairs.append_pair("state", s);
            }
        }

        Ok(logout_url.to_string())
    }

    // ============================================================================
    // Internal methods
    // ============================================================================

    async fn exchange_code(
        &self,
        code: &str,
        state_payload: &CallbackState,
    ) -> Result<KeycloakTokenResponse> {
        let client_uuid = self
            .keycloak
            .get_client_uuid_by_client_id(&state_payload.client_id)
            .await?;
        let client_secret = self.keycloak.get_client_secret(&client_uuid).await?;

        let token_url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.config.url, self.config.realm
        );
        let callback_url = format!("{}/api/v1/auth/callback", self.issuer.trim_end_matches('/'));

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", state_payload.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", callback_url.as_str()),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to exchange code: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to exchange code: {} - {}",
                status, body
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to read token response: {}", e)))?;

        serde_json::from_str(&body)
            .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
    }

    async fn exchange_refresh_token(
        &self,
        refresh_token: &str,
        state_payload: &CallbackState,
    ) -> Result<KeycloakTokenResponse> {
        let client_uuid = self
            .keycloak
            .get_client_uuid_by_client_id(&state_payload.client_id)
            .await?;
        let client_secret = self.keycloak.get_client_secret(&client_uuid).await?;

        let token_url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.config.url, self.config.realm
        );

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", state_payload.client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to refresh token: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to refresh token: {} - {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
    }

    async fn fetch_userinfo(&self, access_token: &str) -> Result<KeycloakUserInfo> {
        let userinfo_url = format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            self.config.url, self.config.realm
        );

        tracing::debug!(
            "Fetching userinfo from {} with token length {}",
            userinfo_url,
            access_token.len()
        );

        let response = self
            .http_client
            .get(&userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to fetch userinfo: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Keycloak(format!(
                "Failed to fetch userinfo: {} - {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to parse userinfo: {}", e)))
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Encode callback state to base64
fn encode_state(state_payload: &CallbackState) -> Result<String> {
    let bytes = serde_json::to_vec(state_payload).map_err(|e| AppError::Internal(e.into()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// Decode callback state from base64
fn decode_state(state: Option<&str>) -> Result<CallbackState> {
    let encoded = state.ok_or_else(|| AppError::BadRequest("Missing state".to_string()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| AppError::BadRequest(format!("Invalid state: {}", e)))?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::Internal(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_state_roundtrip() {
        let state = CallbackState {
            redirect_uri: "https://app.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("user-state".to_string()),
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert_eq!(state.redirect_uri, decoded.redirect_uri);
        assert_eq!(state.client_id, decoded.client_id);
        assert_eq!(state.original_state, decoded.original_state);
    }

    #[test]
    fn test_decode_state_missing() {
        let result = decode_state(None);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_decode_state_invalid_base64() {
        let result = decode_state(Some("not-valid-base64!!!"));
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_encode_state_without_original_state() {
        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "client".to_string(),
            original_state: None,
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert!(decoded.original_state.is_none());
    }

    #[test]
    fn test_callback_result_fields() {
        let result = CallbackResult {
            identity_token: "token".to_string(),
            redirect_url: "https://app.com/callback?token=abc".to_string(),
            expires_in: 3600,
        };

        assert_eq!(result.identity_token, "token");
        assert!(result.redirect_url.contains("callback"));
        assert_eq!(result.expires_in, 3600);
    }

    #[test]
    fn test_authorize_params() {
        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "my-app".to_string(),
            redirect_uri: "https://app.com/callback".to_string(),
            scope: "openid profile email".to_string(),
            state: Some("user-state".to_string()),
            nonce: Some("nonce-123".to_string()),
        };

        assert_eq!(params.response_type, "code");
        assert_eq!(params.client_id, "my-app");
        assert!(params.state.is_some());
        assert!(params.nonce.is_some());
    }

    #[test]
    fn test_oidc_token_response_serialization() {
        let response = OidcTokenResponse {
            access_token: "access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh-token".to_string()),
            id_token: Some("id-token".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_callback_state_with_special_characters() {
        let state = CallbackState {
            redirect_uri: "https://app.com/callback?foo=bar&baz=qux".to_string(),
            client_id: "client-with-dashes".to_string(),
            original_state: Some("state with spaces".to_string()),
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert!(decoded.redirect_uri.contains("foo=bar"));
        assert_eq!(decoded.client_id, "client-with-dashes");
    }
}
