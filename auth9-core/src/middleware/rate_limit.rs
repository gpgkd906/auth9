//! Rate limiting middleware for REST API
//!
//! Implements sliding window rate limiting with Redis backend.
//! Supports tenant-level and per-client rate limiting.

use crate::jwt::JwtManager;
use axum::{
    body::Body,
    extract::MatchedPath,
    extract::State,
    http::header::AUTHORIZATION,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::{aio::ConnectionManager, Script};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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

/// In-memory fallback rate limiter used when Redis is unavailable.
///
/// Uses a simple sliding window counter per key. Not as precise as the
/// Redis-backed implementation but prevents unbounded request flooding
/// during Redis outages.
#[derive(Clone)]
struct InMemoryRateLimiter {
    /// Map of key -> list of request timestamps (epoch seconds)
    buckets: Arc<Mutex<HashMap<String, Vec<u64>>>>,
}

impl InMemoryRateLimiter {
    fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and record a request. Returns `true` if allowed, `false` if rate-limited.
    fn check(&self, key: &str, max_requests: u64, window_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff = now.saturating_sub(window_secs);

        let mut buckets = self.buckets.lock().unwrap();
        let timestamps = buckets.entry(key.to_string()).or_default();

        // Evict expired entries
        timestamps.retain(|&ts| ts > cutoff);

        if timestamps.len() as u64 >= max_requests {
            return false;
        }
        timestamps.push(now);

        // Periodic cleanup: cap total entries to avoid unbounded growth
        if buckets.len() > 10_000 {
            buckets.retain(|_, v| {
                v.retain(|&ts| ts > cutoff);
                !v.is_empty()
            });
        }

        true
    }
}

/// Rate limit state shared across requests
#[derive(Clone)]
pub struct RateLimitState {
    config: Arc<RateLimitConfig>,
    redis: Option<ConnectionManager>,
    jwt_manager: Option<JwtManager>,
    allowed_audiences: Arc<Vec<String>>,
    is_production: bool,
    fallback: InMemoryRateLimiter,
}

impl RateLimitState {
    /// Create a new rate limit state with Redis backend
    pub fn new(
        config: RateLimitConfig,
        redis: ConnectionManager,
        jwt_manager: JwtManager,
        allowed_audiences: Vec<String>,
        is_production: bool,
    ) -> Self {
        Self {
            config: Arc::new(config),
            redis: Some(redis),
            jwt_manager: Some(jwt_manager),
            allowed_audiences: Arc::new(allowed_audiences),
            is_production,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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

    /// Check rate limit and increment counter using sliding window algorithm.
    ///
    /// Uses a Lua script to atomically clean up, count, and conditionally add
    /// the request to the sorted set. This prevents race conditions where
    /// concurrent requests could bypass the rate limit.
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
        let request_id = format!("{}:{}", now, uuid::Uuid::new_v4());

        let mut conn = redis.clone();

        // Atomic Lua script: clean up expired entries, count current, conditionally add.
        // Returns: [allowed (0/1), current_count, oldest_score_or_0]
        let script = Script::new(
            r#"
            -- Remove entries outside the window
            redis.call('ZREMRANGEBYSCORE', KEYS[1], 0, ARGV[1])
            -- Count current requests in window
            local count = redis.call('ZCARD', KEYS[1])
            if count >= tonumber(ARGV[4]) then
                -- Rate limited: get oldest entry for retry-after calculation
                local oldest = redis.call('ZRANGE', KEYS[1], 0, 0, 'WITHSCORES')
                local oldest_score = 0
                if #oldest >= 2 then
                    oldest_score = tonumber(oldest[2])
                end
                return {0, count, oldest_score}
            end
            -- Allowed: add the new request and set expiry
            redis.call('ZADD', KEYS[1], ARGV[2], ARGV[3])
            redis.call('EXPIRE', KEYS[1], ARGV[5])
            return {1, count, 0}
            "#,
        );

        let result: Vec<i64> = script
            .key(&redis_key)
            .arg(window_start as i64)
            .arg(now as i64)
            .arg(&request_id)
            .arg(max_requests as i64)
            .arg((window_secs + 1) as i64)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        let allowed = result[0] == 1;
        let current_count = result[1] as u64;

        if allowed {
            Ok(RateLimitResult {
                allowed: true,
                remaining: max_requests.saturating_sub(current_count + 1),
                reset_at: now + window_secs,
            })
        } else {
            let oldest_score = result[2] as u64;
            let retry_after = if oldest_score > 0 {
                (oldest_score + window_secs).saturating_sub(now)
            } else {
                window_secs
            };

            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at: now + retry_after,
            })
        }
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

fn extract_client_ip(request: &Request<Body>) -> String {
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
        .unwrap_or_else(|| "unknown".to_string())
}

fn normalize_path(path: &str) -> String {
    if path.contains('{') {
        return path.to_string();
    }
    let normalized: Vec<String> = path
        .split('/')
        .map(|segment| {
            if segment.is_empty() {
                String::new()
            } else if segment.parse::<u64>().is_ok() || uuid::Uuid::parse_str(segment).is_ok() {
                ":id".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect();
    normalized.join("/")
}

fn endpoint_key(request: &Request<Body>) -> String {
    let method = request.method().as_str();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| normalize_path(request.uri().path()));
    format!("{}:{}", method, path)
}

#[cfg(test)]
fn is_sensitive_endpoint(endpoint: &str) -> bool {
    matches!(
        endpoint,
        "POST:/api/v1/auth/token"
            | "POST:/api/v1/auth/forgot-password"
            | "POST:/api/v1/auth/reset-password"
    )
}

fn extract_key_from_verified_token(
    rate_limit: &RateLimitState,
    request: &Request<Body>,
) -> Option<(RateLimitKey, Option<String>)> {
    let jwt_manager = rate_limit.jwt_manager.as_ref()?;
    let auth_value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = auth_value.strip_prefix("Bearer ")?;

    if let Ok(claims) = jwt_manager.verify_identity_token(token) {
        return Some((
            RateLimitKey::User {
                user_id: claims.sub,
            },
            None,
        ));
    }

    if let Ok(claims) =
        jwt_manager.verify_tenant_access_token_strict(token, &rate_limit.allowed_audiences)
    {
        let tenant_id = claims.tenant_id;
        return Some((
            RateLimitKey::TenantClient {
                tenant_id: tenant_id.clone(),
                client_id: claims.sub,
            },
            Some(tenant_id),
        ));
    }

    if !rate_limit.is_production {
        if let Ok(claims) =
            jwt_manager.verify_tenant_access_token_with_optional_audience(token, None)
        {
            let tenant_id = claims.tenant_id;
            return Some((
                RateLimitKey::TenantClient {
                    tenant_id: tenant_id.clone(),
                    client_id: claims.sub,
                },
                Some(tenant_id),
            ));
        }
    }

    None
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

    let endpoint = endpoint_key(&request);
    let (key, tenant_id) =
        if let Some((key, tenant_id)) = extract_key_from_verified_token(&rate_limit, &request) {
            (key, tenant_id)
        } else {
            (
                RateLimitKey::Ip {
                    ip: extract_client_ip(&request),
                },
                None,
            )
        };

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
            metrics::counter!("auth9_rate_limit_throttled_total", "endpoint" => endpoint.clone())
                .increment(1);
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
        Err(e) => {
            tracing::warn!(
                endpoint = %endpoint,
                error = ?e,
                "Redis unavailable for rate limiting, using in-memory fallback"
            );

            // Fail-open: use in-memory fallback rate limiter for all endpoints
            let redis_key = key.to_redis_key(&endpoint);
            let rule = rate_limit
                .config
                .endpoints
                .get(&endpoint)
                .unwrap_or(&rate_limit.config.default);
            let max_requests = rule.requests;

            if rate_limit
                .fallback
                .check(&redis_key, max_requests, rule.window_secs)
            {
                metrics::counter!(
                    "auth9_rate_limit_unavailable_total",
                    "endpoint" => endpoint.clone(),
                    "mode" => "fallback_allow"
                )
                .increment(1);
                next.run(request).await
            } else {
                metrics::counter!(
                    "auth9_rate_limit_unavailable_total",
                    "endpoint" => endpoint.clone(),
                    "mode" => "fallback_throttle"
                )
                .increment(1);
                RateLimitExceededResponse {
                    error: "Rate limit exceeded".to_string(),
                    code: "RATE_LIMITED".to_string(),
                    retry_after: rule.window_secs,
                }
                .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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
            jwt_manager: None,
            allowed_audiences: Arc::new(Vec::new()),
            is_production: true,
            fallback: InMemoryRateLimiter::new(),
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

    // ========================================================================
    // Helper Function Tests
    // ========================================================================

    #[test]
    fn test_extract_client_ip_xff_single() {
        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_extract_client_ip_xff_multiple() {
        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1, 172.16.0.1")
            .body(Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_client_ip_real_ip() {
        let request = Request::builder()
            .uri("/test")
            .header("x-real-ip", "10.0.0.5")
            .body(Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "10.0.0.5");
    }

    #[test]
    fn test_extract_client_ip_unknown() {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "unknown");
    }

    #[test]
    fn test_extract_client_ip_xff_takes_priority() {
        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-for", "1.1.1.1")
            .header("x-real-ip", "2.2.2.2")
            .body(Body::empty())
            .unwrap();
        let ip = extract_client_ip(&request);
        assert_eq!(ip, "1.1.1.1"); // xff takes priority
    }

    #[test]
    fn test_normalize_path_with_uuid() {
        let path = "/api/v1/tenants/550e8400-e29b-41d4-a716-446655440000/users";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/tenants/:id/users");
    }

    #[test]
    fn test_normalize_path_with_numeric_id() {
        let path = "/api/v1/tenants/12345/users";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/tenants/:id/users");
    }

    #[test]
    fn test_normalize_path_with_template() {
        let path = "/api/v1/tenants/{tenant_id}/users";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/tenants/{tenant_id}/users");
    }

    #[test]
    fn test_normalize_path_no_ids() {
        let path = "/api/v1/tenants";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/tenants");
    }

    #[test]
    fn test_is_sensitive_endpoint_true() {
        assert!(is_sensitive_endpoint("POST:/api/v1/auth/token"));
        assert!(is_sensitive_endpoint("POST:/api/v1/auth/forgot-password"));
        assert!(is_sensitive_endpoint("POST:/api/v1/auth/reset-password"));
    }

    #[test]
    fn test_is_sensitive_endpoint_false() {
        assert!(!is_sensitive_endpoint("GET:/api/v1/users"));
        assert!(!is_sensitive_endpoint("POST:/api/v1/tenants"));
        assert!(!is_sensitive_endpoint("GET:/health"));
    }

    #[test]
    fn test_endpoint_key_without_matched_path() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/tenants/550e8400-e29b-41d4-a716-446655440000/users")
            .body(Body::empty())
            .unwrap();
        let key = endpoint_key(&request);
        assert_eq!(key, "GET:/api/v1/tenants/:id/users");
    }

    #[test]
    fn test_endpoint_key_post_method() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/token")
            .body(Body::empty())
            .unwrap();
        let key = endpoint_key(&request);
        assert_eq!(key, "POST:/api/v1/auth/token");
    }

    #[test]
    fn test_extract_key_from_verified_token_no_jwt_manager() {
        let state = RateLimitState::noop(); // jwt_manager is None
        let request = Request::builder()
            .uri("/test")
            .header(AUTHORIZATION, "Bearer some-token")
            .body(Body::empty())
            .unwrap();
        let result = extract_key_from_verified_token(&state, &request);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_key_from_verified_token_no_auth_header() {
        let state = RateLimitState::noop();
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let result = extract_key_from_verified_token(&state, &request);
        assert!(result.is_none());
    }

    #[test]
    fn test_rate_limit_exceeded_response_body() {
        let response = RateLimitExceededResponse {
            error: "Rate limit exceeded".to_string(),
            code: "RATE_LIMITED".to_string(),
            retry_after: 60,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Rate limit exceeded"));
        assert!(json.contains("RATE_LIMITED"));
        assert!(json.contains("60"));
    }

    // ========================================================================
    // In-memory fallback rate limiter tests
    // ========================================================================

    #[test]
    fn test_inmemory_rate_limiter_allows_under_limit() {
        let limiter = InMemoryRateLimiter::new();
        for _ in 0..5 {
            assert!(limiter.check("test:key", 10, 60));
        }
    }

    #[test]
    fn test_inmemory_rate_limiter_blocks_at_limit() {
        let limiter = InMemoryRateLimiter::new();
        for _ in 0..10 {
            assert!(limiter.check("test:key", 10, 60));
        }
        // 11th request should be blocked
        assert!(!limiter.check("test:key", 10, 60));
    }

    #[test]
    fn test_inmemory_rate_limiter_separate_keys() {
        let limiter = InMemoryRateLimiter::new();
        for _ in 0..10 {
            assert!(limiter.check("key-a", 10, 60));
        }
        // key-a is exhausted
        assert!(!limiter.check("key-a", 10, 60));
        // key-b should still be allowed
        assert!(limiter.check("key-b", 10, 60));
    }
}
