//! SCIM API handlers and response types

pub mod scim_admin;
pub mod scim_bulk;
pub mod scim_discovery;
pub mod scim_groups;
pub mod scim_users;

use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Wrapper that serializes `T` as JSON with `Content-Type: application/scim+json`.
pub struct ScimJson<T>(pub T);

impl<T: Serialize> IntoResponse for ScimJson<T> {
    fn into_response(self) -> Response {
        let mut response = axum::Json(self.0).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/scim+json;charset=utf-8"),
        );
        response
    }
}
