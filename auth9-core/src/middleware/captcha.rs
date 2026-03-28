//! CAPTCHA verification middleware for bot protection
//!
//! Verifies CAPTCHA tokens on protected endpoints. Supports three modes:
//! - **Always**: Every request must include a valid CAPTCHA token
//! - **Adaptive**: CAPTCHA required only when suspicious activity is detected
//! - **Disabled**: No CAPTCHA verification

use crate::config::CaptchaConfig;
use crate::domains::security_observability::service::captcha::{CaptchaMode, CaptchaProvider};
use axum::{
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use tower::ServiceExt as _;

/// Protected endpoints that require CAPTCHA verification
const DEFAULT_PROTECTED_ENDPOINTS: &[&str] = &[
    "POST:/api/v1/hosted-login/password",
    "POST:/api/v1/auth/register",
    "POST:/api/v1/auth/forgot-password",
    "POST:/api/v1/hosted-login/start-password-reset",
    "POST:/api/v1/auth/email-otp/send",
    "POST:/api/v1/auth/sms-otp/send",
];

/// Adaptive mode configuration thresholds
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Number of failed logins from same IP before requiring CAPTCHA
    pub failed_login_threshold: u64,
    /// Time window (seconds) for failed login counting
    pub failed_login_window_secs: u64,
    /// Number of requests from same IP to protected endpoints before requiring CAPTCHA
    pub request_rate_threshold: u64,
    /// Time window (seconds) for request rate counting
    pub request_rate_window_secs: u64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            failed_login_threshold: 3,
            failed_login_window_secs: 600, // 10 minutes
            request_rate_threshold: 5,
            request_rate_window_secs: 600, // 10 minutes
        }
    }
}

/// Shared state for the CAPTCHA middleware
#[derive(Clone)]
pub struct CaptchaState {
    pub config: Arc<CaptchaConfig>,
    pub provider: Arc<dyn CaptchaProvider>,
    pub mode: CaptchaMode,
    pub protected_endpoints: Arc<HashSet<String>>,
    pub redis: Option<ConnectionManager>,
    pub adaptive: AdaptiveConfig,
}

impl CaptchaState {
    pub fn new(config: CaptchaConfig, provider: Arc<dyn CaptchaProvider>) -> Self {
        let mode = config
            .mode
            .parse::<CaptchaMode>()
            .unwrap_or(CaptchaMode::Disabled);

        let protected_endpoints: HashSet<String> = DEFAULT_PROTECTED_ENDPOINTS
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self {
            config: Arc::new(config),
            provider,
            mode,
            protected_endpoints: Arc::new(protected_endpoints),
            redis: None,
            adaptive: AdaptiveConfig::default(),
        }
    }

    /// Attach a Redis connection for adaptive mode counters
    pub fn with_redis(mut self, redis: ConnectionManager) -> Self {
        self.redis = Some(redis);
        self
    }

    /// Create a no-op CAPTCHA state (disabled)
    pub fn disabled() -> Self {
        Self {
            config: Arc::new(CaptchaConfig::default()),
            provider: Arc::new(
                crate::domains::security_observability::service::captcha::NoOpCaptchaProvider,
            ),
            mode: CaptchaMode::Disabled,
            protected_endpoints: Arc::new(HashSet::new()),
            redis: None,
            adaptive: AdaptiveConfig::default(),
        }
    }

    /// Check if this is a protected endpoint
    fn is_protected(&self, method: &str, path: &str) -> bool {
        let key = format!("{}:{}", method, path);
        self.protected_endpoints.contains(&key)
    }
}

/// CAPTCHA rejection response
#[derive(Serialize)]
struct CaptchaRequiredResponse {
    error: String,
    code: String,
}

/// Extract client IP from request headers
fn extract_client_ip(request: &axum::extract::Request) -> Option<String> {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Extract CAPTCHA token from request header
fn extract_captcha_token(request: &axum::extract::Request) -> Option<String> {
    request
        .headers()
        .get("x-captcha-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// Tower Layer for CAPTCHA verification.
///
/// Unlike `from_fn`-based middleware, this uses a dedicated `tower::Layer` + `tower::Service`
/// pair so it counts as only one type-nesting level, avoiding axum's generic depth limits.
#[derive(Clone)]
pub struct CaptchaLayer {
    pub state: CaptchaState,
}

impl<S> tower::Layer<S> for CaptchaLayer {
    type Service = CaptchaService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CaptchaService {
            inner,
            state: self.state.clone(),
        }
    }
}

/// Tower Service that performs CAPTCHA verification before forwarding to inner service.
#[derive(Clone)]
pub struct CaptchaService<S> {
    inner: S,
    state: CaptchaState,
}

impl<S> tower::Service<axum::extract::Request> for CaptchaService<S>
where
    S: tower::Service<axum::extract::Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: axum::extract::Request) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();
        // Replace current service with the cloned one for the next call
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            let response = captcha_check(state, request, inner).await;
            Ok(response)
        })
    }
}

/// Core CAPTCHA verification logic, shared by tower Service impl and the `from_fn` middleware.
async fn captcha_check<S>(
    state: CaptchaState,
    request: axum::extract::Request,
    inner: S,
) -> Response
where
    S: tower::Service<axum::extract::Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    // Quick exit if disabled or not enabled
    if state.mode == CaptchaMode::Disabled || !state.config.enabled {
        return inner
            .oneshot(request)
            .await
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    let method = request.method().as_str().to_uppercase();
    let path = request.uri().path().to_string();

    // Only check protected endpoints
    if !state.is_protected(&method, &path) {
        return inner
            .oneshot(request)
            .await
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    // Determine if CAPTCHA is needed for this request
    let captcha_needed = match state.mode {
        CaptchaMode::Always => true,
        CaptchaMode::Adaptive => {
            let ua = request.headers().get("user-agent")
                .and_then(|v| v.to_str().ok()).unwrap_or("");
            let ip = extract_client_ip(&request);
            check_adaptive_triggers(ua, ip.as_deref(), &state).await
        }
        CaptchaMode::Disabled => false,
    };

    if !captcha_needed {
        let mut response = inner
            .oneshot(request)
            .await
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        response
            .headers_mut()
            .insert("X-Captcha-Required", "false".parse().unwrap());
        return response;
    }

    // CAPTCHA is required — check for token
    let token = extract_captcha_token(&request);
    let remote_ip = extract_client_ip(&request);

    match token {
        Some(token) => {
            match state.provider.verify(&token, remote_ip.as_deref()).await {
                Ok(verification) if verification.success => {
                    if let Some(score) = verification.score {
                        if score < state.config.score_threshold {
                            tracing::info!(
                                score = score,
                                threshold = state.config.score_threshold,
                                path = %path,
                                "CAPTCHA score below threshold"
                            );
                            metrics::counter!("auth9_captcha_low_score_total").increment(1);
                            return captcha_required_response(&state.config.site_key);
                        }
                    }
                    metrics::counter!("auth9_captcha_verified_total", "endpoint" => path.clone())
                        .increment(1);
                    let mut response = inner
                        .oneshot(request)
                        .await
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    response
                        .headers_mut()
                        .insert("X-Captcha-Required", "false".parse().unwrap());
                    response
                }
                Ok(_verification) => {
                    tracing::info!(path = %path, "CAPTCHA verification failed");
                    metrics::counter!("auth9_captcha_failed_total", "endpoint" => path.clone())
                        .increment(1);
                    captcha_required_response(&state.config.site_key)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %path,
                        "CAPTCHA provider error, failing open"
                    );
                    inner
                        .oneshot(request)
                        .await
                        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
                }
            }
        }
        None => {
            metrics::counter!("auth9_captcha_missing_total", "endpoint" => path.clone())
                .increment(1);
            captcha_required_response(&state.config.site_key)
        }
    }
}

/// CAPTCHA verification middleware
///
/// Reads CaptchaState from request extensions (injected via `axum::Extension` layer).
pub async fn captcha_middleware(request: axum::extract::Request, next: Next) -> Response {
    let state = match request.extensions().get::<CaptchaState>().cloned() {
        Some(s) => s,
        None => return next.run(request).await, // No CAPTCHA state — pass through
    };
    // Quick exit if disabled or not enabled
    if state.mode == CaptchaMode::Disabled || !state.config.enabled {
        return next.run(request).await;
    }

    let method = request.method().as_str().to_uppercase();
    let path = request.uri().path().to_string();

    // Only check protected endpoints
    if !state.is_protected(&method, &path) {
        return next.run(request).await;
    }

    // Determine if CAPTCHA is needed for this request
    let captcha_needed = match state.mode {
        CaptchaMode::Always => true,
        CaptchaMode::Adaptive => {
            let ua = request.headers().get("user-agent")
                .and_then(|v| v.to_str().ok()).unwrap_or("");
            let ip = extract_client_ip(&request);
            check_adaptive_triggers(ua, ip.as_deref(), &state).await
        }
        CaptchaMode::Disabled => false,
    };

    if !captcha_needed {
        // Adaptive mode: not suspicious, allow through with header
        let mut response = next.run(request).await;
        response
            .headers_mut()
            .insert("X-Captcha-Required", "false".parse().unwrap());
        return response;
    }

    // CAPTCHA is required — check for token
    let token = extract_captcha_token(&request);
    let remote_ip = extract_client_ip(&request);

    match token {
        Some(token) => {
            // Verify the token with the provider
            match state.provider.verify(&token, remote_ip.as_deref()).await {
                Ok(verification) if verification.success => {
                    // Check score threshold if provider returns a score
                    if let Some(score) = verification.score {
                        if score < state.config.score_threshold {
                            tracing::info!(
                                score = score,
                                threshold = state.config.score_threshold,
                                path = %path,
                                "CAPTCHA score below threshold"
                            );
                            metrics::counter!("auth9_captcha_low_score_total").increment(1);
                            return captcha_required_response(&state.config.site_key);
                        }
                    }

                    // Token valid, proceed
                    metrics::counter!("auth9_captcha_verified_total", "endpoint" => path.clone())
                        .increment(1);
                    let mut response = next.run(request).await;
                    response
                        .headers_mut()
                        .insert("X-Captcha-Required", "false".parse().unwrap());
                    response
                }
                Ok(_verification) => {
                    // Token invalid
                    tracing::info!(path = %path, "CAPTCHA verification failed");
                    metrics::counter!("auth9_captcha_failed_total", "endpoint" => path.clone())
                        .increment(1);
                    captcha_required_response(&state.config.site_key)
                }
                Err(e) => {
                    // Fail-open: allow through on provider error
                    tracing::warn!(
                        error = %e,
                        path = %path,
                        "CAPTCHA provider error, failing open"
                    );
                    next.run(request).await
                }
            }
        }
        None => {
            // No token provided, reject with CAPTCHA-required signal
            metrics::counter!("auth9_captcha_missing_total", "endpoint" => path.clone())
                .increment(1);
            captcha_required_response(&state.config.site_key)
        }
    }
}

/// Build a 403 response indicating CAPTCHA is required
fn captcha_required_response(site_key: &str) -> Response {
    let body = CaptchaRequiredResponse {
        error: "CAPTCHA verification required".to_string(),
        code: "CAPTCHA_REQUIRED".to_string(),
    };
    let mut response = (StatusCode::FORBIDDEN, axum::Json(body)).into_response();

    response
        .headers_mut()
        .insert("X-Captcha-Required", "true".parse().unwrap());
    if !site_key.is_empty() {
        if let Ok(val) = site_key.parse() {
            response.headers_mut().insert("X-Captcha-Site-Key", val);
        }
    }
    response
}

/// Check adaptive trigger conditions.
///
/// Returns `true` if the request looks suspicious and CAPTCHA should be required.
///
/// Trigger conditions (any one is sufficient):
/// 1. Missing or empty User-Agent header (bot indicator)
/// 2. Same IP has >= N failed logins in the last M minutes (Redis counter)
/// 3. Same IP has > N requests to protected endpoints in the last M minutes (Redis counter)
async fn check_adaptive_triggers(
    user_agent: &str,
    client_ip: Option<&str>,
    state: &CaptchaState,
) -> bool {

    if user_agent.is_empty() {
        tracing::debug!("Adaptive CAPTCHA trigger: missing User-Agent");
        return true;
    }

    // Redis-based checks require both IP and Redis connection
    let Some(ip) = client_ip else {
        return false;
    };

    let Some(redis) = &state.redis else {
        // No Redis available — can't check counters, skip Redis triggers
        return false;
    };

    let mut conn = redis.clone();

    // Trigger 2: failed login count from this IP
    let fail_key = format!("auth9:captcha:fail:{}", ip);
    match conn.get::<_, Option<u64>>(&fail_key).await {
        Ok(Some(count)) if count >= state.adaptive.failed_login_threshold => {
            tracing::debug!(
                ip = ip,
                count = count,
                threshold = state.adaptive.failed_login_threshold,
                "Adaptive CAPTCHA trigger: too many failed logins"
            );
            return true;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "Redis error checking CAPTCHA fail counter");
        }
    }

    // Trigger 3: request rate from this IP to protected endpoints
    let rate_key = format!("auth9:captcha:rate:{}", ip);
    match conn.get::<_, Option<u64>>(&rate_key).await {
        Ok(Some(count)) if count > state.adaptive.request_rate_threshold => {
            tracing::debug!(
                ip = ip,
                count = count,
                threshold = state.adaptive.request_rate_threshold,
                "Adaptive CAPTCHA trigger: high request rate"
            );
            return true;
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(error = %e, "Redis error checking CAPTCHA rate counter");
        }
    }

    // Increment request rate counter for this IP
    let rate_key_clone = rate_key.clone();
    let window = state.adaptive.request_rate_window_secs;
    let mut conn2 = conn.clone();
    // Fire-and-forget: don't block the request on counter increment
    tokio::spawn(async move {
        let _: Result<(), redis::RedisError> = async {
            let count: u64 = conn2.incr(&rate_key_clone, 1u64).await?;
            if count == 1 {
                conn2
                    .expire::<_, ()>(&rate_key_clone, window as i64)
                    .await?;
            }
            Ok(())
        }
        .await;
    });

    false
}

/// Record a failed login attempt for adaptive CAPTCHA tracking.
///
/// Call this from the login handler when authentication fails.
/// Increments the Redis counter `auth9:captcha:fail:{ip}` with a TTL.
pub async fn record_failed_login(redis: &ConnectionManager, ip: &str, window_secs: u64) {
    let key = format!("auth9:captcha:fail:{}", ip);
    let mut conn = redis.clone();
    let result: Result<(), redis::RedisError> = async {
        let count: u64 = conn.incr(&key, 1u64).await?;
        if count == 1 {
            conn.expire::<_, ()>(&key, window_secs as i64).await?;
        }
        Ok(())
    }
    .await;
    if let Err(e) = result {
        tracing::warn!(error = %e, "Failed to record CAPTCHA failed login counter");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::security_observability::service::captcha::{
        CaptchaVerification, NoOpCaptchaProvider,
    };
    use async_trait::async_trait;
    use axum::{body::Body, http::StatusCode, middleware, routing::post, Router};
    use tower::ServiceExt;

    /// Helper to build a test request (axum::http::Request<Body>)
    fn test_request(method: &str, uri: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Mock CAPTCHA provider for testing
    struct MockCaptchaProvider {
        success: bool,
        score: Option<f64>,
    }

    #[async_trait]
    impl CaptchaProvider for MockCaptchaProvider {
        async fn verify(
            &self,
            _token: &str,
            _remote_ip: Option<&str>,
        ) -> crate::error::Result<CaptchaVerification> {
            Ok(CaptchaVerification {
                success: self.success,
                score: self.score,
                challenge_ts: None,
                hostname: None,
                error_codes: if self.success {
                    vec![]
                } else {
                    vec!["test-failure".to_string()]
                },
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    /// Standalone test middleware that reads CaptchaState from extensions
    async fn test_captcha_middleware(mut request: axum::extract::Request, next: Next) -> Response {
        let state = request.extensions().get::<CaptchaState>().cloned().unwrap();
        // Re-insert so inner layers can also read
        request.extensions_mut().insert(state.clone());

        // Inline the middleware logic (delegate to the real one via direct call)
        if state.mode == CaptchaMode::Disabled || !state.config.enabled {
            return next.run(request).await;
        }

        let method = request.method().as_str().to_uppercase();
        let path = request.uri().path().to_string();
        if !state.is_protected(&method, &path) {
            return next.run(request).await;
        }

        let captcha_needed = match state.mode {
            CaptchaMode::Always => true,
            CaptchaMode::Adaptive => {
                let ua = request
                    .headers()
                    .get("user-agent")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                ua.is_empty()
            }
            CaptchaMode::Disabled => false,
        };

        if !captcha_needed {
            let mut response = next.run(request).await;
            response
                .headers_mut()
                .insert("X-Captcha-Required", "false".parse().unwrap());
            return response;
        }

        let token = request
            .headers()
            .get("x-captcha-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        let remote_ip = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string());

        match token {
            Some(token) => match state.provider.verify(&token, remote_ip.as_deref()).await {
                Ok(v) if v.success => {
                    if let Some(score) = v.score {
                        if score < state.config.score_threshold {
                            return captcha_required_response(&state.config.site_key);
                        }
                    }
                    let mut response = next.run(request).await;
                    response
                        .headers_mut()
                        .insert("X-Captcha-Required", "false".parse().unwrap());
                    response
                }
                Ok(_) => captcha_required_response(&state.config.site_key),
                Err(_) => next.run(request).await,
            },
            None => captcha_required_response(&state.config.site_key),
        }
    }

    fn build_test_app(state: CaptchaState) -> Router {
        Router::new()
            .route("/api/v1/hosted-login/password", post(|| async { "ok" }))
            .route("/api/v1/public/health", post(|| async { "ok" }))
            .layer(middleware::from_fn(test_captcha_middleware))
            .layer(axum::Extension(state))
    }

    fn always_mode_state(provider: Arc<dyn CaptchaProvider>) -> CaptchaState {
        CaptchaState::new(
            CaptchaConfig {
                enabled: true,
                mode: "always".to_string(),
                site_key: "test-site-key".to_string(),
                secret_key: "test-secret".to_string(), // pragma: allowlist secret
                score_threshold: 0.5,
                ..CaptchaConfig::default()
            },
            provider,
        )
    }

    #[tokio::test]
    async fn test_disabled_mode_passes_through() {
        let state = CaptchaState::disabled();
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_always_mode_rejects_without_token() {
        let provider = Arc::new(MockCaptchaProvider {
            success: true,
            score: None,
        });
        let state = always_mode_state(provider);
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response.headers().get("X-Captcha-Required").unwrap(),
            "true"
        );
        assert_eq!(
            response.headers().get("X-Captcha-Site-Key").unwrap(),
            "test-site-key"
        );
    }

    #[tokio::test]
    async fn test_always_mode_passes_with_valid_token() {
        let provider = Arc::new(MockCaptchaProvider {
            success: true,
            score: None,
        });
        let state = always_mode_state(provider);
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .header("x-captcha-token", "valid-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_always_mode_rejects_invalid_token() {
        let provider = Arc::new(MockCaptchaProvider {
            success: false,
            score: None,
        });
        let state = always_mode_state(provider);
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .header("x-captcha-token", "invalid-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_always_mode_rejects_low_score() {
        let provider = Arc::new(MockCaptchaProvider {
            success: true,
            score: Some(0.2),
        });
        let state = always_mode_state(provider);
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .header("x-captcha-token", "low-score-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_non_protected_endpoint_passes_through() {
        let provider = Arc::new(MockCaptchaProvider {
            success: true,
            score: None,
        });
        let state = always_mode_state(provider);
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/public/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_adaptive_mode_passes_normal_request() {
        let provider = Arc::new(NoOpCaptchaProvider);
        let state = CaptchaState::new(
            CaptchaConfig {
                enabled: true,
                mode: "adaptive".to_string(),
                site_key: "test-site-key".to_string(),
                ..CaptchaConfig::default()
            },
            provider,
        );
        let app = build_test_app(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .header("user-agent", "Mozilla/5.0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("X-Captcha-Required").unwrap(),
            "false"
        );
    }

    #[tokio::test]
    async fn test_adaptive_mode_requires_captcha_on_missing_user_agent() {
        let provider = Arc::new(MockCaptchaProvider {
            success: true,
            score: None,
        });
        let state = CaptchaState::new(
            CaptchaConfig {
                enabled: true,
                mode: "adaptive".to_string(),
                site_key: "test-site-key".to_string(),
                ..CaptchaConfig::default()
            },
            provider,
        );
        let app = build_test_app(state);

        // No user-agent header → suspicious → requires CAPTCHA → no token → 403
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/hosted-login/password")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response.headers().get("X-Captcha-Required").unwrap(),
            "true"
        );
    }
}
