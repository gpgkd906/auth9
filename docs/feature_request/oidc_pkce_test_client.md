# OIDC PKCE Test Client

**类型**: 测试基础设施
**严重程度**: High
**影响范围**: auth9-core (Backend), QA Testing
**前置依赖**: 无

---

## 背景

QA 测试中多个 OIDC 场景被阻塞，无法完成验证：

1. **`oidc_pkce-flow_scenarios-2-3_blocked`** — PKCE Flow 的 Scenario 2（正确 code_verifier 换 token）和 Scenario 3（错误 code_verifier 换 token）无法测试。原因：demo app（`auth9-demo`，端口 3002）收到 authorization code 回调后立即尝试 token exchange，但不支持 PKCE（不发送 `code_verifier`），导致 exchange 失败。虽然失败，authorization code 仍被消耗（单次使用），后续 curl 测试再用该 code 时已失效。

2. **`oidc_refresh-token_scenario1-2_blocked`** — Refresh Token Flow 的 Scenario 1 和 2 需要先通过 Authorization Code Flow 获取有效 refresh_token，但自动化测试在完成 OIDC 流程时遇到 `state_mismatch` 错误。Portal 期望的 state 与测试脚本提交的 state 不匹配，说明缺少独立于 Portal 的 state 管理能力。

### 核心问题

| 问题 | 影响 |
|------|------|
| Demo app 自动消耗 authorization code | PKCE token exchange 测试无法执行 |
| Demo app 不支持 PKCE | 即使 code 未被消耗，demo app 的 exchange 也会因缺少 code_verifier 而失败 |
| Portal/Demo 的 state 管理与测试脚本不兼容 | Authorization Code Flow 无法由自动化脚本完整走通 |

### 当前架构

- `auth9-demo`（端口 3002）使用 `redirect_uri=http://localhost:3002/auth/callback`，收到回调后自动 exchange code
- `auth9-portal`（端口 3000）使用 `redirect_uri=http://localhost:3000/auth/callback`，有自己的 state 管理逻辑
- 两者均非为 QA 测试设计，无法精确控制 OIDC 流程的每个步骤

---

## 期望行为

### R1: PKCE-capable test client

在 auth9-core 的 seed data（migration）中注册一个专用 QA test client，具备以下特性：

- **client_id**: `auth9-qa-test`
- **public_client**: `true`（公开客户端，强制 PKCE）
- **redirect_uri**: `http://localhost:19876/callback`（使用一个不会被任何服务监听的端口，防止 code 被自动消耗）
- 关联到一个专用 service（`Auth9 QA Test Service`）

**seed SQL 示例**:

```sql
INSERT IGNORE INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at)
VALUES (UUID(), NULL, 'Auth9 QA Test Service', 'http://localhost:19876', '["http://localhost:19876/callback"]', '[]', 'active', NOW(), NOW());

INSERT IGNORE INTO clients (id, service_id, client_id, client_secret_hash, public_client, created_at)
VALUES (UUID(), <service_id>, 'auth9-qa-test', '', true, NOW());
```

QA 测试脚本可使用此 client 发起 OIDC 授权请求，浏览器完成登录后 redirect 到 `localhost:19876/callback?code=XXX`，因无服务监听该端口，authorization code 不会被自动消耗，测试脚本可从浏览器地址栏或 Playwright 拦截中提取 code 后手动执行 token exchange。

### R2: Test client 不自动消耗 authorization code

通过使用无人监听的 redirect_uri 端口（`19876`）实现：

- 浏览器跳转到 `http://localhost:19876/callback?code=XXX&state=YYY` 时，页面加载失败（无服务），但 URL 中的 `code` 和 `state` 参数可被 Playwright 通过 `page.url()` 提取
- 测试脚本拿到 code 后，可自行决定如何 exchange（带正确 code_verifier、带错误 code_verifier、或不带 code_verifier）
- 不再依赖 demo app 或 portal 的回调处理逻辑

**Playwright 提取示例**:

```typescript
// 等待重定向到 QA test client 的 redirect_uri
await page.waitForURL(/localhost:19876\/callback/);
const url = new URL(page.url());
const code = url.searchParams.get("code");
const state = url.searchParams.get("state");

// 手动 exchange with PKCE
const response = await request.post("http://localhost:8080/api/v1/auth/token", {
  form: {
    grant_type: "authorization_code",
    code,
    redirect_uri: "http://localhost:19876/callback",
    client_id: "auth9-qa-test",
    code_verifier: codeVerifier, // 测试脚本自行生成和管理
  },
});
```

### R3: Test client 支持 state 参数管理

测试脚本完全控制 OIDC state 参数的生成和验证，不依赖 Portal 的 state 逻辑：

- 测试脚本自行生成 `state` 值（如 `crypto.randomUUID()`）
- 在 authorize 请求中传递自定义 state：
  ```
  GET /api/v1/auth/authorize?client_id=auth9-qa-test&redirect_uri=http://localhost:19876/callback&response_type=code&scope=openid%20email%20offline_access&state=<自定义state>&code_challenge=<S256hash>&code_challenge_method=S256
  ```
- 回调 URL 中 auth9-core 会原样返回 state 参数
- 测试脚本验证回调中的 state 与发送的一致，不再经过 Portal 的 state 校验

**完整 PKCE + state 管理流程**:

```typescript
import crypto from "crypto";

// 1. 生成 PKCE 参数
const codeVerifier = crypto.randomBytes(32).toString("base64url");
const codeChallenge = crypto
  .createHash("sha256")
  .update(codeVerifier)
  .digest("base64url");

// 2. 生成 state
const state = crypto.randomUUID();

// 3. 发起授权请求
const authorizeUrl = new URL("http://localhost:8080/api/v1/auth/authorize");
authorizeUrl.searchParams.set("client_id", "auth9-qa-test");
authorizeUrl.searchParams.set("redirect_uri", "http://localhost:19876/callback");
authorizeUrl.searchParams.set("response_type", "code");
authorizeUrl.searchParams.set("scope", "openid email offline_access");
authorizeUrl.searchParams.set("state", state);
authorizeUrl.searchParams.set("code_challenge", codeChallenge);
authorizeUrl.searchParams.set("code_challenge_method", "S256");

await page.goto(authorizeUrl.toString());

// 4. 完成登录（Playwright 自动化）
// ... 填写用户名密码并提交 ...

// 5. 提取回调参数
await page.waitForURL(/localhost:19876\/callback/);
const callbackUrl = new URL(page.url());
expect(callbackUrl.searchParams.get("state")).toBe(state); // state 验证
const code = callbackUrl.searchParams.get("code");

// 6. Token exchange with PKCE
const tokenResponse = await fetch("http://localhost:8080/api/v1/auth/token", {
  method: "POST",
  headers: { "Content-Type": "application/x-www-form-urlencoded" },
  body: new URLSearchParams({
    grant_type: "authorization_code",
    code,
    redirect_uri: "http://localhost:19876/callback",
    client_id: "auth9-qa-test",
    code_verifier: codeVerifier,
  }),
});
```

---

## 涉及文件

| 文件 | 变更内容 |
|------|---------|
| `auth9-core/src/migration/mod.rs` | 新增 QA test service + public client seed data |
| `scripts/reset-docker.sh` | 确保重置后 QA test client 存在（由 migration 自动处理） |
| `docs/qa/auth/16-pkce-flow.md` | 更新测试步骤使用 `auth9-qa-test` client |
| `docs/oidc/refresh-token/01-refresh-flow.md` | 更新测试步骤使用 QA test client 获取 refresh_token |

---

## 验证方法

### 代码验证

```bash
# 确认 QA test client 在 seed data 中
grep -r "auth9-qa-test" auth9-core/src/migration/

# 运行后端测试确保 migration 不破坏现有逻辑
cd auth9-core && cargo test
```

### 手动验证

1. 执行 `./scripts/reset-docker.sh` 重置环境
2. 确认数据库中存在 QA test client：
   ```sql
   SELECT c.client_id, c.public_client, s.redirect_uris
   FROM clients c JOIN services s ON c.service_id = s.id
   WHERE c.client_id = 'auth9-qa-test';
   ```
   预期：`public_client=1`, `redirect_uris` 包含 `http://localhost:19876/callback`
3. 使用 QA test client 发起 PKCE authorize 请求，验证返回 `login_challenge`
4. 完成登录后，确认浏览器重定向到 `http://localhost:19876/callback?code=XXX&state=YYY`（页面不加载，但 URL 参数可提取）
5. 使用提取的 code + 正确 code_verifier 执行 token exchange，预期成功获得 access_token + refresh_token
6. 使用错误 code_verifier 执行 token exchange，预期返回 `invalid_grant`

### 解除阻塞验证

实现后应能通过以下 QA 测试：

- `docs/oidc/authz-code/02-pkce-flow.md` Scenario 2 & 3
- `docs/oidc/refresh-token/01-refresh-flow.md` Scenario 1 & 2
