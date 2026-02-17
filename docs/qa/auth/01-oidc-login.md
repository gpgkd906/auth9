# 认证流程 - OIDC 登录测试

**模块**: 认证流程
**测试范围**: OIDC 标准登录流程
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构：
1. **Keycloak** 仅作为底层 OIDC/MFA 认证引擎，终端用户通过 Auth9 登录入口触发 OIDC 流程（非直接使用 Keycloak 原生入口）
2. **auth9-keycloak-theme** 对 Keycloak 的登录/注册页面进行完全自定义（基于 Keycloakify），用户看到的是 Auth9 品牌风格的登录界面，而非 Keycloak 原生 UI
3. **Auth9 Core** 处理所有业务逻辑（用户管理、多租户、RBAC 等）
4. **Token Exchange** 将 Keycloak 签发的 Identity Token 转换为包含角色/权限的 Tenant Access Token

**Portal 登录页三种认证方式**（`/login`）：

| 按钮 | 认证方式 | 流程 |
|------|---------|------|
| **Continue with Enterprise SSO** | 企业 SSO | 输入邮箱 → 域名发现 → Keycloak + `kc_idp_hint` → 直跳企业 IdP |
| **Sign in with password** | 密码登录 | → Keycloak 默认登录页（auth9-keycloak-theme 自定义外观）→ 输入用户名+密码 |
| **Sign in with passkey** | Passkey | WebAuthn API → 无密码认证（不经过 Keycloak） |

**本文档测试的是「Sign in with password」路径**，即通过 Keycloak 品牌化登录页进行用户名+密码认证。

**登录流程中的页面归属**：
- Portal `/login` 页面 → 认证方式选择入口（Auth9 Portal 提供）
- 用户名密码/注册/MFA 页面 → 由 Keycloak 托管，使用 auth9-keycloak-theme 自定义外观
- Dashboard/管理页面 → 由 Auth9 Portal（React Router 7）提供

---

## 场景 1：标准登录流程

### 初始状态
- 用户未登录
- 用户在 Keycloak 中有有效账户

### 目的
验证完整的 OIDC 登录流程

### 测试操作流程
1. 访问 Auth9 Portal（`http://localhost:3000/login`）
2. 点击「**Sign in with password**」按钮
3. 跳转到 Auth9 品牌化登录页（底层由 Keycloak 托管，使用 auth9-keycloak-theme）
4. 输入用户名和密码
5. Keycloak 验证成功
6. 重定向回 Auth9 Portal → `/dashboard`

### 预期结果
- 用户成功登录
- 界面显示用户信息
- 浏览器存储了 session

### 预期数据状态
```sql
SELECT id, user_id, ip_address, created_at FROM sessions
WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在新会话

SELECT event_type FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'success'
```

---

## 场景 2：首次登录（新用户同步）

### 初始状态
- 用户在 Keycloak 中存在
- 用户在 Auth9 数据库中不存在

### 目的
验证首次登录时用户自动同步

### 测试操作流程
1. 用户通过 Auth9 Portal 登录入口点击「**Sign in with password**」完成首次登录
2. Auth9 处理 callback

### 预期结果
- 用户自动创建在 Auth9 数据库中
- 用户信息从 Keycloak 同步

### 预期数据状态
```sql
SELECT id, keycloak_id, email, display_name FROM users WHERE keycloak_id = '{keycloak_user_id}';
-- 预期: 存在记录
```

---

## 场景 3：带 MFA 的登录（Sign in with password 路径）

### 初始状态
- 用户启用了 MFA (TOTP)

### 目的
验证 MFA 登录流程

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」
2. 在 Keycloak 品牌化登录页输入用户名和密码
3. 跳转到 MFA 验证页面
3. 输入正确的 TOTP 代码
4. 验证成功

### 预期结果
- MFA 验证成功后完成登录

### 预期数据状态
```sql
SELECT event_type FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'success'
```

---

## 场景 4：MFA 验证失败（Sign in with password 路径）

### 初始状态
- 用户启用了 MFA

### 目的
验证 MFA 验证失败处理

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」
2. 在 Keycloak 品牌化登录页正确输入密码
3. 在 MFA 页面输入错误代码

### 预期结果
- 显示 MFA 验证失败错误
- 登录失败

### 预期数据状态
```sql
SELECT event_type, failure_reason FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_mfa'
```

---

## 场景 5：登出流程

### 初始状态
- 用户已登录

### 目的
验证登出流程

### 测试操作流程
1. 点击「登出」
2. 确认登出

### 预期结果
- 用户被登出
- Session 被撤销
- 重定向到登录页

### 预期数据状态
```sql
SELECT revoked_at FROM sessions WHERE id = '{session_id}';
-- 预期: revoked_at 有值
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 标准登录流程 | ☐ | | | |
| 2 | 首次登录同步 | ☐ | | | |
| 3 | 带 MFA 登录 | ☐ | | | |
| 4 | MFA 验证失败 | ☐ | | | |
| 5 | 登出流程 | ☐ | | | |
