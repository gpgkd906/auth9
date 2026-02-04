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
    use crate::config::JwtConfig;
    use crate::domain::{Service, ServiceStatus};
    use crate::repository::service::MockServiceRepository;
    use crate::repository::user::MockUserRepository;
    use chrono::Utc;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_keycloak_config(base_url: &str) -> KeycloakConfig {
        KeycloakConfig {
            url: base_url.to_string(),
            public_url: base_url.to_string(),
            realm: "test-realm".to_string(),
            admin_client_id: "auth9-admin".to_string(),
            admin_client_secret: "admin-secret".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        }
    }

    fn create_test_jwt_manager() -> JwtManager {
        let config = JwtConfig {
            secret: "test-secret-key-for-jwt-signing-minimum-32-chars".to_string(),
            issuer: "https://auth9.example.com".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 86400,
            private_key_pem: None,
            public_key_pem: None,
        };
        JwtManager::new(config)
    }

    fn create_test_service(_client_id: &str, redirect_uris: Vec<String>) -> Service {
        let now = Utc::now();
        Service {
            id: crate::domain::StringUuid::new_v4(),
            tenant_id: None,
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris,
            logout_uris: vec!["https://app.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    fn create_test_user(keycloak_id: &str, email: &str) -> crate::domain::User {
        let now = Utc::now();
        crate::domain::User {
            id: crate::domain::StringUuid::new_v4(),
            keycloak_id: keycloak_id.to_string(),
            email: email.to_string(),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
            mfa_enabled: false,
            created_at: now,
            updated_at: now,
        }
    }

    async fn setup_admin_token_mock(mock_server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/realms/master/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "mock-admin-token",
                "expires_in": 300
            })))
            .mount(mock_server)
            .await;
    }

    // ============================================================================
    // build_authorize_url tests
    // ============================================================================

    #[tokio::test]
    async fn test_build_authorize_url_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        let mut mock_service_repo = MockServiceRepository::new();
        mock_service_repo
            .expect_find_by_client_id()
            .withf(|client_id| client_id == "my-app")
            .returning(|_| {
                Ok(Some(create_test_service(
                    "my-app",
                    vec!["https://app.example.com/callback".to_string()],
                )))
            });

        let mock_user_repo = MockUserRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "my-app".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scope: "openid profile email".to_string(),
            state: Some("user-state-123".to_string()),
            nonce: Some("nonce-abc".to_string()),
        };

        let result = service.build_authorize_url(&params).await;
        assert!(result.is_ok());

        let url = result.unwrap();
        assert!(url.contains("/realms/test-realm/protocol/openid-connect/auth"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-app"));
        assert!(url.contains("scope=openid+profile+email"));
        assert!(url.contains("nonce=nonce-abc"));
        // State is encoded, so we just check state param exists
        assert!(url.contains("state="));
    }

    #[tokio::test]
    async fn test_build_authorize_url_without_optional_params() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        let mut mock_service_repo = MockServiceRepository::new();
        mock_service_repo
            .expect_find_by_client_id()
            .returning(|_| {
                Ok(Some(create_test_service(
                    "minimal-app",
                    vec!["https://app.example.com/cb".to_string()],
                )))
            });

        let mock_user_repo = MockUserRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "minimal-app".to_string(),
            redirect_uri: "https://app.example.com/cb".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        let result = service.build_authorize_url(&params).await;
        assert!(result.is_ok());

        let url = result.unwrap();
        assert!(url.contains("response_type=code"));
        assert!(!url.contains("nonce="));
    }

    #[tokio::test]
    async fn test_build_authorize_url_client_not_found() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        let mut mock_service_repo = MockServiceRepository::new();
        mock_service_repo
            .expect_find_by_client_id()
            .returning(|_| Ok(None));

        let mock_user_repo = MockUserRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "nonexistent-app".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        let result = service.build_authorize_url(&params).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_build_authorize_url_invalid_redirect_uri() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        let mut mock_service_repo = MockServiceRepository::new();
        mock_service_repo
            .expect_find_by_client_id()
            .returning(|_| {
                Ok(Some(create_test_service(
                    "my-app",
                    vec!["https://allowed.example.com/callback".to_string()],
                )))
            });

        let mock_user_repo = MockUserRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "my-app".to_string(),
            redirect_uri: "https://malicious.example.com/callback".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        let result = service.build_authorize_url(&params).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    // ============================================================================
    // build_logout_url tests
    // ============================================================================

    #[tokio::test]
    async fn test_build_logout_url_with_all_params() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));
        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service.build_logout_url(
            Some("id-token-hint"),
            Some("https://app.example.com/logged-out"),
            Some("logout-state"),
        );

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("/realms/test-realm/protocol/openid-connect/logout"));
        assert!(url.contains("id_token_hint=id-token-hint"));
        assert!(url.contains("post_logout_redirect_uri="));
        assert!(url.contains("state=logout-state"));
    }

    #[tokio::test]
    async fn test_build_logout_url_minimal() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));
        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service.build_logout_url(None, None, None);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("/realms/test-realm/protocol/openid-connect/logout"));
        assert!(!url.contains("id_token_hint"));
        assert!(!url.contains("post_logout_redirect_uri"));
        assert!(!url.contains("state="));
    }

    #[tokio::test]
    async fn test_build_logout_url_with_id_token_only() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));
        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service.build_logout_url(Some("my-id-token"), None, None);

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("id_token_hint=my-id-token"));
        assert!(!url.contains("post_logout_redirect_uri"));
    }

    // ============================================================================
    // handle_callback tests
    // ============================================================================

    #[tokio::test]
    async fn test_handle_callback_existing_user() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        // Setup admin token mock
        setup_admin_token_mock(&mock_server).await;

        // Mock get client by client_id
        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .and(query_param("clientId", "test-client"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-123",
                "clientId": "test-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        // Mock get client secret
        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-123/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "client-secret-value"
            })))
            .mount(&mock_server)
            .await;

        // Mock token exchange
        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "keycloak-access-token",
                "refresh_token": "keycloak-refresh-token",
                "id_token": "keycloak-id-token"
            })))
            .mount(&mock_server)
            .await;

        // Mock userinfo
        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "keycloak-user-id",
                "email": "user@example.com",
                "name": "Test User"
            })))
            .mount(&mock_server)
            .await;

        let existing_user = create_test_user("keycloak-user-id", "user@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .withf(|kc_id| kc_id == "keycloak-user-id")
            .returning(move |_| Ok(Some(existing_user.clone())));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        // Create encoded state
        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("original-user-state".to_string()),
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code-123", Some(&encoded_state))
            .await;

        assert!(result.is_ok());
        let callback_result = result.unwrap();
        assert!(!callback_result.identity_token.is_empty());
        assert!(callback_result.redirect_url.contains("access_token="));
        assert!(callback_result.redirect_url.contains("state=original-user-state"));
        assert_eq!(callback_result.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_handle_callback_new_user_created() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .and(query_param("clientId", "test-client"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-123",
                "clientId": "test-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-123/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "client-secret-value"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "keycloak-access-token",
                "refresh_token": "keycloak-refresh-token",
                "id_token": "keycloak-id-token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "new-keycloak-user-id",
                "email": "newuser@example.com",
                "name": "New User"
            })))
            .mount(&mock_server)
            .await;

        let created_user = create_test_user("new-keycloak-user-id", "newuser@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None));
        mock_user_repo
            .expect_create()
            .returning(move |_, _| Ok(created_user.clone()));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code-456", Some(&encoded_state))
            .await;

        assert!(result.is_ok());
        let callback_result = result.unwrap();
        assert!(!callback_result.identity_token.is_empty());
        assert!(callback_result
            .redirect_url
            .starts_with("https://app.example.com/callback"));
    }

    #[tokio::test]
    async fn test_handle_callback_missing_state() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));
        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service.handle_callback("auth-code", None).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_token_exchange_error() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-123",
                "clientId": "test-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-123/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "client-secret-value"
            })))
            .mount(&mock_server)
            .await;

        // Token exchange fails
        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "invalid_grant",
                "error_description": "Code not valid"
            })))
            .mount(&mock_server)
            .await;

        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("invalid-code", Some(&encoded_state))
            .await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    // ============================================================================
    // exchange_authorization_code tests
    // ============================================================================

    #[tokio::test]
    async fn test_exchange_authorization_code_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-abc",
                "clientId": "api-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-abc/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "api-client-secret"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "kc-access-token",
                "refresh_token": "kc-refresh-token",
                "id_token": "kc-id-token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "user-sub-id",
                "email": "api-user@example.com",
                "name": "API User"
            })))
            .mount(&mock_server)
            .await;

        let existing_user = create_test_user("user-sub-id", "api-user@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(existing_user.clone())));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service
            .exchange_authorization_code(
                "auth-code-xyz",
                "api-client",
                "https://api.example.com/callback",
            )
            .await;

        assert!(result.is_ok());
        let token_response = result.unwrap();
        assert!(!token_response.access_token.is_empty());
        assert_eq!(token_response.token_type, "Bearer");
        assert_eq!(token_response.expires_in, 3600);
        assert!(token_response.refresh_token.is_some());
        assert!(token_response.id_token.is_some());
    }

    #[tokio::test]
    async fn test_exchange_authorization_code_creates_new_user() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-new",
                "clientId": "new-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-new/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "new-client-secret"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new-kc-access-token",
                "refresh_token": "new-kc-refresh-token",
                "id_token": null
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "brand-new-user-id",
                "email": "brandnew@example.com",
                "name": null
            })))
            .mount(&mock_server)
            .await;

        let created_user = create_test_user("brand-new-user-id", "brandnew@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None));
        mock_user_repo
            .expect_create()
            .returning(move |_, _| Ok(created_user.clone()));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service
            .exchange_authorization_code(
                "new-auth-code",
                "new-client",
                "https://newapp.example.com/callback",
            )
            .await;

        assert!(result.is_ok());
        let token_response = result.unwrap();
        assert!(!token_response.access_token.is_empty());
    }

    // ============================================================================
    // refresh_token tests
    // ============================================================================

    #[tokio::test]
    async fn test_refresh_token_success() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "refresh-client-uuid",
                "clientId": "refresh-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/refresh-client-uuid/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "refresh-client-secret"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new-access-token-after-refresh",
                "refresh_token": "new-refresh-token",
                "id_token": "new-id-token"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "refresh-user-id",
                "email": "refresh-user@example.com",
                "name": "Refresh User"
            })))
            .mount(&mock_server)
            .await;

        let existing_user = create_test_user("refresh-user-id", "refresh-user@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(existing_user.clone())));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service
            .refresh_token("old-refresh-token", "refresh-client")
            .await;

        assert!(result.is_ok());
        let token_response = result.unwrap();
        assert!(!token_response.access_token.is_empty());
        assert_eq!(token_response.token_type, "Bearer");
        assert!(token_response.refresh_token.is_some());
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_token() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid",
                "clientId": "test-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "client-secret"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "invalid_grant",
                "error_description": "Token is not active"
            })))
            .mount(&mock_server)
            .await;

        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service
            .refresh_token("invalid-refresh-token", "test-client")
            .await;

        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_refresh_token_creates_new_user() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "refresh-client-uuid",
                "clientId": "refresh-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/refresh-client-uuid/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "refresh-client-secret"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "refreshed-access-token",
                "refresh_token": "new-refresh-token",
                "id_token": null
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "new-user-from-refresh",
                "email": "newrefresh@example.com",
                "name": "New Refresh User"
            })))
            .mount(&mock_server)
            .await;

        let created_user = create_test_user("new-user-from-refresh", "newrefresh@example.com");
        let mut mock_user_repo = MockUserRepository::new();
        mock_user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None));
        mock_user_repo
            .expect_create()
            .returning(move |_, _| Ok(created_user.clone()));

        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let result = service
            .refresh_token("valid-refresh-token", "refresh-client")
            .await;

        assert!(result.is_ok());
        let token_response = result.unwrap();
        assert!(!token_response.access_token.is_empty());
    }

    // ============================================================================
    // Error handling tests
    // ============================================================================

    #[tokio::test]
    async fn test_handle_callback_userinfo_error() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": "client-uuid-123",
                "clientId": "test-client",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }])))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients/client-uuid-123/client-secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": "client-secret-value"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/realms/test-realm/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "valid-access-token",
                "refresh_token": "valid-refresh-token",
                "id_token": "valid-id-token"
            })))
            .mount(&mock_server)
            .await;

        // Userinfo endpoint returns error
        Mock::given(method("GET"))
            .and(path("/realms/test-realm/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_token"
            })))
            .mount(&mock_server)
            .await;

        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code", Some(&encoded_state))
            .await;

        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_client_not_found_in_keycloak() {
        let mock_server = MockServer::start().await;
        let config = create_test_keycloak_config(&mock_server.uri());
        let jwt_manager = Arc::new(create_test_jwt_manager());
        let keycloak = Arc::new(KeycloakClient::new(config.clone()));

        setup_admin_token_mock(&mock_server).await;

        // Client query returns empty array
        Mock::given(method("GET"))
            .and(path("/admin/realms/test-realm/clients"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&mock_server)
            .await;

        let mock_user_repo = MockUserRepository::new();
        let mock_service_repo = MockServiceRepository::new();

        let service = KeycloakOidcService::new(
            keycloak,
            jwt_manager,
            Arc::new(mock_user_repo),
            Arc::new(mock_service_repo),
            config.clone(),
            "https://auth9.example.com".to_string(),
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "nonexistent-client".to_string(),
            original_state: None,
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code", Some(&encoded_state))
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ============================================================================
    // Original helper function tests
    // ============================================================================

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

    #[test]
    fn test_oidc_token_response_without_optional_fields() {
        let response = OidcTokenResponse {
            access_token: "access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 1800,
            refresh_token: None,
            id_token: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("1800"));
        assert!(!json.contains("refresh_token\":\""));
    }

    #[test]
    fn test_authorize_params_minimal() {
        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "minimal-app".to_string(),
            redirect_uri: "https://app.com/cb".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        assert_eq!(params.response_type, "code");
        assert!(params.state.is_none());
        assert!(params.nonce.is_none());
    }

    #[test]
    fn test_callback_state_serialization() {
        let state = CallbackState {
            redirect_uri: "https://app.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("original".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("redirect_uri"));
        assert!(json.contains("client_id"));
        assert!(json.contains("original_state"));

        let decoded: CallbackState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.redirect_uri, state.redirect_uri);
    }

    #[test]
    fn test_decode_state_invalid_json() {
        // Valid base64 but invalid JSON
        let invalid_json_base64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("not json at all");
        let result = decode_state(Some(&invalid_json_base64));
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_state_empty_strings() {
        let state = CallbackState {
            redirect_uri: "".to_string(),
            client_id: "".to_string(),
            original_state: None,
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert_eq!(decoded.redirect_uri, "");
        assert_eq!(decoded.client_id, "");
    }

    #[test]
    fn test_callback_result_construction() {
        let result = CallbackResult {
            identity_token: "eyJ...".to_string(),
            redirect_url: "https://app.com/callback?access_token=xyz".to_string(),
            expires_in: 7200,
        };

        assert!(result.identity_token.starts_with("eyJ"));
        assert!(result.redirect_url.contains("access_token="));
        assert_eq!(result.expires_in, 7200);
    }
}
