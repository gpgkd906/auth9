//! Invitation API handlers

use crate::api::{
    deserialize_page, deserialize_per_page, write_audit_log_generic, MessageResponse,
    PaginatedResponse, SuccessResponse,
};
use crate::domain::{
    AddUserToTenantInput, AssignRolesInput, CreateInvitationInput, CreateUserInput,
    InvitationResponse, InvitationStatus, StringUuid,
};
use crate::error::{AppError, Result};
use crate::keycloak::{CreateKeycloakUserInput, KeycloakCredential};
use crate::middleware::auth::{AuthUser, TokenType};
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
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub password: Option<String>,
}

/// Query parameters for listing invitations
#[derive(Debug, Clone, Deserialize)]
pub struct InvitationListQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
    /// Optional status filter (pending, accepted, expired, revoked)
    pub status: Option<InvitationStatus>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// List invitations for a tenant
pub async fn list<S: HasInvitations>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<InvitationListQuery>,
) -> Result<impl IntoResponse> {
    // Service client tokens cannot manage invitations
    if auth.token_type == TokenType::ServiceClient {
        return Err(AppError::Forbidden(
            "Service client tokens cannot manage invitations".to_string(),
        ));
    }
    // Scope: TenantAccess tokens can only list their own tenant's invitations
    if let TokenType::TenantAccess = auth.token_type {
        let token_tenant = auth
            .tenant_id
            .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
        if token_tenant != tenant_id {
            return Err(AppError::Forbidden(
                "Cannot list invitations for another tenant".to_string(),
            ));
        }
    }
    // Identity tokens are only allowed for platform admins
    if auth.token_type == TokenType::Identity
        && !state.config().is_platform_admin_email(&auth.email)
    {
        return Err(AppError::Forbidden(
            "Platform admin required to list invitations".to_string(),
        ));
    }
    let tenant_id = StringUuid::from(tenant_id);

    let (invitations, total) = state
        .invitation_service()
        .list_by_tenant(tenant_id, query.status, query.page, query.per_page)
        .await?;

    // Convert to response type (excludes token_hash)
    let responses: Vec<InvitationResponse> = invitations.into_iter().map(Into::into).collect();

    Ok(Json(PaginatedResponse::new(
        responses,
        query.page,
        query.per_page,
        total,
    )))
}

/// Create a new invitation
pub async fn create<S: HasInvitations>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateInvitationInput>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);

    // Service client tokens cannot create invitations
    if auth.token_type == TokenType::ServiceClient {
        return Err(AppError::Forbidden(
            "Service client tokens cannot create invitations".to_string(),
        ));
    }
    // Authorization: verify caller can manage the target tenant
    if let TokenType::TenantAccess = auth.token_type {
        // Tenant user: must be in this tenant with admin/owner role
        let token_tenant = auth
            .tenant_id
            .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
        if StringUuid::from(token_tenant) != tenant_id {
            return Err(AppError::Forbidden(
                "Cannot create invitations for another tenant".to_string(),
            ));
        }
        let has_admin_role = auth.roles.iter().any(|r| r == "admin" || r == "owner");
        if !has_admin_role {
            return Err(AppError::Forbidden(
                "Admin or owner role required to create invitations".to_string(),
            ));
        }
    }
    // Identity tokens are only allowed for platform admins
    if auth.token_type == TokenType::Identity
        && !state.config().is_platform_admin_email(&auth.email)
    {
        return Err(AppError::Forbidden(
            "Platform admin required to create invitations".to_string(),
        ));
    }

    // Get the actor (inviter) from the JWT (prefer auth.user_id, fallback to header extraction)
    let invited_by = StringUuid::from(auth.user_id);
    let _ = &headers; // Used by audit log in future

    // TODO: Get inviter name from user service
    let inviter_name = "Admin"; // Placeholder

    // Validate that all role_ids exist and belong to services within the target tenant
    for role_id in &input.role_ids {
        let role = state
            .rbac_service()
            .get_role(*role_id)
            .await
            .map_err(|_| AppError::BadRequest(format!("Role '{}' does not exist", role_id)))?;
        let service = state
            .client_service()
            .get(*role.service_id)
            .await
            .map_err(|_| {
                AppError::BadRequest(format!("Service for role '{}' does not exist", role_id))
            })?;
        if let Some(ref svc_tenant_id) = service.tenant_id {
            if *svc_tenant_id != tenant_id {
                return Err(AppError::BadRequest(format!(
                    "Role '{}' belongs to a service in a different tenant",
                    role_id
                )));
            }
        }
    }

    // Prevent inviting users who are already members of the tenant
    match state.user_service().get_by_email(&input.email).await {
        Ok(user) => {
            let tenant_users = state.user_service().get_user_tenants(user.id).await?;
            let already_member = tenant_users.iter().any(|tu| tu.tenant_id == tenant_id);
            if already_member {
                return Err(AppError::Conflict(
                    "User is already a member of this tenant".to_string(),
                ));
            }
        }
        Err(AppError::NotFound(_)) => {}
        Err(err) => return Err(err),
    }

    let invitation = state
        .invitation_service()
        .create(tenant_id, invited_by, inviter_name, input)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "invitation.created",
        "invitation",
        Some(*invitation.id),
        None,
        serde_json::to_value(&InvitationResponse::from(invitation.clone())).ok(),
    )
    .await;

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

    let invitation = state
        .invitation_service()
        .get_by_token(&request.token)
        .await?;

    if !invitation.is_valid() {
        if invitation.is_expired() {
            return Err(AppError::BadRequest("Invitation has expired".to_string()));
        }
        return Err(AppError::BadRequest(format!(
            "Invitation is no longer valid (status: {})",
            invitation.status
        )));
    }

    if let Some(email) = &request.email {
        if email.to_lowercase() != invitation.email.to_lowercase() {
            return Err(AppError::Validation(
                "Email does not match invitation".to_string(),
            ));
        }
    }

    let user = match state.user_service().get_by_email(&invitation.email).await {
        Ok(user) => user,
        Err(AppError::NotFound(_)) => {
            let password = request.password.clone().ok_or_else(|| {
                AppError::BadRequest("User not found. Please register.".to_string())
            })?;

            let credentials = vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: password,
                temporary: false,
            }];

            let keycloak_id = state
                .keycloak_client()
                .create_user(&CreateKeycloakUserInput {
                    username: invitation.email.clone(),
                    email: invitation.email.clone(),
                    first_name: request.display_name.clone(),
                    last_name: None,
                    enabled: true,
                    email_verified: false,
                    credentials: Some(credentials),
                })
                .await?;

            state
                .user_service()
                .create(
                    &keycloak_id,
                    CreateUserInput {
                        email: invitation.email.clone(),
                        display_name: request.display_name.clone(),
                        avatar_url: None,
                    },
                )
                .await?
        }
        Err(err) => return Err(err),
    };

    // Ensure user is a member of the tenant
    let tenant_users = state.user_service().get_user_tenants(user.id).await?;
    let already_member = tenant_users
        .iter()
        .any(|tu| tu.tenant_id == invitation.tenant_id);
    if !already_member {
        let _ = state
            .user_service()
            .add_to_tenant(AddUserToTenantInput {
                user_id: *user.id,
                tenant_id: *invitation.tenant_id,
                role_in_tenant: "member".to_string(),
            })
            .await?;
    }

    // Assign invited roles
    if !invitation.role_ids.is_empty() {
        let role_ids: Vec<uuid::Uuid> = invitation.role_ids.iter().map(|id| **id).collect();
        state
            .rbac_service()
            .assign_roles(
                AssignRolesInput {
                    user_id: *user.id,
                    tenant_id: *invitation.tenant_id,
                    role_ids,
                },
                Some(invitation.invited_by),
            )
            .await?;
    }

    let invitation = state
        .invitation_service()
        .mark_accepted(invitation.id)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &HeaderMap::new(),
        "invitation.accepted",
        "invitation",
        Some(*invitation.id),
        None,
        Some(serde_json::json!({
            "user_id": user.id.to_string(),
            "tenant_id": invitation.tenant_id.to_string(),
        })),
    )
    .await;

    let response: InvitationResponse = invitation.into();

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
        let request: AcceptInvitationRequest = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(request.token.len(), 256);
    }

    #[test]
    fn test_accept_invitation_request_special_chars() {
        let json = r#"{"token": "abc-123_XYZ.token"}"#;
        let request: AcceptInvitationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.token, "abc-123_XYZ.token");
    }

    #[test]
    fn test_create_invitation_input_empty_roles() {
        let json = r#"{
            "email": "test@example.com",
            "role_ids": []
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert!(input.role_ids.is_empty());
    }

    #[test]
    fn test_create_invitation_input_with_custom_expiry() {
        let json = r#"{
            "email": "user@test.com",
            "role_ids": ["00000000-0000-0000-0000-000000000001"],
            "expires_in_hours": 24
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.expires_in_hours, Some(24));
    }

    #[test]
    fn test_create_invitation_input_long_expiry() {
        let json = r#"{
            "email": "user@test.com",
            "role_ids": ["00000000-0000-0000-0000-000000000001"],
            "expires_in_hours": 720
        }"#;

        let input: CreateInvitationInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.expires_in_hours, Some(720)); // 30 days
    }

    #[test]
    fn test_invitation_response_with_multiple_roles() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "multi-role@example.com".to_string(),
            role_ids: vec![
                StringUuid::new_v4(),
                StringUuid::new_v4(),
                StringUuid::new_v4(),
            ],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Pending,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(72),
            accepted_at: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("multi-role@example.com"));
        assert_eq!(response.role_ids.len(), 3);
    }

    #[test]
    fn test_invitation_status_serialization() {
        // All status variants
        let statuses = vec![
            InvitationStatus::Pending,
            InvitationStatus::Accepted,
            InvitationStatus::Expired,
            InvitationStatus::Revoked,
        ];

        let expected = vec!["pending", "accepted", "expired", "revoked"];

        for (status, expected_str) in statuses.into_iter().zip(expected.into_iter()) {
            let json = serde_json::to_string(&status).unwrap();
            assert!(json.contains(expected_str));
        }
    }

    #[test]
    fn test_create_invitation_input_email_validation() {
        // Valid email formats
        let valid_emails = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.com",
            "user@subdomain.example.com",
        ];

        for email in valid_emails {
            let json = format!(
                r#"{{"email": "{}", "role_ids": ["00000000-0000-0000-0000-000000000001"]}}"#,
                email
            );
            let input: CreateInvitationInput = serde_json::from_str(&json).unwrap();
            assert_eq!(input.email, email);
        }
    }

    #[test]
    fn test_invitation_response_timestamps() {
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::hours(72);
        let accepted = now + chrono::Duration::hours(1);

        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "test@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Accepted,
            expires_at: expires,
            accepted_at: Some(accepted),
            created_at: now,
        };

        let json = serde_json::to_string(&response).unwrap();
        // Verify all timestamp fields are present
        assert!(json.contains("expires_at"));
        assert!(json.contains("accepted_at"));
        assert!(json.contains("created_at"));
    }

    #[test]
    fn test_accept_invitation_request_unicode_token() {
        // Tokens should typically be ASCII, but test handling
        let json = r#"{"token": "token-with-émojis-🎉"}"#;
        let request: AcceptInvitationRequest = serde_json::from_str(json).unwrap();
        assert!(request.token.contains("🎉"));
    }

    #[test]
    fn test_invitation_response_empty_roles() {
        let response = InvitationResponse {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: "no-roles@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            status: InvitationStatus::Pending,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(72),
            accepted_at: None,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("role_ids"));
        assert!(json.contains("[]")); // Empty array
    }
}
