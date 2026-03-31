# OIDC Authorization Code Flow - 基本流程

**模块**: Authorization Code
**测试范围**: Authorization Code Grant 完整流程、Token 响应验证、错误处理
**场景数**: 5

---

## 前置条件

```bash
# 重置环境（含 Conformance Suite）
./scripts/reset-docker.sh --conformance

# 验证服务健康
curl -sf http://localhost:8080/health && echo "Core OK"
```

OIDC 客户端通过 `scripts/oidc-conformance-setup.sh` 预置，文档中使用占位符：
- `{client_id}` — 测试客户端 ID
- `{client_secret}` — 测试客户端密钥
- `{redirect_uri}` — 已注册的回调地址

Issuer 地址：`http://localhost:8080`（Host 端测试）

---

## 场景 1：完整 Authorization Code 流程「需浏览器」

### 初始状态
- auth9-core 运行中
- 测试客户端已配置（含 `{redirect_uri}`）
- 测试用户已存在

### 目的
验证 Authorization Code 完整流程：authorize → hosted login → complete → token exchange

### 测试操作流程
1. 发起 Authorize 请求（浏览器中打开）：
   ```
   GET http://localhost:8080/api/v1/auth/authorize?
     response_type=code&
     client_id={client_id}&
     redirect_uri={redirect_uri}&
     scope=openid profile email&
     state=random-state-value&
     nonce=random-nonce-value
   ```
2. 浏览器被重定向到 Hosted Login 页面，URL 中包含 `login_challenge` 参数
3. 用户在 Hosted Login 页面输入凭据完成登录
4. 后端调用 `/api/v1/auth/authorize/complete` 完成认证：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/authorize/complete \
     -H "Content-Type: application/json" \
     -d '{
       "identity_token": "<用户登录后获取的 identity_token>",
       "login_challenge": "<步骤 2 中的 login_challenge>"
     }'
   ```
5. 服务端返回 302 重定向到 `{redirect_uri}?code=<auth_code>&state=random-state-value`
6. 使用 auth code 换取 token：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<auth_code>&client_id={client_id}&redirect_uri={redirect_uri}" \
     -u "{client_id}:{client_secret}"
   ```

### 预期结果
- 步骤 2：302 重定向到 Hosted Login，URL 含 `login_challenge`（存储在 Redis，TTL 10 分钟）
- 步骤 5：302 重定向到 `{redirect_uri}`，query 含 `code` 和 `state=random-state-value`
- 步骤 6：HTTP 200，返回 JSON 包含 `access_token`、`id_token`、`token_type`

> **troubleshooting**: `login_challenge` 基于 Redis 存储，需确保 auth9-portal 和 auth9-core 连接到同一 Redis 实例。如果 authorize/complete 返回 `invalid_request` 或找不到 login_challenge，请检查：
> 1. Redis 连接配置是否一致（host、port、db）
> 2. Redis 实例是否正常运行（`redis-cli ping` 应返回 `PONG`）
> 3. login_challenge 是否已过期（默认 TTL 10 分钟）

---

## 场景 2：Token 响应包含 access_token、id_token、refresh_token

### 初始状态
- 已通过场景 1 获取有效 authorization code
- scope 包含 `openid offline_access`

### 目的
验证 Token 端点返回完整的 token 集合，字段格式正确

### 测试操作流程
1. 使用有效 auth code 换取 token：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<auth_code>&client_id={client_id}&redirect_uri={redirect_uri}" \
     -u "{client_id}:{client_secret}" | jq .
   ```
2. 验证返回的所有字段

### 预期结果
- HTTP 200
- 返回 JSON 包含以下字段：

| 字段 | 预期值 |
|------|--------|
| `access_token` | 非空 JWT 字符串 |
| `id_token` | 非空 JWT 字符串 |
| `refresh_token` | 非空字符串（scope 含 `offline_access` 时） |
| `token_type` | `Bearer` |
| `expires_in` | 正整数（秒） |

### 预期数据状态

> **注意**: Authorization codes 和 login challenges 存储在 Redis 中并设置短 TTL，不存储在数据库表中。Auth code 被消费后会从 Redis 中删除。
>
> ```bash
> # 验证 auth code 已被消费（换取 token 后 key 应不存在）
> redis-cli GET "authorization_code:<auth_code>"
> # 预期: (nil) — 已被消费并删除
> ```

---

## 场景 3：id_token 包含必需 claims

### 初始状态
- 已通过 Token 端点获取 `id_token`
- Authorize 请求中包含 `nonce=test-nonce-value`

### 目的
验证 id_token 符合 OIDC Core 规范，包含所有必需 claims

### 测试操作流程
1. 解码 id_token 的 payload：
   ```bash
   echo "<id_token>" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
   ```
2. 验证 header：
   ```bash
   echo "<id_token>" | cut -d. -f1 | base64 -d 2>/dev/null | jq .
   ```

### 预期结果
- id_token header：

| 字段 | 预期值 |
|------|--------|
| `alg` | `RS256` |
| `kid` | `auth9-current` |
| `typ` | `JWT` |

- id_token payload 包含以下必需 claims：

| Claim | 说明 | 验证规则 |
|-------|------|----------|
| `iss` | Issuer | `http://localhost:8080` |
| `sub` | Subject（用户 ID） | 非空 UUID 字符串 |
| `aud` | Audience | 包含 `{client_id}` |
| `exp` | 过期时间 | Unix 时间戳，大于当前时间 |
| `iat` | 签发时间 | Unix 时间戳，小于等于当前时间 |
| `nonce` | 请求中的 nonce | `test-nonce-value` |

---

## 场景 4：使用无效 code 换取 token 返回错误

### 初始状态
- auth9-core 运行中
- 测试客户端已配置

### 目的
验证使用无效、过期或已使用的 authorization code 时 Token 端点返回正确错误

### 测试操作流程
1. 使用完全无效的 code：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=invalid-code-12345&client_id={client_id}&redirect_uri={redirect_uri}" \
     -u "{client_id}:{client_secret}" | jq .
   ```
2. 使用已消费的 code（重放场景 1 中已使用的 code）：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<已使用的code>&client_id={client_id}&redirect_uri={redirect_uri}" \
     -u "{client_id}:{client_secret}" | jq .
   ```
3. 使用过期 code（auth code TTL 为 2 分钟，等待超时后使用）：
   ```bash
   # 获取 code 后等待 2 分钟以上再换取 token
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<过期的code>&client_id={client_id}&redirect_uri={redirect_uri}" \
     -u "{client_id}:{client_secret}" | jq .
   ```

### 预期结果
- 所有情况返回 HTTP 400
- 响应 JSON 包含 `error` 字段：

| 场景 | `error` | `error_description` |
|------|---------|---------------------|
| 无效 code | `invalid_grant` | 包含 "invalid" 相关描述 |
| 已使用 code | `invalid_grant` | 包含 "already used" 或类似描述 |
| 过期 code | `invalid_grant` | 包含 "expired" 相关描述 |

---

## 场景 5：client_id 或 redirect_uri 不匹配时 token 换取失败

### 初始状态
- 已获取有效的 authorization code（绑定了特定 `{client_id}` 和 `{redirect_uri}`）

### 目的
验证 Token 端点严格校验 client_id 和 redirect_uri 与 authorize 阶段一致

### 测试操作流程
1. 使用错误的 client_id：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<valid_code>&client_id=wrong-client-id&redirect_uri={redirect_uri}" \
     -u "wrong-client-id:wrong-secret" | jq .
   ```
2. 使用错误的 redirect_uri：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&code=<valid_code>&client_id={client_id}&redirect_uri=https://evil.example.com/callback" \
     -u "{client_id}:{client_secret}" | jq .
   ```
3. 缺少 state 参数的 Authorize 请求（验证 state 必填）：
   ```bash
   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&scope=openid"
   ```

### 预期结果
- 步骤 1：HTTP 400/401，`error` 为 `invalid_client` 或 `invalid_grant`
- 步骤 2：HTTP 400，`error` 为 `invalid_grant`，redirect_uri 不匹配
- 步骤 3：HTTP 400，拒绝缺少 state 的请求（state 参数必填且不能为空）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 完整 Authorization Code 流程 | ☐ | | | 需浏览器 |
| 2 | Token 响应字段完整性 | ☐ | | | |
| 3 | id_token claims 验证 | ☐ | | | |
| 4 | 无效 code 错误处理 | ☐ | | | |
| 5 | client_id / redirect_uri 不匹配 | ☐ | | | |
