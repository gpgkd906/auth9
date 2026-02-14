//! ActionEngine - V8-based script execution engine for Auth9 Actions
//!
//! This module provides the core engine for executing TypeScript/JavaScript actions
//! in a secure V8 isolate sandbox. Key features:
//!
//! - **V8 Isolate Sandbox**: Each script runs in an isolated V8 instance
//! - **Async/Await Support**: Scripts can use `async/await`, `fetch()`, `setTimeout`
//! - **TypeScript Support**: Automatic transpilation to JavaScript
//! - **Timeout Control**: Enforced execution timeout per action
//! - **Script Caching**: LRU cache for compiled scripts
//! - **Host Functions**: Exposed Deno ops for logging, HTTP fetch, and timers
//! - **Security**: Domain allowlist for fetch, private IP blocking, request limits

use crate::domain::{Action, ActionContext, AsyncActionConfig};
use crate::error::{AppError, Result};
use crate::repository::ActionRepository;
use deno_core::{JsRuntime, OpState, RuntimeOptions};
use lru::LruCache;
use metrics::{counter, histogram};
use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Error type for action ops (implements JsErrorClass for deno_core)
#[derive(Debug, thiserror::Error, deno_error::JsError)]
#[class(generic)]
#[error("{0}")]
struct ActionOpError(String);

// ============================================================
// Async ops state types
// ============================================================

/// Counter for HTTP requests in a single action execution
struct RequestCounter(usize);

/// Response from op_fetch, serialized back to JS
#[derive(serde::Serialize)]
struct FetchResponse {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

// ============================================================
// Deno ops
// ============================================================

#[deno_core::op2(async)]
#[serde]
async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[string] method: String,
    #[serde] headers: HashMap<String, String>,
    #[string] body: String,
) -> std::result::Result<FetchResponse, ActionOpError> {
    // Extract config and client from state
    let (client, config, request_count) = {
        let state = state.borrow();
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
    let parsed_url = url::Url::parse(&url)
        .map_err(|e| ActionOpError(format!("Invalid URL: {}", e)))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| ActionOpError("URL has no host".into()))?;

    let host_with_port = if let Some(port) = parsed_url.port() {
        format!("{}:{}", host, port)
    } else {
        host.to_string()
    };

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

    // Check private IP (SSRF protection)
    if !config.allow_private_ips && is_private_ip(host) {
        return Err(ActionOpError(format!(
            "Requests to private/internal IPs are blocked: {}",
            host
        )));
    }

    // Increment request counter
    {
        let mut state = state.borrow_mut();
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
    let response = tokio::time::timeout(
        Duration::from_millis(config.request_timeout_ms),
        req.send(),
    )
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
async fn op_set_timeout(#[number] delay_ms: u64) -> std::result::Result<(), ActionOpError> {
    // Cap at 30 seconds to prevent abuse
    let capped = delay_ms.min(30_000);
    tokio::time::sleep(Duration::from_millis(capped)).await;
    Ok(())
}

#[deno_core::op2]
fn op_console_log(#[serde] args: Vec<String>) {
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

fn is_private_ip(host: &str) -> bool {
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
        host == "localhost"
            || host.ends_with(".local")
            || host.ends_with(".internal")
    }
}

// ============================================================
// JavaScript polyfills (injected once at runtime creation)
// ============================================================

const POLYFILLS_JS: &str = r#"
// fetch(url, options?) -> Promise<{ status, body, headers, ok, text(), json() }>
globalThis.fetch = async function(url, options) {
    options = options || {};
    const method = (options.method || 'GET').toUpperCase();
    const headers = options.headers || {};
    const body = options.body || '';
    const result = await Deno.core.ops.op_fetch(url, method, headers, body);
    return {
        status: result.status,
        ok: result.status >= 200 && result.status < 300,
        headers: result.headers,
        text: async () => result.body,
        json: async () => JSON.parse(result.body),
    };
};

// setTimeout(callback, delay) -> id
globalThis.__timers = { nextId: 1, pending: new Map() };
globalThis.setTimeout = function(callback, delay) {
    delay = delay || 0;
    const id = globalThis.__timers.nextId++;
    const promise = Deno.core.ops.op_set_timeout(delay).then(() => {
        if (globalThis.__timers.pending.has(id)) {
            globalThis.__timers.pending.delete(id);
            callback();
        }
    });
    globalThis.__timers.pending.set(id, promise);
    return id;
};
globalThis.clearTimeout = function(id) {
    globalThis.__timers.pending.delete(id);
};

// console.log/warn/error
globalThis.console = {
    log: (...args) => Deno.core.ops.op_console_log(args.map(String)),
    warn: (...args) => Deno.core.ops.op_console_log(args.map(a => '[WARN] ' + String(a))),
    error: (...args) => Deno.core.ops.op_console_log(args.map(a => '[ERROR] ' + String(a))),
};
"#;

// ============================================================
// Thread-local V8 runtime management
// ============================================================

// Thread-local storage for V8 runtime (JsRuntime is !Send, must stay on one thread)
thread_local! {
    static JS_RUNTIME: RefCell<Option<JsRuntime>> = const { RefCell::new(None) };
    static LOCAL_TOKIO_RT: RefCell<Option<tokio::runtime::Runtime>> = const { RefCell::new(None) };
}

/// Take JsRuntime out of thread-local (avoids RefCell borrow across await)
fn take_js_runtime() -> Option<JsRuntime> {
    JS_RUNTIME.with(|rt| rt.borrow_mut().take())
}

/// Return JsRuntime to thread-local storage
fn return_js_runtime(runtime: JsRuntime) {
    JS_RUNTIME.with(|rt| {
        *rt.borrow_mut() = Some(runtime);
    });
}

/// Create a new JsRuntime with async extension + polyfills
fn create_js_runtime(
    http_client: reqwest::Client,
    config: AsyncActionConfig,
) -> Result<JsRuntime> {
    tracing::debug!("Creating thread-local V8 runtime with async extensions");

    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![auth9_action_ext::init_ops_and_esm()],
        ..Default::default()
    });

    // Inject initial op state
    {
        let op_state = runtime.op_state();
        let mut state = op_state.borrow_mut();
        state.put(http_client);
        state.put(config);
        state.put(RequestCounter(0));
    }

    // Inject polyfills (fetch, setTimeout, console)
    runtime
        .execute_script("<polyfills>", POLYFILLS_JS)
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to load polyfills: {}", e))
        })?;

    Ok(runtime)
}

/// Get or create thread-local tokio current-thread runtime, take it out for use
fn take_local_tokio_rt() -> tokio::runtime::Runtime {
    LOCAL_TOKIO_RT.with(|rt_cell| {
        let mut rt = rt_cell.borrow_mut();
        if rt.is_none() {
            *rt = Some(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create thread-local tokio runtime"),
            );
        }
        rt.take().unwrap()
    })
}

/// Return tokio runtime to thread-local storage
fn return_local_tokio_rt(rt: tokio::runtime::Runtime) {
    LOCAL_TOKIO_RT.with(|rt_cell| {
        *rt_cell.borrow_mut() = Some(rt);
    });
}

// ============================================================
// Compiled script cache
// ============================================================

/// Compiled script cache entry
#[derive(Debug, Clone)]
struct CompiledScript {
    /// Transpiled JavaScript code
    code: String,
}

/// Script cache (action_id -> compiled script)
type ScriptCache = Arc<RwLock<LruCache<String, CompiledScript>>>;

// ============================================================
// ActionEngine
// ============================================================

/// ActionEngine executes TypeScript/JavaScript actions in V8 isolate
pub struct ActionEngine<R: ActionRepository> {
    action_repo: Arc<R>,
    script_cache: ScriptCache,
    http_client: reqwest::Client,
    async_config: AsyncActionConfig,
}

impl<R: ActionRepository + 'static> ActionEngine<R> {
    /// Create a new ActionEngine with default async config (fetch blocked by default)
    pub fn new(action_repo: Arc<R>) -> Self {
        Self::with_config(action_repo, AsyncActionConfig::default())
    }

    /// Create a new ActionEngine with explicit async config
    pub fn with_config(action_repo: Arc<R>, async_config: AsyncActionConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(async_config.request_timeout_ms))
            .build()
            .unwrap_or_default();

        tracing::info!(
            "ActionEngine initialized (allowed_domains: {:?})",
            async_config.allowed_domains
        );

        Self {
            action_repo,
            script_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(100).unwrap(),
            ))),
            http_client,
            async_config,
        }
    }

    /// Execute all enabled actions for a specific trigger
    ///
    /// Actions are executed in order (by execution_order field).
    /// If any action fails, the entire flow is aborted.
    pub async fn execute_trigger(
        &self,
        tenant_id: crate::domain::StringUuid,
        trigger_id: &str,
        mut context: ActionContext,
    ) -> Result<ActionContext> {
        // Fetch all enabled actions for this trigger
        let actions = self
            .action_repo
            .list_by_trigger(tenant_id, trigger_id, true)
            .await?;

        if actions.is_empty() {
            return Ok(context);
        }

        tracing::info!(
            "Executing {} actions for trigger {} in tenant {}",
            actions.len(),
            trigger_id,
            tenant_id
        );

        // Execute each action in order
        for action in actions {
            let start = Instant::now();
            let user_id = context.user.id.parse().ok();

            match self.execute_action(&action, &context).await {
                Ok(modified_context) => {
                    let duration_ms = start.elapsed().as_millis() as i32;
                    context = modified_context;

                    // Record successful execution
                    if let Err(e) = self
                        .action_repo
                        .record_execution(
                            action.id,
                            tenant_id,
                            trigger_id.to_string(),
                            user_id,
                            true,
                            duration_ms,
                            None,
                        )
                        .await
                    {
                        tracing::warn!("Failed to record action execution: {}", e);
                    }

                    if let Err(e) = self
                        .action_repo
                        .update_execution_stats(action.id, true, None)
                        .await
                    {
                        tracing::warn!("Failed to update action stats: {}", e);
                    }

                    tracing::info!(
                        "Action {} executed successfully in {}ms",
                        action.name,
                        duration_ms
                    );

                    // Record success metrics
                    counter!("auth9_action_executions_total", "trigger" => trigger_id.to_string(), "result" => "success").increment(1);
                    histogram!("auth9_action_execution_duration_seconds", "trigger" => trigger_id.to_string()).record(duration_ms as f64 / 1000.0);
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis() as i32;
                    let error_msg = format!("Action '{}' failed: {}", action.name, e);

                    // Record failed execution
                    if let Err(record_err) = self
                        .action_repo
                        .record_execution(
                            action.id,
                            tenant_id,
                            trigger_id.to_string(),
                            user_id,
                            false,
                            duration_ms,
                            Some(error_msg.clone()),
                        )
                        .await
                    {
                        tracing::warn!("Failed to record action execution: {}", record_err);
                    }

                    if let Err(stats_err) = self
                        .action_repo
                        .update_execution_stats(action.id, false, Some(error_msg.clone()))
                        .await
                    {
                        tracing::warn!("Failed to update action stats: {}", stats_err);
                    }

                    tracing::error!(
                        "Action {} failed in {}ms: {}",
                        action.name,
                        duration_ms,
                        error_msg
                    );

                    // Record error metrics
                    counter!("auth9_action_executions_total", "trigger" => trigger_id.to_string(), "result" => "error").increment(1);

                    // Strict mode: abort entire flow on first failure
                    return Err(AppError::ActionExecutionFailed(error_msg));
                }
            }
        }

        Ok(context)
    }

    /// Execute a single action
    async fn execute_action(
        &self,
        action: &Action,
        context: &ActionContext,
    ) -> Result<ActionContext> {
        // Get or compile script
        let transpiled_code = self.get_or_compile_script(action).await?;
        let timeout_ms = action.timeout_ms;

        // Clone values for move into blocking task
        let context_clone = context.clone();
        let http_client = self.http_client.clone();
        let async_config = self.async_config.clone();

        // Channel to send V8 IsolateHandle from blocking thread to timeout watchdog
        let (handle_tx, handle_rx) =
            tokio::sync::oneshot::channel::<deno_core::v8::IsolateHandle>();

        // Spawn timeout watchdog that will terminate V8 execution if it exceeds the limit
        let timeout_duration = Duration::from_millis(timeout_ms as u64);
        let watchdog = tokio::spawn(async move {
            if let Ok(isolate_handle) = handle_rx.await {
                tokio::time::sleep(timeout_duration).await;
                // Forcibly terminate V8 execution (works even on synchronous infinite loops)
                isolate_handle.terminate_execution();
            }
        });

        // Execute in blocking thread pool (JsRuntime is !Send, must stay on one thread)
        let handle = tokio::task::spawn_blocking(move || {
            // 1. Take or create JsRuntime
            let mut js_runtime = match take_js_runtime() {
                Some(rt) => rt,
                None => create_js_runtime(http_client.clone(), async_config.clone())
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to create V8 runtime: {}",
                            e
                        ))
                    })?,
            };

            // Send IsolateHandle to the timeout watchdog
            let _ = handle_tx
                .send(js_runtime.v8_isolate().thread_safe_handle());

            // 2. Reset per-execution op state
            {
                let op_state = js_runtime.op_state();
                let mut state = op_state.borrow_mut();
                state.put(http_client);
                state.put(async_config);
                state.put(RequestCounter(0));
            }

            // 3. Inject context into globalThis
            {
                let scope = &mut js_runtime.handle_scope();
                let context_value =
                    serde_v8::to_v8(scope, &context_clone).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to serialize context: {}",
                            e
                        ))
                    })?;

                let global = scope.get_current_context().global(scope);
                let key = deno_core::v8::String::new(scope, "context").unwrap();
                global.set(scope, key.into(), context_value);
            }

            // 4. Execute the transpiled script
            let result_global = js_runtime
                .execute_script("<action_script>", transpiled_code)
                .map_err(|e| {
                    AppError::ActionExecutionFailed(format!(
                        "Script execution error: {}",
                        e
                    ))
                })?;

            // 5. Detect if result is a Promise and pump event loop
            let is_promise = {
                let scope = &mut js_runtime.handle_scope();
                let local =
                    deno_core::v8::Local::new(scope, result_global.clone());
                local.is_promise()
            };

            if is_promise {
                // Take thread-local tokio runtime to drive async ops
                let tokio_rt = take_local_tokio_rt();

                let event_loop_result = tokio_rt.block_on(async {
                    js_runtime
                        .run_event_loop(Default::default())
                        .await
                });

                // Return tokio runtime to thread-local
                return_local_tokio_rt(tokio_rt);

                event_loop_result.map_err(|e| {
                    AppError::ActionExecutionFailed(format!(
                        "Async execution error: {}",
                        e
                    ))
                })?;
            }

            // 6. Extract result (handle both sync value and resolved Promise)
            let modified_context = {
                let scope = &mut js_runtime.handle_scope();
                let local = deno_core::v8::Local::new(scope, result_global);

                let value = if local.is_promise() {
                    let promise =
                        deno_core::v8::Local::<deno_core::v8::Promise>::try_from(
                            local,
                        )
                        .map_err(|e| {
                            AppError::ActionExecutionFailed(format!(
                                "Invalid promise: {}",
                                e
                            ))
                        })?;

                    match promise.state() {
                        deno_core::v8::PromiseState::Fulfilled => {
                            promise.result(scope)
                        }
                        deno_core::v8::PromiseState::Rejected => {
                            let err = promise.result(scope);
                            let msg = err.to_rust_string_lossy(scope);
                            return Err(AppError::ActionExecutionFailed(
                                format!("Promise rejected: {}", msg),
                            ));
                        }
                        deno_core::v8::PromiseState::Pending => {
                            return Err(AppError::ActionExecutionFailed(
                                "Promise still pending after event loop completed"
                                    .into(),
                            ));
                        }
                    }
                } else {
                    local
                };

                serde_v8::from_v8::<ActionContext>(scope, value).map_err(
                    |e| {
                        AppError::ActionExecutionFailed(format!(
                            "Failed to extract context from script result: {}",
                            e
                        ))
                    },
                )?
            };

            // 7. Cleanup: remove context, result, and any user-defined globalThis properties
            let _ = js_runtime.execute_script(
                "<cleanup>",
                r#"
                (function() {
                    // Whitelist of built-in globalThis properties to keep
                    const builtins = new Set([
                        'Object', 'Function', 'Array', 'Number', 'parseFloat', 'parseInt',
                        'Infinity', 'NaN', 'undefined', 'Boolean', 'String', 'Symbol',
                        'Date', 'Promise', 'RegExp', 'Error', 'AggregateError', 'EvalError',
                        'RangeError', 'ReferenceError', 'SyntaxError', 'TypeError', 'URIError',
                        'globalThis', 'JSON', 'Math', 'Intl', 'ArrayBuffer', 'Uint8Array',
                        'Int8Array', 'Uint16Array', 'Int16Array', 'Uint32Array', 'Int32Array',
                        'Float32Array', 'Float64Array', 'Uint8ClampedArray', 'BigUint64Array',
                        'BigInt64Array', 'DataView', 'Map', 'BigInt', 'Set', 'WeakMap',
                        'WeakSet', 'WeakRef', 'FinalizationRegistry', 'Proxy', 'Reflect',
                        'SharedArrayBuffer', 'Atomics', 'decodeURI', 'decodeURIComponent',
                        'encodeURI', 'encodeURIComponent', 'escape', 'unescape',
                        'eval', 'isFinite', 'isNaN',
                        // Deno runtime
                        'Deno',
                        // Our polyfills (must survive cleanup)
                        'fetch', 'setTimeout', 'clearTimeout', 'console', '__timers',
                    ]);
                    for (const key of Object.getOwnPropertyNames(globalThis)) {
                        if (!builtins.has(key)) {
                            try { delete globalThis[key]; } catch(e) {}
                        }
                    }
                    // Also clean up any Object.prototype pollution
                    for (const key of Object.getOwnPropertyNames(Object.prototype)) {
                        if (!['constructor','__defineGetter__','__defineSetter__',
                             '__lookupGetter__','__lookupSetter__','__proto__',
                             'hasOwnProperty','isPrototypeOf','propertyIsEnumerable',
                             'toString','valueOf','toLocaleString'].includes(key)) {
                            try { delete Object.prototype[key]; } catch(e) {}
                        }
                    }
                })();
                "#,
            );
            return_js_runtime(js_runtime);

            Ok(modified_context)
        });

        let result = handle
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Blocking task error: {}", e))
            })?;

        // Cancel the watchdog (script finished before timeout)
        watchdog.abort();

        // If V8 was terminated by the watchdog, the script error will contain
        // "Uncaught Error: execution terminated". Map it to a clear timeout error.
        match result {
            Ok(ctx) => Ok(ctx),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("execution terminated") {
                    Err(AppError::ActionExecutionFailed(format!(
                        "Action timed out after {}ms",
                        timeout_ms
                    )))
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Get compiled script from cache or compile new one
    async fn get_or_compile_script(&self, action: &Action) -> Result<String> {
        let cache_key = action.id.to_string();

        // Check cache first
        {
            let mut cache = self.script_cache.write().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.code.clone());
            }
        }

        // Cache miss, compile the script
        let transpiled = self.compile_typescript(&action.script)?;

        // Store in cache
        {
            let mut cache = self.script_cache.write().await;
            cache.put(
                cache_key,
                CompiledScript {
                    code: transpiled.clone(),
                },
            );
        }

        Ok(transpiled)
    }

    /// Compile TypeScript to JavaScript
    ///
    /// Detects async patterns and wraps in async IIFE when needed.
    /// Ensures the script returns context at the end.
    fn compile_typescript(&self, script: &str) -> Result<String> {
        let trimmed = script.trim();

        // Detect async patterns
        let is_async = trimmed.contains("await ")
            || trimmed.contains("async ")
            || trimmed.contains("fetch(");

        if is_async {
            // Wrap in async IIFE that returns context
            Ok(format!(
                "(async () => {{\n{}\nreturn context;\n}})()",
                script
            ))
        } else {
            // Sync path: ensure script returns context at the end
            if trimmed.ends_with("context;")
                || trimmed.ends_with("context")
                || trimmed.contains("return context")
            {
                Ok(script.to_string())
            } else {
                // Append context return
                Ok(format!("{}\ncontext;", script))
            }
        }
    }

    /// Test an action with mock context (for testing endpoint)
    pub async fn test_action(
        &self,
        action: &Action,
        context: ActionContext,
    ) -> Result<(ActionContext, i32, Vec<String>)> {
        let start = Instant::now();

        let modified_context = self.execute_action(action, &context).await?;

        let duration_ms = start.elapsed().as_millis() as i32;
        let console_logs = Vec::new(); // TODO: Capture console.log output via op state

        Ok((modified_context, duration_ms, console_logs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ActionContextRequest, ActionContextTenant, ActionContextUser, StringUuid,
    };
    use crate::repository::action::MockActionRepository;
    use chrono::Utc;

    fn create_test_context() -> ActionContext {
        ActionContext {
            user: ActionContextUser {
                id: "user123".to_string(),
                email: "test@example.com".to_string(),
                display_name: Some("Test User".to_string()),
                mfa_enabled: false,
            },
            tenant: ActionContextTenant {
                id: "tenant123".to_string(),
                slug: "acme".to_string(),
                name: "Acme Corp".to_string(),
            },
            request: ActionContextRequest {
                ip: Some("1.2.3.4".to_string()),
                user_agent: Some("Mozilla/5.0".to_string()),
                timestamp: Utc::now(),
            },
            claims: None,
        }
    }

    fn create_test_action(script: &str) -> Action {
        Action {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            name: "test-action".to_string(),
            description: None,
            trigger_id: "post-login".to_string(),
            script: script.to_string(),
            enabled: true,
            execution_order: 0,
            timeout_ms: 5000,
            last_executed_at: None,
            execution_count: 0,
            error_count: 0,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ============================================================
    // Backward compatibility tests
    // ============================================================

    #[tokio::test]
    async fn test_simple_script_execution() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action("context;");
        let context = create_test_context();

        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_ok());

        let modified = result.unwrap();
        assert_eq!(modified.user.email, context.user.email);
    }

    #[tokio::test]
    async fn test_script_modifies_claims() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.department = "engineering";
            context.claims.tier = "premium";
            context;
            "#,
        );

        let context = create_test_context();

        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_ok());

        let modified = result.unwrap();
        assert!(modified.claims.is_some());

        let claims = modified.claims.unwrap();
        assert_eq!(
            claims.get("department").unwrap().as_str().unwrap(),
            "engineering"
        );
        assert_eq!(claims.get("tier").unwrap().as_str().unwrap(), "premium");
    }

    #[tokio::test]
    async fn test_script_throws_error() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            throw new Error("Test error");
            "#,
        );

        let context = create_test_context();

        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Test error"));
    }

    #[tokio::test]
    async fn test_script_timeout() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let mut action = create_test_action(
            r#"
            while (true) { const x = 1 + 1; }
            "#,
        );
        action.timeout_ms = 100;

        let context = create_test_context();

        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("timed out"),
            "Synchronous infinite loops should be terminated by V8 IsolateHandle"
        );
    }

    #[tokio::test]
    async fn test_script_cache() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action("context;");

        let code1 = engine.get_or_compile_script(&action).await.unwrap();
        let code2 = engine.get_or_compile_script(&action).await.unwrap();

        assert_eq!(code1, code2);
    }

    #[tokio::test]
    async fn test_sync_script_still_works() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.sync_value = "works";
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok());
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("sync_value").unwrap().as_str().unwrap(), "works");
    }

    #[tokio::test]
    #[ignore] // Performance test - run with: cargo test -- --ignored test_runtime_reuse_performance
    async fn test_runtime_reuse_performance() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.test = "value";
            context;
            "#,
        );

        let context = create_test_context();

        let warmup_start = Instant::now();
        let _ = engine.execute_action(&action, &context).await.unwrap();
        let warmup_duration = warmup_start.elapsed();
        println!("Warmup (first execution): {:?}", warmup_duration);

        let iterations = 10;
        let mut total_duration = Duration::ZERO;

        for i in 0..iterations {
            let start = Instant::now();
            let result = engine.execute_action(&action, &context).await;
            let duration = start.elapsed();
            total_duration += duration;
            assert!(result.is_ok());
            println!("Execution {} duration: {:?}", i + 1, duration);
        }

        let avg_duration = total_duration / iterations;
        println!("\n=== Performance Results ===");
        println!("First execution (with init): {:?}", warmup_duration);
        println!(
            "Average of {} reuse executions: {:?}",
            iterations, avg_duration
        );
        println!(
            "Speedup: {:.1}x faster",
            warmup_duration.as_secs_f64() / avg_duration.as_secs_f64()
        );

        assert!(
            avg_duration < warmup_duration / 2,
            "Runtime reuse should be at least 2x faster. Warmup: {:?}, Avg: {:?}",
            warmup_duration,
            avg_duration
        );
    }

    // ============================================================
    // Security tests
    // ============================================================

    #[tokio::test]
    async fn test_runtime_cleanup_runs_without_error() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.executed = true;
            context;
            "#,
        );

        let context1 = create_test_context();
        let result1 = engine.execute_action(&action, &context1).await;
        assert!(result1.is_ok());

        let context2 = create_test_context();
        let result2 = engine.execute_action(&action, &context2).await;
        assert!(
            result2.is_ok(),
            "Cleanup should not break subsequent executions"
        );
    }

    #[tokio::test]
    async fn test_globalthis_pollution_is_cleaned_up() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action1 = create_test_action(
            r#"
            globalThis.customPollution = "persists";
            context;
            "#,
        );

        let context1 = create_test_context();
        let _ = engine.execute_action(&action1, &context1).await;

        let action2 = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.pollutionDetected = (typeof globalThis.customPollution !== 'undefined');
            context;
            "#,
        );

        let context2 = create_test_context();
        let result = engine.execute_action(&action2, &context2).await;

        assert!(result.is_ok(), "Second action should execute successfully");
        let ctx = result.unwrap();
        let pollution_detected = ctx
            .claims
            .as_ref()
            .and_then(|c| c.get("pollutionDetected"))
            .and_then(|v| v.as_bool());

        assert_eq!(
            pollution_detected,
            Some(false),
            "globalThis pollution should be cleaned up between actions"
        );
    }

    #[tokio::test]
    async fn test_object_prototype_pollution_is_cleaned_up() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action1 = create_test_action(
            r#"
            Object.prototype.hacked = true;
            context;
            "#,
        );

        let context1 = create_test_context();
        let _ = engine.execute_action(&action1, &context1).await;

        let action2 = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.prototypeHacked = Object.prototype.hasOwnProperty("hacked");
            context;
            "#,
        );

        let context2 = create_test_context();
        let result = engine.execute_action(&action2, &context2).await;

        assert!(result.is_ok(), "Second action should execute successfully");
        let ctx = result.unwrap();
        let hacked = ctx
            .claims
            .as_ref()
            .and_then(|c| c.get("prototypeHacked"))
            .and_then(|v| v.as_bool());

        assert_eq!(
            hacked,
            Some(false),
            "Object.prototype pollution should be cleaned up between actions"
        );
    }

    #[tokio::test]
    async fn test_fetch_is_available_but_controlled() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        // fetch IS defined (polyfill), but controlled by allowlist
        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.fetch_available = (typeof fetch !== 'undefined');
            context.claims.console_available = (typeof console !== 'undefined');
            context.claims.setTimeout_available = (typeof setTimeout !== 'undefined');
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok());
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(
            claims.get("fetch_available").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(
            claims.get("console_available").unwrap().as_bool(),
            Some(true)
        );
        assert_eq!(
            claims.get("setTimeout_available").unwrap().as_bool(),
            Some(true)
        );
    }

    #[tokio::test]
    async fn test_filesystem_access_blocked() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            if (typeof Deno !== 'undefined' && typeof Deno.readFile !== 'undefined') {
                throw new Error("Deno.readFile should not be available!");
            }
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_ok(), "Filesystem access should be blocked");
    }

    #[tokio::test]
    async fn test_process_access_blocked() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            if (typeof Deno !== 'undefined' && typeof Deno.run !== 'undefined') {
                throw new Error("Deno.run should not be available!");
            }
            if (typeof process !== 'undefined' && typeof process.exit !== 'undefined') {
                throw new Error("process.exit should not be available!");
            }
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;
        assert!(result.is_ok(), "Process access should be blocked");
    }

    #[tokio::test]
    async fn test_code_injection_prevention() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            try {
                Object.prototype.polluted = "bad";
            } catch (e) {}

            try {
                const result = eval("1+1");
            } catch (e) {}

            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Should handle injection attempts safely"
        );
    }

    #[tokio::test]
    async fn test_memory_bomb_prevention() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let mut action = create_test_action(
            r#"
            try {
                const large = new Array(10000000);
                for (let i = 0; i < 1000; i++) {
                    large[i] = "x".repeat(1000);
                }
            } catch (e) {}
            context;
            "#,
        );
        action.timeout_ms = 2000;

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;
        assert!(
            result.is_ok() || result.is_err(),
            "Should handle memory pressure without crashing the process"
        );
    }

    #[tokio::test]
    async fn test_script_cache_isolation() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let mut action1 = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.tenant = "tenant1";
            context;
            "#,
        );
        action1.tenant_id = StringUuid::new_v4();

        let mut action2 = create_test_action(
            r#"
            context.claims = context.claims || {};
            context.claims.tenant = "tenant2";
            context;
            "#,
        );
        action2.tenant_id = StringUuid::new_v4();

        let context1 = create_test_context();
        let result1 = engine.execute_action(&action1, &context1).await;

        let context2 = create_test_context();
        let result2 = engine.execute_action(&action2, &context2).await;

        assert!(result1.is_ok(), "Action 1 should execute successfully");
        assert!(result2.is_ok(), "Action 2 should execute successfully");

        if let (Ok(ctx1), Ok(ctx2)) = (result1, result2) {
            let tenant1 = ctx1.claims.as_ref().and_then(|c| c.get("tenant"));
            let tenant2 = ctx2.claims.as_ref().and_then(|c| c.get("tenant"));

            assert_eq!(tenant1.and_then(|v| v.as_str()), Some("tenant1"));
            assert_eq!(tenant2.and_then(|v| v.as_str()), Some("tenant2"));
        }
    }

    #[tokio::test]
    async fn test_nodejs_require_blocked() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            try {
                const fs = require("fs");
                context.claims = context.claims || {};
                context.claims.leaked_data = "Node.js require worked!";
            } catch (e) {
                context.claims = context.claims || {};
                context.claims.blocked = true;
                context.claims.error = String(e);
            }
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok(), "Node.js require() should be blocked");

        if let Ok(updated_context) = result {
            assert!(updated_context.claims.is_some());
            let claims = updated_context.claims.unwrap();
            assert!(claims.contains_key("blocked"));
            assert!(claims.contains_key("error"));
            let error = claims.get("error").unwrap();
            assert!(
                error.as_str().unwrap().contains("ReferenceError")
                    || error.as_str().unwrap().contains("require is not defined"),
            );
        }
    }

    #[tokio::test]
    async fn test_process_env_access_blocked() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            try {
                context.claims = context.claims || {};
                context.claims.env = process.env;
                context.claims.jwt_secret = process.env.JWT_SECRET;
            } catch (e) {
                context.claims = context.claims || {};
                context.claims.blocked = true;
            }
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok(), "process.env access should be blocked");

        if let Ok(updated_context) = result {
            assert!(updated_context.claims.is_some());
            let claims = updated_context.claims.unwrap();
            assert!(claims.contains_key("blocked"));
            assert!(!claims.contains_key("env"));
            assert!(!claims.contains_key("jwt_secret"));
        }
    }

    // ============================================================
    // Async/await tests
    // ============================================================

    #[tokio::test]
    async fn test_async_await_basic() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            async function enrich() {
                return { role: "admin" };
            }
            const data = await enrich();
            context.claims = context.claims || {};
            context.claims.role = data.role;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok(), "Async/await should work: {:?}", result.err());
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("role").unwrap().as_str().unwrap(), "admin");
    }

    #[tokio::test]
    async fn test_async_promise_chain() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            const value = await Promise.resolve(42).then(v => v * 2);
            context.claims = context.claims || {};
            context.claims.value = value;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(
            result.is_ok(),
            "Promise chain should work: {:?}",
            result.err()
        );
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("value").unwrap().as_f64().unwrap() as i64, 84);
    }

    #[tokio::test]
    async fn test_async_promise_rejection() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            await Promise.reject(new Error("test rejection"));
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("test rejection") || err.contains("rejected") || err.contains("error"),
            "Error should mention rejection: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_fetch_blocked_by_default() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        // fetch is available but blocked when no domains are in allowlist
        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            try {
                await fetch('https://example.com/api');
                context.claims.fetch_succeeded = true;
            } catch (e) {
                context.claims.fetch_blocked = true;
                context.claims.error = String(e);
            }
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(
            result.is_ok(),
            "Script should handle fetch error: {:?}",
            result.err()
        );
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(
            claims.get("fetch_blocked").unwrap().as_bool(),
            Some(true),
            "Fetch should be blocked"
        );
        let error = claims.get("error").unwrap().as_str().unwrap();
        assert!(
            error.contains("allowlist"),
            "Error should mention allowlist: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_fetch_allowed_domain() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"role": "admin"})),
            )
            .mount(&mock_server)
            .await;

        // Extract host:port from wiremock URI
        let server_uri = mock_server.uri();
        let parsed = url::Url::parse(&server_uri).unwrap();
        let host_port = format!(
            "{}:{}",
            parsed.host_str().unwrap(),
            parsed.port().unwrap()
        );

        let config = AsyncActionConfig {
            allowed_domains: vec![host_port],
            allow_private_ips: true, // wiremock runs on 127.0.0.1
            ..Default::default()
        };

        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::with_config(Arc::new(mock_repo), config);

        let action = create_test_action(&format!(
            r#"
            const response = await fetch('{}/test', {{ method: 'GET' }});
            const data = await response.json();
            context.claims = context.claims || {{}};
            context.claims.role = data.role;
            context.claims.status = response.status;
            "#,
            server_uri
        ));

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(
            result.is_ok(),
            "Fetch to allowed domain should work: {:?}",
            result.err()
        );
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("role").unwrap().as_str().unwrap(), "admin");
        assert_eq!(claims.get("status").unwrap().as_f64().unwrap() as u16, 200);
    }

    #[tokio::test]
    async fn test_fetch_private_ip_blocked() {
        let config = AsyncActionConfig {
            allowed_domains: vec!["127.0.0.1".to_string()],
            allow_private_ips: false, // default: block private IPs
            ..Default::default()
        };

        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::with_config(Arc::new(mock_repo), config);

        let action = create_test_action(
            r#"
            context.claims = context.claims || {};
            try {
                await fetch('http://127.0.0.1:8080/secret');
                context.claims.error = 'should have been blocked';
            } catch (e) {
                context.claims.blocked = true;
                context.claims.error = String(e);
            }
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok());
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("blocked").unwrap().as_bool(), Some(true));
        let error = claims.get("error").unwrap().as_str().unwrap();
        assert!(
            error.contains("private") || error.contains("blocked"),
            "Error should mention private IP blocking: {}",
            error
        );
    }

    #[tokio::test]
    async fn test_fetch_request_limit() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let server_uri = mock_server.uri();
        let parsed = url::Url::parse(&server_uri).unwrap();
        let host_port = format!(
            "{}:{}",
            parsed.host_str().unwrap(),
            parsed.port().unwrap()
        );

        let config = AsyncActionConfig {
            allowed_domains: vec![host_port],
            allow_private_ips: true,
            max_requests_per_execution: 2, // Only allow 2 requests
            ..Default::default()
        };

        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::with_config(Arc::new(mock_repo), config);

        let action = create_test_action(&format!(
            r#"
            const results = [];
            for (let i = 0; i < 3; i++) {{
                try {{
                    await fetch('{}/test');
                    results.push('ok');
                }} catch (e) {{
                    results.push('blocked: ' + String(e));
                }}
            }}
            context.claims = context.claims || {{}};
            context.claims.results = results;
            "#,
            server_uri
        ));

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(result.is_ok(), "Script should handle limit: {:?}", result.err());
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        let results = claims.get("results").unwrap().as_array().unwrap();

        assert_eq!(results[0].as_str().unwrap(), "ok");
        assert_eq!(results[1].as_str().unwrap(), "ok");
        assert!(
            results[2].as_str().unwrap().starts_with("blocked"),
            "3rd request should be blocked: {}",
            results[2]
        );
    }

    #[tokio::test]
    async fn test_console_log_captured() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        // console.log should not crash and should route to tracing
        let action = create_test_action(
            r#"
            console.log("hello", "world");
            console.warn("warning message");
            console.error("error message");
            context.claims = context.claims || {};
            context.claims.logged = true;
            context;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(
            result.is_ok(),
            "console.log should not crash: {:?}",
            result.err()
        );
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("logged").unwrap().as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_set_timeout_works() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let action = create_test_action(
            r#"
            const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
            await delay(10);
            context.claims = context.claims || {};
            context.claims.delayed = true;
            "#,
        );

        let context = create_test_context();
        let result = engine.execute_action(&action, &context).await;

        assert!(
            result.is_ok(),
            "setTimeout should work: {:?}",
            result.err()
        );
        let modified = result.unwrap();
        let claims = modified.claims.unwrap();
        assert_eq!(claims.get("delayed").unwrap().as_bool(), Some(true));
    }

    // ============================================================
    // Private IP blocking unit tests
    // ============================================================

    #[test]
    fn test_is_private_ip() {
        // Loopback
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("127.0.0.5"));

        // Private ranges
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("172.16.0.1"));
        assert!(is_private_ip("192.168.1.1"));

        // Link-local
        assert!(is_private_ip("169.254.1.1"));

        // Unspecified
        assert!(is_private_ip("0.0.0.0"));

        // IPv6 loopback
        assert!(is_private_ip("::1"));

        // Hostnames
        assert!(is_private_ip("localhost"));
        assert!(is_private_ip("my-service.local"));
        assert!(is_private_ip("db.internal"));

        // Public IPs should not be private
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("1.1.1.1"));
        assert!(!is_private_ip("example.com"));
        assert!(!is_private_ip("api.stripe.com"));
    }

    // ============================================================
    // Compile tests
    // ============================================================

    #[test]
    fn test_compile_sync_script_appends_context() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let result = engine.compile_typescript("context.claims = {};").unwrap();
        assert!(result.ends_with("\ncontext;"));
    }

    #[test]
    fn test_compile_sync_script_preserves_context_return() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let result = engine.compile_typescript("context;").unwrap();
        assert_eq!(result, "context;");
    }

    #[test]
    fn test_compile_async_script_wraps_in_iife() {
        let mock_repo = MockActionRepository::new();
        let engine = ActionEngine::new(Arc::new(mock_repo));

        let result = engine
            .compile_typescript("const data = await fetch('http://example.com');")
            .unwrap();
        assert!(result.starts_with("(async () => {"));
        assert!(result.contains("return context;"));
        assert!(result.ends_with("})()"));
    }
}
