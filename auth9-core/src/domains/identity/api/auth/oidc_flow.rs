//! OIDC authorization flow handlers: authorize, callback, token, enterprise SSO discovery.

use super::action_helpers::discover_connector_by_domain;
use super::helpers::{
    enforce_pkce_for_public_client, validate_redirect_uri, verify_pkce_s256,
    AuthorizationCodeData, CallbackState, LoginChallengeData, AUTH_CODE_TTL_SECS,
    LOGIN_CHALLENGE_TTL_SECS,
};
use super::types::{
    AuthorizeCompleteRequest, AuthorizeCompleteResponse, AuthorizeRequest, CallbackRequest,
    EnterpriseSsoDiscoveryResponse, TokenRequest, TokenResponse,
};
use super::ALLOWED_SCOPES;
use crate::cache::CacheOperations;
use crate::error::{AppError, Result};
use crate::http_support::SuccessResponse;
use crate::models::enterprise_sso::EnterpriseSsoDiscoveryInput;
use crate::state::{
    HasAnalytics, HasCache, HasIdentityProviders, HasServices, HasSessionManagement,
};
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use url::Url;
use validator::Validate;

/// Filter and validate scope parameter against whitelist
pub(super) fn filter_scopes(requested_scope: &str) -> Result<String> {
    let scopes: Vec<&str> = requested_scope
        .split_whitespace()
        .filter(|s| ALLOWED_SCOPES.contains(s))
        .collect();

    // At minimum, openid scope is required
    if !scopes.contains(&"openid") {
        return Err(AppError::BadRequest(
            "scope must include 'openid'".to_string(),
        ));
    }

    Ok(scopes.join(" "))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    tag = "Identity",
    responses(
        (status = 302, description = "Redirect to OIDC provider")
    )
)]
/// Login redirect (initiates OIDC flow)
pub async fn authorize<S: HasServices + HasCache + crate::state::HasDbPool>(
    State(state): State<S>,
    Query(params): Query<AuthorizeRequest>,
) -> Result<Response> {
    let service = state
        .client_service()
        .get_by_client_id(&params.client_id)
        .await?;

    // Enforce PKCE for public clients (OAuth 2.1 / RFC 7636)
    let client_record = state
        .client_service()
        .get_client_record(&params.client_id)
        .await?;
    enforce_pkce_for_public_client(
        client_record.public_client,
        &params.code_challenge,
        &params.code_challenge_method,
    )?;

    validate_redirect_uri(&service.redirect_uris, &params.redirect_uri)?;

    // Validate state parameter is non-empty for CSRF protection
    if params.state.trim().is_empty() {
        return Err(AppError::BadRequest(
            "State parameter is required and cannot be empty".to_string(),
        ));
    }

    // Validate and filter scope against whitelist
    let filtered_scope = filter_scopes(&params.scope)?;

    // Resolve connector_alias: both OIDC and SAML connectors go to Auth9 enterprise broker.
    if let Some(alias) = params.connector_alias.as_deref() {
        let connector_exists = sqlx::query(
            "SELECT 1 FROM enterprise_sso_connectors WHERE alias = ? AND enabled = TRUE LIMIT 1",
        )
        .bind(alias)
        .fetch_optional(state.db_pool())
        .await
        .ok()
        .flatten()
        .is_some();

        if connector_exists {
            let challenge_data = LoginChallengeData {
                client_id: params.client_id,
                redirect_uri: params.redirect_uri,
                scope: filtered_scope,
                original_state: Some(params.state),
                nonce: params.nonce,
                code_challenge: params.code_challenge,
                code_challenge_method: params.code_challenge_method,
            };
            let challenge_id = uuid::Uuid::new_v4().to_string();
            let challenge_json =
                serde_json::to_string(&challenge_data).map_err(|e| AppError::Internal(e.into()))?;
            state
                .cache()
                .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
                .await?;

            let base = state
                .config()
                .core_public_url
                .as_deref()
                .unwrap_or(&state.config().jwt.issuer);
            let broker_url = format!(
                "{}/api/v1/enterprise-sso/authorize/{}?login_challenge={}",
                base.trim_end_matches('/'),
                alias,
                challenge_id,
            );
            return Ok(Redirect::temporary(&broker_url).into_response());
        }
        // Unknown connector: fall through to standard auth flow
    }

    // Redirect to hosted login with login_challenge
    let challenge_data = LoginChallengeData {
        client_id: params.client_id,
        redirect_uri: params.redirect_uri,
        scope: filtered_scope,
        original_state: Some(params.state),
        nonce: params.nonce,
        code_challenge: params.code_challenge,
        code_challenge_method: params.code_challenge_method,
    };
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let challenge_json =
        serde_json::to_string(&challenge_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
        .await?;

    let portal_url = state
        .config()
        .portal_url
        .as_deref()
        .unwrap_or(&state.config().jwt.issuer);
    let login_url = format!(
        "{}/login?login_challenge={}",
        portal_url.trim_end_matches('/'),
        challenge_id
    );

    Ok(Redirect::temporary(&login_url).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/enterprise-sso/discovery",
    tag = "Identity",
    responses(
        (status = 200, description = "SSO discovery result")
    )
)]
/// Enterprise SSO discovery endpoint.
/// Accepts user email, finds a tenant connector by domain, and returns redirect URL.
pub async fn enterprise_sso_discovery<S: HasServices + HasCache + crate::state::HasDbPool>(
    State(state): State<S>,
    Query(params): Query<AuthorizeRequest>,
    Json(input): Json<EnterpriseSsoDiscoveryInput>,
) -> Result<Json<SuccessResponse<EnterpriseSsoDiscoveryResponse>>> {
    input.validate()?;
    let (_, domain) = input
        .email
        .rsplit_once('@')
        .ok_or_else(|| AppError::Validation("Invalid email".to_string()))?;

    let discovery = discover_connector_by_domain(state.db_pool(), domain).await?;

    // Both OIDC and SAML: Auth9 enterprise broker handles natively
    let filtered_scope = filter_scopes(&params.scope)?;
    let challenge_data = LoginChallengeData {
        client_id: params.client_id,
        redirect_uri: params.redirect_uri,
        scope: filtered_scope,
        original_state: Some(params.state),
        nonce: params.nonce,
        code_challenge: params.code_challenge,
        code_challenge_method: params.code_challenge_method,
    };
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let challenge_json =
        serde_json::to_string(&challenge_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
        .await?;

    let base = state
        .config()
        .core_public_url
        .as_deref()
        .unwrap_or(&state.config().jwt.issuer);
    let mut authorize_url = format!(
        "{}/api/v1/enterprise-sso/authorize/{}?login_challenge={}",
        base.trim_end_matches('/'),
        discovery.connector_alias,
        challenge_id,
    );
    // Pass login_hint (user's email) for IdP to pre-fill
    authorize_url.push_str(&format!(
        "&login_hint={}",
        urlencoding::encode(&input.email)
    ));

    Ok(Json(SuccessResponse::new(EnterpriseSsoDiscoveryResponse {
        tenant_id: discovery.tenant_id,
        tenant_slug: discovery.tenant_slug,
        connector_alias: discovery.connector_alias,
        authorize_url,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/callback",
    tag = "Identity",
    responses(
        (status = 302, description = "Redirect with authorization code")
    )
)]
pub async fn callback<S: HasServices + HasCache>(
    State(state): State<S>,
    Query(params): Query<CallbackRequest>,
) -> Result<Response> {
    let state_nonce = params.state.as_deref().ok_or_else(|| {
        metrics::counter!("auth9_auth_invalid_state_total", "reason" => "missing").increment(1);
        AppError::BadRequest("Missing state".to_string())
    })?;
    let state_payload_json = state
        .cache()
        .consume_oidc_state(state_nonce)
        .await?
        .ok_or_else(|| {
            metrics::counter!("auth9_auth_invalid_state_total", "reason" => "invalid_or_expired")
                .increment(1);
            AppError::BadRequest("Invalid or expired state".to_string())
        })?;
    let state_payload: CallbackState = serde_json::from_str(&state_payload_json).map_err(|e| {
        metrics::counter!("auth9_auth_invalid_state_total", "reason" => "deserialize_error")
            .increment(1);
        AppError::Internal(e.into())
    })?;

    let mut redirect_url = Url::parse(&state_payload.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;

    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &params.code);
        if let Some(original_state) = state_payload.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    let mut response = Redirect::temporary(redirect_url.as_str()).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        "no-store".parse().unwrap(),
    );
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/authorize/complete",
    tag = "Identity",
    responses(
        (status = 200, description = "Authorization complete with redirect URL")
    )
)]
/// Complete the OIDC authorization flow after hosted login.
/// The caller must provide a valid identity token (from hosted-login) and the login_challenge_id.
/// Returns a redirect URL containing the authorization code and original state.
pub async fn authorize_complete<S: HasServices + HasCache>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(params): Json<AuthorizeCompleteRequest>,
) -> Result<Json<crate::http_support::SuccessResponse<AuthorizeCompleteResponse>>> {
    // 1. Extract and verify identity token from Authorization header
    let identity_claims = super::helpers::extract_identity_claims_from_headers(&state, &headers)?;
    let session_id = identity_claims.sid.ok_or_else(|| {
        AppError::BadRequest("Identity token must contain a session ID (sid)".to_string())
    })?;

    // 2. Consume login challenge
    let challenge_json = state
        .cache()
        .consume_login_challenge(&params.login_challenge_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired login challenge".to_string()))?;
    let challenge: LoginChallengeData =
        serde_json::from_str(&challenge_json).map_err(|e| AppError::Internal(e.into()))?;

    // 3. Generate authorization code
    let code = uuid::Uuid::new_v4().to_string();

    // 4. Store authorization code data
    let code_data = AuthorizationCodeData {
        user_id: identity_claims.sub,
        email: identity_claims.email,
        display_name: identity_claims.name,
        session_id,
        client_id: challenge.client_id.clone(),
        redirect_uri: challenge.redirect_uri.clone(),
        scope: challenge.scope,
        nonce: challenge.nonce,
        code_challenge: challenge.code_challenge,
        code_challenge_method: challenge.code_challenge_method,
    };
    let code_json = serde_json::to_string(&code_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_authorization_code(&code, &code_json, AUTH_CODE_TTL_SECS)
        .await?;

    // 5. Build redirect URL
    let mut redirect_url = Url::parse(&challenge.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;
    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &code);
        if let Some(original_state) = challenge.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    Ok(Json(crate::http_support::SuccessResponse::new(
        AuthorizeCompleteResponse {
            redirect_url: redirect_url.to_string(),
        },
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/token",
    tag = "Identity",
    responses(
        (status = 200, description = "Token response")
    )
)]
pub async fn token<
    S: HasServices + HasSessionManagement + HasCache + HasAnalytics + HasIdentityProviders,
>(
    State(state): State<S>,
    _headers: HeaderMap,
    Json(params): Json<TokenRequest>,
) -> Result<Response> {
    let jwt_manager = HasServices::jwt_manager(&state);

    match params.grant_type.as_str() {
        "authorization_code" => {
            let code = params
                .code
                .ok_or_else(|| AppError::BadRequest("Missing code".to_string()))?;
            let client_id = params
                .client_id
                .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
            let redirect_uri = params
                .redirect_uri
                .ok_or_else(|| AppError::BadRequest("Missing redirect_uri".to_string()))?;

            let code_verifier = params.code_verifier;

            // Consume local authorization code
            let code_data_json = state
                .cache()
                .consume_authorization_code(&code)
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest("Invalid or expired authorization code".to_string())
                })?;
            let code_data: AuthorizationCodeData =
                serde_json::from_str(&code_data_json).map_err(|e| AppError::Internal(e.into()))?;

            // Validate client_id and redirect_uri match
            if code_data.client_id != client_id {
                return Err(AppError::BadRequest(
                    "client_id does not match authorization code".to_string(),
                ));
            }
            if code_data.redirect_uri != redirect_uri {
                return Err(AppError::BadRequest(
                    "redirect_uri does not match authorization code".to_string(),
                ));
            }

            // Defensive: public clients must have PKCE (enforced at authorize, belt-and-suspenders)
            let client_record = state
                .client_service()
                .get_client_record(&client_id)
                .await?;
            if client_record.public_client && code_data.code_challenge.is_none() {
                return Err(AppError::BadRequest(
                    "Public clients must use PKCE".to_string(),
                ));
            }

            // PKCE validation
            if let Some(ref challenge) = code_data.code_challenge {
                let verifier = code_verifier.ok_or_else(|| {
                    AppError::BadRequest(
                        "code_verifier is required when code_challenge was set".to_string(),
                    )
                })?;
                let method = code_data.code_challenge_method.as_deref().unwrap_or("S256");
                if method != "S256" {
                    return Err(AppError::BadRequest(format!(
                        "Unsupported code_challenge_method: {}",
                        method
                    )));
                }
                if !verify_pkce_s256(&verifier, challenge) {
                    return Err(AppError::BadRequest("PKCE verification failed".to_string()));
                }
            }

            let user_id: uuid::Uuid = code_data
                .user_id
                .parse()
                .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in auth code")))?;
            let session_id: uuid::Uuid = code_data.session_id.parse().map_err(|_| {
                AppError::Internal(anyhow::anyhow!("Invalid session_id in auth code"))
            })?;

            // Create identity token
            let identity_token = jwt_manager.create_identity_token_with_session(
                user_id,
                &code_data.email,
                code_data.display_name.as_deref(),
                Some(session_id),
            )?;

            // Create id_token (OIDC spec)
            let id_token = jwt_manager.create_id_token(
                user_id,
                &code_data.email,
                code_data.display_name.as_deref(),
                code_data.nonce.as_deref(),
                &client_id,
                Some(session_id),
                &identity_token,
            )?;

            // Create OIDC refresh token
            let refresh_token =
                jwt_manager.create_oidc_refresh_token(user_id, &client_id, session_id)?;

            // Bind refresh token to session
            let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
            state
                .cache()
                .bind_refresh_token_session(&refresh_token, &session_id.to_string(), refresh_ttl)
                .await?;

            metrics::counter!("auth9_auth_login_total", "result" => "success", "backend" => "auth9_oidc").increment(1);

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: jwt_manager.access_token_ttl(),
                refresh_token: Some(refresh_token),
                id_token: Some(id_token),
            })
            .into_response())
        }
        "client_credentials" => {
            let client_id = params
                .client_id
                .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;
            let client_secret = params
                .client_secret
                .ok_or_else(|| AppError::BadRequest("Missing client_secret".to_string()))?;

            let service = state
                .client_service()
                .verify_secret(&client_id, &client_secret)
                .await?;

            let email = format!("service+{}@auth9.local", client_id);
            let tenant_id = service.tenant_id.map(|t| t.0);
            let service_token =
                jwt_manager.create_service_client_token(service.id.0, &email, tenant_id)?;

            Ok(Json(TokenResponse {
                access_token: service_token,
                token_type: "Bearer".to_string(),
                expires_in: jwt_manager.access_token_ttl(),
                refresh_token: None,
                id_token: None,
            })
            .into_response())
        }
        "refresh_token" => {
            let refresh_token = params
                .refresh_token
                .ok_or_else(|| AppError::BadRequest("Missing refresh_token".to_string()))?;
            let client_id = params
                .client_id
                .ok_or_else(|| AppError::BadRequest("Missing client_id".to_string()))?;

            // Validate Auth9 OIDC refresh token
            let refresh_claims = jwt_manager
                .verify_oidc_refresh_token(&refresh_token, &client_id)
                .map_err(|e| AppError::Unauthorized(format!("Invalid refresh token: {}", e)))?;

            // Verify session binding
            let session_id_str = state
                .cache()
                .get_refresh_token_session(&refresh_token)
                .await?
                .ok_or_else(|| {
                    AppError::Unauthorized(
                        "Refresh token is not bound to an active session".to_string(),
                    )
                })?;
            let session_id = uuid::Uuid::parse_str(&session_id_str).map_err(|_| {
                AppError::Internal(anyhow::anyhow!("Invalid session_id in refresh binding"))
            })?;

            let user_id: crate::models::common::StringUuid =
                refresh_claims.sub.parse().map_err(|_| {
                    AppError::Internal(anyhow::anyhow!("Invalid user_id in refresh token"))
                })?;

            let user = state.user_service().get(user_id).await?;

            // Issue new tokens (rotation)
            let new_identity_token = jwt_manager.create_identity_token_with_session(
                *user.id,
                &user.email,
                user.display_name.as_deref(),
                Some(session_id),
            )?;

            let new_id_token = jwt_manager.create_id_token(
                *user.id,
                &user.email,
                user.display_name.as_deref(),
                None, // nonce is only for initial token issuance
                &client_id,
                Some(session_id),
                &new_identity_token,
            )?;

            let new_refresh_token =
                jwt_manager.create_oidc_refresh_token(*user.id, &client_id, session_id)?;

            // Rotate: unbind old, bind new
            let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
            state
                .cache()
                .remove_refresh_token_session(&refresh_token)
                .await?;
            state
                .cache()
                .bind_refresh_token_session(&new_refresh_token, &session_id_str, refresh_ttl)
                .await?;

            Ok(Json(TokenResponse {
                access_token: new_identity_token,
                token_type: "Bearer".to_string(),
                expires_in: jwt_manager.access_token_ttl(),
                refresh_token: Some(new_refresh_token),
                id_token: Some(new_id_token),
            })
            .into_response())
        }
        _ => Err(AppError::BadRequest(format!(
            "Unsupported grant type: {}",
            params.grant_type
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorize_request_deserialization() {
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid profile email",
            "state": "abc123",
            "nonce": "xyz789"
        }"#;

        let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.response_type, "code");
        assert_eq!(request.client_id, "my-app");
        assert_eq!(request.redirect_uri, "https://app.example.com/callback");
        assert_eq!(request.scope, "openid profile email");
        assert_eq!(request.state, "abc123");
        assert_eq!(request.nonce, Some("xyz789".to_string()));
    }

    #[test]
    fn test_authorize_request_minimal() {
        // state is now required for CSRF protection
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid",
            "state": "csrf-protection-state"
        }"#;

        let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.state, "csrf-protection-state");
        assert!(request.nonce.is_none());
    }

    #[test]
    fn test_authorize_request_missing_state_fails() {
        // state is required for CSRF protection, should fail without it
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid"
        }"#;

        let result: serde_json::Result<AuthorizeRequest> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_scopes_valid() {
        let result = filter_scopes("openid profile email").unwrap();
        assert_eq!(result, "openid profile email");
    }

    #[test]
    fn test_filter_scopes_removes_invalid() {
        let result = filter_scopes("openid admin offline_access profile").unwrap();
        assert_eq!(result, "openid profile");
    }

    #[test]
    fn test_filter_scopes_requires_openid() {
        let result = filter_scopes("profile email");
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_scopes_only_openid() {
        let result = filter_scopes("openid").unwrap();
        assert_eq!(result, "openid");
    }

    #[test]
    fn test_token_request_authorization_code() {
        let json = r#"{
            "grant_type": "authorization_code",
            "client_id": "my-app",
            "code": "auth-code-123",
            "redirect_uri": "https://app.example.com/callback"
        }"#;

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.grant_type, "authorization_code");
        assert_eq!(request.client_id, Some("my-app".to_string()));
        assert_eq!(request.code, Some("auth-code-123".to_string()));
    }

    #[test]
    fn test_token_request_client_credentials() {
        let json = r#"{
            "grant_type": "client_credentials",
            "client_id": "service-app",
            "client_secret": "secret123"
        }"#; // pragma: allowlist secret

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.grant_type, "client_credentials");
        assert_eq!(request.client_secret, Some("secret123".to_string()));
    }

    #[test]
    fn test_token_request_refresh_token() {
        let json = r#"{
            "grant_type": "refresh_token",
            "client_id": "my-app",
            "refresh_token": "refresh-token-abc"
        }"#;

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.grant_type, "refresh_token");
        assert_eq!(request.refresh_token, Some("refresh-token-abc".to_string()));
    }

    #[test]
    fn test_token_response_serialization() {
        let response = TokenResponse {
            access_token: "access-token-xyz".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh-token-abc".to_string()),
            id_token: Some("id-token-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_token_response_without_optional_fields() {
        let response = TokenResponse {
            access_token: "access".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None,
            id_token: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("null") || !json.contains("refresh_token")); // Optional field
    }

    #[test]
    fn test_callback_request_deserialization() {
        let json = r#"{
            "code": "auth-code-xyz",
            "state": "encoded-state"
        }"#;

        let request: CallbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.code, "auth-code-xyz");
        assert_eq!(request.state, Some("encoded-state".to_string()));
    }

    #[test]
    fn test_callback_request_without_state() {
        let json = r#"{"code": "auth-code-123"}"#;

        let request: CallbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.code, "auth-code-123");
        assert!(request.state.is_none());
    }

    #[test]
    fn test_authorize_request_with_all_fields() {
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid profile email offline_access",
            "state": "random-state-value",
            "nonce": "random-nonce-value"
        }"#;

        let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
        // offline_access is in the request but will be filtered out by filter_scopes
        assert!(request.scope.contains("offline_access"));
        assert_eq!(request.state, "random-state-value");
        assert!(request.nonce.is_some());
    }

    #[test]
    fn test_token_request_empty_optionals() {
        let json = r#"{"grant_type": "password"}"#;

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert!(request.client_id.is_none());
        assert!(request.client_secret.is_none());
        assert!(request.code.is_none());
        assert!(request.redirect_uri.is_none());
        assert!(request.refresh_token.is_none());
    }

    #[test]
    fn test_token_response_serialization_with_nulls() {
        let response = TokenResponse {
            access_token: "token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None,
            id_token: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        // Optional fields should be present as null
        assert!(json.contains("refresh_token"));
        assert!(json.contains("id_token"));
    }

    #[test]
    fn test_callback_request_with_special_characters() {
        let json = r#"{"code": "code-with-special-chars!@#$%", "state": "state+with/slash"}"#;
        let request: CallbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.code, "code-with-special-chars!@#$%");
        assert_eq!(request.state, Some("state+with/slash".to_string()));
    }

    #[test]
    fn test_token_request_all_fields() {
        let json = r#"{
            "grant_type": "authorization_code",
            "client_id": "app",
            "client_secret": "secret",
            "code": "auth-code",
            "redirect_uri": "https://app.com/cb",
            "refresh_token": "refresh"
        }"#; // pragma: allowlist secret

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.grant_type, "authorization_code");
        assert!(request.client_id.is_some());
        assert!(request.client_secret.is_some());
        assert!(request.code.is_some());
        assert!(request.redirect_uri.is_some());
        assert!(request.refresh_token.is_some());
    }

    #[test]
    fn test_filter_scopes_all_invalid_except_openid() {
        let result = filter_scopes("openid admin root superuser").unwrap();
        assert_eq!(result, "openid");
    }

    #[test]
    fn test_filter_scopes_duplicate_openid() {
        let result = filter_scopes("openid openid profile").unwrap();
        // Both openid entries pass the filter
        assert!(result.contains("openid"));
        assert!(result.contains("profile"));
    }

    #[test]
    fn test_filter_scopes_empty_string() {
        let result = filter_scopes("");
        assert!(result.is_err());
    }

    #[test]
    fn test_authorize_request_with_pkce() {
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid",
            "state": "csrf-state",
            "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            "code_challenge_method": "S256"
        }"#; // pragma: allowlist secret

        let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.code_challenge,
            Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string())
        );
        assert_eq!(request.code_challenge_method, Some("S256".to_string()));
    }

    #[test]
    fn test_authorize_request_without_pkce() {
        let json = r#"{
            "response_type": "code",
            "client_id": "my-app",
            "redirect_uri": "https://app.example.com/callback",
            "scope": "openid",
            "state": "csrf-state"
        }"#;

        let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
        assert!(request.code_challenge.is_none());
        assert!(request.code_challenge_method.is_none());
    }

    #[test]
    fn test_token_request_with_code_verifier() {
        let json = r#"{
            "grant_type": "authorization_code",
            "client_id": "my-app",
            "code": "auth-code",
            "redirect_uri": "https://app.com/cb",
            "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        }"#; // pragma: allowlist secret

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.code_verifier,
            Some("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_string())
        );
    }

    #[test]
    fn test_token_request_without_code_verifier() {
        let json = r#"{
            "grant_type": "authorization_code",
            "client_id": "my-app",
            "code": "auth-code",
            "redirect_uri": "https://app.com/cb"
        }"#;

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert!(request.code_verifier.is_none());
    }
}
