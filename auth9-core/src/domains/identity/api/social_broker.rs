//! Social login broker handlers.
//!
//! Executes the full OAuth2 redirect→callback→token-exchange→profile-mapping flow
//! for social login providers (Google, GitHub, Microsoft, generic OIDC).

use crate::cache::CacheOperations;
use crate::domains::identity::api::auth::helpers::{
    AuthorizationCodeData, LoginChallengeData, AUTH_CODE_TTL_SECS,
};
use crate::domains::security_observability::service::analytics::FederationEventMetadata;
use crate::error::{AppError, Result};
use crate::http_support::SuccessResponse;
use crate::models::linked_identity::{CreateLinkedIdentityInput, PendingMergeData};
use crate::state::{HasAnalytics, HasCache, HasIdentityProviders, HasServices, HasSessionManagement};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use url::Url;
use utoipa::ToSchema;

/// Social login state TTL (10 minutes)
const SOCIAL_STATE_TTL_SECS: u64 = 600;

// ── Data Structures ──

#[derive(Debug, Serialize, Deserialize)]
struct SocialLoginState {
    login_challenge_id: String,
    provider_alias: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocialLinkState {
    user_id: String,
    provider_alias: String,
}

#[derive(Debug, Clone)]
struct SocialProfile {
    external_user_id: String,
    email: Option<String>,
    name: Option<String>,
    #[allow(dead_code)]
    avatar_url: Option<String>,
}

/// Public representation of a social provider (no secrets).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicSocialProvider {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_id: String,
}

/// OAuth2 endpoints resolved for a provider.
struct OAuthEndpoints {
    authorization_url: String,
    token_url: String,
    userinfo_url: String,
    scopes: String,
}

#[derive(Debug, Deserialize)]
pub struct SocialAuthorizeQuery {
    pub login_challenge: String,
}

#[derive(Debug, Deserialize)]
pub struct SocialCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

// ── Provider Endpoint Resolution ──

fn resolve_endpoints(
    provider_type: &str,
    config: &std::collections::HashMap<String, String>,
) -> Result<OAuthEndpoints> {
    match provider_type {
        "google" => Ok(OAuthEndpoints {
            authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".to_string(),
            scopes: "openid email profile".to_string(),
        }),
        "github" => Ok(OAuthEndpoints {
            authorization_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_url: "https://api.github.com/user".to_string(),
            scopes: "read:user user:email".to_string(),
        }),
        "microsoft" => {
            let tenant = config.get("tenant").map(|s| s.as_str()).unwrap_or("common");
            Ok(OAuthEndpoints {
                authorization_url: format!(
                    "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize",
                    tenant
                ),
                token_url: format!(
                    "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
                    tenant
                ),
                userinfo_url: "https://graph.microsoft.com/oidc/userinfo".to_string(),
                scopes: "openid email profile".to_string(),
            })
        }
        "oidc" => {
            let auth_url = config
                .get("authorizationUrl")
                .ok_or_else(|| AppError::BadRequest("Missing authorizationUrl in OIDC config".to_string()))?
                .clone();
            let token_url = config
                .get("tokenUrl")
                .ok_or_else(|| AppError::BadRequest("Missing tokenUrl in OIDC config".to_string()))?
                .clone();
            let userinfo_url = config
                .get("userInfoUrl")
                .ok_or_else(|| AppError::BadRequest("Missing userInfoUrl in OIDC config".to_string()))?
                .clone();
            let scopes = config
                .get("scopes")
                .cloned()
                .unwrap_or_else(|| "openid email profile".to_string());

            Ok(OAuthEndpoints {
                authorization_url: auth_url,
                token_url,
                userinfo_url,
                scopes,
            })
        }
        _ => Err(AppError::BadRequest(format!(
            "Unsupported social provider type: {}",
            provider_type
        ))),
    }
}

// ── Profile Mapping ──

fn map_profile(provider_type: &str, json: &serde_json::Value) -> Result<SocialProfile> {
    match provider_type {
        "google" | "oidc" => Ok(SocialProfile {
            external_user_id: json["sub"]
                .as_str()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing 'sub' in userinfo")))?
                .to_string(),
            email: json["email"].as_str().map(String::from),
            name: json["name"].as_str().map(String::from),
            avatar_url: json["picture"].as_str().map(String::from),
        }),
        "github" => {
            let id = if let Some(id_num) = json["id"].as_i64() {
                id_num.to_string()
            } else {
                json["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing 'id' in GitHub userinfo")))?
                    .to_string()
            };
            Ok(SocialProfile {
                external_user_id: id,
                email: json["email"].as_str().map(String::from),
                name: json["name"]
                    .as_str()
                    .or(json["login"].as_str())
                    .map(String::from),
                avatar_url: json["avatar_url"].as_str().map(String::from),
            })
        }
        "microsoft" => Ok(SocialProfile {
            external_user_id: json["sub"]
                .as_str()
                .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing 'sub' in MS userinfo")))?
                .to_string(),
            email: json["email"].as_str().map(String::from),
            name: json["name"].as_str().map(String::from),
            avatar_url: None,
        }),
        _ => Ok(SocialProfile {
            external_user_id: json["sub"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            email: json["email"].as_str().map(String::from),
            name: json["name"].as_str().map(String::from),
            avatar_url: json["picture"].as_str().map(String::from),
        }),
    }
}

/// Fetch GitHub primary email via /user/emails when main profile has no email.
async fn fetch_github_primary_email(access_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct GithubEmail {
        email: String,
        primary: bool,
        verified: bool,
    }

    let resp = reqwest::Client::new()
        .get("https://api.github.com/user/emails")
        .bearer_auth(access_token)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "auth9-core")
        .send()
        .await
        .ok()?;

    let emails: Vec<GithubEmail> = resp.json().await.ok()?;
    emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
}

// ── Token Exchange ──

async fn exchange_code_for_access_token(
    provider_type: &str,
    endpoints: &OAuthEndpoints,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String> {
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let mut builder = reqwest::Client::new().post(&endpoints.token_url);

    // GitHub requires Accept: application/json header
    if provider_type == "github" {
        builder = builder.header("Accept", "application/json");
        // GitHub doesn't use grant_type in OAuth2
        params.retain(|&(k, _)| k != "grant_type");
    }

    let response = builder
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Token exchange failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!(
            "Token exchange failed ({}): {}",
            status,
            body
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse token response: {}", e)))?;

    body["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "No access_token in token response: {}",
                body
            ))
        })
}

async fn fetch_userinfo(
    provider_type: &str,
    endpoints: &OAuthEndpoints,
    access_token: &str,
) -> Result<serde_json::Value> {
    let mut builder = reqwest::Client::new()
        .get(&endpoints.userinfo_url)
        .bearer_auth(access_token);

    if provider_type == "github" {
        builder = builder.header("User-Agent", "auth9-core");
    }

    let response = builder
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Userinfo fetch failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!(
            "Userinfo fetch failed ({}): {}",
            status,
            body
        )));
    }

    response
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse userinfo: {}", e)))
}

// ── Build authorize URL ──

fn build_social_authorize_url(
    endpoints: &OAuthEndpoints,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
) -> Result<String> {
    let mut url = Url::parse(&endpoints.authorization_url)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid authorization URL: {}", e)))?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("client_id", client_id);
        pairs.append_pair("redirect_uri", redirect_uri);
        pairs.append_pair("response_type", "code");
        pairs.append_pair("scope", &endpoints.scopes);
        pairs.append_pair("state", state);
    }

    Ok(url.to_string())
}

fn social_callback_url(config: &crate::config::Config) -> String {
    let base = config
        .keycloak
        .core_public_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!("{}/api/v1/social-login/callback", base.trim_end_matches('/'))
}

fn social_link_callback_url(config: &crate::config::Config) -> String {
    let base = config
        .keycloak
        .core_public_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!(
        "{}/api/v1/social-login/link/callback",
        base.trim_end_matches('/')
    )
}

fn portal_login_url(config: &crate::config::Config) -> String {
    let portal = config
        .keycloak
        .portal_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!("{}/login", portal.trim_end_matches('/'))
}

fn portal_identities_url(config: &crate::config::Config) -> String {
    let portal = config
        .keycloak
        .portal_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!(
        "{}/dashboard/account/identities",
        portal.trim_end_matches('/')
    )
}

// ══════════════════════════════════════════════════════════════════════
// Handlers
// ══════════════════════════════════════════════════════════════════════

/// List enabled social providers (public, no auth, no secrets).
#[utoipa::path(
    get,
    path = "/api/v1/social-login/providers",
    tag = "Identity",
    responses((status = 200, description = "Enabled social providers"))
)]
pub async fn list_enabled_providers<S: HasIdentityProviders>(
    State(state): State<S>,
) -> Result<Json<SuccessResponse<Vec<PublicSocialProvider>>>> {
    let providers = state.identity_provider_service().list_providers().await?;
    let public: Vec<PublicSocialProvider> = providers
        .into_iter()
        .filter(|p| p.enabled && !p.link_only)
        .map(|p| PublicSocialProvider {
            alias: p.alias,
            display_name: p.display_name,
            provider_id: p.provider_id,
        })
        .collect();
    Ok(Json(SuccessResponse::new(public)))
}

/// Initiate social login: validate login_challenge, redirect to provider.
#[utoipa::path(
    get,
    path = "/api/v1/social-login/authorize/{alias}",
    tag = "Identity",
    responses((status = 302, description = "Redirect to social provider"))
)]
pub async fn authorize<S: HasIdentityProviders + HasServices + HasCache>(
    State(state): State<S>,
    Path(alias): Path<String>,
    Query(params): Query<SocialAuthorizeQuery>,
) -> Result<Response> {
    // 1. Verify login_challenge exists (peek, do NOT consume)
    let challenge_json = state
        .cache()
        .consume_login_challenge(&params.login_challenge)
        .await?;
    let challenge_json = challenge_json.ok_or_else(|| {
        AppError::BadRequest("Invalid or expired login challenge".to_string())
    })?;
    // Re-store it immediately (peek pattern)
    state
        .cache()
        .store_login_challenge(
            &params.login_challenge,
            &challenge_json,
            super::auth::LOGIN_CHALLENGE_TTL_SECS,
        )
        .await?;

    // 2. Look up provider
    let provider = state
        .identity_provider_service()
        .get_provider(&alias)
        .await?;
    if !provider.enabled {
        return Err(AppError::BadRequest(format!(
            "Social provider '{}' is not enabled",
            alias
        )));
    }

    let endpoints = resolve_endpoints(&provider.provider_id, &provider.config)?;

    // 3. Store social login state
    let social_state_id = uuid::Uuid::new_v4().to_string();
    let social_state = SocialLoginState {
        login_challenge_id: params.login_challenge,
        provider_alias: alias,
    };
    let social_state_json =
        serde_json::to_string(&social_state).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_social_login_state(&social_state_id, &social_state_json, SOCIAL_STATE_TTL_SECS)
        .await?;

    // 4. Build authorize URL
    let client_id = provider
        .config
        .get("clientId")
        .ok_or_else(|| AppError::BadRequest("Missing clientId in provider config".to_string()))?;
    let redirect_uri = social_callback_url(state.config());

    let authorize_url =
        build_social_authorize_url(&endpoints, client_id, &redirect_uri, &social_state_id)?;

    metrics::counter!("auth9_social_login_total", "action" => "authorize", "provider" => provider.provider_id.clone())
        .increment(1);

    Ok(Redirect::temporary(&authorize_url).into_response())
}

/// Social provider callback: exchange code, find/create user, complete login challenge.
#[utoipa::path(
    get,
    path = "/api/v1/social-login/callback",
    tag = "Identity",
    responses((status = 302, description = "Redirect with authorization code"))
)]
pub async fn callback<
    S: HasServices + HasIdentityProviders + HasCache + HasSessionManagement + HasAnalytics,
>(
    State(state): State<S>,
    Query(params): Query<SocialCallbackQuery>,
) -> Result<Response> {
    // 1. Check for error from provider
    if let Some(ref error) = params.error {
        tracing::warn!("Social login error from provider: {}", error);
        let login_url = portal_login_url(state.config());
        return Ok(
            Redirect::temporary(&format!("{}?error=social_login_cancelled", login_url))
                .into_response(),
        );
    }

    let code = params
        .code
        .ok_or_else(|| AppError::BadRequest("Missing code parameter".to_string()))?;
    let state_id = params
        .state
        .ok_or_else(|| AppError::BadRequest("Missing state parameter".to_string()))?;

    // 2. Consume social login state
    let social_state_json = state
        .cache()
        .consume_social_login_state(&state_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired social login state".to_string()))?;
    let social_state: SocialLoginState =
        serde_json::from_str(&social_state_json).map_err(|e| AppError::Internal(e.into()))?;

    // 3. Look up provider
    let provider = state
        .identity_provider_service()
        .get_provider(&social_state.provider_alias)
        .await?;

    let endpoints = resolve_endpoints(&provider.provider_id, &provider.config)?;
    let client_id = provider
        .config
        .get("clientId")
        .ok_or_else(|| AppError::BadRequest("Missing clientId in provider config".to_string()))?;
    let client_secret = provider
        .config
        .get("clientSecret")
        .ok_or_else(|| {
            AppError::BadRequest("Missing clientSecret in provider config".to_string())
        })?;

    // 4. Exchange code for access token
    let redirect_uri = social_callback_url(state.config());
    let access_token = exchange_code_for_access_token(
        &provider.provider_id,
        &endpoints,
        client_id,
        client_secret,
        &code,
        &redirect_uri,
    )
    .await?;

    // 5. Fetch userinfo
    let userinfo_json =
        fetch_userinfo(&provider.provider_id, &endpoints, &access_token).await?;

    // 6. Map profile
    let mut profile = map_profile(&provider.provider_id, &userinfo_json)?;

    // GitHub special case: fetch email separately if not present
    if provider.provider_id == "github" && profile.email.is_none() {
        profile.email = fetch_github_primary_email(&access_token).await;
    }

    // 7. Find or create user
    let resolution =
        find_or_create_user(&state, &provider, &profile, &social_state.login_challenge_id).await?;

    // Handle pending merge: redirect to portal confirm-link page
    let user = match resolution {
        UserResolution::Found(user) => user,
        UserResolution::PendingMerge(pending) => {
            let token = uuid::Uuid::new_v4().to_string();
            let pending_json =
                serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.into()))?;
            state
                .cache()
                .store_pending_merge(&token, &pending_json, SOCIAL_STATE_TTL_SECS)
                .await?;
            let portal_base = state
                .config()
                .keycloak
                .portal_url
                .as_deref()
                .unwrap_or(&state.config().jwt.issuer);
            let redirect_url = format!(
                "{}/login/confirm-link?token={}",
                portal_base.trim_end_matches('/'),
                token
            );
            return Ok(Redirect::temporary(&redirect_url).into_response());
        }
    };

    // 8. Create session
    let session = state
        .session_service()
        .create_session(user.id, None, None, None)
        .await?;

    // 9. Consume login challenge and generate authorization code
    let challenge_json = state
        .cache()
        .consume_login_challenge(&social_state.login_challenge_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Login challenge expired during social login".to_string())
        })?;
    let challenge: LoginChallengeData =
        serde_json::from_str(&challenge_json).map_err(|e| AppError::Internal(e.into()))?;

    let auth_code = uuid::Uuid::new_v4().to_string();
    let code_data = AuthorizationCodeData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        session_id: session.id.to_string(),
        client_id: challenge.client_id.clone(),
        redirect_uri: challenge.redirect_uri.clone(),
        scope: challenge.scope,
        nonce: challenge.nonce,
        code_challenge: challenge.code_challenge,
        code_challenge_method: challenge.code_challenge_method,
    };
    let code_json =
        serde_json::to_string(&code_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_authorization_code(&auth_code, &code_json, AUTH_CODE_TTL_SECS)
        .await?;

    // 10. Build redirect URL
    let mut redirect_url = Url::parse(&challenge.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;
    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &auth_code);
        if let Some(original_state) = challenge.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    metrics::counter!("auth9_social_login_total", "action" => "callback_success", "provider" => provider.provider_id.clone())
        .increment(1);

    // Record federation login event
    let fed_meta = FederationEventMetadata {
        user_id: Some(user.id),
        email: Some(user.email.clone()),
        tenant_id: None,
        provider_alias: social_state.provider_alias,
        provider_type: provider.provider_id.clone(),
        ip_address: None,
        user_agent: None,
        session_id: Some(session.id),
    };
    if let Err(e) = state.analytics_service().record_federation_login(fed_meta).await {
        tracing::warn!("Failed to record federation login event: {}", e);
    }

    let mut response = Redirect::temporary(redirect_url.as_str()).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        "no-store".parse().unwrap(),
    );
    Ok(response)
}

/// Initiate social account linking (protected, requires JWT).
#[utoipa::path(
    get,
    path = "/api/v1/social-login/link/{alias}",
    tag = "Identity",
    responses((status = 302, description = "Redirect to social provider for account linking"))
)]
pub async fn link_authorize<S: HasIdentityProviders + HasServices + HasCache>(
    State(state): State<S>,
    Path(alias): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let user_id = super::identity_provider::extract_user_id(&state, &headers)?;

    let provider = state
        .identity_provider_service()
        .get_provider(&alias)
        .await?;
    if !provider.enabled {
        return Err(AppError::BadRequest(format!(
            "Social provider '{}' is not enabled",
            alias
        )));
    }

    let endpoints = resolve_endpoints(&provider.provider_id, &provider.config)?;
    let client_id = provider
        .config
        .get("clientId")
        .ok_or_else(|| AppError::BadRequest("Missing clientId in provider config".to_string()))?;

    // Store link state
    let social_state_id = uuid::Uuid::new_v4().to_string();
    let link_state = SocialLinkState {
        user_id: user_id.to_string(),
        provider_alias: alias,
    };
    let link_state_json =
        serde_json::to_string(&link_state).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_social_login_state(&social_state_id, &link_state_json, SOCIAL_STATE_TTL_SECS)
        .await?;

    let redirect_uri = social_link_callback_url(state.config());
    let authorize_url =
        build_social_authorize_url(&endpoints, client_id, &redirect_uri, &social_state_id)?;

    Ok(Redirect::temporary(&authorize_url).into_response())
}

/// Social account linking callback.
#[utoipa::path(
    get,
    path = "/api/v1/social-login/link/callback",
    tag = "Identity",
    responses((status = 302, description = "Redirect to account identities page"))
)]
pub async fn link_callback<S: HasIdentityProviders + HasServices + HasCache + HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<SocialCallbackQuery>,
) -> Result<Response> {
    if params.error.is_some() {
        let identities_url = portal_identities_url(state.config());
        return Ok(
            Redirect::temporary(&format!("{}?error=link_cancelled", identities_url))
                .into_response(),
        );
    }

    let code = params
        .code
        .ok_or_else(|| AppError::BadRequest("Missing code parameter".to_string()))?;
    let state_id = params
        .state
        .ok_or_else(|| AppError::BadRequest("Missing state parameter".to_string()))?;

    // Consume link state
    let link_state_json = state
        .cache()
        .consume_social_login_state(&state_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired link state".to_string()))?;
    let link_state: SocialLinkState =
        serde_json::from_str(&link_state_json).map_err(|e| AppError::Internal(e.into()))?;

    let provider = state
        .identity_provider_service()
        .get_provider(&link_state.provider_alias)
        .await?;

    let endpoints = resolve_endpoints(&provider.provider_id, &provider.config)?;
    let client_id = provider
        .config
        .get("clientId")
        .ok_or_else(|| AppError::BadRequest("Missing clientId in provider config".to_string()))?;
    let client_secret = provider
        .config
        .get("clientSecret")
        .ok_or_else(|| {
            AppError::BadRequest("Missing clientSecret in provider config".to_string())
        })?;

    let redirect_uri = social_link_callback_url(state.config());
    let access_token = exchange_code_for_access_token(
        &provider.provider_id,
        &endpoints,
        client_id,
        client_secret,
        &code,
        &redirect_uri,
    )
    .await?;

    let userinfo_json =
        fetch_userinfo(&provider.provider_id, &endpoints, &access_token).await?;
    let mut profile = map_profile(&provider.provider_id, &userinfo_json)?;

    if provider.provider_id == "github" && profile.email.is_none() {
        profile.email = fetch_github_primary_email(&access_token).await;
    }

    // Create linked identity
    let user_id = crate::models::common::StringUuid::parse_str(&link_state.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id in link state".to_string()))?;

    let input = CreateLinkedIdentityInput {
        user_id,
        provider_type: provider.provider_id.clone(),
        provider_alias: link_state.provider_alias,
        external_user_id: profile.external_user_id,
        external_email: profile.email,
    };

    // Create linked identity record
    let _ = state
        .identity_provider_service()
        .create_linked_identity(&input)
        .await;

    // Record identity linked event
    if let Err(e) = state
        .analytics_service()
        .record_identity_linked(user_id, &provider.alias, &provider.provider_id)
        .await
    {
        tracing::warn!("Failed to record identity linked event: {}", e);
    }

    let identities_url = portal_identities_url(state.config());
    Ok(Redirect::temporary(&identities_url).into_response())
}

// ── User Resolution ──

/// Result of user resolution: either a user or a pending merge that needs confirmation
enum UserResolution {
    Found(crate::models::user::User),
    PendingMerge(PendingMergeData),
}

async fn find_or_create_user<
    S: HasServices + HasIdentityProviders,
>(
    state: &S,
    provider: &crate::models::identity_provider::IdentityProvider,
    profile: &SocialProfile,
    login_challenge_id: &str,
) -> Result<UserResolution> {
    use crate::models::linked_identity::FirstLoginPolicy;

    // Try to find existing linked identity
    let existing_link = state
        .identity_provider_service()
        .find_linked_identity(&provider.alias, &profile.external_user_id)
        .await?;

    if let Some(linked) = existing_link {
        let user = state.user_service().get(linked.user_id).await?;
        return Ok(UserResolution::Found(user));
    }

    // If link_only, don't auto-create
    if provider.link_only {
        return Err(AppError::Forbidden(
            "This provider is configured for account linking only. Please link your account first."
                .to_string(),
        ));
    }

    // Determine effective first login policy
    // trust_email=true → auto_merge (backward compat)
    // trust_email=false → use first_login_policy field
    let policy = if provider.trust_email {
        FirstLoginPolicy::AutoMerge
    } else {
        provider
            .first_login_policy
            .parse::<FirstLoginPolicy>()
            .unwrap_or(FirstLoginPolicy::CreateNew)
    };

    // Try email-based matching
    if let Some(ref email) = profile.email {
        if let Ok(existing_user) = state.user_service().get_by_email(email).await {
            match policy {
                FirstLoginPolicy::AutoMerge => {
                    // Auto-link to existing user
                    let input = CreateLinkedIdentityInput {
                        user_id: existing_user.id,
                        provider_type: provider.provider_id.clone(),
                        provider_alias: provider.alias.clone(),
                        external_user_id: profile.external_user_id.clone(),
                        external_email: profile.email.clone(),
                    };
                    let _ = state
                        .identity_provider_service()
                        .create_linked_identity(&input)
                        .await;
                    return Ok(UserResolution::Found(existing_user));
                }
                FirstLoginPolicy::PromptConfirm => {
                    // Store pending merge for user confirmation
                    return Ok(UserResolution::PendingMerge(PendingMergeData {
                        existing_user_id: existing_user.id.to_string(),
                        existing_email: existing_user.email.clone(),
                        external_user_id: profile.external_user_id.clone(),
                        provider_alias: provider.alias.clone(),
                        provider_type: provider.provider_id.clone(),
                        external_email: profile.email.clone(),
                        display_name: profile.name.clone(),
                        login_challenge_id: login_challenge_id.to_string(),
                        tenant_id: None,
                        ip_address: None,
                        user_agent: None,
                    }));
                }
                FirstLoginPolicy::CreateNew => {
                    // Fall through to create new user
                }
            }
        }
    }

    // Create new user
    let email = profile.email.clone().ok_or_else(|| {
        AppError::BadRequest(
            "Social provider did not return an email. Cannot create account.".to_string(),
        )
    })?;

    let identity_subject = uuid::Uuid::new_v4().to_string();
    let create_input = crate::models::user::CreateUserInput {
        email: email.clone(),
        display_name: profile.name.clone(),
        avatar_url: None,
    };
    let new_user = state
        .user_service()
        .create(&identity_subject, create_input)
        .await?;

    // Create linked identity
    let input = CreateLinkedIdentityInput {
        user_id: new_user.id,
        provider_type: provider.provider_id.clone(),
        provider_alias: provider.alias.clone(),
        external_user_id: profile.external_user_id.clone(),
        external_email: profile.email.clone(),
    };
    let _ = state
        .identity_provider_service()
        .create_linked_identity(&input)
        .await;

    Ok(UserResolution::Found(new_user))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_endpoints_google() {
        let config = std::collections::HashMap::new();
        let endpoints = resolve_endpoints("google", &config).unwrap();
        assert!(endpoints.authorization_url.contains("accounts.google.com"));
        assert!(endpoints.token_url.contains("googleapis.com"));
        assert_eq!(endpoints.scopes, "openid email profile");
    }

    #[test]
    fn test_resolve_endpoints_github() {
        let config = std::collections::HashMap::new();
        let endpoints = resolve_endpoints("github", &config).unwrap();
        assert!(endpoints.authorization_url.contains("github.com"));
        assert!(endpoints.scopes.contains("user:email"));
    }

    #[test]
    fn test_resolve_endpoints_microsoft_default_tenant() {
        let config = std::collections::HashMap::new();
        let endpoints = resolve_endpoints("microsoft", &config).unwrap();
        assert!(endpoints.authorization_url.contains("/common/"));
    }

    #[test]
    fn test_resolve_endpoints_microsoft_custom_tenant() {
        let mut config = std::collections::HashMap::new();
        config.insert("tenant".to_string(), "my-org".to_string());
        let endpoints = resolve_endpoints("microsoft", &config).unwrap();
        assert!(endpoints.authorization_url.contains("/my-org/"));
        assert!(endpoints.token_url.contains("/my-org/"));
    }

    #[test]
    fn test_resolve_endpoints_oidc() {
        let mut config = std::collections::HashMap::new();
        config.insert(
            "authorizationUrl".to_string(),
            "https://idp.example.com/auth".to_string(),
        );
        config.insert(
            "tokenUrl".to_string(),
            "https://idp.example.com/token".to_string(),
        );
        config.insert(
            "userInfoUrl".to_string(),
            "https://idp.example.com/userinfo".to_string(),
        );
        let endpoints = resolve_endpoints("oidc", &config).unwrap();
        assert_eq!(endpoints.authorization_url, "https://idp.example.com/auth");
    }

    #[test]
    fn test_resolve_endpoints_oidc_missing_url() {
        let config = std::collections::HashMap::new();
        let result = resolve_endpoints("oidc", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_endpoints_unsupported() {
        let config = std::collections::HashMap::new();
        let result = resolve_endpoints("unknown_provider", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_profile_google() {
        let json = serde_json::json!({
            "sub": "google-12345",
            "email": "user@gmail.com",
            "name": "Test User",
            "picture": "https://lh3.googleusercontent.com/photo.jpg"
        });
        let profile = map_profile("google", &json).unwrap();
        assert_eq!(profile.external_user_id, "google-12345");
        assert_eq!(profile.email, Some("user@gmail.com".to_string()));
        assert_eq!(profile.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_map_profile_github_numeric_id() {
        let json = serde_json::json!({
            "id": 12345,
            "login": "testuser",
            "name": "Test User",
            "email": "user@github.com",
            "avatar_url": "https://avatars.githubusercontent.com/u/12345"
        });
        let profile = map_profile("github", &json).unwrap();
        assert_eq!(profile.external_user_id, "12345");
        assert_eq!(profile.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_map_profile_github_no_email() {
        let json = serde_json::json!({
            "id": 12345,
            "login": "testuser",
            "email": null
        });
        let profile = map_profile("github", &json).unwrap();
        assert_eq!(profile.external_user_id, "12345");
        assert!(profile.email.is_none());
        assert_eq!(profile.name, Some("testuser".to_string()));
    }

    #[test]
    fn test_map_profile_microsoft() {
        let json = serde_json::json!({
            "sub": "ms-sub-123",
            "email": "user@outlook.com",
            "name": "MS User"
        });
        let profile = map_profile("microsoft", &json).unwrap();
        assert_eq!(profile.external_user_id, "ms-sub-123");
        assert_eq!(profile.email, Some("user@outlook.com".to_string()));
    }

    #[test]
    fn test_map_profile_oidc() {
        let json = serde_json::json!({
            "sub": "oidc-user-1",
            "email": "user@example.com",
            "name": "OIDC User",
            "picture": "https://example.com/avatar.jpg"
        });
        let profile = map_profile("oidc", &json).unwrap();
        assert_eq!(profile.external_user_id, "oidc-user-1");
    }

    #[test]
    fn test_build_social_authorize_url() {
        let endpoints = OAuthEndpoints {
            authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: String::new(),
            userinfo_url: String::new(),
            scopes: "openid email profile".to_string(),
        };
        let url =
            build_social_authorize_url(&endpoints, "my-client-id", "https://auth9.example.com/api/v1/social-login/callback", "state-123")
                .unwrap();
        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("scope=openid"));
    }

    #[test]
    fn test_social_login_state_roundtrip() {
        let state = SocialLoginState {
            login_challenge_id: "challenge-123".to_string(),
            provider_alias: "google".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let decoded: SocialLoginState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.login_challenge_id, "challenge-123");
        assert_eq!(decoded.provider_alias, "google");
    }

    #[test]
    fn test_social_link_state_roundtrip() {
        let state = SocialLinkState {
            user_id: "user-456".to_string(),
            provider_alias: "github".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let decoded: SocialLinkState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.user_id, "user-456");
    }
}
