# Auth9 Demo - 完整认证流程回归测试

**模块**: 认证流程 / Auth9 Demo
**测试范围**: auth9-demo 通过 Auth9 Core + 底层 OIDC 引擎的完整 OAuth 登录 & gRPC Token Exchange 流程
**场景数**: 5
**前置条件**: `./scripts/reset-docker.sh` 完成，所有服务健康

---

## 架构说明

auth9-demo 作为第三方应用，演示 Auth9 SDK 集成的完整流程：

```
auth9-demo (localhost:3002)
   ↓ GET /api/v1/auth/authorize
Auth9 Core (localhost:8080)
   ↓ 302 Redirect (redirect_uri=Auth9 Core callback)
Auth9 OIDC 引擎
   ↓ 用户登录后 302 回 Auth9 Core
Auth9 Core /api/v1/auth/callback
   ↓ 存储 state → 重定向到 demo /auth/callback
auth9-demo /auth/callback
   ↓ POST /api/v1/auth/token (exchange code for Auth9-signed Identity Token)
Auth9 Core
   ↓ 验证并签发 Identity Token (public client, no secret)
   ↓ Return Auth9-signed Identity Token
auth9-demo /dashboard
   ↓ POST /demo/exchange-token (gRPC)
Auth9 Core gRPC → Tenant Access Token (with roles/permissions)
```

关键点：
- auth9-demo 是 **public client**（无 client_secret），token exchange 不传 secret
- auth9-demo 配置了 `pkce.code.challenge.method=S256`，**强制要求 PKCE**
- Auth9 Core 的 callback URL 必须在 OIDC client 的 redirect_uris 中注册
- gRPC exchangeToken 的 `tenantId` 支持 UUID 和 slug 两种格式
- Demo 使用 Auth9-signed `access_token` 进行 gRPC 调用

> **与 Portal 登录流程的区别**：auth9-demo 是独立的第三方示例应用，其「Login with Auth9」按钮直接调用 `/api/v1/auth/authorize`（不带 `connector_alias`），进入 Auth9 托管的密码认证链路。这等价于 Auth9 Portal `/login` 页面上的「**Sign in with password**」路径。Demo 应用不涉及 Enterprise SSO 或 Passkey。

---

## 场景 1：Demo 首页加载 & 未登录状态

### 初始状态
- 用户未登录（无 session）

### 目的
验证 auth9-demo 首页正常加载，显示登录入口

### 测试操作流程
1. 浏览器访问 `http://localhost:3002`

### 预期结果
- 页面标题: "Auth9 Demo - SDK Integration Guide"
- 显示 "You are currently not logged in."
- 显示 "Login with Auth9" 链接，指向 `/login`
- 显示可用 API 端点列表（/health, /api/me, /api/admin 等）
- 配置表显示 `AUTH9_AUDIENCE: auth9-demo`

---

## 场景 2：OAuth 授权 → 托管认证页跳转

### 初始状态
- 用户未登录

### 目的
验证点击登录后正确进入托管认证链路，且 redirect_uri 指向 Auth9 Core（非 demo 直连）

### 测试操作流程
1. 访问 `http://localhost:3002`
2. 点击 "Login with Auth9"

### 预期结果
- 浏览器进入托管认证页，对应的授权请求由底层 OIDC 引擎处理
- URL 参数包含:
  - `client_id=auth9-demo`
  - `redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fapi%2Fv1%2Fauth%2Fcallback`（Auth9 Core 的 callback，不是 demo 的）
  - `scope=openid+profile+email`
- 托管认证页正常显示（**不出现** "Invalid parameter: redirect_uri" 错误）
- 登录页显示 Auth9 品牌主题

---

## 场景 3：托管认证页登录 → Dashboard 跳转 & Identity Token 验证

> **环境排查注意**: The default seed user password may be flagged by HIBP breach checking. If `breach_check_mode=blocking` (Docker default), login will fail with '邮箱或密码无效'. To test, either change the seed password to a non-breached value or set `BREACH_CHECK_MODE=off` in the environment.

### 前置检查

执行测试前请确认以下条件，避免环境问题导致误报：

```bash
# 1. Redis 运行且可连接（OAuth state 存储在 Redis 中，TTL=300s）
docker exec auth9-redis redis-cli PING
# 预期: PONG

# 2. auth9-core 健康
curl -sf http://localhost:8080/health
# 预期: HTTP 200
```

### 初始状态
- 已进入 Auth9 托管认证页
- admin 用户存在（admin / SecurePass123!）
- `users.mfa_enabled` 对 `admin@auth9.local` 为 `0`

### 步骤 0：验证测试数据完整性

在执行登录前，先确认种子用户未被历史测试污染：

```sql
SELECT id, email, mfa_enabled
FROM users
WHERE email = 'admin@auth9.local';
-- 预期: mfa_enabled = 0
```

若 `mfa_enabled = 1`，说明本地数据已被先前测试修改。先执行 `./scripts/reset-docker.sh` 重建环境，再继续本场景；否则会被认证流程正常引导到 MFA 配置页，这不是 OAuth Demo Flow 缺陷。

> **泄露密码拦截**: 默认管理员密码 `SecurePass123!` 在 HIBP 数据库中有 610+ 次泄露记录。若租户启用了 `breach_check_mode=blocking`（默认），托管登录页会返回 HTTP 422 "This password has been found in a data breach"，导致登录失败。解决方法：
> 1. 执行 `./scripts/reset-docker.sh` 并配置一个非泄露密码，或
> 2. 在测试前将目标租户的 breach_check_mode 设置为 `disabled`：
>    ```sql
>    UPDATE tenants SET breach_check_mode = 'disabled' WHERE slug = 'auth9-platform';
>    ```

### 目的
验证输入凭证后完成 OAuth code exchange，获取 Auth9-signed Identity Token，正确显示用户信息

### 测试操作流程
1. 在 Auth9 品牌认证页输入 email: **`admin@auth9.local`**, password: `SecurePass123!`
   > **注意**: 必须输入完整邮箱地址 `admin@auth9.local`，不能只输入 `admin`。Hosted Login API 要求有效邮箱格式（包含 `@`），否则返回 400 Bad Request。
2. 点击 "Sign In"

### 预期结果
- 成功跳转到 `http://localhost:3002/dashboard`
- 页面标题: "Auth9 Demo - Dashboard"
- User Profile 区域显示:
  - User ID (sub): 有效 UUID
  - Email: `admin@auth9.local`
  - Name: `Admin User`
- Authentication Debug 区域:
  - 标签为 "Identity Token (Auth9-signed)"
  - Decoded Payload 包含 `sub`, `email`, `name` 字段
  - 展开 "View raw token" 显示 JWT 字符串（以 `eyJ` 开头）
- 页面右上角显示 "Logout" 链接
- **不出现** "Authentication failed" 或 "Token exchange failed" 错误

### OAuth Callback 常见故障排查

| 现象 | 原因 | 解决方法 |
|------|------|----------|
| "Missing state" 或 "Invalid state" 错误 | Redis 未运行或 state key 已过期（TTL=300s） | `docker exec auth9-redis redis-cli PING` 确认 Redis 可达；若登录耗时超过 5 分钟需重新发起 |
| "Missing state" 但 Redis 正常 | OIDC 引擎未在重定向中保留 state 参数 | 检查 OIDC client 的 Valid Redirect URIs 配置，确认包含 `http://localhost:8080/api/v1/auth/callback` |
| callback 返回 500 或连接拒绝 | auth9-demo 的 `AUTH9_URL` 配置错误，指向了错误的 auth9-core 地址 | 检查 docker-compose 中 `AUTH9_URL` 环境变量，应为 `http://localhost:8080`（或容器内地址） |
| 登录后停留在空白页 | 浏览器 cookie 阻止了跨域重定向 | 使用 Chrome 无痕模式，或确认 `localhost` 域名一致（不要混用 `127.0.0.1`） |

### 预期数据状态
```sql
-- Auth9 Core 应创建用户记录
SELECT id, identity_subject, email, display_name FROM users WHERE email = 'admin@auth9.local';
-- 预期: 存在记录，identity_subject 非空

-- 应创建 session
SELECT id, user_id, created_at FROM sessions
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在新会话

-- 应记录登录事件
SELECT event_type, email FROM login_events
WHERE email = 'admin@auth9.local' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'login'
```

---

## 场景 4：Token Exchange — Identity Token → Tenant Access Token (gRPC)

### 初始状态
- 用户已登录 Dashboard
- Demo tenant (slug: `demo`) 存在，admin 用户已关联

### 目的
验证 gRPC Token Exchange：使用 Auth9-signed Identity Token 换取含 roles/permissions 的 Tenant Access Token

### 测试操作流程
1. 在 Dashboard 页面，找到 "Tenant Access (Token Exchange)" 区域
2. 点击 "Exchange Token for 'demo' tenant" 按钮

### 预期结果
- 按钮下方显示 "Tenant Access Token Payload:"
- 返回 JSON 包含:
  - `accessToken`: 有效 JWT 字符串（以 `eyJ` 开头）
  - `tokenType`: `"Bearer"`
  - `expiresIn`: 正整数（如 `3600`）
  - `refreshToken`: 有效 JWT 字符串
- 解码 `accessToken` 应包含:
  - `sub`: 与 Identity Token 中相同的 user ID
  - `email`: `admin@auth9.local`
  - `iss`: `http://localhost:8080`
  - `aud`: `auth9-demo`
  - `tenant_id`: 有效 UUID（demo 租户的 ID）
  - `roles`: 数组（可能为空或含 admin 角色）
  - `permissions`: 数组
- **不出现** "InvalidSignature"、"Invalid tenant ID"、"Client not found" 错误

### 验证要点
```bash
# 可用 jq 解码 JWT payload 验证
echo "<accessToken>" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 应包含 tenant_id, aud, roles, permissions 字段
```

---

## 场景 5：登出 & 重新访问保护页面

### 初始状态
- 用户已登录 Dashboard

### 目的
验证登出后 session 清除，受保护页面无法直接访问

### 测试操作流程
1. 在 Dashboard 点击 "Logout"
2. 确认跳转回首页
3. 直接访问 `http://localhost:3002/dashboard`

### 预期结果
- 点击 Logout 后跳转回 `http://localhost:3002/`
- 首页显示 "You are currently not logged in."
- 直接访问 `/dashboard` 被重定向到首页（无 session 保护）
- 不显示之前的用户信息或 token

---

## 回归测试检查清单

以下是此文档覆盖的关键修复点，确保不出现回归：

| 检查项 | 修复内容 | 验证场景 |
|--------|----------|----------|
| OIDC redirect_uri | seeder 在 auth9-demo 客户端注册 Auth9 Core callback URL | 场景 2 |
| Public client token exchange | auth9-core 对 public client 不传 client_secret | 场景 3 |
| Identity Token 来源 | demo 使用 Auth9-signed `access_token` | 场景 3, 4 |
| gRPC tenant slug 支持 | exchangeToken 接受 tenant slug（如 `demo`）而非仅 UUID | 场景 4 |
| AUTH9_AUDIENCE 配置 | docker-compose 中 `AUTH9_AUDIENCE=auth9-demo`（匹配 client_id） | 场景 4 |
