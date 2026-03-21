//! Deno ops for ActionEngine: op_fetch, op_set_timeout, op_console_log
//! Also includes SSRF protection (is_private_ip) and related types.

use crate::models::action::AsyncActionConfig;
use deno_core::OpState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

/// Error type for action ops (implements JsErrorClass for deno_core)
#[derive(Debug, thiserror::Error, deno_error::JsError)]
#[class(generic)]
#[error("{0}")]
pub(super) struct ActionOpError(pub String);

/// Counter for HTTP requests in a single action execution
pub(super) struct RequestCounter(pub usize);

/// Response from op_fetch, serialized back to JS
#[derive(serde::Serialize)]
pub(super) struct FetchResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

#[deno_core::op2(async)]
#[serde]
pub(crate) async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[string] method: String,
    #[serde] headers: HashMap<String, String>,
    #[string] body: String,
) -> std::result::Result<FetchResponse, ActionOpError> {
    op_fetch_impl(state, url, method, headers, body).await
}

pub(super) async fn op_fetch_impl(
    state: Rc<RefCell<OpState>>,
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: String,
) -> std::result::Result<FetchResponse, ActionOpError> {
    // Extract config and client from state
    let (client, config, request_count) = {
        let state = state
            .try_borrow()
            .map_err(|_| ActionOpError("op state is busy (read borrow conflict)".into()))?;
        let client = state.borrow::<reqwest::Client>().clone();
        let config = state.borrow::<AsyncActionConfig>().clone();
        let count = state.borrow::<RequestCounter>().0;
        (client, config, count)
    };

    // Check request limit
    if request_count >= config.max_requests_per_execution {
        return Err(ActionOpError(format!(
            "Request limit exceeded (max {} per execution)",
            config.max_requests_per_execution
        )));
    }

    // Parse URL and check domain
    let parsed_url =
        url::Url::parse(&url).map_err(|e| ActionOpError(format!("Invalid URL: {}", e)))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| ActionOpError("URL has no host".into()))?;

    let host_with_port = if let Some(port) = parsed_url.port() {
        format!("{}:{}", host, port)
    } else {
        host.to_string()
    };

    // Check private IP first (SSRF protection) — specific error before generic allowlist
    if !config.allow_private_ips && is_private_ip(host) {
        return Err(ActionOpError(format!(
            "Requests to private/internal IPs are blocked: {}",
            host
        )));
    }

    // Check allowlist (match on host alone or host:port)
    if !config
        .allowed_domains
        .iter()
        .any(|d| d == host || d == &host_with_port)
    {
        return Err(ActionOpError(format!(
            "Domain '{}' not in allowlist. Allowed: {:?}",
            host, config.allowed_domains
        )));
    }

    // Increment request counter
    {
        let mut state = state
            .try_borrow_mut()
            .map_err(|_| ActionOpError("op state is busy (write borrow conflict)".into()))?;
        state.borrow_mut::<RequestCounter>().0 += 1;
    }

    // Build HTTP request
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| ActionOpError(format!("Invalid HTTP method: {}", e)))?;

    let mut req = client.request(method, &url);

    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }

    if !body.is_empty() {
        req = req.body(body);
    }

    // Execute with per-request timeout
    let response =
        tokio::time::timeout(Duration::from_millis(config.request_timeout_ms), req.send())
            .await
            .map_err(|_| {
                ActionOpError(format!(
                    "Request timed out after {}ms",
                    config.request_timeout_ms
                ))
            })?
            .map_err(|e| ActionOpError(format!("HTTP request failed: {}", e)))?;

    let status = response.status().as_u16();
    let resp_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| ActionOpError(format!("Failed to read body: {}", e)))?;

    // Truncate at max_response_bytes
    let body = if body_bytes.len() > config.max_response_bytes {
        String::from_utf8_lossy(&body_bytes[..config.max_response_bytes]).to_string()
    } else {
        String::from_utf8_lossy(&body_bytes).to_string()
    };

    Ok(FetchResponse {
        status,
        body,
        headers: resp_headers,
    })
}

#[deno_core::op2(async)]
pub(crate) async fn op_set_timeout(
    #[number] delay_ms: u64,
) -> std::result::Result<(), ActionOpError> {
    // Cap at 30 seconds to prevent abuse
    let capped = delay_ms.min(30_000);
    tokio::time::sleep(Duration::from_millis(capped)).await;
    Ok(())
}

#[deno_core::op2]
pub(crate) fn op_console_log(#[serde] args: Vec<String>) {
    tracing::info!("[Action Script] {}", args.join(" "));
}

// Register extension
deno_core::extension!(
    auth9_action_ext,
    ops = [op_fetch, op_set_timeout, op_console_log],
);

// ============================================================
// Private IP blocking (SSRF protection)
// ============================================================

pub(super) fn is_private_ip(host: &str) -> bool {
    use std::net::IpAddr;

    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                v4.is_loopback()      // 127.0.0.0/8
                || v4.is_private()    // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || v4.is_link_local() // 169.254.0.0/16
                || v4.is_unspecified() // 0.0.0.0
            }
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
        }
    } else {
        // Hostname: block common internal names
        host == "localhost" || host.ends_with(".local") || host.ends_with(".internal")
    }
}
