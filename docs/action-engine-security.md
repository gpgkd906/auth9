# Action Engine Security

## Overview

Auth9 Action Engine uses V8 isolate sandboxing to safely execute user-provided JavaScript/TypeScript code. This document describes the security architecture, implemented protections, known limitations, and testing methodology.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Auth9 Core (Rust)                       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │            ActionEngine (action_engine.rs)            │  │
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
│  │  │  │  Sandbox Boundaries:                      │  │  │  │
│  │  │  │  ❌ No network (fetch unavailable)        │  │  │  │
│  │  │  │  ❌ No filesystem (Deno.* unavailable)    │  │  │  │
│  │  │  │  ❌ No process APIs (no exec/spawn)       │  │  │  │
│  │  │  │  ✅ Pure computation only                 │  │  │  │
│  │  │  └──────────────────────────────────────────┘  │  │  │
│  │  │                                                 │  │  │
│  │  │  LRU Script Cache (256 entries)                │  │  │
│  │  │  - Key: (action_id, script_hash)               │  │  │
│  │  │  - Prevents recompilation                      │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │                                                         │  │
│  │  Timeout Enforcement: action.timeout_ms (default 5s)   │  │
│  │  Memory: V8 default limits (~1.4GB heap)               │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Security Boundaries

### ✅ Implemented Protections

| Protection | Implementation | File Reference |
|------------|----------------|----------------|
| **Execution Isolation** | Each execution gets fresh V8 context, thread-local runtime | `action_engine.rs:25-43` |
| **Timeout Enforcement** | Tokio timeout wrapper (default 5000ms, configurable) | `action_engine.rs:281-294` |
| **Network Blocking** | No extensions registered, fetch/XMLHttpRequest unavailable | `action_engine.rs:36-38` |
| **Filesystem Blocking** | No Deno.* APIs registered | `action_engine.rs:36-38` |
| **Process Blocking** | No spawn/exec capabilities | `action_engine.rs:36-38` |
| **Tenant Isolation** | Script cache keys include action_id, separate execution contexts | `action_engine.rs:62-72, 107-214` |
| **TypeScript Support** | Compile to JS before execution, syntax validation | `action_engine.rs:328-345` |

### ❌ Known Limitations

1. **Timeout Enforcement Constraints**
   - **Issue**: V8 doesn't support pre-emptive script termination from Rust
   - **Current Behavior**: Timeout kills the async execution, but synchronous infinite loops may not be interrupted immediately
   - **Mitigation**: Scripts are expected to be async; synchronous code still respects Tokio task timeout
   - **Reference**: `action_engine.rs:477` (ignored test comment)

2. **Memory Limits**
   - **Issue**: No explicit heap size limit configured
   - **Current Behavior**: Relies on V8 default limits (~1.4GB per isolate)
   - **Mitigation**: V8 will throw OOM errors, Rust catches and returns error to caller
   - **Risk**: Low - default limits are reasonable for typical actions

3. **Context Cleanup**
   - **Issue**: `cleanup_runtime()` only drops context and result variables
   - **Current Behavior**: V8 garbage collection handles most cleanup
   - **Mitigation**: Thread-local runtime limits cross-request pollution
   - **Risk**: Very Low - isolated execution prevents state leakage
   - **Reference**: `action_engine.rs:62-72`

### 🔍 Validation Methods

All security boundaries are validated in the test suite:

| Test | Purpose | File |
|------|---------|------|
| `test_execution_isolation_between_requests` | Validates globalThis pollution doesn't leak | `tests/action_security_test.rs:92-128` |
| `test_timeout_enforcement` | Validates infinite loops are terminated | `tests/action_security_test.rs:131-157` |
| `test_network_access_blocked` | Validates fetch API is unavailable | `tests/action_security_test.rs:160-182` |
| `test_filesystem_access_blocked` | Validates Deno.readFile is unavailable | `tests/action_security_test.rs:185-207` |
| `test_process_access_blocked` | Validates Deno.run/process.exit unavailable | `tests/action_security_test.rs:210-235` |
| `test_code_injection_prevention` | Validates prototype pollution/eval handled | `tests/action_security_test.rs:238-271` |
| `test_memory_bomb_prevention` | Validates large allocations don't crash | `tests/action_security_test.rs:274-297` |
| `test_script_cache_isolation` | Validates different tenants don't mix scripts | `tests/action_security_test.rs:300-347` |

Run security tests:
```bash
cd auth9-core
cargo test action_security_test --test '*'
```

## Security Testing Checklist

### Pre-Production Validation

- [ ] All 8 security tests passing
- [ ] Manual test: infinite loop terminates within timeout
- [ ] Manual test: network access attempts fail gracefully
- [ ] Manual test: filesystem access attempts fail gracefully
- [ ] Verify script cache isolation with concurrent tenant requests
- [ ] Load test: 100 concurrent action executions complete without crash
- [ ] Memory profile: no memory leaks over 1000 executions

### Production Monitoring

- [ ] Set up alerts for `auth9_action_executions_total{result="error"}`
- [ ] Monitor `auth9_action_execution_duration_seconds` P99 latency
- [ ] Track action timeout rate vs total executions
- [ ] Review error logs for V8 runtime errors weekly

## Threat Model

### In Scope (Protected)

✅ **Malicious Script Execution**
- Arbitrary code execution is expected (Actions are user-provided)
- Sandbox prevents escape to host system

✅ **Resource Exhaustion**
- CPU: Timeout enforcement prevents infinite loops
- Memory: V8 heap limits prevent unbounded allocation
- Network: No external requests possible

✅ **Cross-Tenant Attacks**
- Script cache isolation prevents code sharing
- Execution context isolation prevents data leakage

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

1. **Never disable timeout**: Even for debugging, use long timeout (60s) instead of removing it
2. **Test with malicious scripts**: Regularly run security test suite in CI
3. **Review V8 updates**: Monitor deno_core releases for security patches

### For Operations

1. **Set conservative timeouts**: Start with 5s default, increase only if needed
2. **Monitor error rates**: Spike in execution errors may indicate attack attempts
3. **Enable audit logging**: Log all action create/update operations with script content
4. **Limit action count per tenant**: Consider upper bound (e.g., 100 actions) to prevent resource exhaustion

### For Future Enhancements

1. **Memory Limits**: Configure explicit V8 heap size per isolate
2. **Pre-execution Analysis**: Static analysis to detect suspicious patterns (large loops, prototype manipulation)
3. **Rate Limiting**: Per-tenant execution quota (e.g., 1000 executions/hour)
4. **Observability**: Add metrics for V8 heap usage, GC pauses

## Incident Response

If malicious action is suspected:

1. **Immediate**: Disable the action via API
   ```bash
   curl -X PUT http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
     -H "Authorization: Bearer $TOKEN" \
     -d '{"enabled": false}'
   ```

2. **Investigation**: Query execution logs
   ```sql
   SELECT * FROM action_execution_logs
   WHERE action_id = '{action_id}'
   ORDER BY executed_at DESC LIMIT 100;
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
- Auth9 Action System: `auth9-core/src/service/action_engine.rs`
- Security Tests: `auth9-core/tests/action_security_test.rs`
