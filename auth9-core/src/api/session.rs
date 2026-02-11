//! Session management API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::{SessionInfo, StringUuid};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasServices, HasSessionManagement};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

/// List current user's active sessions
pub async fn list_my_sessions<S: HasSessionManagement>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<SuccessResponse<Vec<SessionInfo>>>, AppError> {
    let (user_id, current_session_id) = extract_session_info(&state, &headers)?;

    let sessions = state
        .session_service()
        .get_user_sessions(user_id, Some(current_session_id))
        .await?;

    Ok(Json(SuccessResponse::new(sessions)))
}

/// Revoke a specific session
pub async fn revoke_session<S: HasSessionManagement>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(session_id): Path<StringUuid>,
) -> Result<Json<MessageResponse>, AppError> {
    let (user_id, _) = extract_session_info(&state, &headers)?;

    state
        .session_service()
        .revoke_session(session_id, user_id)
        .await?;

    Ok(Json(MessageResponse::new("Session revoked successfully.")))
}

/// Revoke all other sessions (except current)
pub async fn revoke_other_sessions<S: HasSessionManagement>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<SuccessResponse<RevokeSessionsResponse>>, AppError> {
    let (user_id, current_session_id) = extract_session_info(&state, &headers)?;

    let count = state
        .session_service()
        .revoke_other_sessions(user_id, current_session_id)
        .await?;

    Ok(Json(SuccessResponse::new(RevokeSessionsResponse {
        revoked_count: count,
    })))
}

/// Admin: Force logout a user (revoke all sessions)
pub async fn force_logout_user<S: HasSessionManagement + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(user_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<RevokeSessionsResponse>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SessionForceLogout,
            scope: ResourceScope::User(user_id),
        },
    )?;

    let count = state.session_service().force_logout_user(user_id).await?;

    Ok(Json(SuccessResponse::new(RevokeSessionsResponse {
        revoked_count: count,
    })))
}

/// Admin: List sessions for a specific user
pub async fn list_user_sessions<S: HasSessionManagement + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(user_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<Vec<SessionInfo>>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SessionForceLogout,
            scope: ResourceScope::User(user_id),
        },
    )?;

    let sessions = state
        .session_service()
        .get_user_sessions_admin(user_id)
        .await?;

    Ok(Json(SuccessResponse::new(sessions)))
}

/// Response for session revocation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RevokeSessionsResponse {
    pub revoked_count: u64,
}

/// Extract user ID and current session ID from JWT token
fn extract_session_info<S: HasSessionManagement>(
    state: &S,
    headers: &HeaderMap,
) -> Result<(StringUuid, StringUuid), AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    // Try identity token first
    if let Ok(claims) = state.jwt_manager().verify_identity_token(token) {
        let user_id = StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))?;

        // Extract session ID from token's 'sid' claim
        let session_id = claims
            .sid
            .as_ref()
            .and_then(|sid| StringUuid::parse_str(sid).ok())
            .ok_or_else(|| {
                AppError::Unauthorized("Unable to identify current session".to_string())
            })?;

        return Ok((user_id, session_id));
    }

    Err(AppError::Unauthorized(
        "Invalid or expired token".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revoke_sessions_response() {
        let response = RevokeSessionsResponse { revoked_count: 5 };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"revoked_count\":5"));
    }
}
