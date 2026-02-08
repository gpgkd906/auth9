# Passkeys (WebAuthn) - API 端点测试

**模块**: Passkeys
**测试范围**: WebAuthn API 端点直接调用测试（curl）
**场景数**: 5
**优先级**: 高

---

## 背景说明

WebAuthn 功能涉及 6 个 API 端点，分为三组：

**注册（需要认证）：**
- `POST /api/v1/users/me/passkeys/register/start` — 生成注册挑战
- `POST /api/v1/users/me/passkeys/register/complete` — 完成注册

**认证（公开，无需认证）：**
- `POST /api/v1/auth/webauthn/authenticate/start` — 生成认证挑战
- `POST /api/v1/auth/webauthn/authenticate/complete` — 完成认证，返回 Token

**管理（需要认证）：**
- `GET /api/v1/users/me/passkeys` — 列出 Passkeys
- `DELETE /api/v1/users/me/passkeys/{credential_id}` — 删除 Passkey

挑战状态存储在 Redis，默认 TTL 300 秒。

---

## 数据库表结构参考

### webauthn_credentials 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 所属用户 ID |
| credential_id | VARCHAR(512) | WebAuthn 凭据 ID（UNIQUE） |
| credential_data | JSON | 序列化的 webauthn-rs Passkey 结构 |
| user_label | VARCHAR(255) | 用户自定义名称 |
| aaguid | VARCHAR(64) | 认证器标识 |
| created_at | TIMESTAMP | 创建时间 |
| last_used_at | TIMESTAMP | 最后使用时间 |

---

## 场景 1：注册挑战生成（start_registration）

### 初始状态
- 用户已通过 SSO 或 Passkey 登录，持有有效的 Identity Token

### 目的
验证注册挑战端点返回正确的 `CreationChallengeResponse` 格式

### 测试操作流程
1. 使用有效 Token 调用注册开始端点：

```bash
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start \
  -H "Authorization: Bearer {access_token}" \
  -H "Content-Type: application/json" | jq .
```

### 预期结果
- HTTP 200 响应
- 返回 JSON 包含 WebAuthn `CreationChallengeResponse` 格式：
  - `publicKey.challenge` — base64url 编码的挑战值
  - `publicKey.rp` — 包含 `name` 和 `id`（RP 域名）
  - `publicKey.user` — 包含 `id`、`name`、`displayName`
  - `publicKey.pubKeyCredParams` — 支持的算法列表
  - `publicKey.excludeCredentials` — 已注册凭据 ID 列表（用于排重）
- 如用户已有 Passkey，`excludeCredentials` 应包含已注册凭据的 ID

```bash
# 验证无 Token 时拒绝访问
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start \
  -H "Content-Type: application/json"
# 预期: HTTP 401 {"error": "Missing authorization header"}
```

### 预期数据状态
```sql
-- Redis 验证（使用 redis-cli）
-- redis-cli GET "auth9:webauthn_reg:{user_id}"
-- 预期: 存在序列化的 PasskeyRegistration 状态，TTL ~300 秒
```

---

## 场景 2：认证挑战生成（start_authentication）

### 初始状态
- 无需登录（公开端点）

### 目的
验证认证挑战端点返回正确的 discoverable authentication 格式

### 测试操作流程
1. 调用认证开始端点（不需要 Token）：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/start \
  -H "Content-Type: application/json" | jq .
```

### 预期结果
- HTTP 200 响应
- 返回 JSON 包含：
  - `challenge_id` — UUID 格式的挑战标识
  - `public_key.challenge` — base64url 编码的挑战值
  - `public_key.rpId` — RP 域名
  - `public_key.allowCredentials` — 空数组或无此字段（discoverable authentication 不指定凭据）
  - `public_key.userVerification` — `"preferred"` 或 `"required"`

```json
{
  "challenge_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "public_key": {
    "challenge": "...",
    "rpId": "localhost",
    "userVerification": "preferred"
  }
}
```

### 预期数据状态
```sql
-- Redis 验证
-- redis-cli GET "auth9:webauthn_auth:{challenge_id}"
-- 预期: 存在序列化的 DiscoverableAuthentication 状态，TTL ~300 秒
```

---

## 场景 3：Passkeys 列表查询

### 初始状态
- 用户已登录，持有有效 Token
- 用户已注册 1 个或多个 Passkeys

### 目的
验证列表端点返回正确的凭据数据

### 测试操作流程
1. 查询 Passkey 列表：

```bash
curl -s http://localhost:8080/api/v1/users/me/passkeys \
  -H "Authorization: Bearer {access_token}" | jq .
```

### 预期结果
- HTTP 200 响应
- 返回 JSON 包含 `data` 数组，每个元素：
  - `id` — 凭据 UUID
  - `user_label` — 用户自定义名称（可为空字符串）
  - `credential_type` — 凭据类型（如 `webauthn-passwordless`、`webauthn`）
  - `created_at` — ISO 8601 时间格式

```json
{
  "data": [
    {
      "id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
      "user_label": "MacBook Pro TouchID",
      "credential_type": "webauthn-passwordless",
      "created_at": "2026-02-08T12:00:00Z"
    }
  ]
}
```

```bash
# 验证无 Token 时拒绝
curl -s http://localhost:8080/api/v1/users/me/passkeys
# 预期: HTTP 401
```

### 预期数据状态
```sql
SELECT id, user_label, created_at
FROM webauthn_credentials
WHERE user_id = '{user_id}';
-- 预期: 与 API 返回结果一致
```

---

## 场景 4：删除 Passkey（API）

### 初始状态
- 用户已登录
- 用户已注册至少 1 个 Passkey
- 已获取要删除的 `credential_id`

### 目的
验证删除端点正确移除凭据

### 测试操作流程
1. 先查询列表获取 `credential_id`：

```bash
CRED_ID=$(curl -s http://localhost:8080/api/v1/users/me/passkeys \
  -H "Authorization: Bearer {access_token}" | jq -r '.data[0].id')
```

2. 调用删除端点：

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/users/me/passkeys/${CRED_ID}" \
  -H "Authorization: Bearer {access_token}" | jq .
```

### 预期结果
- HTTP 200 响应
- 返回 `{"message": "Passkey deleted successfully."}`
- 再次查询列表时该 Passkey 不再出现

```bash
# 验证删除其他用户的 Passkey 被拒绝
curl -s -X DELETE "http://localhost:8080/api/v1/users/me/passkeys/{other_user_cred_id}" \
  -H "Authorization: Bearer {access_token}"
# 预期: HTTP 404 或错误响应

# 验证删除不存在的 ID
curl -s -X DELETE "http://localhost:8080/api/v1/users/me/passkeys/nonexistent-id" \
  -H "Authorization: Bearer {access_token}"
# 预期: HTTP 404 或错误响应
```

### 预期数据状态
```sql
SELECT COUNT(*) FROM webauthn_credentials
WHERE id = '{credential_id}';
-- 预期: 0
```

---

## 场景 5：认证完成端点错误处理

### 初始状态
- 无需登录

### 目的
验证认证完成端点在无效数据时正确返回错误

### 测试操作流程
1. 使用无效的 `challenge_id` 调用完成端点：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/complete \
  -H "Content-Type: application/json" \
  -d '{
    "challenge_id": "nonexistent-challenge-id",
    "credential": {
      "id": "test",
      "rawId": "dGVzdA",
      "type": "public-key",
      "response": {
        "authenticatorData": "dGVzdA",
        "clientDataJSON": "dGVzdA",
        "signature": "dGVzdA"
      }
    }
  }' | jq .
```

2. 使用过期的挑战（等待超过 5 分钟后）：

```bash
# 先获取一个挑战
CHALLENGE=$(curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/start | jq -r '.challenge_id')
# 等待 > 300 秒后...
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/complete \
  -H "Content-Type: application/json" \
  -d "{\"challenge_id\": \"${CHALLENGE}\", \"credential\": {...}}"
```

3. 发送格式错误的请求体：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/complete \
  -H "Content-Type: application/json" \
  -d '{"invalid": "data"}' | jq .
```

### 预期结果
- 无效 `challenge_id`：返回错误（如 HTTP 400 "No pending authentication state"）
- 过期挑战：Redis 中状态已过期，返回相同错误
- 格式错误请求：返回 HTTP 400/422 解析错误
- 所有错误响应不泄露内部实现细节

---

## 通用场景：注册端点认证保护

### 测试操作流程
1. 不带 Authorization 头调用所有需认证的端点：

```bash
# 注册开始
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start
# 注册完成
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/complete \
  -H "Content-Type: application/json" -d '{}'
# 列表
curl -s http://localhost:8080/api/v1/users/me/passkeys
# 删除
curl -s -X DELETE http://localhost:8080/api/v1/users/me/passkeys/test-id
```

### 预期结果
- 所有请求返回 HTTP 401
- 返回错误消息 `"Missing authorization header"` 或类似
- 认证端点（`/api/v1/auth/webauthn/authenticate/*`）**不需要**认证，可正常访问

---

## Agent 自动化测试：Playwright MCP 工具

本文档的所有场景可由 AI Agent 通过 Playwright MCP 工具和 Bash 工具（curl）执行。API 端点测试主要用 curl 直接调用，部分场景需要结合虚拟认证器通过 UI 注册凭据后再验证 API。

> **前提条件**: 全栈环境运行中（Docker + auth9-core on :8080 + auth9-portal on :3000）。

### 场景 2：认证挑战（公开端点）

> 此场景最简单，不需要浏览器，直接用 Bash curl 验证。

调用 **Bash**:
```bash
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/start \
  -H "Content-Type: application/json" | jq .
```

**验证**: 返回 HTTP 200，JSON 包含 `challenge_id`（UUID）和 `public_key.challenge`（base64url 字符串）。

### 场景 5 + 通用：错误处理和认证保护

> 同样不需要浏览器，纯 curl 测试。

调用 **Bash**（多个命令）:
```bash
# 无效 challenge_id
curl -s -w "\nHTTP %{http_code}" -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/complete \
  -H "Content-Type: application/json" \
  -d '{"challenge_id":"nonexistent","credential":{"id":"t","rawId":"dA","type":"public-key","response":{"authenticatorData":"dA","clientDataJSON":"dA","signature":"dA"}}}'

# 格式错误请求
curl -s -w "\nHTTP %{http_code}" -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/complete \
  -H "Content-Type: application/json" -d '{"invalid":"data"}'

# 认证保护：无 Token 调用受保护端点
curl -s -w "\nHTTP %{http_code}" -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start
curl -s -w "\nHTTP %{http_code}" http://localhost:8080/api/v1/users/me/passkeys
curl -s -w "\nHTTP %{http_code}" -X DELETE http://localhost:8080/api/v1/users/me/passkeys/test-id
```

**验证**:
- 无效 challenge: HTTP 400
- 格式错误: HTTP 400/422
- 无 Token 的受保护端点: 全部 HTTP 401
- 认证端点（`/auth/webauthn/authenticate/*`）不需要 Token

### 场景 1、3、4：注册挑战 + 列表 + 删除

> 这些场景需要有效的 Identity Token。通过 `browser_run_code` 在浏览器中登录并用虚拟认证器注册 Passkey，然后用 UI 验证列表和删除。

**步骤 1**: 调用 **`browser_run_code`** 创建虚拟认证器 + 登录 + 注册:
```javascript
async (page) => {
  // 创建虚拟认证器
  const client = await page.context().newCDPSession(page);
  await client.send('WebAuthn.enable');
  await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal',
      hasResidentKey: true, hasUserVerification: true,
      isUserVerified: true, automaticPresenceSimulation: true,
    },
  });

  // SSO 登录
  await page.goto('http://localhost:3000/login');
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/, { timeout: 10000 });
  await page.getByLabel(/username/i).fill('e2e-test-user');
  await page.getByLabel(/password/i).fill('Test123!');
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL(/localhost:3000/, { timeout: 15000 });

  // 注册 Passkey
  await page.goto('http://localhost:3000/dashboard/account/passkeys');
  await page.waitForSelector('text=Passkeys', { timeout: 5000 });
  await page.getByRole('button', { name: /Add passkey/i }).first().click();
  await page.waitForSelector('text=Passkey registered successfully', { timeout: 10000 });

  return { success: true };
}
```

**步骤 2**: 调用 **`browser_snapshot`** 验证场景 3（列表显示）:
- 确认列表中有 Passkey 条目
- 确认显示类型标签、创建日期、「Remove」按钮

**步骤 3**: 调用 **`browser_click`** 点击「Remove」按钮（场景 4）

**步骤 4**: 调用 **`browser_snapshot`** 验证删除结果:
- 确认 Passkey 已从列表消失

**步骤 5**（可选）: 用 Bash curl 验证场景 1（注册挑战格式），需要从浏览器提取 Token:
```bash
# 如果有 Token（可通过 QA 工具脚本获取）
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start \
  -H "Authorization: Bearer {access_token}" | jq .
# 验证: 返回 publicKey.challenge, publicKey.rp, publicKey.user 等字段
```

### 自动化覆盖总结

| 场景 | 自动化 | Agent 工具 | 说明 |
|------|--------|------------|------|
| 1. 注册挑战生成 | ✅ | Bash (curl) 或 browser_run_code | 需要 Token |
| 2. 认证挑战生成 | ✅ | Bash (curl) | 公开端点，最简单 |
| 3. 列表查询 | ✅ | browser_run_code → browser_snapshot | 通过 UI 验证 |
| 4. 删除 Passkey | ✅ | browser_click → browser_snapshot | 通过 UI 验证 |
| 5. 错误处理 | ✅ | Bash (curl) | 无需浏览器 |
| 通用. 认证保护 | ✅ | Bash (curl) | 验证 401 响应 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 注册挑战生成（start_registration） | ☐ | | | |
| 2 | 认证挑战生成（start_authentication） | ☐ | | | 公开端点 |
| 3 | Passkeys 列表查询 | ☐ | | | |
| 4 | 删除 Passkey（API） | ☐ | | | |
| 5 | 认证完成端点错误处理 | ☐ | | | |
| 6 | 注册端点认证保护 | ☐ | | | |
