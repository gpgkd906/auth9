use crate::domains::integration::api as integration_api;
use crate::domains::integration::context::IntegrationContext;
use axum::{
    routing::{get, post},
    Router,
};

pub fn public_routes<S>() -> Router<S>
where
    S: IntegrationContext,
{
    Router::new().route(
        "/api/v1/keycloak/events",
        post(integration_api::keycloak_event::receive::<S>),
    )
}

pub fn protected_routes<S>() -> Router<S>
where
    S: IntegrationContext,
{
    Router::new()
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks",
            get(integration_api::webhook::list_webhooks::<S>)
                .post(integration_api::webhook::create_webhook::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}",
            get(integration_api::webhook::get_webhook::<S>)
                .put(integration_api::webhook::update_webhook::<S>)
                .delete(integration_api::webhook::delete_webhook::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/test",
            post(integration_api::webhook::test_webhook::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/regenerate-secret",
            post(integration_api::webhook::regenerate_webhook_secret::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions",
            get(integration_api::action::list_actions::<S>)
                .post(integration_api::action::create_action::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/{action_id}",
            get(integration_api::action::get_action::<S>)
                .patch(integration_api::action::update_action::<S>)
                .delete(integration_api::action::delete_action::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/batch",
            post(integration_api::action::batch_upsert_actions::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/{action_id}/test",
            post(integration_api::action::test_action::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/{action_id}/stats",
            get(integration_api::action::get_action_stats::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/logs",
            get(integration_api::action::query_action_logs::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/actions/logs/{log_id}",
            get(integration_api::action::get_action_log::<S>),
        )
        .route(
            "/api/v1/actions/triggers",
            get(integration_api::action::get_triggers::<S>),
        )
}
