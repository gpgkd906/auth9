//! Invitation API handlers

use crate::api::{
    extract_actor_id_generic, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{CreateInvitationInput, InvitationResponse, StringUuid};
use crate::error::{AppError, Result};
use crate::state::HasInvitations;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

/// Request body for accepting an invitation
#[derive(Debug, Clone, Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
}

/// List invitations for a tenant
pub async fn list<S: HasInvitations>(
    State(state): State<S>,
    Path(tenant_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);

    let (invitations, total) = state
        .invitation_service()
        .list_by_tenant(tenant_id, pagination.page, pagination.per_page)
        .await?;

    // Convert to response type (excludes token_hash)
    let responses: Vec<InvitationResponse> = invitations.into_iter().map(Into::into).collect();

    Ok(Json(PaginatedResponse::new(
        responses,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Create a new invitation
pub async fn create<S: HasInvitations>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateInvitationInput>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);

    // Get the actor (inviter) from the JWT
    let invited_by = extract_actor_id_generic(&state, &headers)
        .map(StringUuid::from)
        .ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;

    // TODO: Get inviter name from user service
    let inviter_name = "Admin"; // Placeholder

    let invitation = state
        .invitation_service()
        .create(tenant_id, invited_by, inviter_name, input)
        .await?;

    let response: InvitationResponse = invitation.into();

    Ok((StatusCode::CREATED, Json(SuccessResponse::new(response))))
}

/// Get invitation by ID
pub async fn get<S: HasInvitations>(
    State(state): State<S>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);

    let invitation = state.invitation_service().get(id).await?;
    let response: InvitationResponse = invitation.into();

    Ok(Json(SuccessResponse::new(response)))
}

/// Revoke an invitation
pub async fn revoke<S: HasInvitations>(
    State(state): State<S>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);

    let invitation = state.invitation_service().revoke(id).await?;
    let response: InvitationResponse = invitation.into();

    Ok(Json(SuccessResponse::new(response)))
}

/// Delete an invitation
pub async fn delete<S: HasInvitations>(
    State(state): State<S>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);

    state.invitation_service().delete(id).await?;

    Ok(Json(MessageResponse::new("Invitation deleted")))
}

/// Resend invitation email
pub async fn resend<S: HasInvitations>(
    State(state): State<S>,
    _headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);

    // TODO: Get inviter name from user service
    let inviter_name = "Admin";

    let invitation = state.invitation_service().resend(id, inviter_name).await?;
    let response: InvitationResponse = invitation.into();

    Ok(Json(SuccessResponse::new(response)))
}

/// Accept an invitation (public endpoint)
pub async fn accept<S: HasInvitations>(
    State(state): State<S>,
    Json(request): Json<AcceptInvitationRequest>,
) -> Result<impl IntoResponse> {
    if request.token.is_empty() {
        return Err(AppError::Validation("Token is required".to_string()));
    }

    let invitation = state.invitation_service().accept(&request.token).await?;
    let response: InvitationResponse = invitation.into();

    // The caller should now create the user in Keycloak and assign roles
    // This is typically handled by a frontend flow

    Ok(Json(SuccessResponse::new(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::InvitationStatus;

    #[test]
    fn test_create_invitation_input_deserialization() {
        let json = r#"{
            "email": "user@example.com",
            "role_ids": ["550e8400-e29b-41d4-a716-446655440000"],
            "expires_in_hours": 48
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "user@example.com");
        assert_eq!(input.role_ids.len(), 1);
        assert_eq!(input.expires_in_hours, Some(48));
    }

    #[test]
    fn test_create_invitation_input_minimal() {
        let json = r#"{
            "email": "user@example.com",
            "role_ids": ["550e8400-e29b-41d4-a716-446655440000"]
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert!(input.expires_in_hours.is_none());
    }

    #[test]
    fn test_accept_invitation_request() {
        let json = r#"{"token": "abc123xyz"}"#;
        let request: AcceptInvitationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.token, "abc123xyz");
    }

    #[test]
    fn test_invitation_response_serialization() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "test@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Pending,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(72),
            accepted_at: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("pending"));
        // token_hash should not be in the response
        assert!(!json.contains("token_hash"));
    }

    #[test]
    fn test_invitation_response_accepted() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "accepted@example.com".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Accepted,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(72),
            accepted_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("accepted@example.com"));
        assert!(json.contains("accepted"));
        assert!(json.contains("accepted_at"));
    }

    #[test]
    fn test_invitation_response_revoked() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "revoked@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Revoked,
            expires_at: chrono::Utc::now(),
            accepted_at: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("revoked"));
    }

    #[test]
    fn test_invitation_response_expired() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "expired@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Expired,
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
            accepted_at: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("expired"));
    }

    #[test]
    fn test_create_invitation_input_multiple_roles() {
        let json = r#"{
            "email": "multi-role@example.com",
            "role_ids": [
                "550e8400-e29b-41d4-a716-446655440000",
                "550e8400-e29b-41d4-a716-446655440001",
                "550e8400-e29b-41d4-a716-446655440002"
            ],
            "expires_in_hours": 168
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "multi-role@example.com");
        assert_eq!(input.role_ids.len(), 3);
        assert_eq!(input.expires_in_hours, Some(168));
    }

    #[test]
    fn test_accept_invitation_request_empty_token() {
        let json = r#"{"token": ""}"#;
        let request: AcceptInvitationRequest = serde_json::from_str(json).unwrap();
        assert!(request.token.is_empty());
    }

    #[test]
    fn test_accept_invitation_request_long_token() {
        let long_token = "a".repeat(256);
        let json = format!(r#"{{"token": "{}"}}"#, long_token);
        let request: AcceptInvitationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.token.len(), 256);
    }
}
