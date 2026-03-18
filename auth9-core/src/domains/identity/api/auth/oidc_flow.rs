//! OIDC authorization flow handlers: authorize, callback, token, enterprise SSO discovery.

use super::action_helpers::{
    discover_connector_by_domain, resolve_action_ids, resolve_action_tenant_profile,
    resolve_service_ids_for_actions,
};
use super::helpers::{
    build_callback_url, build_keycloak_auth_url, extract_client_ip, validate_redirect_uri,
    CallbackState, KeycloakAuthUrlParams,
};
use super::helpers::{
    verify_pkce_s256, AuthorizationCodeData, LoginChallengeData, AUTH_CODE_TTL_SECS,
    LOGIN_CHALLENGE_TTL_SECS,
};
use super::keycloak_client::{exchange_code_for_tokens, exchange_refresh_token, fetch_userinfo};
use super::types::{
    AuthorizeCompleteRequest, AuthorizeCompleteResponse, AuthorizeRequest, CallbackRequest,
    EnterpriseSsoDiscoveryResponse, TokenRequest, TokenResponse,
};
use super::{ALLOWED_SCOPES, OIDC_STATE_TTL_SECS};
use crate::cache::CacheOperations;
use crate::config::IdentityBackend;
use crate::domains::security_observability::service::analytics::LoginEventMetadata;
use crate::error::{AppError, Result};
use crate::http_support::SuccessResponse;
use crate::models::action::{
    ActionContext, ActionContextRequest, ActionContextTenant, ActionContextUser,
};
use crate::models::common::StringUuid;
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
use chrono::Utc;
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

    validate_redirect_uri(&service.redirect_uris, &params.redirect_uri)?;

    // Validate state parameter is non-empty for CSRF protection
    if params.state.trim().is_empty() {
        return Err(AppError::BadRequest(
            "State parameter is required and cannot be empty".to_string(),
        ));
    }

    // Validate and filter scope against whitelist
    let filtered_scope = filter_scopes(&params.scope)?;

    // ===== Auth9-OIDC backend: redirect to hosted login with login_challenge =====
    if state.config().identity_backend == IdentityBackend::Auth9Oidc {
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
        let challenge_json = serde_json::to_string(&challenge_data)
            .map_err(|e| AppError::Internal(e.into()))?;
        state
            .cache()
            .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
            .await?;

        let portal_url = state
            .config()
            .keycloak
            .portal_url
            .as_deref()
            .unwrap_or(&state.config().jwt.issuer);
        let login_url = format!(
            "{}/login?login_challenge={}",
            portal_url.trim_end_matches('/'),
            challenge_id
        );

        return Ok(Redirect::temporary(&login_url).into_response());
    }

    // ===== Keycloak backend: redirect to Keycloak auth URL (existing path) =====
    let callback_url = build_callback_url(
        state
            .config()
            .keycloak
            .core_public_url
            .as_deref()
            .unwrap_or(&state.config().jwt.issuer),
    );

    let state_payload = CallbackState {
        redirect_uri: params.redirect_uri,
        client_id: params.client_id,
        original_state: Some(params.state),
    };

    let state_nonce = uuid::Uuid::new_v4().to_string();
    let state_payload_json =
        serde_json::to_string(&state_payload).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_oidc_state(&state_nonce, &state_payload_json, OIDC_STATE_TTL_SECS)
        .await?;

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
            // Redirect to Auth9 enterprise broker with login_challenge
            let challenge_data = LoginChallengeData {
                client_id: state_payload.client_id,
                redirect_uri: state_payload.redirect_uri,
                scope: filtered_scope,
                original_state: state_payload.original_state,
                nonce: params.nonce,
                code_challenge: params.code_challenge,
                code_challenge_method: params.code_challenge_method,
            };
            let challenge_id = uuid::Uuid::new_v4().to_string();
            let challenge_json = serde_json::to_string(&challenge_data)
                .map_err(|e| AppError::Internal(e.into()))?;
            state
                .cache()
                .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
                .await?;

            let base = state
                .config()
                .keycloak
                .core_public_url
                .as_deref()
                .unwrap_or(&state.config().jwt.issuer);
            let broker_url = format!(
                "{}/api/v1/enterprise-sso/authorize/{}?login_challenge={}",
                base.trim_end_matches('/'),
                alias,
                challenge_id,
            );
            // Clean up the stored OIDC state since we won't use Keycloak callback
            let _ = state.cache().consume_oidc_state(&state_nonce).await;
            return Ok(Redirect::temporary(&broker_url).into_response());
        }

        // Unknown connector: fall through to standard Keycloak auth flow
    }

    // Also store a login_challenge under the same state_nonce so that Portal's
    // social login flow can use the state as a login_challenge ID.
    // This bridges the Keycloak-mode authorize with the Auth9-native social broker.
    {
        let challenge_data = LoginChallengeData {
            client_id: state_payload.client_id.clone(),
            redirect_uri: state_payload.redirect_uri.clone(),
            scope: filtered_scope.clone(),
            original_state: state_payload.original_state.clone(),
            nonce: params.nonce.clone(),
            code_challenge: params.code_challenge.clone(),
            code_challenge_method: params.code_challenge_method.clone(),
        };
        if let Ok(challenge_json) = serde_json::to_string(&challenge_data) {
            let _ = state
                .cache()
                .store_login_challenge(&state_nonce, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
                .await;
        }
    }

    // No connector_alias: standard Keycloak auth flow
    let auth_url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
        keycloak_public_url: &state.config().keycloak.public_url,
        realm: &state.config().keycloak.realm,
        response_type: &params.response_type,
        client_id: &state_payload.client_id,
        callback_url: &callback_url,
        scope: &filtered_scope,
        encoded_state: &state_nonce,
        nonce: params.nonce.as_deref(),
        connector_alias: None,
        kc_action: params.kc_action.as_deref(),
        ui_locales: params.ui_locales.as_deref(),
        code_challenge: params.code_challenge.as_deref(),
        code_challenge_method: params.code_challenge_method.as_deref(),
    })?;

    Ok(Redirect::temporary(&auth_url).into_response())
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
    let challenge_json = serde_json::to_string(&challenge_data)
        .map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_login_challenge(&challenge_id, &challenge_json, LOGIN_CHALLENGE_TTL_SECS)
        .await?;

    let base = state
        .config()
        .keycloak
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
    authorize_url.push_str(&format!("&login_hint={}", urlencoding::encode(&input.email)));

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
    let identity_claims =
        super::helpers::extract_identity_claims_from_headers(&state, &headers)?;
    let session_id = identity_claims.sid.ok_or_else(|| {
        AppError::BadRequest("Identity token must contain a session ID (sid)".to_string())
    })?;

    // 2. Consume login challenge
    let challenge_json = state
        .cache()
        .consume_login_challenge(&params.login_challenge_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Invalid or expired login challenge".to_string())
        })?;
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
    let code_json =
        serde_json::to_string(&code_data).map_err(|e| AppError::Internal(e.into()))?;
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
    headers: HeaderMap,
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

            // ===== Auth9-OIDC backend: consume local authorization code =====
            if state.config().identity_backend == IdentityBackend::Auth9Oidc {
                let code_data_json = state
                    .cache()
                    .consume_authorization_code(&code)
                    .await?
                    .ok_or_else(|| {
                        AppError::BadRequest(
                            "Invalid or expired authorization code".to_string(),
                        )
                    })?;
                let code_data: AuthorizationCodeData =
                    serde_json::from_str(&code_data_json)
                        .map_err(|e| AppError::Internal(e.into()))?;

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

                // PKCE validation
                if let Some(ref challenge) = code_data.code_challenge {
                    let verifier = code_verifier.ok_or_else(|| {
                        AppError::BadRequest(
                            "code_verifier is required when code_challenge was set".to_string(),
                        )
                    })?;
                    let method = code_data
                        .code_challenge_method
                        .as_deref()
                        .unwrap_or("S256");
                    if method != "S256" {
                        return Err(AppError::BadRequest(format!(
                            "Unsupported code_challenge_method: {}",
                            method
                        )));
                    }
                    if !verify_pkce_s256(&verifier, challenge) {
                        return Err(AppError::BadRequest(
                            "PKCE verification failed".to_string(),
                        ));
                    }
                }

                let user_id: uuid::Uuid = code_data
                    .user_id
                    .parse()
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in auth code")))?;
                let session_id: uuid::Uuid = code_data
                    .session_id
                    .parse()
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid session_id in auth code")))?;

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
                let refresh_token = jwt_manager.create_oidc_refresh_token(
                    user_id,
                    &client_id,
                    session_id,
                )?;

                // Bind refresh token to session
                let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
                state
                    .cache()
                    .bind_refresh_token_session(
                        &refresh_token,
                        &session_id.to_string(),
                        refresh_ttl,
                    )
                    .await?;

                metrics::counter!("auth9_auth_login_total", "result" => "success", "backend" => "auth9_oidc").increment(1);

                return Ok(Json(TokenResponse {
                    access_token: identity_token,
                    token_type: "Bearer".to_string(),
                    expires_in: jwt_manager.access_token_ttl(),
                    refresh_token: Some(refresh_token),
                    id_token: Some(id_token),
                })
                .into_response());
            }

            // ===== Keycloak backend: exchange code with Keycloak =====
            let state_payload = CallbackState {
                redirect_uri,
                client_id,
                original_state: None,
            };

            let token_response =
                exchange_code_for_tokens(&state, &state_payload, &code, code_verifier.as_deref())
                    .await?;
            let userinfo = fetch_userinfo(&state, &token_response.access_token).await?;
            let ip_address = extract_client_ip(&headers);
            let user_agent = headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let (action_service_id, service_tenant_id) =
                resolve_service_ids_for_actions(&state, &state_payload.client_id).await;

            let user = match state
                .user_service()
                .get_by_identity_subject(&userinfo.sub)
                .await
            {
                Ok(existing) => existing,
                Err(AppError::NotFound(_)) => {
                    if let Some(service_id) = action_service_id {
                        if let Some(tenant_id) = service_tenant_id {
                            let (tenant_slug, tenant_name) =
                                resolve_action_tenant_profile(&state, tenant_id).await;
                            let pre_reg_context = ActionContext {
                                user: ActionContextUser {
                                    id: StringUuid::new_v4().to_string(),
                                    email: userinfo.email.clone(),
                                    display_name: userinfo.name.clone(),
                                    mfa_enabled: false,
                                },
                                tenant: ActionContextTenant {
                                    id: tenant_id.to_string(),
                                    slug: tenant_slug.clone(),
                                    name: tenant_name.clone(),
                                },
                                request: ActionContextRequest {
                                    ip: ip_address.clone(),
                                    user_agent: user_agent.clone(),
                                    timestamp: Utc::now(),
                                },
                                claims: None,
                                service: None,
                            };
                            // Pre* trigger errors are blocking.
                            state
                                .action_service()
                                .execute_trigger(
                                    service_id,
                                    "pre-user-registration",
                                    pre_reg_context,
                                )
                                .await?;
                        }
                    }

                    let input = crate::models::user::CreateUserInput {
                        email: userinfo.email.clone(),
                        display_name: userinfo.name.clone(),
                        avatar_url: None,
                    };
                    let new_user = state.user_service().create(&userinfo.sub, input).await?;

                    if let Some(service_id) = action_service_id {
                        if let Some(tenant_id) = service_tenant_id {
                            let (tenant_slug, tenant_name) =
                                resolve_action_tenant_profile(&state, tenant_id).await;
                            let post_reg_context = ActionContext {
                                user: ActionContextUser {
                                    id: new_user.id.to_string(),
                                    email: new_user.email.clone(),
                                    display_name: new_user.display_name.clone(),
                                    mfa_enabled: new_user.mfa_enabled,
                                },
                                tenant: ActionContextTenant {
                                    id: tenant_id.to_string(),
                                    slug: tenant_slug,
                                    name: tenant_name,
                                },
                                request: ActionContextRequest {
                                    ip: ip_address.clone(),
                                    user_agent: user_agent.clone(),
                                    timestamp: Utc::now(),
                                },
                                claims: None,
                                service: None,
                            };
                            // Post* trigger errors are non-blocking.
                            if let Err(e) = state
                                .action_service()
                                .execute_trigger(
                                    service_id,
                                    "post-user-registration",
                                    post_reg_context,
                                )
                                .await
                            {
                                tracing::warn!(
                                    user_id = %new_user.id,
                                    "PostUserRegistration action failed: {}",
                                    e
                                );
                            }
                        }
                    }

                    new_user
                }
                Err(e) => return Err(e),
            };

            // Create session record for authorization_code flow
            let session = state
                .session_service()
                .create_session(user.id, None, ip_address.clone(), user_agent.clone())
                .await?;

            // Record login event
            {
                let mut metadata =
                    LoginEventMetadata::new(user.id, &userinfo.email).with_session_id(session.id);
                if let Some(ref ip) = ip_address {
                    metadata = metadata.with_ip_address(ip.clone());
                }
                if let Some(ref ua) = user_agent {
                    metadata = metadata.with_user_agent(ua.clone());
                }
                if let Err(e) = state
                    .analytics_service()
                    .record_successful_login(metadata)
                    .await
                {
                    tracing::warn!(
                        user_id = %user.id,
                        "Failed to record login event: {}",
                        e
                    );
                }
            }

            // Execute post-login Actions (if any are configured for this service)
            let custom_claims = {
                let (resolved_service_id, resolved_tenant_id) = resolve_action_ids(
                    &state,
                    &state_payload.client_id,
                    user.id,
                    action_service_id,
                    service_tenant_id,
                )
                .await;

                if let (Some(service_id), Some(tenant_id)) =
                    (resolved_service_id, resolved_tenant_id)
                {
                    // Resolve tenant slug/name for ActionContext
                    let (tenant_slug, tenant_name) =
                        resolve_action_tenant_profile(&state, tenant_id).await;

                    let action_context = ActionContext {
                        user: ActionContextUser {
                            id: user.id.to_string(),
                            email: user.email.clone(),
                            display_name: user.display_name.clone(),
                            mfa_enabled: user.mfa_enabled,
                        },
                        tenant: ActionContextTenant {
                            id: tenant_id.to_string(),
                            slug: tenant_slug,
                            name: tenant_name,
                        },
                        request: ActionContextRequest {
                            ip: ip_address,
                            user_agent,
                            timestamp: Utc::now(),
                        },
                        claims: None,
                        service: None,
                    };

                    match state
                        .action_service()
                        .execute_trigger(service_id, "post-login", action_context)
                        .await
                    {
                        Ok(modified_context) => {
                            if modified_context.claims.is_some() {
                                tracing::info!(
                                    "PostLogin actions executed with custom claims for user {} (service {}) via token endpoint",
                                    user.id,
                                    service_id
                                );
                            } else {
                                tracing::debug!(
                                    "PostLogin action trigger completed for user {} (service {}), no claims modified",
                                    user.id,
                                    service_id
                                );
                            }
                            modified_context.claims
                        }
                        Err(e) => {
                            tracing::warn!(
                                "PostLogin action failed (strict_mode) for user {}: {}",
                                user.id,
                                e
                            );
                            return Err(e);
                        }
                    }
                } else {
                    None
                }
            };

            // Create identity token with session ID and optional custom claims
            let identity_token = if let Some(claims) = custom_claims {
                jwt_manager.create_identity_token_with_session_and_claims(
                    *user.id,
                    &userinfo.email,
                    userinfo.name.as_deref(),
                    Some(*session.id),
                    claims,
                )?
            } else {
                jwt_manager.create_identity_token_with_session(
                    *user.id,
                    &userinfo.email,
                    userinfo.name.as_deref(),
                    Some(*session.id),
                )?
            };

            if let Some(refresh_token) = token_response.refresh_token.as_deref() {
                let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
                state
                    .cache()
                    .bind_refresh_token_session(refresh_token, &session.id.to_string(), refresh_ttl)
                    .await?;
            }

            metrics::counter!("auth9_auth_login_total", "result" => "success", "backend" => "oidc").increment(1);

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: jwt_manager.access_token_ttl(),
                refresh_token: token_response.refresh_token,
                id_token: token_response.id_token,
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

            // ===== Auth9-OIDC backend: validate Auth9 OIDC refresh token =====
            if state.config().identity_backend == IdentityBackend::Auth9Oidc {
                let refresh_claims = jwt_manager
                    .verify_oidc_refresh_token(&refresh_token, &client_id)
                    .map_err(|e| {
                        AppError::Unauthorized(format!("Invalid refresh token: {}", e))
                    })?;

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

                let user_id: crate::models::common::StringUuid = refresh_claims
                    .sub
                    .parse()
                    .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in refresh token")))?;

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

                let new_refresh_token = jwt_manager.create_oidc_refresh_token(
                    *user.id,
                    &client_id,
                    session_id,
                )?;

                // Rotate: unbind old, bind new
                let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
                state
                    .cache()
                    .remove_refresh_token_session(&refresh_token)
                    .await?;
                state
                    .cache()
                    .bind_refresh_token_session(
                        &new_refresh_token,
                        &session_id_str,
                        refresh_ttl,
                    )
                    .await?;

                return Ok(Json(TokenResponse {
                    access_token: new_identity_token,
                    token_type: "Bearer".to_string(),
                    expires_in: jwt_manager.access_token_ttl(),
                    refresh_token: Some(new_refresh_token),
                    id_token: Some(new_id_token),
                })
                .into_response());
            }

            // ===== Keycloak backend: exchange refresh token with Keycloak =====
            let state_payload = CallbackState {
                redirect_uri: String::new(),
                client_id,
                original_state: None,
            };

            let token_response =
                exchange_refresh_token(&state, &state_payload, &refresh_token).await?;
            let userinfo = fetch_userinfo(&state, &token_response.access_token).await?;
            let ip_address = extract_client_ip(&headers);
            let user_agent = headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let (action_svc_id, svc_tenant_id) =
                resolve_service_ids_for_actions(&state, &state_payload.client_id).await;

            let user = match state
                .user_service()
                .get_by_identity_subject(&userinfo.sub)
                .await
            {
                Ok(existing) => existing,
                Err(AppError::NotFound(_)) => {
                    let input = crate::models::user::CreateUserInput {
                        email: userinfo.email.clone(),
                        display_name: userinfo.name.clone(),
                        avatar_url: None,
                    };
                    state.user_service().create(&userinfo.sub, input).await?
                }
                Err(e) => return Err(e),
            };

            let session_id = state
                .cache()
                .get_refresh_token_session(&refresh_token)
                .await?
                .and_then(|sid| uuid::Uuid::parse_str(&sid).ok())
                .ok_or_else(|| {
                    AppError::Unauthorized(
                        "Refresh token is not bound to an active session".to_string(),
                    )
                })?;

            // Execute pre-token-refresh Actions (blocking)
            let custom_claims = {
                let (resolved_service_id, resolved_tenant_id) = resolve_action_ids(
                    &state,
                    &state_payload.client_id,
                    user.id,
                    action_svc_id,
                    svc_tenant_id,
                )
                .await;

                if let (Some(service_id), Some(tenant_id)) =
                    (resolved_service_id, resolved_tenant_id)
                {
                    let (tenant_slug, tenant_name) =
                        resolve_action_tenant_profile(&state, tenant_id).await;
                    let pre_refresh_context = ActionContext {
                        user: ActionContextUser {
                            id: user.id.to_string(),
                            email: user.email.clone(),
                            display_name: user.display_name.clone(),
                            mfa_enabled: user.mfa_enabled,
                        },
                        tenant: ActionContextTenant {
                            id: tenant_id.to_string(),
                            slug: tenant_slug,
                            name: tenant_name,
                        },
                        request: ActionContextRequest {
                            ip: ip_address,
                            user_agent,
                            timestamp: Utc::now(),
                        },
                        claims: None,
                        service: None,
                    };
                    let modified_context = state
                        .action_service()
                        .execute_trigger(service_id, "pre-token-refresh", pre_refresh_context)
                        .await?;
                    modified_context.claims
                } else {
                    None
                }
            };

            let identity_token = if let Some(claims) = custom_claims {
                jwt_manager.create_identity_token_with_session_and_claims(
                    *user.id,
                    &userinfo.email,
                    userinfo.name.as_deref(),
                    Some(session_id),
                    claims,
                )?
            } else {
                jwt_manager.create_identity_token_with_session(
                    *user.id,
                    &userinfo.email,
                    userinfo.name.as_deref(),
                    Some(session_id),
                )?
            };

            if let Some(new_refresh_token) = token_response.refresh_token.as_deref() {
                let refresh_ttl = state.config().jwt.refresh_token_ttl_secs.max(1) as u64;
                if new_refresh_token != refresh_token {
                    state
                        .cache()
                        .remove_refresh_token_session(&refresh_token)
                        .await?;
                }
                state
                    .cache()
                    .bind_refresh_token_session(
                        new_refresh_token,
                        &session_id.to_string(),
                        refresh_ttl,
                    )
                    .await?;
            }

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: jwt_manager.access_token_ttl(),
                refresh_token: token_response.refresh_token,
                id_token: token_response.id_token,
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
