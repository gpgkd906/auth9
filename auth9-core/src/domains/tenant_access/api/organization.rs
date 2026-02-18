//! Organization self-service API handlers (B2B onboarding)

use crate::api::SuccessResponse;
use crate::domain::{AddUserToTenantInput, CreateOrganizationInput, Tenant};
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::state::HasServices;
use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateOrganizationResponse {
    #[serde(flatten)]
    pub organization: Tenant,
}

/// POST /api/v1/organizations
/// Self-service organization creation for authenticated users.
/// Creates a new tenant and adds the creator as owner.
#[utoipa::path(
    post,
    path = "/api/v1/organizations",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn create_organization<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Json(input): Json<CreateOrganizationInput>,
) -> Result<impl IntoResponse> {
    let creator_email = &auth.email;

    // Create the organization (tenant) with domain-based status
    let tenant = state
        .tenant_service()
        .create_organization(input, creator_email)
        .await?;

    // Add the creator as owner of the new organization
    let add_input = AddUserToTenantInput {
        user_id: auth.user_id.into(),
        tenant_id: tenant.id.into(),
        role_in_tenant: "owner".to_string(),
    };
    state.user_service().add_to_tenant(add_input).await?;

    Ok(Json(SuccessResponse::new(CreateOrganizationResponse {
        organization: tenant,
    })))
}

/// GET /api/v1/users/me/tenants
/// Get the authenticated user's tenant memberships.
#[utoipa::path(
    get,
    path = "/api/v1/users/me/tenants",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_my_tenants<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    let user_id = auth.user_id.into();
    let tenants = state
        .user_service()
        .get_user_tenants_with_tenant(user_id)
        .await?;
    Ok(Json(SuccessResponse::new(tenants)))
}
