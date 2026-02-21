use crate::domains::platform::api as platform_api;
use crate::domains::platform::context::PlatformContext;
use axum::{
    routing::{get, post},
    Router,
};

pub fn public_routes<S>() -> Router<S>
where
    S: PlatformContext,
{
    Router::new().route(
        "/api/v1/public/branding",
        get(platform_api::branding::get_public_branding::<S>),
    )
}

pub fn protected_routes<S>() -> Router<S>
where
    S: PlatformContext,
{
    Router::new()
        .route(
            "/api/v1/system/email",
            get(platform_api::system_settings::get_email_settings::<S>)
                .put(platform_api::system_settings::update_email_settings::<S>),
        )
        .route(
            "/api/v1/system/email/test",
            post(platform_api::system_settings::test_email_connection::<S>),
        )
        .route(
            "/api/v1/system/email/send-test",
            post(platform_api::system_settings::send_test_email::<S>),
        )
        .route(
            "/api/v1/system/email-templates",
            get(platform_api::email_template::list_templates::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}",
            get(platform_api::email_template::get_template::<S>)
                .put(platform_api::email_template::update_template::<S>)
                .delete(platform_api::email_template::reset_template::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}/preview",
            post(platform_api::email_template::preview_template::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}/send-test",
            post(platform_api::email_template::send_test_email::<S>),
        )
        .route(
            "/api/v1/system/branding",
            get(platform_api::branding::get_branding::<S>)
                .put(platform_api::branding::update_branding::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/branding",
            get(platform_api::branding::get_service_branding::<S>)
                .put(platform_api::branding::update_service_branding::<S>)
                .delete(platform_api::branding::delete_service_branding::<S>),
        )
}
