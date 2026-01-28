//! Authentication API handlers

use crate::error::{AppError, Result};
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use base64::Engine;
use serde::{Deserialize, Serialize};
use url::Url;

/// OIDC Authorization request
#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub nonce: Option<String>,
}

/// Login redirect (initiates OIDC flow)
pub async fn authorize(
    State(state): State<AppState>,
    Query(params): Query<AuthorizeRequest>,
) -> Result<Response> {
    let service = state
        .client_service
        .get_by_client_id(&params.client_id)
        .await?;

    if !service.redirect_uris.contains(&params.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    let callback_url = format!(
        "{}/api/v1/auth/callback",
        state.config.jwt.issuer.trim_end_matches('/')
    );

    let state_payload = CallbackState {
        redirect_uri: params.redirect_uri,
        client_id: params.client_id,
        original_state: params.state,
    };

    let encoded_state = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&state_payload).map_err(|e| AppError::Internal(e.into()))?);

    let mut auth_url = Url::parse(&format!(
        "{}/realms/{}/protocol/openid-connect/auth",
        state.config.keycloak.url, state.config.keycloak.realm
    ))
    .map_err(|e| AppError::Internal(e.into()))?;

    {
        let mut pairs = auth_url.query_pairs_mut();
        pairs.append_pair("response_type", &params.response_type);
        pairs.append_pair("client_id", &state_payload.client_id);
        pairs.append_pair("redirect_uri", &callback_url);
        pairs.append_pair("scope", &params.scope);
        pairs.append_pair("state", &encoded_state);
        if let Some(nonce) = params.nonce {
            pairs.append_pair("nonce", &nonce);
        }
    }

    Ok(Redirect::temporary(auth_url.as_str()).into_response())
}

/// OIDC callback handler
#[derive(Debug, Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackRequest>,
) -> Result<Response> {
    let state_payload = decode_state(params.state.as_deref())?;

    let token_response = exchange_code_for_tokens(&state, &state_payload, &params.code).await?;
    let userinfo = fetch_userinfo(&state, &token_response.access_token).await?;

    let user = match state.user_service.get_by_keycloak_id(&userinfo.sub).await {
        Ok(existing) => existing,
        Err(AppError::NotFound(_)) => {
            let input = crate::domain::CreateUserInput {
                email: userinfo.email.clone(),
                display_name: userinfo.name.clone(),
                avatar_url: None,
            };
            state.user_service.create(&userinfo.sub, input).await?
        }
        Err(e) => return Err(e),
    };

    let identity_token = state.jwt_manager.create_identity_token(
        user.id,
        &userinfo.email,
        userinfo.name.as_deref(),
    )?;

    let mut redirect_url = Url::parse(&state_payload.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;

    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("access_token", &identity_token);
        pairs.append_pair("token_type", "Bearer");
        pairs.append_pair(
            "expires_in",
            &state.jwt_manager.access_token_ttl().to_string(),
        );
        if let Some(original_state) = state_payload.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    Ok(Redirect::temporary(redirect_url.as_str()).into_response())
}

/// Token endpoint (for client credentials, etc.)
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub refresh_token: Option<String>,
}

pub async fn token(
    State(state): State<AppState>,
    Json(params): Json<TokenRequest>,
) -> Result<Response> {
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

            let user = match state.user_service.get_by_keycloak_id(&userinfo.sub).await {
                Ok(existing) => existing,
                Err(AppError::NotFound(_)) => {
                    let input = crate::domain::CreateUserInput {
                        email: userinfo.email.clone(),
                        display_name: userinfo.name.clone(),
                        avatar_url: None,
                    };
                    state.user_service.create(&userinfo.sub, input).await?
                }
                Err(e) => return Err(e),
            };

            let identity_token = state.jwt_manager.create_identity_token(
                user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
            )?;

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: state.jwt_manager.access_token_ttl(),
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
                .client_service
                .verify_secret(&client_id, &client_secret)
                .await?;

            let email = format!("service+{}@auth9.local", service.client_id);
            let identity_token = state
                .jwt_manager
                .create_identity_token(service.id, &email, None)?;

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: state.jwt_manager.access_token_ttl(),
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

            let user = match state.user_service.get_by_keycloak_id(&userinfo.sub).await {
                Ok(existing) => existing,
                Err(AppError::NotFound(_)) => {
                    let input = crate::domain::CreateUserInput {
                        email: userinfo.email.clone(),
                        display_name: userinfo.name.clone(),
                        avatar_url: None,
                    };
                    state.user_service.create(&userinfo.sub, input).await?
                }
                Err(e) => return Err(e),
            };

            let identity_token = state.jwt_manager.create_identity_token(
                user.id,
                &userinfo.email,
                userinfo.name.as_deref(),
            )?;

            Ok(Json(TokenResponse {
                access_token: identity_token,
                token_type: "Bearer".to_string(),
                expires_in: state.jwt_manager.access_token_ttl(),
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

/// Logout endpoint
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub id_token_hint: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
}

pub async fn logout(
    State(state): State<AppState>,
    Query(params): Query<LogoutRequest>,
) -> Result<Response> {
    let mut logout_url = Url::parse(&format!(
        "{}/realms/{}/protocol/openid-connect/logout",
        state.config.keycloak.url, state.config.keycloak.realm
    ))
    .map_err(|e| AppError::Internal(e.into()))?;

    {
        let mut pairs = logout_url.query_pairs_mut();
        if let Some(id_token_hint) = params.id_token_hint {
            pairs.append_pair("id_token_hint", &id_token_hint);
        }
        if let Some(uri) = params.post_logout_redirect_uri {
            pairs.append_pair("post_logout_redirect_uri", &uri);
        }
        if let Some(state_value) = params.state {
            pairs.append_pair("state", &state_value);
        }
    }

    Ok(Redirect::temporary(logout_url.as_str()).into_response())
}

/// UserInfo endpoint
pub async fn userinfo(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Response> {
    let claims = state.jwt_manager.verify_identity_token(auth.token())?;

    Ok(Json(claims).into_response())
}

#[derive(Debug, Serialize, Deserialize)]
struct CallbackState {
    redirect_uri: String,
    client_id: String,
    original_state: Option<String>,
}

fn decode_state(state: Option<&str>) -> Result<CallbackState> {
    let encoded = state.ok_or_else(|| AppError::BadRequest("Missing state".to_string()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|e| AppError::BadRequest(format!("Invalid state: {}", e)))?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::Internal(e.into()))
}

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

async fn exchange_code_for_tokens(
    state: &AppState,
    callback_state: &CallbackState,
    code: &str,
) -> Result<KeycloakTokenResponse> {
    let client_uuid = state
        .keycloak_client
        .get_client_uuid_by_client_id(&callback_state.client_id)
        .await?;
    let client_secret = state
        .keycloak_client
        .get_client_secret(&client_uuid)
        .await?;

    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        state.config.keycloak.url, state.config.keycloak.realm
    );
    let callback_url = format!(
        "{}/api/v1/auth/callback",
        state.config.jwt.issuer.trim_end_matches('/')
    );

    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", callback_state.client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code),
        ("redirect_uri", callback_url.as_str()),
    ];

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

    response
        .json()
        .await
        .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
}

async fn exchange_refresh_token(
    state: &AppState,
    callback_state: &CallbackState,
    refresh_token: &str,
) -> Result<KeycloakTokenResponse> {
    let client_uuid = state
        .keycloak_client
        .get_client_uuid_by_client_id(&callback_state.client_id)
        .await?;
    let client_secret = state
        .keycloak_client
        .get_client_secret(&client_uuid)
        .await?;

    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        state.config.keycloak.url, state.config.keycloak.realm
    );

    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", callback_state.client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("refresh_token", refresh_token),
    ];

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
            "Failed to refresh token: {} - {}",
            status, body
        )));
    }

    response
        .json()
        .await
        .map_err(|e| AppError::Keycloak(format!("Failed to parse token response: {}", e)))
}

async fn fetch_userinfo(state: &AppState, access_token: &str) -> Result<KeycloakUserInfo> {
    let userinfo_url = format!(
        "{}/realms/{}/protocol/openid-connect/userinfo",
        state.config.keycloak.url, state.config.keycloak.realm
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
#[derive(Debug, Serialize)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub end_session_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub claims_supported: Vec<String>,
}

pub async fn openid_configuration(State(state): State<AppState>) -> impl IntoResponse {
    let base_url = &state.config.jwt.issuer;

    Json(OpenIdConfiguration {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{}/api/v1/auth/authorize", base_url),
        token_endpoint: format!("{}/api/v1/auth/token", base_url),
        userinfo_endpoint: format!("{}/api/v1/auth/userinfo", base_url),
        jwks_uri: format!("{}/.well-known/jwks.json", base_url),
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
        id_token_signing_alg_values_supported: vec!["HS256".to_string(), "RS256".to_string()],
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
