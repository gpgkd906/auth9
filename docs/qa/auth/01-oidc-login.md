# 认证流程 - OIDC 登录测试

**模块**: 认证流程
**测试范围**: OIDC 标准登录流程
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构：
1. Keycloak 处理 OIDC/MFA 认证
2. Auth9 Core 处理业务逻辑
3. Token Exchange 将 Identity Token 转换为 Tenant Access Token

---

## 场景 1：标准登录流程

### 初始状态
- 用户未登录
- 用户在 Keycloak 中有有效账户

### 目的
验证完整的 OIDC 登录流程

### 测试操作流程
1. 访问 Auth9 Portal
2. 点击「登录」
3. 重定向到 Keycloak 登录页面
4. 输入用户名和密码
5. Keycloak 验证成功
6. 重定向回 Auth9

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
1. 用户通过 Keycloak 登录（首次）
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

## 场景 3：带 MFA 的登录

### 初始状态
- 用户启用了 MFA (TOTP)

### 目的
验证 MFA 登录流程

### 测试操作流程
1. 输入用户名和密码
2. 跳转到 MFA 验证页面
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

## 场景 4：MFA 验证失败

### 初始状态
- 用户启用了 MFA

### 目的
验证 MFA 验证失败处理

### 测试操作流程
1. 正确输入密码
2. 在 MFA 页面输入错误代码

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
