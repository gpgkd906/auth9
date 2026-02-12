//! Keycloak OIDC service for authentication flow
//!
//! This service encapsulates OIDC authentication flows with Keycloak,
//! providing a clean interface for authorization, callback handling,
//! token exchange, and logout operations.

use crate::config::KeycloakConfig;
use crate::domain::{
    ActionContext, ActionContextRequest, ActionContextTenant, ActionContextUser, CreateUserInput,
    StringUuid,
};
use crate::error::{AppError, Result};
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::{ActionRepository, ServiceRepository, UserRepository};
use crate::service::ActionEngine;
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

/// Minimal Keycloak admin API needed by [`KeycloakOidcService`].
///
/// This indirection keeps unit tests fast and hermetic (no wiremock/TCP listener required).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait KeycloakOidcAdminApi: Send + Sync {
    async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String>;
    async fn get_client_secret(&self, client_uuid: &str) -> Result<String>;
}

#[async_trait]
impl KeycloakOidcAdminApi for KeycloakClient {
    async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String> {
        KeycloakClient::get_client_uuid_by_client_id(self, client_id).await
    }

    async fn get_client_secret(&self, client_uuid: &str) -> Result<String> {
        KeycloakClient::get_client_secret(self, client_uuid).await
    }
}

/// Minimal HTTP interface for OIDC token/userinfo calls.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OidcHttpClient: Send + Sync {
    async fn post_form(
        &self,
        url: &str,
        params: Vec<(String, String)>,
    ) -> std::result::Result<(u16, String), String>;

    async fn get_bearer(
        &self,
        url: &str,
        bearer_token: &str,
    ) -> std::result::Result<(u16, String), String>;
}

#[derive(Clone)]
struct ReqwestOidcHttpClient {
    client: reqwest::Client,
}

#[async_trait]
impl OidcHttpClient for ReqwestOidcHttpClient {
    async fn post_form(
        &self,
        url: &str,
        params: Vec<(String, String)>,
    ) -> std::result::Result<(u16, String), String> {
        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        Ok((status, body))
    }

    async fn get_bearer(
        &self,
        url: &str,
        bearer_token: &str,
    ) -> std::result::Result<(u16, String), String> {
        let resp = self
            .client
            .get(url)
            .bearer_auth(bearer_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        Ok((status, body))
    }
}

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
pub struct KeycloakOidcService<U: UserRepository, S: ServiceRepository, A: ActionRepository> {
    keycloak: Arc<dyn KeycloakOidcAdminApi>,
    jwt_manager: Arc<JwtManager>,
    user_repo: Arc<U>,
    service_repo: Arc<S>,
    config: KeycloakConfig,
    issuer: String,
    http_client: Arc<dyn OidcHttpClient>,
    action_engine: Option<Arc<ActionEngine<A>>>,
}

impl<U: UserRepository, S: ServiceRepository, A: ActionRepository + 'static>
    KeycloakOidcService<U, S, A>
{
    /// Create a new KeycloakOidcService
    pub fn new(
        keycloak: Arc<dyn KeycloakOidcAdminApi>,
        jwt_manager: Arc<JwtManager>,
        user_repo: Arc<U>,
        service_repo: Arc<S>,
        config: KeycloakConfig,
        issuer: String,
        action_engine: Option<Arc<ActionEngine<A>>>,
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
            http_client: Arc::new(ReqwestOidcHttpClient {
                client: http_client,
            }),
            action_engine,
        }
    }

    #[cfg(test)]
    fn new_with_http(
        keycloak: Arc<dyn KeycloakOidcAdminApi>,
        jwt_manager: Arc<JwtManager>,
        user_repo: Arc<U>,
        service_repo: Arc<S>,
        config: KeycloakConfig,
        issuer: String,
        http_client: Arc<dyn OidcHttpClient>,
        action_engine: Option<Arc<ActionEngine<A>>>,
    ) -> Self {
        Self {
            keycloak,
            jwt_manager,
            user_repo,
            service_repo,
            config,
            issuer,
            http_client,
            action_engine,
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

        // Get service to determine tenant_id (moved before user creation for action triggers)
        let service = self
            .service_repo
            .find_by_client_id(&state_payload.client_id)
            .await?
            .ok_or_else(|| {
                AppError::BadRequest(format!("Service not found: {}", state_payload.client_id))
            })?;

        let tenant_id = service
            .tenant_id
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Service has no tenant_id")))?;

        // Find or create user
        let user = match self.user_repo.find_by_keycloak_id(&userinfo.sub).await? {
            Some(existing) => existing,
            None => {
                let input = CreateUserInput {
                    email: userinfo.email.clone(),
                    display_name: userinfo.name.clone(),
                    avatar_url: None,
                };

                // Execute PreUserRegistration action (before creating user)
                if let Some(ref action_engine) = self.action_engine {
                    let pre_reg_context = ActionContext {
                        user: ActionContextUser {
                            id: StringUuid::new_v4().to_string(), // Temporary ID for pre-registration
                            email: input.email.clone(),
                            display_name: input.display_name.clone(),
                            mfa_enabled: false,
                        },
                        tenant: ActionContextTenant {
                            id: tenant_id.to_string(),
                            slug: "unknown".to_string(), // TODO: Fetch tenant details if needed
                            name: "Unknown Tenant".to_string(),
                        },
                        request: ActionContextRequest {
                            ip: None, // TODO: Extract from request context
                            user_agent: None,
                            timestamp: Utc::now(),
                        },
                        claims: None,
                    };

                    // Execute pre-registration trigger (can block registration)
                    match action_engine
                        .execute_trigger(tenant_id, "pre-user-registration", pre_reg_context)
                        .await
                    {
                        Ok(_) => {
                            tracing::info!("PreUserRegistration actions passed for email {}", input.email);
                        }
                        Err(e) => {
                            // Strict mode: abort registration on action failure
                            tracing::error!("PreUserRegistration action failed for email {}: {}", input.email, e);
                            return Err(e);
                        }
                    }
                }

                // Create user in repository
                let new_user = self.user_repo.create(&userinfo.sub, &input).await?;

                // Execute PostUserRegistration action (after creating user)
                if let Some(ref action_engine) = self.action_engine {
                    let post_reg_context = ActionContext {
                        user: ActionContextUser {
                            id: new_user.id.to_string(),
                            email: new_user.email.clone(),
                            display_name: new_user.display_name.clone(),
                            mfa_enabled: false,
                        },
                        tenant: ActionContextTenant {
                            id: tenant_id.to_string(),
                            slug: "unknown".to_string(),
                            name: "Unknown Tenant".to_string(),
                        },
                        request: ActionContextRequest {
                            ip: None,
                            user_agent: None,
                            timestamp: Utc::now(),
                        },
                        claims: None,
                    };

                    // Execute post-registration trigger (failure won't block, but will log)
                    if let Err(e) = action_engine
                        .execute_trigger(tenant_id, "post-user-registration", post_reg_context)
                        .await
                    {
                        // Log error but don't abort (user already created)
                        tracing::error!("PostUserRegistration action failed for user {}: {}", new_user.id, e);
                    } else {
                        tracing::info!("PostUserRegistration actions executed for user {}", new_user.id);
                    }
                }

                new_user
            }
        };

        // Execute PostLogin Actions (if ActionEngine is configured)
        let custom_claims = if let Some(ref action_engine) = self.action_engine {
            // Build action context (tenant_id already obtained above)
            let action_context = ActionContext {
                user: ActionContextUser {
                    id: user.id.to_string(),
                    email: user.email.clone(),
                    display_name: user.display_name.clone(),
                    mfa_enabled: false, // TODO: Get actual MFA status when implemented
                },
                tenant: ActionContextTenant {
                    id: tenant_id.to_string(),
                    slug: "unknown".to_string(), // TODO: Fetch tenant details if needed
                    name: "Unknown Tenant".to_string(),
                },
                request: ActionContextRequest {
                    ip: None, // TODO: Extract from request context
                    user_agent: None,
                    timestamp: Utc::now(),
                },
                claims: None,
            };

            // Execute PostLogin trigger
            match action_engine
                .execute_trigger(tenant_id, "post-login", action_context)
                .await
            {
                Ok(modified_context) => {
                    tracing::info!("PostLogin actions executed successfully for user {}", user.id);
                    modified_context.claims
                }
                Err(e) => {
                    // Strict mode: abort login on action failure
                    tracing::error!("PostLogin action failed for user {}: {}", user.id, e);
                    return Err(e);
                }
            }
        } else {
            None
        };

        // Create identity token with custom claims
        let identity_token = if let Some(claims) = custom_claims {
            self.jwt_manager.create_identity_token_with_claims(
                *user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
                claims,
            )?
        } else {
            self.jwt_manager.create_identity_token(
                *user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
            )?
        };

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

        // Get service to determine tenant_id
        let service = self
            .service_repo
            .find_by_client_id(client_id)
            .await?
            .ok_or_else(|| AppError::BadRequest(format!("Service not found: {}", client_id)))?;

        let tenant_id = service
            .tenant_id
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Service has no tenant_id")))?;

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

        // Execute PreTokenRefresh action (before creating new identity token)
        let custom_claims = if let Some(ref action_engine) = self.action_engine {
            let pre_refresh_context = ActionContext {
                user: ActionContextUser {
                    id: user.id.to_string(),
                    email: user.email.clone(),
                    display_name: user.display_name.clone(),
                    mfa_enabled: false,
                },
                tenant: ActionContextTenant {
                    id: tenant_id.to_string(),
                    slug: "unknown".to_string(),
                    name: "Unknown Tenant".to_string(),
                },
                request: ActionContextRequest {
                    ip: None,
                    user_agent: None,
                    timestamp: Utc::now(),
                },
                claims: None,
            };

            // Execute pre-token-refresh trigger (can block token refresh)
            match action_engine
                .execute_trigger(tenant_id, "pre-token-refresh", pre_refresh_context)
                .await
            {
                Ok(modified_context) => {
                    tracing::info!("PreTokenRefresh actions passed for user {}", user.id);
                    modified_context.claims
                }
                Err(e) => {
                    // Strict mode: abort token refresh on action failure
                    tracing::error!("PreTokenRefresh action failed for user {}: {}", user.id, e);
                    return Err(e);
                }
            }
        } else {
            None
        };

        // Create identity token with custom claims (if any)
        let identity_token = if let Some(claims) = custom_claims {
            self.jwt_manager.create_identity_token_with_claims(
                *user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
                claims,
            )?
        } else {
            self.jwt_manager.create_identity_token(
                *user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
            )?
        };

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

        let params = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), state_payload.client_id.clone()),
            ("client_secret".to_string(), client_secret),
            ("code".to_string(), code.to_string()),
            ("redirect_uri".to_string(), callback_url),
        ];

        let (status, body) = self
            .http_client
            .post_form(&token_url, params)
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to exchange code: {}", e)))?;

        if !(200..=299).contains(&status) {
            return Err(AppError::Keycloak(format!(
                "Failed to exchange code: {} - {}",
                status, body
            )));
        }

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

        let params = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), state_payload.client_id.clone()),
            ("client_secret".to_string(), client_secret),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ];

        let (status, body) = self
            .http_client
            .post_form(&token_url, params)
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to refresh token: {}", e)))?;

        if !(200..=299).contains(&status) {
            return Err(AppError::Keycloak(format!(
                "Failed to refresh token: {} - {}",
                status, body
            )));
        }

        serde_json::from_str(&body)
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

        let (status, body) = self
            .http_client
            .get_bearer(&userinfo_url, access_token)
            .await
            .map_err(|e| AppError::Keycloak(format!("Failed to fetch userinfo: {}", e)))?;

        if !(200..=299).contains(&status) {
            return Err(AppError::Keycloak(format!(
                "Failed to fetch userinfo: {} - {}",
                status, body
            )));
        }

        serde_json::from_str(&body)
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
    use crate::domain::{Action, Service, ServiceStatus};
    use crate::repository::action::MockActionRepository;
    use crate::repository::service::MockServiceRepository;
    use crate::repository::user::MockUserRepository;
    use chrono::Utc;
    use mockall::predicate::*;

    fn create_test_keycloak_config() -> KeycloakConfig {
        KeycloakConfig {
            url: "https://keycloak.example.com".to_string(),
            public_url: "https://keycloak.example.com".to_string(),
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

    fn create_test_service(redirect_uris: Vec<String>) -> Service {
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

    fn token_url(config: &KeycloakConfig) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/token",
            config.url, config.realm
        )
    }

    fn userinfo_url(config: &KeycloakConfig) -> String {
        format!(
            "{}/realms/{}/protocol/openid-connect/userinfo",
            config.url, config.realm
        )
    }

    fn build_service(
        keycloak: Arc<dyn KeycloakOidcAdminApi>,
        http: Arc<dyn OidcHttpClient>,
        user_repo: Arc<MockUserRepository>,
        service_repo: Arc<MockServiceRepository>,
        config: KeycloakConfig,
    ) -> KeycloakOidcService<MockUserRepository, MockServiceRepository, MockActionRepository> {
        KeycloakOidcService::new_with_http(
            keycloak,
            Arc::new(create_test_jwt_manager()),
            user_repo,
            service_repo,
            config,
            "https://auth9.example.com".to_string(),
            http,
            None, // No ActionEngine
        )
    }

    fn build_service_with_actions(
        keycloak: Arc<dyn KeycloakOidcAdminApi>,
        http: Arc<dyn OidcHttpClient>,
        user_repo: Arc<MockUserRepository>,
        service_repo: Arc<MockServiceRepository>,
        action_repo: Arc<MockActionRepository>,
        config: KeycloakConfig,
    ) -> KeycloakOidcService<MockUserRepository, MockServiceRepository, MockActionRepository> {
        let action_engine = Arc::new(ActionEngine::new(action_repo));
        KeycloakOidcService::new_with_http(
            keycloak,
            Arc::new(create_test_jwt_manager()),
            user_repo,
            service_repo,
            config,
            "https://auth9.example.com".to_string(),
            http,
            Some(action_engine),
        )
    }

    fn create_test_action(trigger_id: &str, script: &str, tenant_id: StringUuid) -> Action {
        let now = Utc::now();
        Action {
            id: StringUuid::new_v4(),
            tenant_id,
            name: format!("Test {} Action", trigger_id),
            description: Some("Test action".to_string()),
            trigger_id: trigger_id.to_string(),
            script: script.to_string(),
            enabled: true,
            execution_order: 0,
            timeout_ms: 3000,
            last_executed_at: None,
            execution_count: 0,
            error_count: 0,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_build_authorize_url_success() {
        let config = create_test_keycloak_config();

        let mut service_repo = MockServiceRepository::new();
        service_repo
            .expect_find_by_client_id()
            .with(eq("my-app"))
            .returning(|_| {
                Ok(Some(create_test_service(vec![
                    "https://app.example.com/callback".to_string(),
                ])))
            });

        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(service_repo),
            config.clone(),
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "my-app".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scope: "openid profile email".to_string(),
            state: Some("user-state-123".to_string()),
            nonce: Some("nonce-abc".to_string()),
        };

        let url = service.build_authorize_url(&params).await.unwrap();
        assert!(url.contains("/realms/test-realm/protocol/openid-connect/auth"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-app"));
        assert!(url.contains("scope=openid+profile+email"));
        assert!(url.contains("nonce=nonce-abc"));
        assert!(url.contains("state="));
    }

    #[tokio::test]
    async fn test_build_authorize_url_invalid_redirect_uri() {
        let config = create_test_keycloak_config();

        let mut service_repo = MockServiceRepository::new();
        service_repo.expect_find_by_client_id().returning(|_| {
            Ok(Some(create_test_service(vec![
                "https://allowed.example.com/callback".to_string(),
            ])))
        });

        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(service_repo),
            config,
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

    #[tokio::test]
    async fn test_build_logout_url_minimal() {
        let config = create_test_keycloak_config();
        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let url = service.build_logout_url(None, None, None).unwrap();
        assert!(url.contains("/realms/test-realm/protocol/openid-connect/logout"));
        assert!(!url.contains("id_token_hint"));
        assert!(!url.contains("post_logout_redirect_uri"));
        assert!(!url.contains("state="));
    }

    #[tokio::test]
    async fn test_handle_callback_missing_state() {
        let config = create_test_keycloak_config();
        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let result = service.handle_callback("auth-code", None).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_existing_user_success() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid-123".to_string()))
            .times(1);
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid-123"))
            .returning(|_| Ok("client-secret-value".to_string()))
            .times(1);

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, params| {
                url == token_url.as_str()
                    && params
                        .iter()
                        .any(|(k, v)| k == "grant_type" && v == "authorization_code")
            })
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "keycloak-access-token",
                        "refresh_token": "keycloak-refresh-token",
                        "id_token": "keycloak-id-token"
                    })
                    .to_string(),
                ))
            })
            .times(1);

        http.expect_get_bearer()
            .withf(move |url, token| {
                url == userinfo_url.as_str() && token == "keycloak-access-token"
            })
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "keycloak-user-id",
                        "email": "user@example.com",
                        "name": "Test User"
                    })
                    .to_string(),
                ))
            })
            .times(1);

        let existing_user = create_test_user("keycloak-user-id", "user@example.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .with(eq("keycloak-user-id"))
            .returning(move |_| Ok(Some(existing_user.clone())))
            .times(1);

        let mut service_repo = MockServiceRepository::new();
        let test_service = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(StringUuid::new_v4()),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec![],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .with(eq("test-client"))
            .returning(move |_| Ok(Some(test_service.clone())))
            .times(1);

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            config,
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("original-user-state".to_string()),
        };
        let encoded_state = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code-123", Some(&encoded_state))
            .await
            .unwrap();

        assert!(!result.identity_token.is_empty());
        assert!(result.redirect_url.contains("access_token="));
        assert!(result.redirect_url.contains("state=original-user-state"));
        assert_eq!(result.expires_in, 3600);
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_token_propagates_error() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid-123".to_string()))
            .times(1);
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid-123"))
            .returning(|_| Ok("client-secret-value".to_string()))
            .times(1);

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, params| {
                url == token_url.as_str()
                    && params
                        .iter()
                        .any(|(k, v)| k == "grant_type" && v == "refresh_token")
            })
            .returning(|_, _| Ok((400, "invalid_grant".to_string())))
            .times(1);

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let result = service.refresh_token("bad-refresh", "test-client").await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[test]
    fn test_encode_decode_state_roundtrip() {
        let state = CallbackState {
            redirect_uri: "https://app.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("original".to_string()),
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert_eq!(decoded.redirect_uri, state.redirect_uri);
        assert_eq!(decoded.client_id, state.client_id);
        assert_eq!(decoded.original_state, state.original_state);
    }

    #[test]
    fn test_decode_state_invalid_json() {
        let invalid_json_base64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("not json at all");
        let result = decode_state(Some(&invalid_json_base64));
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_build_authorize_url_client_not_found() {
        let config = create_test_keycloak_config();

        let mut service_repo = MockServiceRepository::new();
        service_repo
            .expect_find_by_client_id()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(service_repo),
            config,
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "nonexistent".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        let result = service.build_authorize_url(&params).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_build_authorize_url_without_nonce() {
        let config = create_test_keycloak_config();

        let mut service_repo = MockServiceRepository::new();
        service_repo
            .expect_find_by_client_id()
            .with(eq("my-app"))
            .returning(|_| {
                Ok(Some(create_test_service(vec![
                    "https://app.example.com/callback".to_string(),
                ])))
            });

        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(service_repo),
            config,
        );

        let params = AuthorizeParams {
            response_type: "code".to_string(),
            client_id: "my-app".to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scope: "openid".to_string(),
            state: None,
            nonce: None,
        };

        let url = service.build_authorize_url(&params).await.unwrap();
        assert!(!url.contains("nonce="));
        assert!(url.contains("response_type=code"));
    }

    #[tokio::test]
    async fn test_build_logout_url_with_all_params() {
        let config = create_test_keycloak_config();
        let service = build_service(
            Arc::new(MockKeycloakOidcAdminApi::new()),
            Arc::new(MockOidcHttpClient::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let url = service
            .build_logout_url(
                Some("id-token-hint-value"),
                Some("https://app.com/post-logout"),
                Some("logout-state"),
            )
            .unwrap();

        assert!(url.contains("id_token_hint=id-token-hint-value"));
        assert!(url.contains("post_logout_redirect_uri="));
        assert!(url.contains("state=logout-state"));
    }

    #[tokio::test]
    async fn test_handle_callback_new_user_created() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("client-uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-access",
                        "refresh_token": "kc-refresh"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "new-kc-user-id",
                        "email": "newuser@example.com",
                        "name": "New User"
                    })
                    .to_string(),
                ))
            });

        let new_user = create_test_user("new-kc-user-id", "newuser@example.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .with(eq("new-kc-user-id"))
            .returning(|_| Ok(None));
        user_repo
            .expect_create()
            .returning(move |_, _| Ok(new_user.clone()));

        let mut service_repo = MockServiceRepository::new();
        let test_service = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(StringUuid::new_v4()),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec![],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .with(eq("test-client"))
            .returning(move |_| Ok(Some(test_service.clone())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            config,
        );

        let state_payload = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state_payload).unwrap();

        let result = service
            .handle_callback("auth-code", Some(&encoded))
            .await
            .unwrap();

        assert!(!result.identity_token.is_empty());
        // No original_state, so "state=" should not appear
        assert!(!result.redirect_url.contains("state="));
    }

    #[tokio::test]
    async fn test_handle_callback_token_exchange_http_error() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| Ok((401, "unauthorized".to_string())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("code", Some(&encoded)).await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_token_exchange_network_error() {
        let config = create_test_keycloak_config();

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .returning(|_, _| Err("connection refused".to_string()));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("code", Some(&encoded)).await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_userinfo_http_error() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "tok",
                        "refresh_token": "ref"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| Ok((401, "token expired".to_string())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("code", Some(&encoded)).await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_handle_callback_userinfo_parse_error() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "tok",
                        "refresh_token": "ref"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| Ok((200, "not json".to_string())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("code", Some(&encoded)).await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_exchange_authorization_code_existing_user() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-tok",
                        "refresh_token": "kc-ref",
                        "id_token": "kc-id"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "user-sub",
                        "email": "user@test.com",
                        "name": "User"
                    })
                    .to_string(),
                ))
            });

        let user = create_test_user("user-sub", "user@test.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(user.clone())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let result = service
            .exchange_authorization_code("auth-code", "client-id", "https://app.com/cb")
            .await
            .unwrap();

        assert!(!result.access_token.is_empty());
        assert_eq!(result.token_type, "Bearer");
        assert_eq!(result.expires_in, 3600);
        assert_eq!(result.refresh_token, Some("kc-ref".to_string()));
        assert_eq!(result.id_token, Some("kc-id".to_string()));
    }

    #[tokio::test]
    async fn test_exchange_authorization_code_new_user() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-tok"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "new-sub",
                        "email": "new@test.com"
                    })
                    .to_string(),
                ))
            });

        let new_user = create_test_user("new-sub", "new@test.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None));
        user_repo
            .expect_create()
            .returning(move |_, _| Ok(new_user.clone()));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let result = service
            .exchange_authorization_code("code", "cid", "https://app.com/cb")
            .await
            .unwrap();

        assert!(!result.access_token.is_empty());
        assert!(result.refresh_token.is_none());
        assert!(result.id_token.is_none());
    }

    #[tokio::test]
    async fn test_refresh_token_success_existing_user() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, params| {
                url == token_url.as_str()
                    && params
                        .iter()
                        .any(|(k, v)| k == "grant_type" && v == "refresh_token")
            })
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "new-access",
                        "refresh_token": "new-refresh"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "user-sub",
                        "email": "user@test.com",
                        "name": "User"
                    })
                    .to_string(),
                ))
            });

        let user = create_test_user("user-sub", "user@test.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(user.clone())));

        let mut service_repo = MockServiceRepository::new();
        let test_service = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(StringUuid::new_v4()),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec![],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .with(eq("test-client"))
            .returning(move |_| Ok(Some(test_service.clone())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            config,
        );

        let result = service
            .refresh_token("valid-refresh-token", "test-client")
            .await
            .unwrap();

        assert!(!result.access_token.is_empty());
        assert_eq!(result.token_type, "Bearer");
        assert_eq!(result.refresh_token, Some("new-refresh".to_string()));
    }

    #[tokio::test]
    async fn test_refresh_token_new_user() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "new-access"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "new-sub",
                        "email": "new@test.com"
                    })
                    .to_string(),
                ))
            });

        let new_user = create_test_user("new-sub", "new@test.com");
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None));
        user_repo
            .expect_create()
            .returning(move |_, _| Ok(new_user.clone()));

        let mut service_repo = MockServiceRepository::new();
        let test_service = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(StringUuid::new_v4()),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec![],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .with(eq("client-id"))
            .returning(move |_| Ok(Some(test_service.clone())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            config,
        );

        let result = service
            .refresh_token("refresh-tok", "client-id")
            .await
            .unwrap();

        assert!(!result.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_refresh_token_network_error() {
        let config = create_test_keycloak_config();

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .returning(|_, _| Err("network error".to_string()));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let result = service.refresh_token("tok", "cid").await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[tokio::test]
    async fn test_exchange_code_malformed_json() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .returning(|_| Ok("uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .returning(|_| Ok("secret".to_string()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| Ok((200, "not-json".to_string())));

        let service = build_service(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockServiceRepository::new()),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("code", Some(&encoded)).await;
        assert!(matches!(result, Err(AppError::Keycloak(_))));
    }

    #[test]
    fn test_decode_state_missing() {
        let result = decode_state(None);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_decode_state_invalid_base64() {
        let result = decode_state(Some("!!!invalid-base64!!!"));
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_encode_decode_state_without_original_state() {
        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
            client_id: "c".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();
        assert_eq!(decoded.redirect_uri, "https://app.com/cb");
        assert!(decoded.original_state.is_none());
    }

    // ============================================================
    // Action Trigger Tests
    // ============================================================

    #[tokio::test]
    async fn test_pre_user_registration_blocks_registration() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid"))
            .returning(|_| Ok("client-secret".to_string()));

        let tenant_id = StringUuid::new_v4();
        let mut service_repo = MockServiceRepository::new();
        let service_with_tenant = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(tenant_id),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec!["https://app.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .returning(move |_| Ok(Some(service_with_tenant.clone())));

        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None)); // User doesn't exist, will create

        // Action that blocks registration (throws error)
        let mut action_repo = MockActionRepository::new();
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("pre-user-registration"), eq(true))
            .returning(move |_, _, _| {
                let script = r#"throw new Error("Email domain not allowed");"#;
                Ok(vec![create_test_action("pre-user-registration", script, tenant_id)])
            });
        action_repo
            .expect_record_execution()
            .returning(|_, _, _, _, _, _, _| Ok(()));
        action_repo
            .expect_update_execution_stats()
            .returning(|_, _, _| Ok(()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-access",
                        "refresh_token": "kc-refresh"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "keycloak-user-123",
                        "email": "newuser@blocked-domain.com",
                        "name": "New User"
                    })
                    .to_string(),
                ))
            });

        let service = build_service_with_actions(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            Arc::new(action_repo),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        // PreUserRegistration should block registration
        let result = service.handle_callback("authcode", Some(&encoded)).await;
        assert!(result.is_err());
        // Verify it's an action error
        match result {
            Err(AppError::ActionExecutionFailed(_)) => {
                // Expected: action threw an error that blocked registration
            }
            other => panic!("Expected ActionExecutionFailed error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_post_user_registration_executes_successfully() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid"))
            .returning(|_| Ok("client-secret".to_string()));

        let tenant_id = StringUuid::new_v4();
        let mut service_repo = MockServiceRepository::new();
        let service_with_tenant = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(tenant_id),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec!["https://app.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .returning(move |_| Ok(Some(service_with_tenant.clone())));

        let new_user = create_test_user("keycloak-user-123", "newuser@example.com");
        let new_user_clone = new_user.clone();
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(|_| Ok(None)); // User doesn't exist
        user_repo
            .expect_create()
            .returning(move |_, _| Ok(new_user_clone.clone()));

        // Actions that execute successfully
        let mut action_repo = MockActionRepository::new();

        // PreUserRegistration - passes
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("pre-user-registration"), eq(true))
            .returning(move |_, _, _| {
                let script = r#"context;"#; // Just return context unchanged
                Ok(vec![create_test_action("pre-user-registration", script, tenant_id)])
            });

        // PostUserRegistration - executes
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("post-user-registration"), eq(true))
            .returning(move |_, _, _| {
                let script = r#"console.log('User registered successfully'); context;"#;
                Ok(vec![create_test_action("post-user-registration", script, tenant_id)])
            });

        // PostLogin - no actions
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("post-login"), eq(true))
            .returning(|_, _, _| Ok(vec![]));

        action_repo
            .expect_record_execution()
            .returning(|_, _, _, _, _, _, _| Ok(()));
        action_repo
            .expect_update_execution_stats()
            .returning(|_, _, _| Ok(()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-access",
                        "refresh_token": "kc-refresh",
                        "id_token": "kc-id"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "keycloak-user-123",
                        "email": "newuser@example.com",
                        "name": "New User"
                    })
                    .to_string(),
                ))
            });

        let service = build_service_with_actions(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            Arc::new(action_repo),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        // Should succeed and trigger PostUserRegistration
        let result = service.handle_callback("authcode", Some(&encoded)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_login_modifies_claims() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid"))
            .returning(|_| Ok("client-secret".to_string()));

        let tenant_id = StringUuid::new_v4();
        let mut service_repo = MockServiceRepository::new();
        let service_with_tenant = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(tenant_id),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec!["https://app.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .returning(move |_| Ok(Some(service_with_tenant.clone())));

        let existing_user = create_test_user("keycloak-user-456", "existinguser@example.com");
        let existing_user_clone = existing_user.clone();
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(existing_user_clone.clone())));

        // PostLogin action that adds custom claims
        let mut action_repo = MockActionRepository::new();
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("post-login"), eq(true))
            .returning(move |_, _, _| {
                let script = r#"
                    context.claims = context.claims || {};
                    context.claims.department = "engineering";
                    context.claims.tier = "premium";
                    context;
                "#;
                Ok(vec![create_test_action("post-login", script, tenant_id)])
            });
        action_repo
            .expect_record_execution()
            .returning(|_, _, _, _, _, _, _| Ok(()));
        action_repo
            .expect_update_execution_stats()
            .returning(|_, _, _| Ok(()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, _| url == token_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "kc-access",
                        "refresh_token": "kc-refresh",
                        "id_token": "kc-id"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "keycloak-user-456",
                        "email": "existinguser@example.com",
                        "name": "Existing User"
                    })
                    .to_string(),
                ))
            });

        let service = build_service_with_actions(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            Arc::new(action_repo),
            config,
        );

        let state = CallbackState {
            redirect_uri: "https://app.example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };
        let encoded = encode_state(&state).unwrap();

        let result = service.handle_callback("authcode", Some(&encoded)).await;
        assert!(result.is_ok());

        // Decode the identity token to verify custom claims
        let callback_result = result.unwrap();
        let jwt_manager = create_test_jwt_manager();
        let claims = jwt_manager.verify_identity_token(&callback_result.identity_token).unwrap();

        // Verify custom claims were added
        let extra = claims.extra.as_ref().expect("extra claims should be present");
        assert!(extra.contains_key("department"));
        assert_eq!(extra.get("department").unwrap(), "engineering");
        assert!(extra.contains_key("tier"));
        assert_eq!(extra.get("tier").unwrap(), "premium");
    }

    #[tokio::test]
    async fn test_pre_token_refresh_blocks_refresh() {
        let config = create_test_keycloak_config();
        let token_url = token_url(&config);
        let userinfo_url = userinfo_url(&config);

        let mut keycloak = MockKeycloakOidcAdminApi::new();
        keycloak
            .expect_get_client_uuid_by_client_id()
            .with(eq("test-client"))
            .returning(|_| Ok("client-uuid".to_string()));
        keycloak
            .expect_get_client_secret()
            .with(eq("client-uuid"))
            .returning(|_| Ok("client-secret".to_string()));

        let tenant_id = StringUuid::new_v4();
        let mut service_repo = MockServiceRepository::new();
        let service_with_tenant = Service {
            id: StringUuid::new_v4(),
            tenant_id: Some(tenant_id),
            name: "Test Service".to_string(),
            base_url: Some("https://app.example.com".to_string()),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: vec!["https://app.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        service_repo
            .expect_find_by_client_id()
            .returning(move |_| Ok(Some(service_with_tenant.clone())));

        let existing_user = create_test_user("keycloak-user-789", "user@example.com");
        let existing_user_clone = existing_user.clone();
        let mut user_repo = MockUserRepository::new();
        user_repo
            .expect_find_by_keycloak_id()
            .returning(move |_| Ok(Some(existing_user_clone.clone())));

        // PreTokenRefresh action that blocks refresh
        let mut action_repo = MockActionRepository::new();
        action_repo
            .expect_list_by_trigger()
            .with(eq(tenant_id), eq("pre-token-refresh"), eq(true))
            .returning(move |_, _, _| {
                let script = r#"throw new Error("User account suspended");"#;
                Ok(vec![create_test_action("pre-token-refresh", script, tenant_id)])
            });
        action_repo
            .expect_record_execution()
            .returning(|_, _, _, _, _, _, _| Ok(()));
        action_repo
            .expect_update_execution_stats()
            .returning(|_, _, _| Ok(()));

        let mut http = MockOidcHttpClient::new();
        http.expect_post_form()
            .withf(move |url, params| {
                url == token_url.as_str()
                    && params
                        .iter()
                        .any(|(k, v)| k == "grant_type" && v == "refresh_token")
            })
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "access_token": "new-access",
                        "refresh_token": "new-refresh"
                    })
                    .to_string(),
                ))
            });
        http.expect_get_bearer()
            .withf(move |url, _| url == userinfo_url.as_str())
            .returning(|_, _| {
                Ok((
                    200,
                    serde_json::json!({
                        "sub": "keycloak-user-789",
                        "email": "user@example.com",
                        "name": "User"
                    })
                    .to_string(),
                ))
            });

        let service = build_service_with_actions(
            Arc::new(keycloak),
            Arc::new(http),
            Arc::new(user_repo),
            Arc::new(service_repo),
            Arc::new(action_repo),
            config,
        );

        // PreTokenRefresh should block token refresh
        let result = service.refresh_token("old-refresh-token", "test-client").await;
        assert!(result.is_err());
        match result {
            Err(AppError::ActionExecutionFailed(_)) => {
                // Expected: action threw an error that blocked refresh
            }
            other => panic!("Expected ActionExecutionFailed error, got: {:?}", other),
        }
    }
}
