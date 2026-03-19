//! Required actions API handlers.
//!
//! Protected endpoints for listing and completing pending required actions.

use crate::domains::identity::service::required_actions::PendingActionResponse;
use crate::error::{AppError, Result};
use crate::http_support::{write_audit_log_generic, MessageResponse};
use crate::middleware::auth::AuthUser;
use crate::models::common::StringUuid;
use crate::state::{HasRequiredActions, HasServices};
use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteActionRequest {
    pub action_id: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/hosted-login/pending-actions",
    tag = "Identity",
    responses(
        (status = 200, description = "List of pending actions", body = Vec<PendingActionResponse>),
    )
)]
/// List pending required actions for the authenticated user.
///
/// GET /api/v1/hosted-login/pending-actions
pub async fn get_pending_actions<S: HasServices + HasRequiredActions>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<Json<Vec<PendingActionResponse>>> {
    let user = state
        .user_service()
        .get(StringUuid::from(auth.user_id))
        .await?;

    let actions = state
        .required_actions_service()
        .get_pending_actions(&user.identity_subject)
        .await?;

    Ok(Json(actions))
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/complete-action",
    tag = "Identity",
    request_body = CompleteActionRequest,
    responses(
        (status = 200, description = "Action completed", body = MessageResponse),
        (status = 404, description = "Action not found"),
    )
)]
/// Complete a pending required action.
///
/// POST /api/v1/hosted-login/complete-action
pub async fn complete_action<S: HasServices + HasRequiredActions>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<CompleteActionRequest>,
) -> Result<Json<MessageResponse>> {
    if input.action_id.is_empty() {
        return Err(AppError::BadRequest("Action ID is required.".to_string()));
    }

    state
        .required_actions_service()
        .complete_action(&input.action_id)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "required_action.completed",
        "user",
        Some(auth.user_id),
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new("Action completed successfully.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_action_request_deserialization() {
        let json = r#"{"action_id": "act-123"}"#;
        let input: CompleteActionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.action_id, "act-123");
    }
}
