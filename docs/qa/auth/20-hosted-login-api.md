# Auth - Hosted Login API

**模块**: Auth
**测试范围**: Hosted Login API 端点（密码登录、登出、密码重置）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Hosted Login API 提供 Auth9 自有域名下的统一认证端点，前端表单直接调用这些 API 完成认证流程，无需 OIDC 重定向。

端点：
- `POST /api/v1/hosted-login/password` — 密码登录，返回 identity token（含 `pending_actions`）
- `POST /api/v1/hosted-login/logout` — 登出，撤销 session 并返回 JSON
- `POST /api/v1/hosted-login/start-password-reset` — 发起密码重置
- `POST /api/v1/hosted-login/complete-password-reset` — 完成密码重置
- `POST /api/v1/hosted-login/send-verification` — 发送邮箱验证邮件（详见 `auth/22-email-verification.md`）
- `POST /api/v1/hosted-login/verify-email` — 消费验证 token（详见 `auth/22-email-verification.md`）
- `GET /api/v1/hosted-login/pending-actions` — 列出 pending actions（详见 `auth/23-required-actions.md`）
- `POST /api/v1/hosted-login/complete-action` — 完成 pending action（详见 `auth/23-required-actions.md`）

请求/响应示例：

```json
// POST /api/v1/hosted-login/password
// Request:
{ "email": "user@example.com", "password": "MyPassword123!" } <!-- pragma: allowlist secret -->
// Response (200):
{ "access_token": "eyJ...", "token_type": "Bearer", "expires_in": 3600 }
// Response (401):
{ "error": "unauthorized", "message": "Invalid email or password." }
```

---

## 场景 1：密码登录成功

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中：`curl -sf http://localhost:8080/health`
- 已有注册用户，邮箱和密码已知

### 初始状态
- 系统中存在用户 `qa-user@example.com`，密码为 `QaTest123!`

### 目的
验证通过 Hosted Login API 使用邮箱密码可成功获取 identity token

### 测试操作流程
1. 发送密码登录请求：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "QaTest123!"}' # pragma: allowlist secret \
  | jq .
```
2. 验证返回的 token 有效：
```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "QaTest123!"}' # pragma: allowlist secret \
  | jq -r '.access_token')

curl -s http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer $TOKEN" \
  | jq .
```

### 预期结果
- 步骤 1 返回 HTTP 200，包含 `access_token`、`token_type: "Bearer"`、`expires_in > 0`
- 步骤 2 userinfo 返回用户的 email 和 sub

---

## 场景 2：密码登录失败 — 错误密码（统一错误语义）

### 初始状态
- 系统中存在用户 `qa-user@example.com`

### 目的
验证错误密码返回统一错误码，不暴露 Keycloak 原始错误结构

### 测试操作流程
1. 使用错误密码登录：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "WrongPassword!"}' # pragma: allowlist secret \
  | jq .
```
2. 使用不存在的邮箱登录：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "nonexistent@example.com", "password": "SomePassword!"}' # pragma: allowlist secret \
  | jq .
```

### 预期结果
- 两个请求都返回 HTTP 401
- 响应体为 `{"error": "unauthorized", "message": "Invalid email or password."}`
- 响应中**不包含** Keycloak 错误字段（如 `error_description`、`invalid_grant` 等）
- 两个错误响应结构一致（防止邮箱枚举）

---

## 场景 3：Hosted Logout 撤销 Session

### 步骤 0（Gate Check）
- 已通过场景 1 获取有效 token

### 初始状态
- 用户已通过 `/api/v1/hosted-login/password` 登录并持有 token

### 目的
验证 Hosted Logout 端点能撤销 session 并返回 JSON（非重定向）

### 测试操作流程
1. 先登录获取 token：
```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "QaTest123!"}' # pragma: allowlist secret \
  | jq -r '.access_token')
```
2. 调用 hosted logout：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/logout \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{}' \
  | jq .
```
3. 尝试使用旧 token 访问 userinfo：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer $TOKEN" \
  | jq .
```

### 预期结果
- 步骤 2 返回 HTTP 200，`{"message": "Logged out successfully."}`
- 步骤 3 返回 HTTP 401（token 已失效）

### 预期数据状态
```sql
-- 验证 session 已被撤销
SELECT id, revoked_at FROM sessions WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: revoked_at IS NOT NULL
```

---

## 场景 4：密码重置流程（发起 + 完成）

### 初始状态
- 系统中存在用户 `qa-user@example.com`

### 目的
验证 Hosted Login 密码重置端点可正常发起和完成密码重置

### 测试操作流程
1. 发起密码重置：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/start-password-reset \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com"}' \
  | jq .
```
2. 对不存在的邮箱发起密码重置（应返回相同响应）：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/start-password-reset \
  -H "Content-Type: application/json" \
  -d '{"email": "nonexistent@example.com"}' \
  | jq .
```
3. 从数据库获取 reset token 并完成重置：
```bash
# 从数据库获取 token hash（仅限测试环境）
# 实际场景中 token 通过邮件发送给用户
TOKEN_HASH=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e \
  "SELECT token_hash FROM password_reset_tokens WHERE user_id = '{user_id}' AND used = 0 ORDER BY created_at DESC LIMIT 1")
```
4. 使用错误 token 完成重置（应失败）：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/complete-password-reset \
  -H "Content-Type: application/json" \
  -d '{"token": "invalid-token", "new_password": "NewPassword123!"}' # pragma: allowlist secret \
  | jq .
```

### 预期结果
- 步骤 1 返回 HTTP 200，消息提示已发送重置邮件
- 步骤 2 返回 HTTP 200，与步骤 1 相同消息（防止邮箱枚举）
- 步骤 4 返回 HTTP 400，提示 token 无效

---

## 场景 5：Backend Flag 切换验证

### 步骤 0（Gate Check）
- 确认 `IDENTITY_BACKEND` 环境变量可配置

### 初始状态
- 默认环境运行 `IDENTITY_BACKEND=keycloak`

### 目的
验证 Hosted Login API 在不同 `IDENTITY_BACKEND` 模式下均能响应请求

### 测试操作流程
1. 确认当前 backend 模式：
```bash
# 检查 docker-compose 或环境变量
docker exec auth9-core env | grep IDENTITY_BACKEND
```
2. 在 `keycloak` 模式下执行密码登录（同场景 1）
3. 切换到 `auth9_oidc` 模式（修改环境变量并重启服务）
4. 在 `auth9_oidc` 模式下执行密码登录：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "QaTest123!"}' # pragma: allowlist secret \
  | jq .
```

### 预期结果
- `keycloak` 模式：密码登录正常返回 200 和 token
- `auth9_oidc` 模式：端点正常响应（当前返回 501 Not Implemented 或类似 stub 错误，因 auth9-oidc adapter 尚未完整实现）
- 两种模式下**错误结构一致**，均为 `{"error": "...", "message": "..."}`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 密码登录成功 | ☐ | | | |
| 2 | 密码登录失败 — 统一错误语义 | ☐ | | | |
| 3 | Hosted Logout 撤销 Session | ☐ | | | |
| 4 | 密码重置流程 | ☐ | | | |
| 5 | Backend Flag 切换验证 | ☐ | | | |
