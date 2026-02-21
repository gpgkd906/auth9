//! Prometheus metrics setup and metric definitions

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Install the Prometheus recorder and return a handle for rendering metrics.
pub fn install_prometheus_recorder() -> PrometheusHandle {
    // Default histogram buckets (seconds) for HTTP/gRPC/Redis latency metrics.
    // These match common Prometheus defaults plus sub-millisecond buckets for fast endpoints.
    let buckets = vec![
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets(&buckets)
        .expect("failed to set histogram buckets")
        .install_recorder()
        .expect("failed to install Prometheus recorder")
}

/// Register metric descriptions and emit initial zero values so Prometheus output
/// includes HELP/TYPE lines for all metrics from startup (not just after first use).
pub fn describe_metrics() {
    // HTTP metrics
    describe_counter!("auth9_http_requests_total", "Total number of HTTP requests");
    describe_histogram!(
        "auth9_http_request_duration_seconds",
        "HTTP request duration in seconds"
    );
    describe_gauge!(
        "auth9_http_requests_in_flight",
        "Number of HTTP requests currently being processed"
    );

    // gRPC metrics
    describe_counter!("auth9_grpc_requests_total", "Total number of gRPC requests");
    describe_histogram!(
        "auth9_grpc_request_duration_seconds",
        "gRPC request duration in seconds"
    );

    // Database pool metrics
    describe_gauge!(
        "auth9_db_pool_connections_active",
        "Number of active database connections"
    );
    describe_gauge!(
        "auth9_db_pool_connections_idle",
        "Number of idle database connections"
    );

    // Redis metrics
    describe_counter!(
        "auth9_redis_operations_total",
        "Total number of Redis operations"
    );
    describe_histogram!(
        "auth9_redis_operation_duration_seconds",
        "Redis operation duration in seconds"
    );
    describe_counter!(
        "auth9_keycloak_stream_poll_total",
        "Keycloak Redis Stream poll results by outcome (timeout/empty/messages)"
    );
    describe_counter!(
        "auth9_keycloak_stream_read_errors_total",
        "Keycloak Redis Stream read/connect errors"
    );
    describe_counter!(
        "auth9_keycloak_stream_events_total",
        "Keycloak Redis Stream event processing outcomes"
    );

    // Auth metrics
    describe_counter!("auth9_auth_login_total", "Total number of login attempts");
    describe_counter!(
        "auth9_auth_token_exchange_total",
        "Total number of token exchange requests"
    );
    describe_counter!(
        "auth9_auth_token_validation_total",
        "Total number of token validation requests"
    );
    describe_counter!(
        "auth9_auth_invalid_state_total",
        "Total number of invalid OIDC callback state events"
    );

    // Security metrics
    describe_counter!(
        "auth9_security_alerts_total",
        "Total number of security alerts"
    );
    describe_counter!(
        "auth9_rate_limit_throttled_total",
        "Total number of rate-limited requests"
    );
    describe_counter!(
        "auth9_rate_limit_unavailable_total",
        "Total number of requests fail-closed because rate-limit backend was unavailable"
    );

    // Business metrics
    describe_gauge!("auth9_tenants_active_total", "Number of active tenants");
    describe_gauge!("auth9_users_active_total", "Number of active users");
    describe_gauge!("auth9_sessions_active_total", "Number of active sessions");

    // Action metrics
    describe_counter!(
        "auth9_action_operations_total",
        "Total action CRUD operations"
    );
    describe_histogram!(
        "auth9_action_operation_duration_seconds",
        "Action operation duration in seconds"
    );
    describe_counter!("auth9_action_executions_total", "Total action executions");
    describe_histogram!(
        "auth9_action_execution_duration_seconds",
        "Action execution duration in seconds"
    );
    describe_gauge!("auth9_actions_enabled_total", "Enabled actions per tenant");

    // Emit initial zero values for lazily-registered metrics so that
    // HELP/TYPE lines appear in Prometheus output from startup.
    // Gauges and metrics driven by background tasks (db_pool, business gauges)
    // or by the HTTP middleware (http_requests_*) self-initialise quickly, but
    // counters gated behind specific code-paths need an explicit zero-increment.
    counter!("auth9_grpc_requests_total", "service" => "TokenExchange", "method" => "exchange_token", "status" => "ok").absolute(0);
    histogram!("auth9_grpc_request_duration_seconds", "service" => "TokenExchange", "method" => "exchange_token").record(0.0);
    counter!("auth9_auth_login_total", "result" => "success").absolute(0);
    counter!("auth9_auth_token_exchange_total", "result" => "success").absolute(0);
    counter!("auth9_auth_token_validation_total", "result" => "valid").absolute(0);
    counter!("auth9_auth_invalid_state_total", "reason" => "missing").absolute(0);
    counter!("auth9_security_alerts_total", "type" => "brute_force", "severity" => "high")
        .absolute(0);
    counter!("auth9_rate_limit_throttled_total", "endpoint" => "").absolute(0);
    counter!(
        "auth9_rate_limit_unavailable_total",
        "endpoint" => "POST:/api/v1/auth/token",
        "mode" => "fail_close"
    )
    .absolute(0);
    counter!("auth9_redis_operations_total", "operation" => "get").absolute(0);
    histogram!("auth9_redis_operation_duration_seconds", "operation" => "get").record(0.0);
    counter!("auth9_keycloak_stream_poll_total", "result" => "timeout").absolute(0);
    counter!(
        "auth9_keycloak_stream_read_errors_total",
        "error_type" => "xreadgroup_failed"
    )
    .absolute(0);
    counter!(
        "auth9_keycloak_stream_events_total",
        "result" => "processed"
    )
    .absolute(0);
    gauge!("auth9_http_requests_in_flight").set(0.0);

    // Action metrics initial values
    counter!("auth9_action_operations_total", "operation" => "create", "result" => "success")
        .absolute(0);
    histogram!("auth9_action_operation_duration_seconds", "operation" => "create").record(0.0);
    counter!("auth9_action_executions_total", "trigger" => "post-login", "result" => "success")
        .absolute(0);
    histogram!("auth9_action_execution_duration_seconds", "trigger" => "post-login").record(0.0);
    gauge!("auth9_actions_enabled_total", "tenant_id" => "").set(0.0);
}
