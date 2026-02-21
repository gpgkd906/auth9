//! SCIM provisioning routes

use crate::domains::provisioning::api as scim_api;
use crate::domains::provisioning::context::ProvisioningContext;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// SCIM protocol routes (Bearer Token auth — uses scim_auth_middleware, NOT JWT).
///
/// These routes are mounted separately from public/protected routes in `build_full_router`
/// with `scim_auth_middleware` applied as a layer.
pub fn scim_routes<S>() -> Router<S>
where
    S: ProvisioningContext,
{
    Router::new()
        // Discovery
        .route(
            "/api/v1/scim/v2/ServiceProviderConfig",
            get(scim_api::scim_discovery::service_provider_config::<S>),
        )
        .route(
            "/api/v1/scim/v2/Schemas",
            get(scim_api::scim_discovery::schemas::<S>),
        )
        .route(
            "/api/v1/scim/v2/ResourceTypes",
            get(scim_api::scim_discovery::resource_types::<S>),
        )
        // Users
        .route(
            "/api/v1/scim/v2/Users",
            get(scim_api::scim_users::list_users::<S>).post(scim_api::scim_users::create_user::<S>),
        )
        .route(
            "/api/v1/scim/v2/Users/{id}",
            get(scim_api::scim_users::get_user::<S>)
                .put(scim_api::scim_users::replace_user::<S>)
                .patch(scim_api::scim_users::patch_user::<S>)
                .delete(scim_api::scim_users::delete_user::<S>),
        )
        // Groups
        .route(
            "/api/v1/scim/v2/Groups",
            get(scim_api::scim_groups::list_groups::<S>)
                .post(scim_api::scim_groups::create_group::<S>),
        )
        .route(
            "/api/v1/scim/v2/Groups/{id}",
            get(scim_api::scim_groups::get_group::<S>)
                .put(scim_api::scim_groups::replace_group::<S>)
                .patch(scim_api::scim_groups::patch_group::<S>)
                .delete(scim_api::scim_groups::delete_group::<S>),
        )
        // Bulk
        .route(
            "/api/v1/scim/v2/Bulk",
            post(scim_api::scim_bulk::bulk_operations::<S>),
        )
}

/// Admin/management routes for SCIM (JWT-protected, merged into protected_routes).
pub fn protected_routes<S>() -> Router<S>
where
    S: ProvisioningContext,
{
    Router::new()
        .route(
            "/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/scim/tokens",
            get(scim_api::scim_admin::list_tokens::<S>)
                .post(scim_api::scim_admin::create_token::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/scim/tokens/{token_id}",
            delete(scim_api::scim_admin::revoke_token::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/scim/logs",
            get(scim_api::scim_admin::list_logs::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/scim/group-mappings",
            get(scim_api::scim_admin::list_group_mappings::<S>)
                .put(scim_api::scim_admin::update_group_mappings::<S>),
        )
}
