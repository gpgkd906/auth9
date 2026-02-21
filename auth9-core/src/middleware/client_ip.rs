//! Middleware that injects `X-Real-IP` header from the socket address
//! when no proxy headers (`X-Forwarded-For`, `X-Real-IP`) are present.
//!
//! This ensures `extract_ip()` always has an IP to read, even for
//! direct connections without a reverse proxy.

use axum::{extract::Request, middleware::Next, response::Response};
use std::net::SocketAddr;

pub async fn inject_client_ip(mut request: Request, next: Next) -> Response {
    let headers = request.headers();
    let has_forwarded = headers.contains_key("x-forwarded-for");
    let has_real_ip = headers.contains_key("x-real-ip");

    if !has_forwarded && !has_real_ip {
        // Try to get ConnectInfo from request extensions (injected by axum::serve)
        if let Some(addr) = request
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
        {
            let ip = addr.0.ip().to_string();
            if let Ok(value) = ip.parse() {
                request.headers_mut().insert("x-real-ip", value);
            }
        }
    }

    next.run(request).await
}
