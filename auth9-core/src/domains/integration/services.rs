//! Integration domain service facade.

pub use crate::domains::integration::service::action::ActionService;
pub use crate::domains::integration::service::action_engine::ActionEngine;
pub use crate::domains::integration::service::webhook::{
    WebhookEventPublisher, WebhookService, WebhookTestResult,
};
