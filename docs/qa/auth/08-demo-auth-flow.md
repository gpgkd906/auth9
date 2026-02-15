# Auth9 Demo - 完整认证流程回归测试

**模块**: 认证流程 / Auth9 Demo
**测试范围**: auth9-demo 通过 Auth9 Core + Keycloak 的完整 OAuth 登录 & gRPC Token Exchange 流程
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
Keycloak (localhost:8081)
   ↓ 用户登录后 302 回 Auth9 Core
Auth9 Core /api/v1/auth/callback
   ↓ 存储 state → 重定向到 demo /auth/callback
auth9-demo /auth/callback
   ↓ POST /api/v1/auth/token (exchange code for Auth9-signed Identity Token)
Auth9 Core
   ↓ Exchange with Keycloak (public client, no secret)
   ↓ Return Auth9-signed Identity Token
auth9-demo /dashboard
   ↓ POST /demo/exchange-token (gRPC)
Auth9 Core gRPC → Tenant Access Token (with roles/permissions)
```

关键点：
- auth9-demo 是 **public client**（无 client_secret），token exchange 不传 secret
- Auth9 Core 的 callback URL 必须在 Keycloak 的 redirect_uris 中注册
- gRPC exchangeToken 的 `tenantId` 支持 UUID 和 slug 两种格式
- Demo 使用 Auth9-signed `access_token`（非 Keycloak 的 `id_token`）进行 gRPC 调用

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

## 场景 2：OAuth 授权 → Keycloak 登录页跳转

### 初始状态
- 用户未登录

### 目的
验证点击登录后正确跳转到 Keycloak，且 redirect_uri 指向 Auth9 Core（非 demo 直连）

### 测试操作流程
1. 访问 `http://localhost:3002`
2. 点击 "Login with Auth9"

### 预期结果
- 浏览器跳转到 Keycloak 登录页: `http://localhost:8081/realms/auth9/protocol/openid-connect/auth?...`
- URL 参数包含:
  - `client_id=auth9-demo`
  - `redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fapi%2Fv1%2Fauth%2Fcallback`（Auth9 Core 的 callback，不是 demo 的）
  - `scope=openid+profile+email`
- Keycloak 登录页正常显示（**不出现** "Invalid parameter: redirect_uri" 错误）
- 登录页显示 Auth9 Keycloak 主题

---

## 场景 3：Keycloak 登录 → Dashboard 跳转 & Identity Token 验证

### 初始状态
- 已跳转到 Keycloak 登录页
- admin 用户存在（admin / SecurePass123!）

### 目的
验证输入凭证后完成 OAuth code exchange，获取 Auth9-signed Identity Token，正确显示用户信息

### 测试操作流程
1. 在 Keycloak 登录页输入 username: `admin`, password: `SecurePass123!`
2. 点击 "Sign In"

### 预期结果
- 成功跳转到 `http://localhost:3002/dashboard`
- 页面标题: "Auth9 Demo - Dashboard"
- User Profile 区域显示:
  - User ID (sub): 有效 UUID
  - Email: `admin@auth9.local`
  - Name: `Admin User`
- Authentication Debug 区域:
  - 标签为 "Identity Token (Auth9-signed)"（非 "from Keycloak"）
  - Decoded Payload 包含 `sub`, `email`, `name` 字段
  - 展开 "View raw token" 显示 JWT 字符串（以 `eyJ` 开头）
- 页面右上角显示 "Logout" 链接
- **不出现** "Authentication failed" 或 "Token exchange failed" 错误

### 预期数据状态
```sql
-- Auth9 Core 应创建用户记录
SELECT id, keycloak_id, email, display_name FROM users WHERE email = 'admin@auth9.local';
-- 预期: 存在记录，keycloak_id 非空

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
| Keycloak redirect_uri | seeder 在 auth9-demo 客户端注册 Auth9 Core callback URL | 场景 2 |
| Public client token exchange | auth9-core 对 public client 不传 client_secret | 场景 3 |
| Identity Token 来源 | demo 使用 Auth9-signed `access_token`（非 Keycloak `id_token`） | 场景 3, 4 |
| gRPC tenant slug 支持 | exchangeToken 接受 tenant slug（如 `demo`）而非仅 UUID | 场景 4 |
| AUTH9_AUDIENCE 配置 | docker-compose 中 `AUTH9_AUDIENCE=auth9-demo`（匹配 client_id） | 场景 4 |
