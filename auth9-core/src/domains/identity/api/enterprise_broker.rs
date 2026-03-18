//! Enterprise SSO broker handlers (OIDC + SAML dispatch).
//!
//! The `authorize` handler dispatches to OIDC or SAML based on connector provider_type.
//! The OIDC `callback` handles the OAuth2 code exchange flow.
//! SAML uses a separate ACS endpoint in `enterprise_saml_broker`.

use crate::cache::CacheOperations;
use crate::domains::identity::api::enterprise_common::{
    self, ConnectorRecord, EnterpriseSsoLoginState, UserResolution, ENTERPRISE_SSO_STATE_TTL_SECS,
};
use crate::domains::security_observability::service::analytics::FederationEventMetadata;
use crate::error::{AppError, Result};
use crate::state::{HasAnalytics, HasCache, HasDbPool, HasIdentityProviders, HasServices, HasSessionManagement};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use url::Url;

// ── Data Structures ──

struct OAuthEndpoints {
    authorization_url: String,
    token_url: String,
    userinfo_url: String,
    scopes: String,
}

#[derive(Debug, Deserialize)]
pub struct EnterpriseSsoAuthorizeQuery {
    pub login_challenge: String,
    pub login_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnterpriseSsoCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

// ── Endpoint Resolution ──

fn resolve_endpoints(
    config: &std::collections::HashMap<String, String>,
) -> Result<OAuthEndpoints> {
    let authorization_url = config
        .get("authorizationUrl")
        .ok_or_else(|| {
            AppError::BadRequest("Missing authorizationUrl in connector config".to_string())
        })?
        .clone();
    let token_url = config
        .get("tokenUrl")
        .ok_or_else(|| AppError::BadRequest("Missing tokenUrl in connector config".to_string()))?
        .clone();
    let userinfo_url = config
        .get("userInfoUrl")
        .ok_or_else(|| {
            AppError::BadRequest("Missing userInfoUrl in connector config".to_string())
        })?
        .clone();
    let scopes = config
        .get("scopes")
        .cloned()
        .unwrap_or_else(|| "openid email profile".to_string());

    Ok(OAuthEndpoints {
        authorization_url,
        token_url,
        userinfo_url,
        scopes,
    })
}

// ── Profile Mapping ──

fn map_profile(
    config: &std::collections::HashMap<String, String>,
    json: &serde_json::Value,
) -> Result<enterprise_common::EnterpriseProfile> {
    let sub_claim = config.get("claimSub").map(|s| s.as_str()).unwrap_or("sub");
    let email_claim = config
        .get("claimEmail")
        .map(|s| s.as_str())
        .unwrap_or("email");
    let name_claim = config
        .get("claimName")
        .map(|s| s.as_str())
        .unwrap_or("name");

    let external_user_id = json[sub_claim]
        .as_str()
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Missing '{}' claim in userinfo response",
                sub_claim
            ))
        })?
        .to_string();

    Ok(enterprise_common::EnterpriseProfile {
        external_user_id,
        email: json[email_claim].as_str().map(String::from),
        name: json[name_claim].as_str().map(String::from),
    })
}

// ── Token Exchange ──

async fn exchange_code_for_access_token(
    endpoints: &OAuthEndpoints,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String> {
    let params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let response = reqwest::Client::new()
        .post(&endpoints.token_url)
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
    endpoints: &OAuthEndpoints,
    access_token: &str,
) -> Result<serde_json::Value> {
    let response = reqwest::Client::new()
        .get(&endpoints.userinfo_url)
        .bearer_auth(access_token)
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

fn build_enterprise_authorize_url(
    endpoints: &OAuthEndpoints,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    login_hint: Option<&str>,
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
        if let Some(hint) = login_hint {
            pairs.append_pair("login_hint", hint);
        }
    }

    Ok(url.to_string())
}

// ══════════════════════════════════════════════════════════════════════
// Handlers
// ══════════════════════════════════════════════════════════════════════

/// Initiate enterprise SSO login: validate login_challenge, dispatch to OIDC or SAML.
#[utoipa::path(
    get,
    path = "/api/v1/enterprise-sso/authorize/{alias}",
    tag = "Identity",
    responses((status = 302, description = "Redirect to enterprise IdP"))
)]
pub async fn authorize<S: HasServices + HasCache + HasDbPool>(
    State(state): State<S>,
    Path(alias): Path<String>,
    Query(params): Query<EnterpriseSsoAuthorizeQuery>,
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

    // 2. Load connector (any provider_type)
    let connector = enterprise_common::load_connector(state.db_pool(), &alias).await?;

    // 3. Dispatch by provider_type
    if connector.provider_type == "saml" {
        return super::enterprise_saml_broker::saml_authorize_redirect(
            &state,
            connector,
            params.login_challenge,
            params.login_hint,
        )
        .await;
    }

    // ── OIDC flow ──
    oidc_authorize(&state, connector, params).await
}

async fn oidc_authorize<S: HasServices + HasCache + HasDbPool>(
    state: &S,
    connector: ConnectorRecord,
    params: EnterpriseSsoAuthorizeQuery,
) -> Result<Response> {
    let endpoints = resolve_endpoints(&connector.config)?;

    // Store enterprise SSO login state
    let sso_state_id = uuid::Uuid::new_v4().to_string();
    let sso_state = EnterpriseSsoLoginState {
        login_challenge_id: params.login_challenge,
        connector_alias: connector.alias.clone(),
        tenant_id: connector.tenant_id.clone(),
        authn_request_id: None,
        link_user_id: None,
    };
    let sso_state_json =
        serde_json::to_string(&sso_state).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_enterprise_sso_state(&sso_state_id, &sso_state_json, ENTERPRISE_SSO_STATE_TTL_SECS)
        .await?;

    // Build authorize URL
    let client_id = connector.config.get("clientId").ok_or_else(|| {
        AppError::BadRequest("Missing clientId in connector config".to_string())
    })?;
    let redirect_uri = enterprise_common::enterprise_callback_url(state.config());

    let authorize_url = build_enterprise_authorize_url(
        &endpoints,
        client_id,
        &redirect_uri,
        &sso_state_id,
        params.login_hint.as_deref(),
    )?;

    metrics::counter!("auth9_enterprise_sso_total", "action" => "authorize", "connector" => connector.alias.clone())
        .increment(1);

    Ok(Redirect::temporary(&authorize_url).into_response())
}

/// Enterprise OIDC callback: exchange code, find/create user, complete login challenge.
#[utoipa::path(
    get,
    path = "/api/v1/enterprise-sso/callback",
    tag = "Identity",
    responses((status = 302, description = "Redirect with authorization code"))
)]
pub async fn callback<S: HasServices + HasIdentityProviders + HasCache + HasSessionManagement + HasDbPool + HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<EnterpriseSsoCallbackQuery>,
) -> Result<Response> {
    // 1. Check for error from provider
    if params.error.is_some() {
        let login_url = enterprise_common::portal_login_url(state.config());
        return Ok(
            Redirect::temporary(&format!("{}?error=enterprise_sso_cancelled", login_url))
                .into_response(),
        );
    }

    let code = params
        .code
        .ok_or_else(|| AppError::BadRequest("Missing code parameter".to_string()))?;
    let state_id = params
        .state
        .ok_or_else(|| AppError::BadRequest("Missing state parameter".to_string()))?;

    // 2. Consume enterprise SSO state
    let sso_state_json = state
        .cache()
        .consume_enterprise_sso_state(&state_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Invalid or expired enterprise SSO state".to_string())
        })?;
    let sso_state: EnterpriseSsoLoginState =
        serde_json::from_str(&sso_state_json).map_err(|e| AppError::Internal(e.into()))?;

    // 3. Look up connector (OIDC only for this callback)
    let connector = enterprise_common::load_connector(state.db_pool(), &sso_state.connector_alias).await?;
    if connector.provider_type != "oidc" {
        return Err(AppError::BadRequest(
            "OIDC callback received for non-OIDC connector".to_string(),
        ));
    }
    let endpoints = resolve_endpoints(&connector.config)?;
    let client_id = connector.config.get("clientId").ok_or_else(|| {
        AppError::BadRequest("Missing clientId in connector config".to_string())
    })?;
    let client_secret = connector.config.get("clientSecret").ok_or_else(|| {
        AppError::BadRequest("Missing clientSecret in connector config".to_string())
    })?;

    // 4. Exchange code for access token
    let redirect_uri = enterprise_common::enterprise_callback_url(state.config());
    let access_token = exchange_code_for_access_token(
        &endpoints,
        client_id,
        client_secret,
        &code,
        &redirect_uri,
    )
    .await?;

    // 5. Fetch userinfo
    let userinfo_json = fetch_userinfo(&endpoints, &access_token).await?;

    // 6. Map profile
    let profile = map_profile(&connector.config, &userinfo_json)?;

    // 7a. If this is a link flow (link_user_id present), create linked identity and redirect
    if let Some(ref link_uid) = sso_state.link_user_id {
        let user_id = crate::models::common::StringUuid::parse_str(link_uid)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid link_user_id")))?;
        let input = crate::models::linked_identity::CreateLinkedIdentityInput {
            user_id,
            provider_type: "oidc".to_string(),
            provider_alias: connector.alias.clone(),
            external_user_id: profile.external_user_id.clone(),
            external_email: profile.email.clone(),
        };
        let _ = state
            .identity_provider_service()
            .create_linked_identity(&input)
            .await;

        // Record identity linked event
        if let Err(e) = state
            .analytics_service()
            .record_identity_linked(user_id, &connector.alias, "oidc")
            .await
        {
            tracing::warn!("Failed to record identity linked event: {}", e);
        }

        let portal_base = state.config().keycloak.portal_url.as_deref().unwrap_or(&state.config().jwt.issuer);
        let identities_url = format!("{}/dashboard/account/identities", portal_base.trim_end_matches('/'));
        return Ok(Redirect::temporary(&identities_url).into_response());
    }

    // 7b. Find or create user (enterprise SSO is tenant-scoped)
    let resolution = enterprise_common::find_or_create_enterprise_user(
        &state,
        &connector,
        &sso_state.tenant_id,
        &profile,
        "oidc",
        &sso_state.login_challenge_id,
    )
    .await?;

    // Handle pending merge: redirect to portal confirm-link page
    let user = match resolution {
        UserResolution::Found(user) => user,
        UserResolution::PendingMerge(pending) => {
            let token = uuid::Uuid::new_v4().to_string();
            let pending_json = serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.into()))?;
            state.cache().store_pending_merge(&token, &pending_json, ENTERPRISE_SSO_STATE_TTL_SECS).await?;
            let portal_base = state.config().keycloak.portal_url.as_deref().unwrap_or(&state.config().jwt.issuer);
            let redirect_url = format!("{}/login/confirm-link?token={}", portal_base.trim_end_matches('/'), token);
            return Ok(Redirect::temporary(&redirect_url).into_response());
        }
    };

    // 8. Create session
    let session = state
        .session_service()
        .create_session(user.id, None, None, None)
        .await?;

    // 9. Complete login flow
    let redirect_url = enterprise_common::complete_login_flow(
        &state,
        &sso_state.login_challenge_id,
        &user,
        session.id,
    )
    .await?;

    metrics::counter!("auth9_enterprise_sso_total", "action" => "callback_success", "connector" => connector.alias.clone())
        .increment(1);

    // Record federation login event
    let fed_meta = FederationEventMetadata {
        user_id: Some(user.id),
        email: Some(user.email.clone()),
        tenant_id: crate::models::common::StringUuid::parse_str(&sso_state.tenant_id).ok(),
        provider_alias: connector.alias.clone(),
        provider_type: "oidc".to_string(),
        ip_address: None,
        user_agent: None,
        session_id: Some(session.id),
    };
    if let Err(e) = state.analytics_service().record_federation_login(fed_meta).await {
        tracing::warn!("Failed to record enterprise OIDC federation login event: {}", e);
    }

    let mut response = Redirect::temporary(&redirect_url).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        "no-store".parse().unwrap(),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_endpoints_success() {
        let mut config = HashMap::new();
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
        let endpoints = resolve_endpoints(&config).unwrap();
        assert_eq!(endpoints.authorization_url, "https://idp.example.com/auth");
        assert_eq!(endpoints.token_url, "https://idp.example.com/token");
        assert_eq!(endpoints.userinfo_url, "https://idp.example.com/userinfo");
        assert_eq!(endpoints.scopes, "openid email profile");
    }

    #[test]
    fn test_resolve_endpoints_custom_scopes() {
        let mut config = HashMap::new();
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
        config.insert("scopes".to_string(), "openid email".to_string());
        let endpoints = resolve_endpoints(&config).unwrap();
        assert_eq!(endpoints.scopes, "openid email");
    }

    #[test]
    fn test_resolve_endpoints_missing_url() {
        let config = HashMap::new();
        let result = resolve_endpoints(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_endpoints_missing_userinfo() {
        let mut config = HashMap::new();
        config.insert(
            "authorizationUrl".to_string(),
            "https://idp.example.com/auth".to_string(),
        );
        config.insert(
            "tokenUrl".to_string(),
            "https://idp.example.com/token".to_string(),
        );
        let result = resolve_endpoints(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_profile_default_claims() {
        let config = HashMap::new();
        let json = serde_json::json!({
            "sub": "enterprise-user-123",
            "email": "user@corp.example.com",
            "name": "Enterprise User"
        });
        let profile = map_profile(&config, &json).unwrap();
        assert_eq!(profile.external_user_id, "enterprise-user-123");
        assert_eq!(profile.email, Some("user@corp.example.com".to_string()));
        assert_eq!(profile.name, Some("Enterprise User".to_string()));
    }

    #[test]
    fn test_map_profile_custom_claims() {
        let mut config = HashMap::new();
        config.insert("claimSub".to_string(), "oid".to_string());
        config.insert("claimEmail".to_string(), "upn".to_string());
        config.insert("claimName".to_string(), "display_name".to_string());
        let json = serde_json::json!({
            "oid": "azure-oid-456",
            "upn": "user@corp.example.com",
            "display_name": "Corp User"
        });
        let profile = map_profile(&config, &json).unwrap();
        assert_eq!(profile.external_user_id, "azure-oid-456");
        assert_eq!(profile.email, Some("user@corp.example.com".to_string()));
        assert_eq!(profile.name, Some("Corp User".to_string()));
    }

    #[test]
    fn test_map_profile_missing_sub_claim() {
        let config = HashMap::new();
        let json = serde_json::json!({
            "email": "user@example.com"
        });
        let result = map_profile(&config, &json);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_profile_minimal() {
        let config = HashMap::new();
        let json = serde_json::json!({
            "sub": "user-789"
        });
        let profile = map_profile(&config, &json).unwrap();
        assert_eq!(profile.external_user_id, "user-789");
        assert!(profile.email.is_none());
        assert!(profile.name.is_none());
    }

    #[test]
    fn test_build_enterprise_authorize_url() {
        let endpoints = OAuthEndpoints {
            authorization_url: "https://idp.corp.example.com/authorize".to_string(),
            token_url: String::new(),
            userinfo_url: String::new(),
            scopes: "openid email profile".to_string(),
        };
        let url = build_enterprise_authorize_url(
            &endpoints,
            "my-client-id",
            "https://auth9.example.com/api/v1/enterprise-sso/callback",
            "state-123",
            None,
        )
        .unwrap();
        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("scope=openid"));
        assert!(!url.contains("login_hint"));
    }

    #[test]
    fn test_build_enterprise_authorize_url_with_login_hint() {
        let endpoints = OAuthEndpoints {
            authorization_url: "https://idp.corp.example.com/authorize".to_string(),
            token_url: String::new(),
            userinfo_url: String::new(),
            scopes: "openid email profile".to_string(),
        };
        let url = build_enterprise_authorize_url(
            &endpoints,
            "client-id",
            "https://auth9.example.com/api/v1/enterprise-sso/callback",
            "state-456",
            Some("user@corp.example.com"),
        )
        .unwrap();
        assert!(url.contains("login_hint=user%40corp.example.com"));
    }
}

// ── Enterprise SSO Account Linking ──

use crate::domains::identity::api::identity_provider::extract_user_id;
use axum::http::HeaderMap;

/// Initiate enterprise SSO account linking (protected, requires JWT).
///
/// GET /api/v1/enterprise-sso/link/{alias}
#[utoipa::path(
    get,
    path = "/api/v1/enterprise-sso/link/{alias}",
    tag = "Identity",
    responses((status = 302, description = "Redirect to enterprise IdP for linking"))
)]
pub async fn link_authorize<
    S: HasServices + HasIdentityProviders + HasCache + HasDbPool,
>(
    State(state): State<S>,
    Path(alias): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let user_id = extract_user_id(&state, &headers)?;

    let pool = state.db_pool();
    let connector = enterprise_common::load_connector(pool, &alias).await?;

    // Build enterprise SSO state with link_user_id
    let sso_state_id = uuid::Uuid::new_v4().to_string();
    let sso_state = EnterpriseSsoLoginState {
        // Use a dummy login_challenge_id — the link flow doesn't use the OIDC authorize flow
        login_challenge_id: String::new(),
        connector_alias: connector.alias.clone(),
        tenant_id: connector.tenant_id.clone(),
        authn_request_id: None,
        link_user_id: Some(user_id.to_string()),
    };
    let sso_state_json =
        serde_json::to_string(&sso_state).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_enterprise_sso_state(&sso_state_id, &sso_state_json, ENTERPRISE_SSO_STATE_TTL_SECS)
        .await?;

    if connector.provider_type == "saml" {
        // SAML link flow — redirect to SAML authorize
        return crate::domains::identity::api::enterprise_saml_broker::saml_authorize_redirect(
            &state,
            connector,
            String::new(), // no login_challenge for link flow
            None,
        )
        .await;
    }

    // OIDC link flow
    let endpoints = resolve_endpoints(&connector.config)?;
    let callback_url = enterprise_common::enterprise_callback_url(state.config());
    let client_id = connector
        .config
        .get("clientId")
        .ok_or_else(|| AppError::BadRequest("Missing clientId in connector config".to_string()))?;

    let authorize_url = build_enterprise_authorize_url(
        &endpoints,
        client_id,
        &callback_url,
        &sso_state_id,
        None,
    )?;

    Ok(Redirect::temporary(&authorize_url).into_response())
}
