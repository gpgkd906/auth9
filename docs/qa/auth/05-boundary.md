# 认证流程 - 边界测试

**模块**: 认证流程
**测试范围**: 并发、Token 刷新、CORS
**场景数**: 3

---

## 场景 1：并发登录请求

### 初始状态
- 同一用户同时从多个设备登录

### 目的
验证并发登录处理

### 测试操作流程
1. 从设备 A 和设备 B 同时发起登录

### 预期结果
- 两个登录都成功
- 创建两个独立会话

### 预期数据状态
```sql
SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}' AND revoked_at IS NULL;
-- 预期: 2
```

---

## 场景 2：Token 刷新（Keycloak OIDC Refresh Token）

### 初始状态
- 用户已通过 OIDC 登录流程获取 Identity Token
- 持有 Keycloak 签发的 Refresh Token（从 `/api/v1/auth/token` 的 `grant_type=authorization_code` 响应中获得）

### 重要说明
- **本场景测试的是 Keycloak OIDC Refresh Token**，用于刷新 Identity Token 会话。
- **不要**使用 gRPC `ExchangeToken` 返回的 Auth9 签发 Refresh Token——该 Token 用于 Tenant Access Token 刷新，属于独立功能（当前仅实现了创建，消费端尚未实现）。
- Keycloak Refresh Token 来自 OIDC 登录流程，Auth9 Refresh Token 来自 gRPC Token Exchange 流程，两者不可互换。

### 目的
验证使用 Keycloak OIDC Refresh Token 刷新 Identity Token 的流程

### 测试操作流程
1. 通过 Portal 完成 OIDC 登录，获取 `refresh_token`（Keycloak 签发）
2. 使用 Keycloak refresh token 调用刷新接口：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=refresh_token&refresh_token={keycloak_refresh_token}&client_id=auth9-portal"
   ```
3. 检查返回的新 Identity Token

### 预期结果
- 获得新的 Identity Token（Auth9 签发的 JWT）
- Keycloak Refresh Token 可能被轮换（取决于 Keycloak 配置）
- 会话 Session 保持不变

### 如何获取 Keycloak Refresh Token

> **重要**: 不能使用 Keycloak admin-cli 获取的 refresh token。admin-cli 生成的 token 绑定到 `admin-cli` 客户端，
> 与 `auth9-portal` 客户端不匹配，Keycloak 会拒绝并返回 `"Token client and authorized client don't match"`。

获取正确 refresh token 的方法：
1. **通过浏览器完成 Portal OIDC 登录**，在浏览器 DevTools Network 面板中抓取 `/api/v1/auth/token` 响应中的 `refresh_token`
2. 该 token 由 Keycloak 签发，绑定到 `auth9-portal` 客户端，才能正确用于刷新

### 故障排查

| 症状 | 原因 | 解决方案 |
|------|------|---------|
| Keycloak 返回 400 `Token client and authorized client don't match` | 使用了 admin-cli 或其他客户端的 refresh token | **必须**使用 Portal OIDC 登录流程返回的 Keycloak refresh token |
| Keycloak 返回 400 错误 | 使用了 Auth9 gRPC 签发的 refresh token | 使用 OIDC 登录流程返回的 Keycloak refresh token |
| gRPC `RefreshToken` 方法不存在 | Auth9 gRPC refresh token 消费端未实现 | 这是已知限制，gRPC refresh token 功能仅完成创建，尚无消费接口 |

---

## 场景 3：跨域登录（CORS）

### 初始状态
- Auth9 Core 运行在 localhost:8080
- 默认 CORS 允许的 origin: `http://localhost:3000`, `http://localhost:5173`, `http://localhost:8081`
- 可通过环境变量 `CORS_ALLOWED_ORIGINS` 配置额外的 origin（逗号分隔，或 `*` 允许所有）

### 目的
验证 CORS 配置正确，允许的 origin 收到正确的 CORS 响应头

### 测试操作流程
1. 使用默认允许的 origin 发起 CORS 预检请求：
   ```bash
   curl -s -X OPTIONS http://localhost:8080/api/v1/auth/login \
     -H "Origin: http://localhost:3000" \
     -H "Access-Control-Request-Method: POST" \
     -H "Access-Control-Request-Headers: content-type" \
     -v 2>&1 | grep -i "access-control"
   ```
2. 使用未配置的 origin 发起 CORS 预检请求：
   ```bash
   curl -s -X OPTIONS http://localhost:8080/api/v1/auth/login \
     -H "Origin: https://app.example.com" \
     -H "Access-Control-Request-Method: POST" \
     -H "Access-Control-Request-Headers: content-type" \
     -v 2>&1 | grep -i "access-control-allow-origin"
   ```

### 预期结果
- 步骤 1：响应包含 `access-control-allow-origin: http://localhost:3000` 和 `access-control-allow-credentials: true`
- 步骤 2：响应不包含 `access-control-allow-origin` 头（未配置的 origin 被正确拒绝）
- 如需允许额外 origin，设置 `CORS_ALLOWED_ORIGINS` 环境变量后重启服务

---

## 测试数据准备

```sql
-- 准备测试用户（需要在 Keycloak 中也创建）
INSERT INTO users (id, keycloak_id, email, display_name, mfa_enabled) VALUES
('user-auth-1111-1111-111111111111', 'kc-auth-1', 'auth-test@example.com', 'Auth Test', false),
('user-auth-2222-2222-222222222222', 'kc-auth-2', 'mfa-test@example.com', 'MFA Test', true);

-- 准备测试租户
INSERT INTO tenants (id, name, slug, settings, status) VALUES
('tenant-auth-1111-1111-111111111111', 'Auth Test Tenant', 'auth-test', '{}', 'active');

-- 用户加入租户
INSERT INTO tenant_users (id, tenant_id, user_id, role_in_tenant) VALUES
('tu-auth-1111-1111-111111111111', 'tenant-auth-1111-1111-111111111111', 'user-auth-1111-1111-111111111111', 'member');

-- 清理
DELETE FROM tenant_users WHERE id LIKE 'tu-auth-%';
DELETE FROM tenants WHERE id LIKE 'tenant-auth-%';
DELETE FROM users WHERE id LIKE 'user-auth-%';
```

---

## Keycloak 测试用户

在 Keycloak 管理界面创建：

1. **标准用户**
   - Username: `auth-test@example.com`
   - Password: `TestPass123!`
   - MFA: 未启用

2. **MFA 用户**
   - Username: `mfa-test@example.com`
   - Password: `TestPass123!`
   - MFA: 已启用 TOTP

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 并发登录 | ☐ | | | |
| 2 | Token 刷新 | ☐ | | | |
| 3 | 跨域登录 | ☐ | | | |
