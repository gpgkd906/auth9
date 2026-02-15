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

## 场景 2：Token 刷新

### 初始状态
- 持有即将过期的 Access Token
- 持有有效的 Refresh Token

### 目的
验证 Token 刷新流程

### 测试操作流程
1. 使用 Refresh Token 请求新的 Access Token

### 预期结果
- 获得新的 Access Token
- Refresh Token 可能被轮换

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
