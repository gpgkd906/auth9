# Action Engine Security

## Overview

Auth9 Action Engine uses V8 isolate sandboxing via `deno_core` to safely execute user-provided JavaScript/TypeScript code. This document describes the security architecture, implemented protections, known limitations, and testing methodology.

**最后更新**: 2026-02-19

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Auth9 Core (Rust)                       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │     ActionEngine (domains/integration/service/)       │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │   Thread-Local V8 Runtime (deno_core)          │  │  │
│  │  │  ┌──────────────────────────────────────────┐  │  │  │
│  │  │  │    V8 Isolate (per execution)            │  │  │
│  │  │  │  ┌────────────────────────────────────┐  │  │  │  │
│  │  │  │  │  User Script Module                │  │  │  │  │
│  │  │  │  │  - TypeScript → JavaScript         │  │  │  │  │
│  │  │  │  │  - export default async function   │  │  │  │  │
│  │  │  │  └────────────────────────────────────┘  │  │  │  │
│  │  │  │                                            │  │  │  │
│  │  │  │  Host Functions (auth9_action_ext):        │  │  │  │
│  │  │  │  ✅ op_fetch (domain allowlist + SSRF)     │  │  │  │
│  │  │  │  ✅ op_console_log (log capture)           │  │  │  │
│  │  │  │  ✅ op_set_timeout (async timer)           │  │  │  │
│  │  │  │  ❌ No filesystem (Deno.* unavailable)     │  │  │  │
│  │  │  │  ❌ No process APIs (no exec/spawn)        │  │  │  │
│  │  │  └──────────────────────────────────────────┘  │  │  │
│  │  │                                                 │  │  │
│  │  │  LRU Script Cache (256 entries)                │  │  │
│  │  │  - Key: (action_id, script_hash)               │  │  │
│  │  │  - Prevents recompilation                      │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │                                                         │  │
│  │  Timeout Enforcement: action.timeout_ms (default 3s)   │  │
│  │  Heap Limit: 64MB per isolate (configurable)           │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Security Boundaries

### ✅ Implemented Protections

| Protection | Implementation | Details |
|------------|----------------|---------|
| **Execution Isolation** | Each execution gets fresh V8 context, thread-local runtime | 终止的运行时不复用，OOM/超时后丢弃 |
| **Timeout Enforcement** | Tokio timeout + V8 isolate `terminate_execution()` | 默认 3000ms，范围 1-30s |
| **Heap Memory Limit** | V8 `create_params().heap_limits(0, max_heap_mb)` + `near_heap_limit_callback` | 默认 64MB，超限后终止执行 |
| **Network Control** | `op_fetch` with domain allowlist + private IP blocking | 仅允许白名单域名，阻断 SSRF |
| **Request Limiting** | 每次执行最多 N 次 HTTP 请求 | 默认 5 次/执行 |
| **Response Size Limit** | HTTP 响应体大小限制 | 默认 1MB |
| **Filesystem Blocking** | No Deno.* APIs registered | 无文件系统访问 |
| **Process Blocking** | No spawn/exec capabilities | 无进程操作 |
| **Tenant Isolation** | Script cache keys include action_id, separate execution contexts | 不同租户脚本隔离 |
| **TypeScript Support** | Compile to JS before execution, syntax validation | 编译错误提前拦截 |

### AsyncActionConfig 默认值

```rust
pub struct AsyncActionConfig {
    pub allowed_domains: Vec<String>,       // 空 = 阻断所有 fetch
    pub request_timeout_ms: u64,            // 10,000ms (单次 HTTP 请求超时)
    pub max_response_bytes: usize,          // 1,048,576 (1MB)
    pub max_requests_per_execution: usize,  // 5
    pub allow_private_ips: bool,            // false (SSRF 防护)
    pub max_heap_mb: usize,                 // 64MB
}
```

### ⚠️ Known Limitations

1. **Timeout Enforcement Constraints**
   - **Issue**: V8 同步无限循环可能不会立即被中断
   - **Current Behavior**: Tokio timeout 终止 async 执行；`terminate_execution()` 中断 V8 isolate
   - **Mitigation**: 终止的运行时被丢弃不复用，避免状态泄露

2. **Context Cleanup**
   - **Issue**: `cleanup_runtime()` 只清理 context 和 result 变量
   - **Current Behavior**: V8 GC 处理大部分清理；OOM/超时后运行时整体丢弃
   - **Mitigation**: Thread-local runtime 限制跨请求污染
   - **Risk**: Very Low

### 🔍 Validation Methods

All security boundaries are validated in the test suite:

| Test | Purpose | File |
|------|---------|------|
| `test_execution_isolation_between_requests` | Validates globalThis pollution doesn't leak | `tests/action_security_test.rs` |
| `test_timeout_enforcement` | Validates infinite loops are terminated | `tests/action_security_test.rs` |
| `test_network_access_blocked` | Validates unauthorized fetch is blocked | `tests/action_security_test.rs` |
| `test_filesystem_access_blocked` | Validates Deno.readFile is unavailable | `tests/action_security_test.rs` |
| `test_process_access_blocked` | Validates Deno.run/process.exit unavailable | `tests/action_security_test.rs` |
| `test_code_injection_prevention` | Validates prototype pollution/eval handled | `tests/action_security_test.rs` |
| `test_memory_bomb_prevention` | Validates large allocations don't crash | `tests/action_security_test.rs` |
| `test_script_cache_isolation` | Validates different tenants don't mix scripts | `tests/action_security_test.rs` |
| `test_fetch_request_limit` | Validates per-execution request quota | `action_engine.rs` (unit test) |
| `test_fetch_private_ip_blocking` | Validates SSRF private IP rejection | `action_engine.rs` (unit test) |

Run security tests:
```bash
cd auth9-core
cargo test action_security_test --test '*'
```

## Security Testing Checklist

### Pre-Production Validation

- [ ] All security tests passing
- [ ] Manual test: infinite loop terminates within timeout
- [ ] Manual test: fetch to non-allowlisted domain fails
- [ ] Manual test: fetch to private IP fails
- [ ] Manual test: filesystem access attempts fail gracefully
- [ ] Verify script cache isolation with concurrent tenant requests
- [ ] Load test: 100 concurrent action executions complete without crash
- [ ] Memory profile: no memory leaks over 1000 executions

### Production Monitoring

- [ ] Set up alerts for `auth9_action_operations_total{operation="error"}`
- [ ] Monitor `auth9_action_operation_duration_seconds` P99 latency
- [ ] Track action timeout rate vs total executions
- [ ] Review error logs for V8 runtime errors weekly

## Threat Model

### In Scope (Protected)

✅ **Malicious Script Execution**
- Arbitrary code execution is expected (Actions are user-provided)
- Sandbox prevents escape to host system

✅ **Resource Exhaustion**
- CPU: Timeout enforcement prevents infinite loops (default 3s)
- Memory: V8 heap limit 64MB + near_heap_limit_callback 终止
- Network: Domain allowlist + request count limit + private IP blocking

✅ **Cross-Tenant Attacks**
- Script cache isolation prevents code sharing
- Execution context isolation prevents data leakage

✅ **SSRF Attacks**
- Private IP blocking (192.168.x.x, 10.x.x.x, 172.16-31.x.x, 127.x.x.x)
- Domain allowlist enforcement
- DNS rebinding protection

### Out of Scope (User Responsibility)

⚠️ **Business Logic Attacks**
- Actions can modify the provided context object
- Caller must validate returned context before using it
- Example: Action could set `ctx.user.admin = true`
- Mitigation: Validate returned context in ActionEngine caller

⚠️ **Denial of Service via Valid Actions**
- Users can create many valid actions
- Each action consumes CPU during execution
- Mitigation: Rate limiting at API layer (not in ActionEngine)

⚠️ **Script Content Validation**
- ActionEngine doesn't inspect script semantics
- Users responsible for reviewing action code before enabling
- Mitigation: UI/UX should show script diffs on updates

## Recommendations

### For Development

1. **Never disable timeout**: Even for debugging, use long timeout (30s) instead of removing it
2. **Test with malicious scripts**: Regularly run security test suite in CI
3. **Review V8 updates**: Monitor deno_core releases for security patches

### For Operations

1. **Set conservative timeouts**: Start with 3s default, increase only if needed
2. **Configure domain allowlist carefully**: Only add trusted external APIs
3. **Monitor error rates**: Spike in execution errors may indicate attack attempts
4. **Enable audit logging**: Log all action create/update operations with script content
5. **Limit action count per tenant**: Consider upper bound (e.g., 100 actions) to prevent resource exhaustion

### For Future Enhancements

1. **Pre-execution Analysis**: Static analysis to detect suspicious patterns (large loops, prototype manipulation)
2. **Rate Limiting**: Per-tenant execution quota (e.g., 1000 executions/hour)
3. **Observability**: Add metrics for V8 heap usage, GC pauses
4. **Tunable Heap Limit**: Per-tenant configurable heap size

## Incident Response

If malicious action is suspected:

1. **Immediate**: Disable the action via API
   ```bash
   curl -X PATCH http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"enabled": false}'
   ```

2. **Investigation**: Query execution logs
   ```bash
   curl "http://localhost:8080/api/v1/tenants/{tenant_id}/actions/logs?action_id={action_id}&limit=100" \
     -H "Authorization: Bearer $TOKEN"
   ```

3. **Containment**: Delete the action if confirmed malicious
   ```bash
   curl -X DELETE http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
     -H "Authorization: Bearer $TOKEN"
   ```

4. **Analysis**: Review script content from audit logs
5. **Communication**: Notify tenant admin if action was externally created
6. **Prevention**: Update input validation if new attack vector discovered

## References

- [Deno Core Security Model](https://github.com/denoland/deno_core)
- [V8 Isolate Documentation](https://v8.dev/docs/embed)
- Auth9 Action Engine: `auth9-core/src/domains/integration/service/action_engine.rs`
- Action Service: `auth9-core/src/domains/integration/service/action.rs`
- Security Tests: `auth9-core/tests/action_security_test.rs`
