//! Rate limiting middleware for REST API
//!
//! Implements sliding window rate limiting with Redis backend.
//! Supports tenant-level and per-client rate limiting.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Default rate limit rule
    pub default: RateLimitRule,
    /// Per-endpoint overrides (key: "METHOD:path")
    #[serde(default)]
    pub endpoints: HashMap<String, RateLimitRule>,
    /// Per-tenant multipliers (key: tenant_id, value: multiplier)
    #[serde(default)]
    pub tenant_multipliers: HashMap<String, f64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default: RateLimitRule::default(),
            endpoints: HashMap::new(),
            tenant_multipliers: HashMap::new(),
        }
    }
}

/// Rate limit rule specifying requests per time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitRule {
    /// Maximum number of requests allowed
    pub requests: u64,
    /// Time window in seconds
    pub window_secs: u64,
}

impl Default for RateLimitRule {
    fn default() -> Self {
        Self {
            requests: 100,
            window_secs: 60,
        }
    }
}

/// Key types for rate limiting
#[derive(Debug, Clone)]
pub enum RateLimitKey {
    /// Rate limit by tenant ID
    Tenant { tenant_id: String },
    /// Rate limit by tenant and client
    TenantClient {
        tenant_id: String,
        client_id: String,
    },
    /// Rate limit by IP address
    Ip { ip: String },
    /// Rate limit by user ID
    User { user_id: String },
}

impl RateLimitKey {
    /// Build the Redis key for this rate limit
    pub fn to_redis_key(&self, endpoint: &str) -> String {
        match self {
            RateLimitKey::Tenant { tenant_id } => {
                format!("auth9:ratelimit:tenant:{}:{}", tenant_id, endpoint)
            }
            RateLimitKey::TenantClient {
                tenant_id,
                client_id,
            } => {
                format!(
                    "auth9:ratelimit:tenant:{}:client:{}:{}",
                    tenant_id, client_id, endpoint
                )
            }
            RateLimitKey::Ip { ip } => {
                format!("auth9:ratelimit:ip:{}:{}", ip, endpoint)
            }
            RateLimitKey::User { user_id } => {
                format!("auth9:ratelimit:user:{}:{}", user_id, endpoint)
            }
        }
    }
}

/// Rate limit state shared across requests
#[derive(Clone)]
pub struct RateLimitState {
    config: Arc<RateLimitConfig>,
    redis: Option<ConnectionManager>,
}

impl RateLimitState {
    /// Create a new rate limit state with Redis backend
    pub fn new(config: RateLimitConfig, redis: ConnectionManager) -> Self {
        Self {
            config: Arc::new(config),
            redis: Some(redis),
        }
    }

    /// Create a no-op rate limit state (for testing or when disabled)
    pub fn noop() -> Self {
        Self {
            config: Arc::new(RateLimitConfig {
                enabled: false,
                ..Default::default()
            }),
            redis: None,
        }
    }

    /// Check if rate limiting is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.redis.is_some()
    }

    /// Get the rate limit rule for an endpoint
    pub fn get_rule(&self, method: &str, path: &str) -> &RateLimitRule {
        let key = format!("{}:{}", method, path);
        self.config
            .endpoints
            .get(&key)
            .unwrap_or(&self.config.default)
    }

    /// Get the tenant multiplier (1.0 if not configured)
    pub fn get_tenant_multiplier(&self, tenant_id: &str) -> f64 {
        self.config
            .tenant_multipliers
            .get(tenant_id)
            .copied()
            .unwrap_or(1.0)
    }

    /// Check rate limit and increment counter using sliding window algorithm
    ///
    /// Returns Ok(remaining) if allowed, Err(retry_after_secs) if rate limited
    pub async fn check_and_increment(
        &self,
        key: &RateLimitKey,
        endpoint: &str,
        tenant_id: Option<&str>,
    ) -> Result<RateLimitResult, RateLimitError> {
        if !self.is_enabled() {
            return Ok(RateLimitResult {
                allowed: true,
                remaining: u64::MAX,
                reset_at: 0,
            });
        }

        let redis = self.redis.as_ref().ok_or(RateLimitError::NotConfigured)?;

        // endpoint is already in "METHOD:path" format, look up directly
        let rule = self
            .config
            .endpoints
            .get(endpoint)
            .unwrap_or(&self.config.default);
        let multiplier = tenant_id
            .map(|t| self.get_tenant_multiplier(t))
            .unwrap_or(1.0);

        let max_requests = (rule.requests as f64 * multiplier) as u64;
        let window_secs = rule.window_secs;

        let redis_key = key.to_redis_key(endpoint);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - window_secs;

        // Use Redis sorted set for sliding window
        // Score = timestamp, Member = unique request ID
        let mut conn = redis.clone();

        // Clean up old entries and count current requests in a transaction
        let request_id = format!("{}:{}", now, uuid::Uuid::new_v4());

        // Remove entries outside the window
        let _: () = conn
            .zrembyscore(&redis_key, 0i64, window_start as i64)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        // Count current requests in window
        let current_count: u64 = conn
            .zcount(&redis_key, window_start as i64, now as i64)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        if current_count >= max_requests {
            // Get the oldest entry to calculate retry-after
            let oldest: Vec<(String, f64)> = conn
                .zrange_withscores(&redis_key, 0, 0)
                .await
                .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

            let retry_after = if let Some((_, score)) = oldest.first() {
                let oldest_time = *score as u64;
                (oldest_time + window_secs).saturating_sub(now)
            } else {
                window_secs
            };

            return Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at: now + retry_after,
            });
        }

        // Add the new request
        let _: () = conn
            .zadd(&redis_key, &request_id, now as i64)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        // Set expiry on the key
        let _: () = conn
            .expire(&redis_key, (window_secs + 1) as i64)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        Ok(RateLimitResult {
            allowed: true,
            remaining: max_requests - current_count - 1,
            reset_at: now + window_secs,
        })
    }
}

/// Result of rate limit check
#[derive(Debug)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in current window
    pub remaining: u64,
    /// Unix timestamp when the rate limit resets
    pub reset_at: u64,
}

/// Rate limit errors
#[derive(Debug)]
pub enum RateLimitError {
    /// Rate limiting not configured
    NotConfigured,
    /// Redis error
    RedisError(String),
}

/// Rate limit exceeded response
#[derive(Debug, Serialize)]
struct RateLimitExceededResponse {
    error: String,
    code: String,
    retry_after: u64,
}

impl IntoResponse for RateLimitExceededResponse {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self).unwrap();
        let mut response = Response::new(body.into());
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response
            .headers_mut()
            .insert("Retry-After", self.retry_after.to_string().parse().unwrap());
        response
            .headers_mut()
            .insert("Content-Type", "application/json".parse().unwrap());
        response
    }
}

/// Axum layer for rate limiting
#[derive(Clone)]
pub struct RateLimitLayer {
    _state: RateLimitState,
}

impl RateLimitLayer {
    pub fn new(state: RateLimitState) -> Self {
        Self { _state: state }
    }
}

/// Rate limiting middleware function
///
/// Extracts rate limit key from request and checks against Redis.
/// Returns 429 Too Many Requests if rate limit is exceeded.
pub async fn rate_limit_middleware(
    State(rate_limit): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !rate_limit.is_enabled() {
        return next.run(request).await;
    }

    // Extract rate limit key from request
    // Priority: tenant_id from header > user_id from token > IP address
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let client_ip = request
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
        .unwrap_or_else(|| "unknown".to_string());

    let key = if let Some(ref tid) = tenant_id {
        RateLimitKey::Tenant {
            tenant_id: tid.clone(),
        }
    } else {
        RateLimitKey::Ip { ip: client_ip }
    };

    let method = request.method().as_str();
    let path = request.uri().path();
    let endpoint = format!("{}:{}", method, path);

    match rate_limit
        .check_and_increment(&key, &endpoint, tenant_id.as_deref())
        .await
    {
        Ok(result) if result.allowed => {
            let mut response = next.run(request).await;

            // Add rate limit headers
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                result.remaining.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                "X-RateLimit-Reset",
                result.reset_at.to_string().parse().unwrap(),
            );

            response
        }
        Ok(result) => {
            // Rate limit exceeded
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let retry_after = result.reset_at.saturating_sub(now);

            RateLimitExceededResponse {
                error: "Rate limit exceeded".to_string(),
                code: "RATE_LIMITED".to_string(),
                retry_after,
            }
            .into_response()
        }
        Err(_) => {
            // On error, allow the request through (fail open)
            next.run(request).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default.requests, 100);
        assert_eq!(config.default.window_secs, 60);
    }

    #[test]
    fn test_rate_limit_rule_default() {
        let rule = RateLimitRule::default();
        assert_eq!(rule.requests, 100);
        assert_eq!(rule.window_secs, 60);
    }

    #[test]
    fn test_rate_limit_key_to_redis_key() {
        let key = RateLimitKey::Tenant {
            tenant_id: "tenant-123".to_string(),
        };
        assert_eq!(
            key.to_redis_key("GET:/api/v1/users"),
            "auth9:ratelimit:tenant:tenant-123:GET:/api/v1/users"
        );

        let key = RateLimitKey::TenantClient {
            tenant_id: "tenant-123".to_string(),
            client_id: "client-456".to_string(),
        };
        assert_eq!(
            key.to_redis_key("POST:/api/v1/auth/token"),
            "auth9:ratelimit:tenant:tenant-123:client:client-456:POST:/api/v1/auth/token"
        );

        let key = RateLimitKey::Ip {
            ip: "192.168.1.1".to_string(),
        };
        assert_eq!(
            key.to_redis_key("GET:/health"),
            "auth9:ratelimit:ip:192.168.1.1:GET:/health"
        );

        let key = RateLimitKey::User {
            user_id: "user-789".to_string(),
        };
        assert_eq!(
            key.to_redis_key("DELETE:/api/v1/users/1"),
            "auth9:ratelimit:user:user-789:DELETE:/api/v1/users/1"
        );
    }

    #[test]
    fn test_rate_limit_state_noop() {
        let state = RateLimitState::noop();
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_rate_limit_config_with_endpoints() {
        let mut endpoints = HashMap::new();
        endpoints.insert(
            "POST:/api/v1/auth/token".to_string(),
            RateLimitRule {
                requests: 10,
                window_secs: 60,
            },
        );

        let config = RateLimitConfig {
            enabled: true,
            default: RateLimitRule::default(),
            endpoints,
            tenant_multipliers: HashMap::new(),
        };

        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };

        let _rule = state.get_rule("POST", "/api/v1/auth/token");
        // Note: get_rule concatenates method:path, so we need to check the right key
        let rule = state
            .config
            .endpoints
            .get("POST:/api/v1/auth/token")
            .unwrap();
        assert_eq!(rule.requests, 10);
    }

    #[test]
    fn test_rate_limit_tenant_multiplier() {
        let mut tenant_multipliers = HashMap::new();
        tenant_multipliers.insert("premium-tenant".to_string(), 2.0);
        tenant_multipliers.insert("enterprise-tenant".to_string(), 5.0);

        let config = RateLimitConfig {
            enabled: true,
            default: RateLimitRule::default(),
            endpoints: HashMap::new(),
            tenant_multipliers,
        };

        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };

        assert_eq!(state.get_tenant_multiplier("premium-tenant"), 2.0);
        assert_eq!(state.get_tenant_multiplier("enterprise-tenant"), 5.0);
        assert_eq!(state.get_tenant_multiplier("normal-tenant"), 1.0);
    }

    #[tokio::test]
    async fn test_rate_limit_noop_always_allows() {
        let state = RateLimitState::noop();

        let key = RateLimitKey::Ip {
            ip: "127.0.0.1".to_string(),
        };

        let result = state
            .check_and_increment(&key, "GET:/test", None)
            .await
            .unwrap();

        assert!(result.allowed);
        assert_eq!(result.remaining, u64::MAX);
    }

    #[test]
    fn test_rate_limit_result_debug() {
        let result = RateLimitResult {
            allowed: true,
            remaining: 99,
            reset_at: 1000000,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("allowed: true"));
        assert!(debug_str.contains("remaining: 99"));
    }

    #[test]
    fn test_rate_limit_exceeded_response() {
        let response = RateLimitExceededResponse {
            error: "Rate limit exceeded".to_string(),
            code: "RATE_LIMITED".to_string(),
            retry_after: 30,
        };

        let http_response = response.into_response();
        assert_eq!(http_response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_rate_limit_config_serialization() {
        let config = RateLimitConfig {
            enabled: true,
            default: RateLimitRule {
                requests: 50,
                window_secs: 30,
            },
            endpoints: HashMap::new(),
            tenant_multipliers: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"requests\":50"));

        let parsed: RateLimitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default.requests, 50);
    }

    #[test]
    fn test_rate_limit_layer_new() {
        let state = RateLimitState::noop();
        let _layer = RateLimitLayer::new(state);
    }

    #[test]
    fn test_rate_limit_state_is_enabled_false_when_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_rate_limit_state_is_enabled_false_without_redis() {
        let config = RateLimitConfig {
            enabled: true,
            ..Default::default()
        };
        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };
        // enabled=true but no redis => still disabled
        assert!(!state.is_enabled());
    }

    #[test]
    fn test_rate_limit_get_rule_default_fallback() {
        let config = RateLimitConfig {
            enabled: true,
            default: RateLimitRule {
                requests: 200,
                window_secs: 120,
            },
            endpoints: HashMap::new(),
            tenant_multipliers: HashMap::new(),
        };
        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };
        // Non-matching endpoint should fall back to default
        let rule = state.get_rule("GET", "/api/v1/unknown");
        assert_eq!(rule.requests, 200);
        assert_eq!(rule.window_secs, 120);
    }

    #[test]
    fn test_rate_limit_get_rule_matching_endpoint() {
        let mut endpoints = HashMap::new();
        endpoints.insert(
            "POST:/api/v1/auth/login".to_string(),
            RateLimitRule {
                requests: 5,
                window_secs: 60,
            },
        );
        let config = RateLimitConfig {
            enabled: true,
            default: RateLimitRule::default(),
            endpoints,
            tenant_multipliers: HashMap::new(),
        };
        let state = RateLimitState {
            config: Arc::new(config),
            redis: None,
        };
        let rule = state.get_rule("POST", "/api/v1/auth/login");
        assert_eq!(rule.requests, 5);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_disabled_passes_through() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let state = RateLimitState::noop();

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_middleware,
            ))
            .with_state(state);

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_rate_limit_error_debug() {
        let err = RateLimitError::NotConfigured;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotConfigured"));

        let err = RateLimitError::RedisError("connection refused".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("connection refused"));
    }

    #[test]
    fn test_rate_limit_config_deserialization() {
        let json = r#"{
            "enabled": true,
            "default": {"requests": 100, "window_secs": 60},
            "endpoints": {"POST:/api/v1/auth/token": {"requests": 10, "window_secs": 30}},
            "tenant_multipliers": {"premium": 2.0}
        }"#;
        let config: RateLimitConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.default.requests, 100);
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.tenant_multipliers.get("premium"), Some(&2.0));
    }

    #[test]
    fn test_rate_limit_exceeded_response_headers() {
        let response = RateLimitExceededResponse {
            error: "Rate limit exceeded".to_string(),
            code: "RATE_LIMITED".to_string(),
            retry_after: 45,
        };

        let http_response = response.into_response();
        assert_eq!(http_response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(http_response.headers().get("Retry-After").unwrap(), "45");
        assert_eq!(
            http_response.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_rate_limit_key_clone() {
        let key = RateLimitKey::Tenant {
            tenant_id: "t-1".to_string(),
        };
        let cloned = key.clone();
        assert_eq!(
            key.to_redis_key("GET:/test"),
            cloned.to_redis_key("GET:/test")
        );
    }

    #[test]
    fn test_rate_limit_state_clone() {
        let state = RateLimitState::noop();
        let cloned = state.clone();
        assert!(!cloned.is_enabled());
    }
}
