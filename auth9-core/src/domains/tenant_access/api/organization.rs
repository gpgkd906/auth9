//! Organization self-service API handlers (B2B onboarding)

use crate::api::SuccessResponse;
use crate::domain::{AddUserToTenantInput, CreateOrganizationInput, Tenant};
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::state::HasServices;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
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
        (status = 201, description = "Created")
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
        user_id: auth.user_id,
        tenant_id: tenant.id.into(),
        role_in_tenant: "owner".to_string(),
    };
    state.user_service().add_to_tenant(add_input).await?;

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::new(CreateOrganizationResponse {
            organization: tenant,
        })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct MyTenantsQuery {
    pub service_id: Option<String>,
}

/// GET /api/v1/users/me/tenants
/// Get the authenticated user's tenant memberships.
/// When `service_id` is provided, filters tenants based on the service's tenant_id constraint.
#[utoipa::path(
    get,
    path = "/api/v1/users/me/tenants",
    tag = "Tenant Access",
    params(
        ("service_id" = Option<String>, Query, description = "Filter tenants by service client_id")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_my_tenants<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(query): Query<MyTenantsQuery>,
) -> Result<impl IntoResponse> {
    let user_id = auth.user_id.into();
    let mut tenants = state
        .user_service()
        .get_user_tenants_with_tenant(user_id)
        .await?;

    if let Some(service_client_id) = &query.service_id {
        if let Ok(service) = state
            .client_service()
            .get_by_client_id(service_client_id)
            .await
        {
            if let Some(service_tenant_id) = service.tenant_id {
                tenants.retain(|t| t.tenant_id == service_tenant_id);
            }
        }
    }

    Ok(Json(SuccessResponse::new(tenants)))
}
