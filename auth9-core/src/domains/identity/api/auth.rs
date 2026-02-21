//! Authentication API handlers

use crate::api::{write_audit_log_generic, SuccessResponse};
use crate::cache::CacheOperations;
use crate::domain::{
    ActionContext, ActionContextRequest, ActionContextTenant, ActionContextUser,
    EnterpriseSsoDiscoveryInput, EnterpriseSsoDiscoveryResult, StringUuid,
};
use crate::domains::security_observability::service::analytics::LoginEventMetadata;
use crate::error::{AppError, Result};
use crate::jwt::IdentityClaims;
use crate::state::{HasAnalytics, HasCache, HasServices, HasSessionManagement};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use base64::Engine;
use chrono::Utc;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use url::Url;
use utoipa::ToSchema;
use validator::Validate;

/// OIDC Authorization request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    /// State parameter is required for CSRF protection
    pub state: String,
    pub nonce: Option<String>,
    pub connector_alias: Option<String>,
}

/// Allowed OIDC scopes whitelist
const ALLOWED_SCOPES: &[&str] = &["openid", "profile", "email"];
const OIDC_STATE_TTL_SECS: u64 = 300;

/// Filter and validate scope parameter against whitelist
fn filter_scopes(requested_scope: &str) -> Result<String> {
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
pub async fn authorize<S: HasServices + HasCache>(
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

    let auth_url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
        keycloak_public_url: &state.config().keycloak.public_url,
        realm: &state.config().keycloak.realm,
        response_type: &params.response_type,
        client_id: &state_payload.client_id,
        callback_url: &callback_url,
        scope: &filtered_scope,
        encoded_state: &state_nonce,
        nonce: params.nonce.as_deref(),
        connector_alias: params.connector_alias.as_deref(),
    })?;

    Ok(Redirect::temporary(&auth_url).into_response())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnterpriseSsoDiscoveryResponse {
    pub tenant_id: StringUuid,
    pub tenant_slug: String,
    pub connector_alias: String,
    pub authorize_url: String,
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

    let callback_url = build_callback_url(
        state
            .config()
            .keycloak
            .core_public_url
            .as_deref()
            .unwrap_or(&state.config().jwt.issuer),
    );
    let filtered_scope = filter_scopes(&params.scope)?;

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

    let authorize_url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
        keycloak_public_url: &state.config().keycloak.public_url,
        realm: &state.config().keycloak.realm,
        response_type: &params.response_type,
        client_id: &state_payload.client_id,
        callback_url: &callback_url,
        scope: &filtered_scope,
        encoded_state: &state_nonce,
        nonce: params.nonce.as_deref(),
        connector_alias: Some(&discovery.keycloak_alias),
    })?;

    Ok(Json(SuccessResponse::new(EnterpriseSsoDiscoveryResponse {
        tenant_id: discovery.tenant_id,
        tenant_slug: discovery.tenant_slug,
        connector_alias: discovery.connector_alias,
        authorize_url,
    })))
}

/// OIDC callback handler
#[derive(Debug, Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
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

/// Token endpoint (for client credentials, etc.)
#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TenantTokenExchangeRequest {
    pub tenant_id: String,
    pub service_id: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/token",
    tag = "Identity",
    responses(
        (status = 200, description = "Token response")
    )
)]
pub async fn token<S: HasServices + HasSessionManagement + HasCache + HasAnalytics>(
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

            let state_payload = CallbackState {
                redirect_uri,
                client_id,
                original_state: None,
            };

            let token_response = exchange_code_for_tokens(&state, &state_payload, &code).await?;
            let userinfo = fetch_userinfo(&state, &token_response.access_token).await?;
            let ip_address = extract_client_ip(&headers);
            let user_agent = headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let (action_service_id, service_tenant_id) =
                resolve_service_ids_for_actions(&state, &state_payload.client_id).await;

            let user = match state.user_service().get_by_keycloak_id(&userinfo.sub).await {
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
                                .execute_trigger(service_id, "pre-user-registration", pre_reg_context)
                                .await?;
                        }
                    }

                    let input = crate::domain::CreateUserInput {
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
                                    mfa_enabled: false,
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
                                .execute_trigger(service_id, "post-user-registration", post_reg_context)
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

                if let (Some(service_id), Some(tenant_id)) = (resolved_service_id, resolved_tenant_id) {
                    // Resolve tenant slug/name for ActionContext
                    let (tenant_slug, tenant_name) =
                        resolve_action_tenant_profile(&state, tenant_id).await;

                    let action_context = ActionContext {
                        user: ActionContextUser {
                            id: user.id.to_string(),
                            email: user.email.clone(),
                            display_name: user.display_name.clone(),
                            mfa_enabled: false,
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
                            tracing::warn!("PostLogin action failed for user {}: {}", user.id, e);
                            None
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

            metrics::counter!("auth9_auth_login_total", "result" => "success").increment(1);

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

            let user = match state.user_service().get_by_keycloak_id(&userinfo.sub).await {
                Ok(existing) => existing,
                Err(AppError::NotFound(_)) => {
                    let input = crate::domain::CreateUserInput {
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

                if let (Some(service_id), Some(tenant_id)) = (resolved_service_id, resolved_tenant_id) {
                    let (tenant_slug, tenant_name) =
                        resolve_action_tenant_profile(&state, tenant_id).await;
                    let pre_refresh_context = ActionContext {
                        user: ActionContextUser {
                            id: user.id.to_string(),
                            email: user.email.clone(),
                            display_name: user.display_name.clone(),
                            mfa_enabled: false,
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/tenant-token",
    tag = "Identity",
    responses(
        (status = 200, description = "Tenant access token")
    )
)]
pub async fn tenant_token<S: HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(params): Json<TenantTokenExchangeRequest>,
) -> Result<Response> {
    let tenant_ref = params.tenant_id.trim();
    let service_id = params.service_id.trim();
    if tenant_ref.is_empty() {
        return Err(AppError::BadRequest("Missing tenant_id".to_string()));
    }
    if service_id.is_empty() {
        return Err(AppError::BadRequest("Missing service_id".to_string()));
    }

    let identity_claims = extract_identity_claims_from_headers(&state, &headers)?;
    let user_id = identity_claims
        .sub
        .parse::<StringUuid>()
        .map_err(|_| AppError::Unauthorized("Invalid user ID in identity token".to_string()))?;

    let tenant_id = match tenant_ref.parse::<StringUuid>() {
        Ok(id) => id,
        Err(_) => state.tenant_service().get_by_slug(tenant_ref).await?.id,
    };

    // Verify tenant is active before allowing token exchange
    let tenant = state.tenant_service().get(tenant_id).await?;
    if tenant.status != crate::domain::TenantStatus::Active {
        return Err(AppError::Forbidden(format!(
            "Tenant is not active (status: '{}')",
            tenant.status
        )));
    }

    state
        .rbac_service()
        .ensure_tenant_membership(user_id, tenant_id)
        .await?;

    let service = state
        .client_service()
        .get_by_client_id(service_id)
        .await
        .map_err(|e| match e {
            AppError::NotFound(_) => {
                AppError::Forbidden("Service does not belong to the requested tenant".to_string())
            }
            other => other,
        })?;
    if let Some(service_tenant_id) = service.tenant_id {
        if service_tenant_id != tenant_id {
            return Err(AppError::Forbidden(
                "Service does not belong to the requested tenant".to_string(),
            ));
        }
    }

    let user_roles = state
        .rbac_service()
        .get_user_roles_for_service(user_id, tenant_id, service.id)
        .await?;

    let jwt_manager = state.jwt_manager();
    let access_token = jwt_manager.create_tenant_access_token_with_session(
        *user_id,
        &identity_claims.email,
        *tenant_id,
        service_id,
        user_roles.roles,
        user_roles.permissions,
        identity_claims.sid.clone(),
    )?;
    let refresh_token = jwt_manager.create_refresh_token(*user_id, *tenant_id, service_id)?;

    // Write audit log for tenant token exchange
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "token_exchange.rest.succeeded",
        "token_exchange",
        Some(*tenant_id),
        None,
        Some(serde_json::json!({
            "user_id": user_id.to_string(),
            "tenant_id": tenant_id.to_string(),
            "service_id": service_id,
        })),
    )
    .await;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
        refresh_token: Some(refresh_token),
        id_token: None,
    })
    .into_response())
}

/// Logout endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub client_id: Option<String>,
    pub id_token_hint: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/logout",
    tag = "Identity",
    responses(
        (status = 302, description = "Redirect to logout")
    )
)]
/// GET logout - redirect-only, no session revocation (CSRF-safe).
/// Per OIDC spec, the end_session_endpoint supports GET for browser redirects.
/// Session revocation requires POST with a bearer token.
pub async fn logout_redirect<S: HasServices>(
    State(state): State<S>,
    Query(params): Query<LogoutRequest>,
) -> Result<Response> {
    // Validate post_logout_redirect_uri against the service's logout_uris
    if let Some(ref redirect_uri) = params.post_logout_redirect_uri {
        if let Some(ref client_id) = params.client_id {
            let service = state.client_service().get_by_client_id(client_id).await?;
            if !service.logout_uris.contains(redirect_uri) {
                return Err(AppError::BadRequest(
                    "Invalid post_logout_redirect_uri".to_string(),
                ));
            }
        } else {
            return Err(AppError::BadRequest(
                "client_id is required when post_logout_redirect_uri is specified".to_string(),
            ));
        }
    }

    let logout_url = build_keycloak_logout_url(
        &state.config().keycloak.public_url,
        &state.config().keycloak.realm,
        params.id_token_hint.as_deref(),
        params.post_logout_redirect_uri.as_deref(),
        params.state.as_deref(),
    )?;

    Ok(Redirect::temporary(&logout_url).into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Identity",
    responses(
        (status = 302, description = "Logout and redirect")
    )
)]
/// POST logout - revokes session and redirects to Keycloak.
/// Requires bearer token for session revocation. CSRF-protected by requiring POST.
pub async fn logout<S: HasServices + HasSessionManagement + HasCache>(
    State(state): State<S>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    Query(params): Query<LogoutRequest>,
) -> Result<Response> {
    // Try to revoke session from token before redirecting to Keycloak
    if let Some(TypedHeader(Authorization(bearer))) = auth {
        // Use HasServices::jwt_manager to disambiguate (both traits have jwt_manager)
        match HasServices::jwt_manager(&state).verify_identity_token(bearer.token()) {
            Ok(claims) => {
                if let Some(ref sid) = claims.sid {
                    if let Ok(session_id) = uuid::Uuid::parse_str(sid) {
                        if let Ok(user_id) = uuid::Uuid::parse_str(&claims.sub) {
                            match state
                                .session_service()
                                .revoke_session(session_id.into(), user_id.into())
                                .await
                            {
                                Ok(_) => {
                                    tracing::info!(
                                        user_id = %claims.sub,
                                        session_id = %sid,
                                        "Session revoked successfully on logout"
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        user_id = %claims.sub,
                                        session_id = %sid,
                                        error = %e,
                                        "Failed to revoke session on logout (may already be revoked)"
                                    );
                                }
                            }
                        }
                    }

                    // Add session to token blacklist for immediate revocation
                    // Use remaining token TTL as the blacklist entry's TTL
                    let now = Utc::now().timestamp();
                    let remaining_ttl = if claims.exp > now {
                        (claims.exp - now) as u64
                    } else {
                        0
                    };

                    if remaining_ttl > 0 {
                        if let Err(e) = state
                            .cache()
                            .add_to_token_blacklist(sid, remaining_ttl)
                            .await
                        {
                            tracing::warn!(
                                session_id = %sid,
                                error = %e,
                                "Failed to add session to token blacklist"
                            );
                        } else {
                            tracing::debug!(
                                session_id = %sid,
                                remaining_ttl_secs = remaining_ttl,
                                "Added session to token blacklist"
                            );
                        }
                    }

                    // Clean up all refresh token sessions bound to this session
                    if let Err(e) = state
                        .cache()
                        .remove_all_refresh_sessions_for_session(sid)
                        .await
                    {
                        tracing::warn!(
                            session_id = %sid,
                            error = %e,
                            "Failed to clean up refresh sessions on logout"
                        );
                    } else {
                        tracing::debug!(
                            session_id = %sid,
                            "Cleaned up refresh token sessions on logout"
                        );
                    }
                } else {
                    tracing::debug!("Logout request has valid token but no session ID (sid claim)");
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "Logout request with invalid/expired token");
            }
        }
    } else {
        tracing::debug!("Logout request without authorization header");
    }

    // Validate post_logout_redirect_uri against the service's logout_uris
    if let Some(ref redirect_uri) = params.post_logout_redirect_uri {
        if let Some(ref client_id) = params.client_id {
            let service = state.client_service().get_by_client_id(client_id).await?;
            if !service.logout_uris.contains(redirect_uri) {
                return Err(AppError::BadRequest(
                    "Invalid post_logout_redirect_uri".to_string(),
                ));
            }
        } else {
            // No client_id provided but post_logout_redirect_uri specified — reject
            return Err(AppError::BadRequest(
                "client_id is required when post_logout_redirect_uri is specified".to_string(),
            ));
        }
    }

    let logout_url = build_keycloak_logout_url(
        &state.config().keycloak.public_url,
        &state.config().keycloak.realm,
        params.id_token_hint.as_deref(),
        params.post_logout_redirect_uri.as_deref(),
        params.state.as_deref(),
    )?;

    Ok(Redirect::temporary(&logout_url).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/userinfo",
    tag = "Identity",
    responses(
        (status = 200, description = "User info")
    )
)]
/// UserInfo endpoint
pub async fn userinfo<S: HasServices>(
    State(state): State<S>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Response> {
    let claims = state.jwt_manager().verify_identity_token(auth.token())?;

    Ok(Json(claims).into_response())
}

// ============================================================================
// Helper functions (testable without AppState)
// ============================================================================

/// Resolve service_id and tenant_id for action execution.
/// Returns (service_id, tenant_id) — both optional.
async fn resolve_service_ids_for_actions<S: HasServices>(
    state: &S,
    client_id: &str,
) -> (Option<StringUuid>, Option<StringUuid>) {
    match state.client_service().get_by_client_id(client_id).await {
        Ok(service) => (Some(service.id), service.tenant_id),
        Err(e) => {
            tracing::warn!(
                "Failed to look up service for client_id '{}': {}, skipping actions",
                client_id,
                e
            );
            (None, None)
        }
    }
}

/// Resolve (service_id, tenant_id) for action execution at post-login.
/// Falls back to the user's first tenant membership if no service-level tenant found.
async fn resolve_action_ids<S: HasServices>(
    state: &S,
    client_id: &str,
    user_id: StringUuid,
    service_id: Option<StringUuid>,
    service_tenant_id: Option<StringUuid>,
) -> (Option<StringUuid>, Option<StringUuid>) {
    if let (Some(sid), Some(tid)) = (service_id, service_tenant_id) {
        return (Some(sid), Some(tid));
    }

    match state.user_service().get_user_tenants(user_id).await {
        Ok(tenants) => {
            let active = tenants.into_iter().next();
            if active.is_none() {
                tracing::debug!(
                    "Service '{}' is cross-tenant and user {} has no tenant memberships, skipping actions",
                    client_id,
                    user_id
                );
            }
            (service_id, active.map(|tu| tu.tenant_id))
        }
        Err(e) => {
            tracing::warn!(
                "Failed to look up tenants for user {}: {}, skipping actions",
                user_id,
                e
            );
            (service_id, None)
        }
    }
}

async fn resolve_action_tenant_profile<S: HasServices>(
    state: &S,
    tenant_id: StringUuid,
) -> (String, String) {
    match state.tenant_service().get(tenant_id).await {
        Ok(tenant) => (tenant.slug, tenant.name),
        Err(_) => (String::new(), String::new()),
    }
}

/// Extract client IP address from request headers
/// Checks X-Forwarded-For, X-Real-IP, then falls back to None
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // Check X-Forwarded-For first (may contain multiple IPs)
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            // Take the first IP (original client)
            if let Some(ip) = xff_str.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
    }

    // Check X-Real-IP
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(ip) = xri.to_str() {
            return Some(ip.to_string());
        }
    }

    None
}

#[derive(Debug, Serialize, Deserialize)]
struct CallbackState {
    redirect_uri: String,
    client_id: String,
    original_state: Option<String>,
}

// Legacy helpers kept for unit tests and backward-compatibility checks.
#[cfg(test)]
fn encode_state(state_payload: &CallbackState) -> Result<String> {
    let bytes = serde_json::to_vec(state_payload).map_err(|e| AppError::Internal(e.into()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
fn decode_state(state: Option<&str>) -> Result<CallbackState> {
    let encoded = state.ok_or_else(|| AppError::BadRequest("Missing state".to_string()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| AppError::BadRequest(format!("Invalid state: {}", e)))?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::Internal(e.into()))
}

/// Validate that a redirect URI is allowed for the service
pub fn validate_redirect_uri(allowed_uris: &[String], redirect_uri: &str) -> Result<()> {
    if allowed_uris.contains(&redirect_uri.to_string()) {
        Ok(())
    } else {
        Err(AppError::BadRequest("Invalid redirect_uri".to_string()))
    }
}

/// Build the callback URL from issuer
pub fn build_callback_url(issuer: &str) -> String {
    format!("{}/api/v1/auth/callback", issuer.trim_end_matches('/'))
}

/// Parameters for building Keycloak authorization URL
#[derive(Debug)]
pub struct KeycloakAuthUrlParams<'a> {
    pub keycloak_public_url: &'a str,
    pub realm: &'a str,
    pub response_type: &'a str,
    pub client_id: &'a str,
    pub callback_url: &'a str,
    pub scope: &'a str,
    pub encoded_state: &'a str,
    pub nonce: Option<&'a str>,
    pub connector_alias: Option<&'a str>,
}

/// Build Keycloak authorization URL
pub fn build_keycloak_auth_url(params: &KeycloakAuthUrlParams) -> Result<String> {
    let mut auth_url = Url::parse(&format!(
        "{}/realms/{}/protocol/openid-connect/auth",
        params.keycloak_public_url, params.realm
    ))
    .map_err(|e| AppError::Internal(e.into()))?;

    {
        let mut pairs = auth_url.query_pairs_mut();
        pairs.append_pair("response_type", params.response_type);
        pairs.append_pair("client_id", params.client_id);
        pairs.append_pair("redirect_uri", params.callback_url);
        pairs.append_pair("scope", params.scope);
        pairs.append_pair("state", params.encoded_state);
        if let Some(n) = params.nonce {
            pairs.append_pair("nonce", n);
        }
        if let Some(alias) = params.connector_alias {
            pairs.append_pair("kc_idp_hint", alias);
        }
    }

    Ok(auth_url.to_string())
}

async fn discover_connector_by_domain(
    pool: &sqlx::MySqlPool,
    domain: &str,
) -> Result<EnterpriseSsoDiscoveryResult> {
    let row = sqlx::query(
        r#"
        SELECT c.tenant_id, t.slug as tenant_slug, c.alias as connector_alias,
               c.keycloak_alias, c.provider_type
        FROM enterprise_sso_domains d
        INNER JOIN enterprise_sso_connectors c ON c.id = d.connector_id
        INNER JOIN tenants t ON t.id = c.tenant_id
        WHERE d.domain = ? AND c.enabled = TRUE
        LIMIT 1
        "#,
    )
    .bind(domain.to_lowercase())
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::NotFound(format!(
            "No enterprise SSO connector configured for domain '{}'",
            domain
        ))
    })?;

    Ok(EnterpriseSsoDiscoveryResult {
        tenant_id: row.try_get("tenant_id")?,
        tenant_slug: row.try_get("tenant_slug")?,
        connector_alias: row.try_get("connector_alias")?,
        keycloak_alias: row.try_get("keycloak_alias")?,
        provider_type: row.try_get("provider_type")?,
    })
}

/// Build Keycloak logout URL
pub fn build_keycloak_logout_url(
    keycloak_public_url: &str,
    realm: &str,
    id_token_hint: Option<&str>,
    post_logout_redirect_uri: Option<&str>,
    state: Option<&str>,
) -> Result<String> {
    let mut logout_url = Url::parse(&format!(
        "{}/realms/{}/protocol/openid-connect/logout",
        keycloak_public_url, realm
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

fn extract_identity_claims_from_headers<S: HasServices>(
    state: &S,
    headers: &HeaderMap,
) -> Result<IdentityClaims> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization token".to_string()))?;
    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;
    let token = auth_str.strip_prefix("Bearer ").ok_or_else(|| {
        AppError::Unauthorized("Authorization must use Bearer scheme".to_string())
    })?;

    state
        .jwt_manager()
        .verify_identity_token(token)
        .map_err(|e| AppError::Unauthorized(format!("Invalid identity token: {}", e)))
}

// ============================================================================
// Internal types
// ============================================================================

#[derive(Debug, Deserialize)]
struct KeycloakTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeycloakUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
}

async fn exchange_code_for_tokens<S: HasServices>(
    state: &S,
    callback_state: &CallbackState,
    code: &str,
) -> Result<KeycloakTokenResponse> {
    let kc_client = state
        .keycloak_client()
        .get_client_by_client_id(&callback_state.client_id)
        .await?;
    let client_uuid = kc_client
        .id
        .ok_or_else(|| AppError::Keycloak("Client UUID missing".to_string()))?;

    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        state.config().keycloak.url,
        state.config().keycloak.realm
    );
    let callback_url = format!(
        "{}/api/v1/auth/callback",
        state.config().jwt.issuer.trim_end_matches('/')
    );

    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", callback_state.client_id.clone()),
        ("code", code.to_string()),
        ("redirect_uri", callback_url),
    ];

    // Public clients don't have a secret; only fetch and send secret for confidential clients
    if !kc_client.public_client {
        let client_secret = state
            .keycloak_client()
            .get_client_secret(&client_uuid)
            .await?;
        params.push(("client_secret", client_secret));
    }

    let response = reqwest::Client::new()
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

    // Debug: log raw response for troubleshooting
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Keycloak(format!("Failed to read token response: {}", e)))?;
    tracing::debug!("Token exchange response length: {} bytes", body.len());

    serde_json::from_str(&body)
        .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
}

async fn exchange_refresh_token<S: HasServices>(
    state: &S,
    callback_state: &CallbackState,
    refresh_token: &str,
) -> Result<KeycloakTokenResponse> {
    let kc_client = state
        .keycloak_client()
        .get_client_by_client_id(&callback_state.client_id)
        .await?;
    let client_uuid = kc_client
        .id
        .ok_or_else(|| AppError::Keycloak("Client UUID missing".to_string()))?;

    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        state.config().keycloak.url,
        state.config().keycloak.realm
    );

    let mut params = vec![
        ("grant_type", "refresh_token".to_string()),
        ("client_id", callback_state.client_id.clone()),
        ("refresh_token", refresh_token.to_string()),
    ];

    if !kc_client.public_client {
        let client_secret = state
            .keycloak_client()
            .get_client_secret(&client_uuid)
            .await?;
        params.push(("client_secret", client_secret));
    }

    let response = reqwest::Client::new()
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Keycloak(format!("Failed to refresh token: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Keycloak(format!(
            "Token refresh failed ({}). This endpoint requires a Keycloak refresh_token \
            (obtained from OIDC login), not an Auth9 gRPC refresh_token. \
            Details: {}",
            status, body
        )));
    }

    response
        .json()
        .await
        .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
}

async fn fetch_userinfo<S: HasServices>(state: &S, access_token: &str) -> Result<KeycloakUserInfo> {
    let userinfo_url = format!(
        "{}/realms/{}/protocol/openid-connect/userinfo",
        state.config().keycloak.url,
        state.config().keycloak.realm
    );

    tracing::debug!(
        "Fetching userinfo from {} with token length {}",
        userinfo_url,
        access_token.len()
    );

    let response = reqwest::Client::new()
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

/// OpenID Connect Discovery endpoint
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: Option<String>,
    pub end_session_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub claims_supported: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/.well-known/openid-configuration",
    tag = "Identity",
    responses(
        (status = 200, description = "OpenID configuration")
    )
)]
pub async fn openid_configuration<S: HasServices>(State(state): State<S>) -> impl IntoResponse {
    let base_url = &state.config().jwt.issuer;
    // Always include jwks_uri - it returns empty keys array for HS256 mode
    let jwks_uri = Some(format!("{}/.well-known/jwks.json", base_url));
    let algs = if state.jwt_manager().uses_rsa() {
        vec!["RS256".to_string()]
    } else {
        vec!["HS256".to_string()]
    };

    Json(OpenIdConfiguration {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{}/api/v1/auth/authorize", base_url),
        token_endpoint: format!("{}/api/v1/auth/token", base_url),
        userinfo_endpoint: format!("{}/api/v1/auth/userinfo", base_url),
        jwks_uri,
        end_session_endpoint: format!("{}/api/v1/auth/logout", base_url),
        response_types_supported: vec![
            "code".to_string(),
            "token".to_string(),
            "id_token".to_string(),
        ],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "client_credentials".to_string(),
            "refresh_token".to_string(),
        ],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: algs,
        scopes_supported: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic".to_string(),
            "client_secret_post".to_string(),
        ],
        claims_supported: vec![
            "sub".to_string(),
            "email".to_string(),
            "name".to_string(),
            "iss".to_string(),
            "aud".to_string(),
            "exp".to_string(),
            "iat".to_string(),
        ],
    })
}

#[derive(Debug, Serialize)]
struct Jwks {
    keys: Vec<JwkKey>,
}

#[derive(Debug, Serialize)]
struct JwkKey {
    kty: String,
    #[serde(rename = "use")]
    use_: String,
    alg: String,
    kid: String,
    n: String,
    e: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/jwks.json",
    tag = "Identity",
    responses(
        (status = 200, description = "JSON Web Key Set")
    )
)]
pub async fn jwks<S: HasServices>(State(state): State<S>) -> impl IntoResponse {
    let public_key_pem = match state.jwt_manager().public_key_pem() {
        Some(key) => key,
        None => {
            // Return empty JWKS for HS256 mode (symmetric keys are not exposed via JWKS)
            return Json(Jwks { keys: vec![] }).into_response();
        }
    };

    let public_key = match RsaPublicKey::from_public_key_pem(public_key_pem) {
        Ok(key) => key,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    let mut keys = vec![JwkKey {
        kty: "RSA".to_string(),
        use_: "sig".to_string(),
        alg: "RS256".to_string(),
        kid: "auth9-current".to_string(),
        n,
        e,
    }];

    // Include previous key for rotation support (allows verifying tokens signed with old key)
    if let Some(prev_pem) = state.jwt_manager().previous_public_key_pem() {
        if let Ok(prev_key) = RsaPublicKey::from_public_key_pem(prev_pem) {
            let prev_n =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(prev_key.n().to_bytes_be());
            let prev_e =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(prev_key.e().to_bytes_be());
            keys.push(JwkKey {
                kty: "RSA".to_string(),
                use_: "sig".to_string(),
                alg: "RS256".to_string(),
                kid: "auth9-previous".to_string(),
                n: prev_n,
                e: prev_e,
            });
        }
    }

    Json(Jwks { keys }).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_state_success() {
        let state_payload = CallbackState {
            redirect_uri: "https://example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: Some("original".to_string()),
        };

        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&state_payload).unwrap());

        let result = decode_state(Some(&encoded));
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert_eq!(decoded.redirect_uri, "https://example.com/callback");
        assert_eq!(decoded.client_id, "test-client");
        assert_eq!(decoded.original_state, Some("original".to_string()));
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
    fn test_decode_state_invalid_json() {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not valid json");

        let result = decode_state(Some(&encoded));
        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    #[test]
    fn test_decode_state_without_original_state() {
        let state_payload = CallbackState {
            redirect_uri: "https://example.com/callback".to_string(),
            client_id: "test-client".to_string(),
            original_state: None,
        };

        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&state_payload).unwrap());

        let result = decode_state(Some(&encoded));
        assert!(result.is_ok());
        assert!(result.unwrap().original_state.is_none());
    }

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
        }"#;

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
    fn test_logout_request_full() {
        let json = r#"{
            "client_id": "my-client",
            "id_token_hint": "token123",
            "post_logout_redirect_uri": "https://app.example.com/logged-out",
            "state": "logout-state"
        }"#;

        let request: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.client_id, Some("my-client".to_string()));
        assert_eq!(request.id_token_hint, Some("token123".to_string()));
        assert_eq!(
            request.post_logout_redirect_uri,
            Some("https://app.example.com/logged-out".to_string())
        );
        assert_eq!(request.state, Some("logout-state".to_string()));
    }

    #[test]
    fn test_logout_request_empty() {
        let json = r#"{}"#;

        let request: LogoutRequest = serde_json::from_str(json).unwrap();
        assert!(request.client_id.is_none());
        assert!(request.id_token_hint.is_none());
        assert!(request.post_logout_redirect_uri.is_none());
        assert!(request.state.is_none());
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
    fn test_callback_state_roundtrip() {
        let original = CallbackState {
            redirect_uri: "https://example.com/cb".to_string(),
            client_id: "my-client".to_string(),
            original_state: Some("state123".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded: CallbackState = serde_json::from_str(&json).unwrap();

        assert_eq!(original.redirect_uri, decoded.redirect_uri);
        assert_eq!(original.client_id, decoded.client_id);
        assert_eq!(original.original_state, decoded.original_state);
    }

    #[test]
    fn test_openid_configuration_structure() {
        let config = OpenIdConfiguration {
            issuer: "https://auth9.example.com".to_string(),
            authorization_endpoint: "https://auth9.example.com/api/v1/auth/authorize".to_string(),
            token_endpoint: "https://auth9.example.com/api/v1/auth/token".to_string(),
            userinfo_endpoint: "https://auth9.example.com/api/v1/auth/userinfo".to_string(),
            jwks_uri: Some("https://auth9.example.com/.well-known/jwks.json".to_string()),
            end_session_endpoint: "https://auth9.example.com/api/v1/auth/logout".to_string(),
            response_types_supported: vec!["code".to_string()],
            grant_types_supported: vec!["authorization_code".to_string()],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec!["RS256".to_string()],
            scopes_supported: vec!["openid".to_string()],
            token_endpoint_auth_methods_supported: vec!["client_secret_post".to_string()],
            claims_supported: vec!["sub".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("issuer"));
        assert!(json.contains("authorization_endpoint"));
        assert!(json.contains("jwks_uri"));
    }

    #[test]
    fn test_openid_configuration_without_jwks() {
        let config = OpenIdConfiguration {
            issuer: "https://auth9.example.com".to_string(),
            authorization_endpoint: "https://auth9.example.com/api/v1/auth/authorize".to_string(),
            token_endpoint: "https://auth9.example.com/api/v1/auth/token".to_string(),
            userinfo_endpoint: "https://auth9.example.com/api/v1/auth/userinfo".to_string(),
            jwks_uri: None,
            end_session_endpoint: "https://auth9.example.com/api/v1/auth/logout".to_string(),
            response_types_supported: vec![],
            grant_types_supported: vec![],
            subject_types_supported: vec![],
            id_token_signing_alg_values_supported: vec![],
            scopes_supported: vec![],
            token_endpoint_auth_methods_supported: vec![],
            claims_supported: vec![],
        };

        assert!(config.jwks_uri.is_none());
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
    fn test_decode_state_with_special_characters() {
        let state_payload = CallbackState {
            redirect_uri: "https://example.com/callback?foo=bar&baz=qux".to_string(),
            client_id: "client-with-dashes_and_underscores".to_string(),
            original_state: Some("state with spaces and émojis 🎉".to_string()),
        };

        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&state_payload).unwrap());

        let result = decode_state(Some(&encoded));
        assert!(result.is_ok());

        let decoded = result.unwrap();
        assert!(decoded.redirect_uri.contains("foo=bar"));
        assert!(decoded.original_state.as_ref().unwrap().contains("🎉"));
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
    fn test_openid_configuration_serialization() {
        let config = OpenIdConfiguration {
            issuer: "https://test.example.com".to_string(),
            authorization_endpoint: "https://test.example.com/auth".to_string(),
            token_endpoint: "https://test.example.com/token".to_string(),
            userinfo_endpoint: "https://test.example.com/userinfo".to_string(),
            jwks_uri: Some("https://test.example.com/jwks".to_string()),
            end_session_endpoint: "https://test.example.com/logout".to_string(),
            response_types_supported: vec![
                "code".to_string(),
                "token".to_string(),
                "id_token".to_string(),
            ],
            grant_types_supported: vec![
                "authorization_code".to_string(),
                "client_credentials".to_string(),
            ],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec!["RS256".to_string(), "HS256".to_string()],
            scopes_supported: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ],
            token_endpoint_auth_methods_supported: vec![
                "client_secret_basic".to_string(),
                "client_secret_post".to_string(),
            ],
            claims_supported: vec!["sub".to_string(), "email".to_string(), "name".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: OpenIdConfiguration = serde_json::from_str(&json).unwrap();

        assert_eq!(config.issuer, parsed.issuer);
        assert_eq!(
            config.response_types_supported.len(),
            parsed.response_types_supported.len()
        );
    }

    // ========================================================================
    // Tests for extracted helper functions
    // ========================================================================

    #[test]
    fn test_validate_redirect_uri_valid() {
        let allowed = vec![
            "https://app.example.com/callback".to_string(),
            "https://app.example.com/oauth".to_string(),
        ];
        let result = validate_redirect_uri(&allowed, "https://app.example.com/callback");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_invalid() {
        let allowed = vec!["https://app.example.com/callback".to_string()];
        let result = validate_redirect_uri(&allowed, "https://evil.com/callback");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_redirect_uri_empty_list() {
        let allowed: Vec<String> = vec![];
        let result = validate_redirect_uri(&allowed, "https://any.com/callback");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_callback_url() {
        let url = build_callback_url("https://auth9.example.com");
        assert_eq!(url, "https://auth9.example.com/api/v1/auth/callback");
    }

    #[test]
    fn test_build_callback_url_strips_trailing_slash() {
        let url = build_callback_url("https://auth9.example.com/");
        assert_eq!(url, "https://auth9.example.com/api/v1/auth/callback");
    }

    #[test]
    fn test_encode_state_roundtrip() {
        let state = CallbackState {
            redirect_uri: "https://app.com/cb".to_string(),
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
    fn test_build_keycloak_auth_url() {
        let url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
            keycloak_public_url: "https://keycloak.example.com",
            realm: "my-realm",
            response_type: "code",
            client_id: "my-client",
            callback_url: "https://app.com/callback",
            scope: "openid profile",
            encoded_state: "encoded-state",
            nonce: None,
            connector_alias: None,
        })
        .unwrap();

        assert!(url.contains("keycloak.example.com"));
        assert!(url.contains("my-realm"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-client"));
        assert!(url.contains("scope=openid"));
    }

    #[test]
    fn test_build_keycloak_auth_url_with_nonce() {
        let url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
            keycloak_public_url: "https://keycloak.example.com",
            realm: "test",
            response_type: "code",
            client_id: "client",
            callback_url: "https://app.com/cb",
            scope: "openid",
            encoded_state: "state",
            nonce: Some("my-nonce"),
            connector_alias: None,
        })
        .unwrap();

        assert!(url.contains("nonce=my-nonce"));
    }

    #[test]
    fn test_build_keycloak_logout_url_minimal() {
        let url =
            build_keycloak_logout_url("https://keycloak.example.com", "my-realm", None, None, None)
                .unwrap();

        assert!(url.contains("keycloak.example.com"));
        assert!(url.contains("my-realm"));
        assert!(url.contains("logout"));
        // No query params when all options are None
        assert!(!url.contains("id_token_hint"));
    }

    #[test]
    fn test_build_keycloak_logout_url_full() {
        let url = build_keycloak_logout_url(
            "https://keycloak.example.com",
            "my-realm",
            Some("token-hint"),
            Some("https://app.com/logged-out"),
            Some("logout-state"),
        )
        .unwrap();

        assert!(url.contains("id_token_hint=token-hint"));
        assert!(url.contains("post_logout_redirect_uri="));
        assert!(url.contains("state=logout-state"));
    }

    #[test]
    fn test_build_keycloak_logout_url_partial() {
        // Only id_token_hint
        let url = build_keycloak_logout_url(
            "https://keycloak.example.com",
            "test",
            Some("hint"),
            None,
            None,
        )
        .unwrap();
        assert!(url.contains("id_token_hint=hint"));
        assert!(!url.contains("post_logout_redirect_uri"));

        // Only redirect_uri
        let url = build_keycloak_logout_url(
            "https://keycloak.example.com",
            "test",
            None,
            Some("https://app.com/logout"),
            None,
        )
        .unwrap();
        assert!(!url.contains("id_token_hint"));
        assert!(url.contains("post_logout_redirect_uri="));
    }

    #[test]
    fn test_encode_state_with_empty_original_state() {
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
    fn test_validate_redirect_uri_with_multiple_uris() {
        let allowed = vec![
            "https://app1.com/cb".to_string(),
            "https://app2.com/cb".to_string(),
            "https://app3.com/cb".to_string(),
        ];

        assert!(validate_redirect_uri(&allowed, "https://app1.com/cb").is_ok());
        assert!(validate_redirect_uri(&allowed, "https://app2.com/cb").is_ok());
        assert!(validate_redirect_uri(&allowed, "https://app3.com/cb").is_ok());
        assert!(validate_redirect_uri(&allowed, "https://app4.com/cb").is_err());
    }

    #[test]
    fn test_validate_redirect_uri_exact_match() {
        let allowed = vec!["https://app.com/callback".to_string()];

        // Should not match partial or similar URIs
        assert!(validate_redirect_uri(&allowed, "https://app.com/callback").is_ok());
        assert!(validate_redirect_uri(&allowed, "https://app.com/callback/").is_err());
        assert!(validate_redirect_uri(&allowed, "https://app.com/callback?foo=bar").is_err());
    }

    #[test]
    fn test_build_callback_url_with_path() {
        let url = build_callback_url("https://auth9.example.com/api");
        assert_eq!(url, "https://auth9.example.com/api/api/v1/auth/callback");
    }

    #[test]
    fn test_build_keycloak_auth_url_encodes_special_chars() {
        let url = build_keycloak_auth_url(&KeycloakAuthUrlParams {
            keycloak_public_url: "https://keycloak.example.com",
            realm: "test",
            response_type: "code",
            client_id: "my-app",
            callback_url: "https://app.com/cb?foo=bar",
            scope: "openid profile email",
            encoded_state: "state123",
            nonce: Some("nonce with spaces"),
            connector_alias: None,
        })
        .unwrap();

        // Verify URL encoding
        assert!(
            url.contains("scope=openid+profile+email")
                || url.contains("scope=openid%20profile%20email")
        );
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
    fn test_decode_state_with_empty_string() {
        let result = decode_state(Some(""));
        // Empty string decodes to empty bytes, which is invalid JSON
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_state_with_unicode() {
        let state = CallbackState {
            redirect_uri: "https://例え.jp/callback".to_string(),
            client_id: "日本語クライアント".to_string(),
            original_state: Some("состояние".to_string()),
        };

        let encoded = encode_state(&state).unwrap();
        let decoded = decode_state(Some(&encoded)).unwrap();

        assert_eq!(decoded.redirect_uri, state.redirect_uri);
        assert_eq!(decoded.client_id, state.client_id);
        assert_eq!(decoded.original_state, state.original_state);
    }

    #[test]
    fn test_callback_request_with_special_characters() {
        let json = r#"{"code": "code-with-special-chars!@#$%", "state": "state+with/slash"}"#;
        let request: CallbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.code, "code-with-special-chars!@#$%");
        assert_eq!(request.state, Some("state+with/slash".to_string()));
    }

    #[test]
    fn test_openid_configuration_deserialization() {
        let json = r#"{
            "issuer": "https://test.com",
            "authorization_endpoint": "https://test.com/auth",
            "token_endpoint": "https://test.com/token",
            "userinfo_endpoint": "https://test.com/userinfo",
            "jwks_uri": "https://test.com/jwks",
            "end_session_endpoint": "https://test.com/logout",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code"],
            "subject_types_supported": ["public"],
            "id_token_signing_alg_values_supported": ["RS256"],
            "scopes_supported": ["openid"],
            "token_endpoint_auth_methods_supported": ["client_secret_post"],
            "claims_supported": ["sub"]
        }"#;

        let config: OpenIdConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.issuer, "https://test.com");
        assert_eq!(config.jwks_uri, Some("https://test.com/jwks".to_string()));
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
        }"#;

        let request: TokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.grant_type, "authorization_code");
        assert!(request.client_id.is_some());
        assert!(request.client_secret.is_some());
        assert!(request.code.is_some());
        assert!(request.redirect_uri.is_some());
        assert!(request.refresh_token.is_some());
    }

    #[test]
    fn test_jwks_key_structure() {
        let key = JwkKey {
            kty: "RSA".to_string(),
            use_: "sig".to_string(),
            alg: "RS256".to_string(),
            kid: "key-1".to_string(),
            n: "modulus".to_string(),
            e: "AQAB".to_string(),
        };

        let json = serde_json::to_string(&key).unwrap();
        assert!(json.contains("\"kty\":\"RSA\""));
        assert!(json.contains("\"use\":\"sig\""));
        assert!(json.contains("\"alg\":\"RS256\""));
        assert!(json.contains("\"kid\":\"key-1\""));
        assert!(json.contains("\"n\":\"modulus\""));
        assert!(json.contains("\"e\":\"AQAB\""));
    }

    #[test]
    fn test_jwks_structure() {
        let jwks = Jwks {
            keys: vec![JwkKey {
                kty: "RSA".to_string(),
                use_: "sig".to_string(),
                alg: "RS256".to_string(),
                kid: "default".to_string(),
                n: "n".to_string(),
                e: "e".to_string(),
            }],
        };

        let json = serde_json::to_string(&jwks).unwrap();
        assert!(json.contains("\"keys\""));
        assert!(json.contains("RSA"));
    }

    // ========================================================================
    // Tests for extract_client_ip
    // ========================================================================

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.100".parse().unwrap());
        assert_eq!(
            extract_client_ip(&headers),
            Some("192.168.1.100".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_from_x_forwarded_for_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "10.0.0.1, 192.168.1.1, 172.16.0.1".parse().unwrap(),
        );
        // Should take the first IP (original client)
        assert_eq!(extract_client_ip(&headers), Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.50".parse().unwrap());
        assert_eq!(
            extract_client_ip(&headers),
            Some("203.0.113.50".to_string())
        );
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for_takes_priority() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1".parse().unwrap());
        headers.insert("x-real-ip", "203.0.113.50".parse().unwrap());
        // X-Forwarded-For takes priority over X-Real-IP
        assert_eq!(extract_client_ip(&headers), Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_no_headers() {
        let headers = HeaderMap::new();
        assert_eq!(extract_client_ip(&headers), None);
    }

    #[test]
    fn test_extract_client_ip_ipv6() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "::1".parse().unwrap());
        assert_eq!(extract_client_ip(&headers), Some("::1".to_string()));
    }

    // ========================================================================
    // Additional filter_scopes edge cases
    // ========================================================================

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

    // ========================================================================
    // Jwks empty keys
    // ========================================================================

    #[test]
    fn test_jwks_empty_keys() {
        let jwks = Jwks { keys: vec![] };
        let json = serde_json::to_string(&jwks).unwrap();
        assert!(json.contains("\"keys\":[]"));
    }
}
