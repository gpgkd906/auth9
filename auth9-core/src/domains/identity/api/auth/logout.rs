//! Logout endpoints.

use crate::cache::CacheOperations;
use crate::error::{AppError, Result};
use crate::state::{HasCache, HasServices, HasSessionManagement};
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;

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

    if let Some(ref redirect_uri) = params.post_logout_redirect_uri {
        Ok(Redirect::temporary(redirect_uri).into_response())
    } else {
        Ok(axum::http::StatusCode::OK.into_response())
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Identity",
    responses(
        (status = 302, description = "Logout and redirect")
    )
)]
/// POST logout - revokes session and redirects.
/// Requires bearer token for session revocation. CSRF-protected by requiring POST.
pub async fn logout<S: HasServices + HasSessionManagement + HasCache>(
    State(state): State<S>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    Query(params): Query<LogoutRequest>,
) -> Result<Response> {
    // Try to revoke session from token before redirecting
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
            // No client_id provided but post_logout_redirect_uri specified -- reject
            return Err(AppError::BadRequest(
                "client_id is required when post_logout_redirect_uri is specified".to_string(),
            ));
        }
    }

    if let Some(ref redirect_uri) = params.post_logout_redirect_uri {
        Ok(Redirect::temporary(redirect_uri).into_response())
    } else {
        Ok(axum::http::StatusCode::OK.into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
