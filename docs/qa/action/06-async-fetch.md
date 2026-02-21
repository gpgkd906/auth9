# Action - Async/Await 与 Fetch 支持

**模块**: Action Engine
**测试范围**: Action 脚本中的 async/await、fetch()、setTimeout、console.log 功能及安全限制
**场景数**: 5
**优先级**: 高

---

## 背景说明

Action Engine 新增异步执行能力，允许用户脚本使用 `async/await`、`fetch()` 调用外部 API、`setTimeout` 延时、`console.log` 日志输出。

核心安全机制：
- **域名白名单**：`fetch()` 仅允许请求 `allowed_domains` 中的域名，默认全部拒绝
- **私有 IP 拦截**：阻止对 `127.0.0.1`、`10.x`、`172.16-31.x`、`192.168.x`、`localhost` 等的请求（SSRF 防护）
- **请求数限制**：每次执行最多 `max_requests_per_execution` 次 HTTP 请求（默认 5 次）
- **超时控制**：`request_timeout_ms`（默认 10s）与 Action 整体 `timeout_ms` 共同约束
- **响应体大小**：`max_response_bytes`（默认 1MB）截断过大的响应

配置项在 `AsyncActionConfig` 中管理，通过 `ActionEngine::with_config()` 初始化。

---

## 场景 1：基本 async/await 脚本执行

### 初始状态
- auth9-core 运行正常
- 存在测试租户

### 目的
验证 Action 脚本中的 `async/await` 语法正常工作，同步脚本不受影响

### 测试操作流程

#### 1.1 创建同步 Action（向后兼容）
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
SERVICE_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services LIMIT 1;")

curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Sync Compat Test",
    "trigger_id": "post-login",
    "script": "context.claims = context.claims || {}; context.claims.sync_test = true; context;",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

#### 1.2 测试同步 Action 执行
```bash
ACTION_ID="<from_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

**预期**: `success: true`，`modified_context.claims.sync_test = true`

#### 1.3 创建 async/await Action
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Async Basic Test",
    "trigger_id": "post-login",
    "script": "async function enrich() {\n  const result = await Promise.resolve({ role: \"admin\" });\n  context.claims = context.claims || {};\n  context.claims.enriched_role = result.role;\n}\nawait enrich();",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

#### 1.4 测试 async Action 执行
```bash
ACTION_ID="<from_async_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

### 预期结果
- 同步脚本正常执行，`claims.sync_test = true`
- async/await 脚本正常执行，`claims.enriched_role = "admin"`
- 两者返回 `success: true`

### 预期数据状态
```sql
SELECT id, name, script FROM actions
WHERE service_id = '{service_id}' AND name IN ('Sync Compat Test', 'Async Basic Test');
-- 预期: 两条记录均存在
```

---

## 场景 2：fetch() 请求外部 API

### 初始状态
- auth9-core 运行正常，`AsyncActionConfig.allowed_domains` 已配置目标域名
- 外部 API 可访问（或使用 wiremock 模拟）

### 目的
验证 Action 脚本中 `fetch()` 能成功请求白名单域名的外部 API，并将响应数据注入 context

### 测试操作流程

#### 2.1 创建使用 fetch() 的 Action
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Fetch API Test",
    "trigger_id": "post-login",
    "script": "const resp = await fetch(\"https://httpbin.org/get\", { method: \"GET\" });\ncontext.claims = context.claims || {};\ncontext.claims.fetch_status = resp.status;\ncontext.claims.fetch_ok = resp.ok;",
    "enabled": true,
    "timeout_ms": 15000
  }' | jq '.'
```

> **注意**：需确保 `httpbin.org` 在 `allowed_domains` 配置中。若未配置，此请求将被安全策略拒绝（见场景 3）。

#### 2.2 测试 fetch Action
```bash
ACTION_ID="<from_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

#### 2.3 验证 fetch POST 请求
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Fetch POST Test",
    "trigger_id": "post-login",
    "script": "const resp = await fetch(\"https://httpbin.org/post\", {\n  method: \"POST\",\n  headers: { \"Content-Type\": \"application/json\" },\n  body: JSON.stringify({ user: context.user.email })\n});\nconst data = await resp.json();\ncontext.claims = context.claims || {};\ncontext.claims.post_echo = data.json?.user;",
    "enabled": true,
    "timeout_ms": 15000
  }' | jq '.'
```

### 预期结果
- GET 请求返回 `claims.fetch_status = 200`，`claims.fetch_ok = true`
- POST 请求返回 `claims.post_echo` 等于 `context.user.email` 的值
- `resp.json()` 和 `resp.text()` 方法均可正常使用

---

## 场景 3：安全限制 — 域名白名单与私有 IP 拦截

### 初始状态
- auth9-core 运行正常
- `AsyncActionConfig.allowed_domains` 为空或不包含目标域名

### 目的
验证 fetch() 的三层安全机制：域名白名单、私有 IP 拦截、请求数限制

### 测试操作流程

#### 3.1 fetch 未授权域名 — 被拒绝
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Blocked Domain Test",
    "trigger_id": "post-login",
    "script": "const resp = await fetch(\"https://evil.example.com/data\");\ncontext.claims = { fetched: true };",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

测试执行：
```bash
ACTION_ID="<from_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

**预期**: `success: false`，错误信息包含 "Domain not in allowlist" 或类似安全拒绝提示

#### 3.2 fetch 私有 IP — 被拦截（SSRF 防护）
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "SSRF Test",
    "trigger_id": "post-login",
    "script": "const resp = await fetch(\"http://127.0.0.1:8080/health\");\ncontext.claims = { ssrf: true };",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

**预期**: `success: false`，错误信息包含 "private/internal" 或 "blocked"

#### 3.3 验证其他私有 IP 地址也被拦截
依次测试以下地址均应被拒绝：
- `http://10.0.0.1/api` — RFC 1918 (10.0.0.0/8)
- `http://192.168.1.1/api` — RFC 1918 (192.168.0.0/16)
- `http://172.16.0.1/api` — RFC 1918 (172.16.0.0/12)
- `http://localhost:3000/api` — localhost
- `http://169.254.169.254/metadata` — AWS metadata（link-local）

### 预期结果
- 未在白名单中的域名请求被拒绝，返回安全错误
- 所有私有 IP、loopback、link-local 地址均被拦截
- 错误信息清晰指出拒绝原因

---

## 场景 4：setTimeout 与 console.log

### 初始状态
- auth9-core 运行正常

### 目的
验证 `setTimeout` 延时执行和 `console.log` 日志输出功能

### 测试操作流程

#### 4.1 使用 setTimeout 的 Action
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "SetTimeout Test",
    "trigger_id": "post-login",
    "script": "const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));\nawait delay(100);\ncontext.claims = context.claims || {};\ncontext.claims.delayed = true;",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

测试执行：
```bash
ACTION_ID="<from_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

**预期**: `success: true`，`claims.delayed = true`，执行时间 >= 100ms

#### 4.2 使用 console.log 的 Action
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Console Log Test",
    "trigger_id": "post-login",
    "script": "console.log(\"Action started for user:\", context.user.email);\nconsole.warn(\"This is a warning\");\nconsole.error(\"This is an error log\");\ncontext.claims = context.claims || {};\ncontext.claims.logged = true;",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

**预期**:
- 脚本执行成功，`claims.logged = true`
- auth9-core 日志中可见 `[Action Script]` 前缀的日志输出
- `console.log`、`console.warn`、`console.error` 均不导致脚本崩溃

#### 4.3 验证 setTimeout 上限（30s 封顶）
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Timeout Cap Test",
    "trigger_id": "post-login",
    "script": "const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));\nawait delay(60000);\ncontext.claims = { waited: true };",
    "enabled": true,
    "timeout_ms": 35000
  }' | jq '.'
```

**预期**: `setTimeout(60000)` 被封顶至 30000ms；若 Action `timeout_ms` 先触发则返回超时错误

### 预期结果
- `setTimeout` 正常工作，可用于 `await delay(ms)` 模式
- `console.log/warn/error` 输出到服务端日志，不影响脚本执行
- `setTimeout` 延时被封顶在 30 秒

---

## 场景 5：Promise 拒绝与错误处理

### 初始状态
- auth9-core 运行正常

### 目的
验证 async 脚本中 Promise 拒绝、未捕获异常、超时的错误处理行为

### 测试操作流程

#### 5.1 Promise.reject — 明确拒绝
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Promise Reject Test",
    "trigger_id": "post-login",
    "script": "await Promise.reject(new Error(\"User not authorized\"));",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

测试执行：
```bash
ACTION_ID="<from_create_response>"
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": { "id": "u1", "email": "test@example.com", "mfa_enabled": false },
    "tenant": { "id": "'$SERVICE_ID'", "slug": "test", "name": "Test" },
    "request": { "timestamp": "2026-02-13T00:00:00Z" }
  }' | jq '.'
```

**预期**: `success: false`，错误信息包含 "User not authorized"

#### 5.2 async 函数中 throw Error
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Async Throw Test",
    "trigger_id": "post-login",
    "script": "async function validate() {\n  if (!context.user.mfa_enabled) {\n    throw new Error(\"MFA required for this tenant\");\n  }\n}\nawait validate();",
    "enabled": true,
    "timeout_ms": 5000
  }' | jq '.'
```

**预期**: `success: false`，错误信息包含 "MFA required for this tenant"

#### 5.3 Action 超时
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Timeout Test",
    "trigger_id": "post-login",
    "script": "const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));\nawait delay(10000);\ncontext.claims = { never_reached: true };",
    "enabled": true,
    "timeout_ms": 2000
  }' | jq '.'
```

**预期**: 执行超时，`success: false`，错误信息包含 "timed out" 或 "timeout"，`claims.never_reached` 不存在

#### 5.4 fetch 网络错误的优雅处理
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Fetch Error Handle Test",
    "trigger_id": "post-login",
    "script": "try {\n  const resp = await fetch(\"https://nonexistent.invalid/api\");\n  context.claims = { fetched: true };\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.fetch_error = e.message;\n  context.claims.graceful = true;\n}",
    "enabled": true,
    "timeout_ms": 15000
  }' | jq '.'
```

**预期**: 脚本通过 try/catch 捕获错误后继续执行，`claims.graceful = true`，`claims.fetch_error` 包含错误信息

### 预期结果
- Promise 拒绝返回清晰的错误信息
- async 函数中的 throw 被正确传播
- 超时机制正常工作，阻止无限执行
- 用户可通过 try/catch 优雅处理 fetch 失败

---

## 通用场景：请求数限制验证

### 测试操作流程

1. 创建一个执行多次 fetch 的 Action：
```bash
curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Request Limit Test",
    "trigger_id": "post-login",
    "script": "let count = 0;\nfor (let i = 0; i < 10; i++) {\n  try {\n    await fetch(\"https://httpbin.org/get\");\n    count++;\n  } catch(e) {\n    context.claims = context.claims || {};\n    context.claims.blocked_at = i;\n    context.claims.error = e.message;\n    break;\n  }\n}\ncontext.claims = context.claims || {};\ncontext.claims.completed_requests = count;",
    "enabled": true,
    "timeout_ms": 30000
  }' | jq '.'
```

> **注意**：需确保 `httpbin.org` 在 `allowed_domains` 中。

### 预期结果
- 前 5 次请求成功（`max_requests_per_execution` 默认值为 5）
- 第 6 次请求失败，错误信息包含 "request limit" 或 "exceeded"
- `claims.completed_requests = 5`，`claims.blocked_at = 5`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 基本 async/await 脚本执行 | ☐ | | | 含同步向后兼容 |
| 2 | fetch() 请求外部 API | ☐ | | | GET + POST |
| 3 | 安全限制 — 域名白名单与私有 IP 拦截 | ☐ | | | SSRF 防护 |
| 4 | setTimeout 与 console.log | ☐ | | | 含 30s 封顶 |
| 5 | Promise 拒绝与错误处理 | ☐ | | | 超时 + try/catch |
| - | 通用：请求数限制验证 | ☐ | | | 默认 5 次限制 |
