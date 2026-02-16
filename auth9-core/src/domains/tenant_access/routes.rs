use crate::domains::tenant_access::api as tenant_access_api;
use crate::domains::tenant_access::context::TenantAccessContext;
use axum::{
    routing::{delete, get, post},
    Router,
};

pub fn public_routes<S>() -> Router<S>
where
    S: TenantAccessContext,
{
    Router::new()
        .route(
            "/api/v1/invitations/accept",
            post(tenant_access_api::invitation::accept::<S>),
        )
        .route(
            "/api/v1/users",
            get(tenant_access_api::user::list::<S>).post(tenant_access_api::user::create::<S>),
        )
}

pub fn protected_routes<S>() -> Router<S>
where
    S: TenantAccessContext,
{
    Router::new()
        .route(
            "/api/v1/tenants",
            get(tenant_access_api::tenant::list::<S>).post(tenant_access_api::tenant::create::<S>),
        )
        .route(
            "/api/v1/tenants/{id}",
            get(tenant_access_api::tenant::get::<S>)
                .put(tenant_access_api::tenant::update::<S>)
                .delete(tenant_access_api::tenant::delete::<S>),
        )
        .route(
            "/api/v1/users/me",
            get(tenant_access_api::user::get_me::<S>).put(tenant_access_api::user::update_me::<S>),
        )
        .route(
            "/api/v1/users/{id}",
            get(tenant_access_api::user::get::<S>)
                .put(tenant_access_api::user::update::<S>)
                .delete(tenant_access_api::user::delete::<S>),
        )
        .route(
            "/api/v1/users/{id}/mfa",
            post(tenant_access_api::user::enable_mfa::<S>)
                .delete(tenant_access_api::user::disable_mfa::<S>),
        )
        .route(
            "/api/v1/users/{id}/tenants",
            get(tenant_access_api::user::get_tenants::<S>)
                .post(tenant_access_api::user::add_to_tenant::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}",
            delete(tenant_access_api::user::remove_from_tenant::<S>)
                .put(tenant_access_api::user::update_role_in_tenant::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/users",
            get(tenant_access_api::user::list_by_tenant::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/invitations",
            get(tenant_access_api::invitation::list::<S>)
                .post(tenant_access_api::invitation::create::<S>),
        )
        .route(
            "/api/v1/invitations/{id}",
            get(tenant_access_api::invitation::get::<S>)
                .delete(tenant_access_api::invitation::delete::<S>),
        )
        .route(
            "/api/v1/invitations/{id}/revoke",
            post(tenant_access_api::invitation::revoke::<S>),
        )
        .route(
            "/api/v1/invitations/{id}/resend",
            post(tenant_access_api::invitation::resend::<S>),
        )
}
