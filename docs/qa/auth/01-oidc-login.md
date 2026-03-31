# 认证流程 - OIDC 登录测试

**模块**: 认证流程
**测试范围**: OIDC 标准登录流程
**场景数**: 4

---

> **语言说明**: Portal 支持三种语言（`zh-CN` / `en-US` / `ja`），默认为 `zh-CN`。如需按英文按钮/标题执行本用例，请先在页面右上角切换到 `English`，或预置 `auth9_locale=en-US` cookie；如需日语，切换到 `日本語` 或预置 `auth9_locale=ja`。若未切换，测试时应以中文文案为准。

## 架构说明

Auth9 采用内置 OIDC 引擎架构（注：Keycloak 已退役，所有认证流程由 Auth9 内置 OIDC 引擎处理）：
1. **Auth9 OIDC 引擎** 作为底层认证引擎，终端用户通过 Auth9 登录入口触发 OIDC 流程
2. **Auth9 品牌认证页** 对登录/注册页面进行完全自定义，用户看到的是 Auth9 品牌风格的登录界面
3. **Auth9 Core** 处理所有业务逻辑（用户管理、多租户、RBAC 等）
4. **Token Exchange** 将 Auth9 签发的 Identity Token 转换为包含角色/权限的 Tenant Access Token

**Portal 登录页三种认证方式**（`/login`）：

| 按钮 | 认证方式 | 流程 |
|------|---------|------|
| **Continue with Enterprise SSO** | 企业 SSO | 输入邮箱 → 域名发现 → Auth9 broker + `kc_idp_hint` → 直跳企业 IdP |
| **Sign in with password** | 密码登录 | → Auth9 品牌认证页（由 Auth9 托管认证页承载）→ 输入用户名+密码 |
| **Sign in with email code** | Email OTP | → `/auth/email-otp` → 输入邮箱 → 输入 6 位验证码（需启用 `email_otp_enabled`，见 [17-email-otp-login.md](./17-email-otp-login.md)） |
| **Sign in with passkey** | Passkey | WebAuthn API → 无密码认证 |

**本文档测试的是「Sign in with password」路径**，即通过 Auth9 品牌认证页进行用户名+密码认证。

**登录流程中的页面归属**：
- Portal `/login` 页面 → 认证方式选择入口（Auth9 Portal 提供）
- 用户名密码/注册/MFA 页面 → 由 Auth9 托管认证页承载
- Tenant 选择页面 `/tenant/select` 与 Dashboard/管理页面 → 由 Auth9 Portal（React Router 7）提供

---

## 前置条件（所有场景通用）

> **重要**：在执行本文档的任何测试场景前，必须确保环境已正确初始化。

```bash
# 1. 重置 Docker 环境（清空数据库、重建容器、执行 seeder）
./scripts/reset-docker.sh

# 2. 验证服务健康
curl -sf http://localhost:8080/health && echo "Core OK"
curl -sf http://localhost:3000/ > /dev/null && echo "Portal OK"
```

若登录时出现「邮箱或密码无效」，**首先确认是否已执行 `reset-docker.sh`**。该脚本初始化测试用户密码凭证；未执行时数据库中可能没有密码记录。

> **Browser Session Persistence (Playwright)**
> Playwright CLI headless browser may not persist cookies between page navigations in ephemeral contexts.
> The portal sets a `_session` cookie that must survive across redirects (login -> callback -> /tenant/select -> /dashboard).
> If multi-step flows fail with unexpected redirects back to `/login`, ensure you are using a **persistent browser context**:
> - Use `--save-storage` / `--load-storage` to persist cookies across Playwright CLI invocations
> - Or maintain a **single `BrowserContext`** for the entire multi-step flow (do not create a new context per step)
> - Ephemeral (incognito-like) contexts will lose the `_session` cookie and break post-login navigation

---

## 场景 1：标准登录流程

### 初始状态
- 用户未登录
- 用户在底层认证主体中有有效账户

### 目的
验证完整的 OIDC 登录流程

### 测试操作流程
1. 访问 Auth9 Portal（`http://localhost:3000/login`）
2. 点击「**Sign in with password**」按钮
3. 跳转到 Auth9 托管认证页
4. 输入用户名和密码
5. 底层认证验证成功
6. 重定向回 Auth9 Portal → `/tenant/select`
7. 选择 tenant 并完成 token exchange（单 tenant 账号可自动跳过）
8. 进入 `/dashboard`

### 预期结果
- 用户成功登录
- 多 tenant 账号先到 `/tenant/select` 明确选择后再进入 Dashboard；单 tenant 账号可自动进入 Dashboard
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

> **注意**: Auth9 内置 OIDC 引擎不提供公开的用户创建 API。此场景需通过 Portal 注册流程（`/register`）创建新用户来测试首次登录同步。不可使用外部 IdP（Keycloak 已退役）。

### 初始状态
- 用户在底层认证主体中存在
- 用户在 Auth9 数据库中不存在

### 目的
验证首次登录时用户自动同步

### 测试操作流程
1. 通过 Portal 注册页面（`/register`）创建新测试用户
2. 新用户通过 Auth9 Portal 登录入口点击「**Sign in with password**」完成首次登录
3. Auth9 处理 callback

### 预期结果
- 用户自动创建在 Auth9 数据库中
- 用户信息从认证引擎同步

### 预期数据状态
```sql
SELECT id, identity_subject, email, display_name FROM users WHERE email = '{user_email}';
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
2. 在 Auth9 品牌认证页输入用户名和密码
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

### 前置条件
- **事件兼容入口已配置**：MFA 失败事件由内置 OIDC 引擎产生，通过事件兼容入口写入 `login_events` 表。（注：Keycloak 已退役，原 ext-event-http SPI 事件桥接由 Auth9 内置事件系统替代）
- **注意**：auth9-core 的 OIDC 回调仅记录**成功登录**事件（`record_successful_login`）。失败事件（密码错误、MFA 失败）通过事件兼容入口记录。
- 如果事件兼容入口未配置，本场景的 UI 行为测试仍然有效，但 `login_events` 数据库断言将不适用。

### 目的
验证 MFA 验证失败处理

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」
2. 在 Auth9 品牌认证页正确输入密码
3. 在 MFA 页面输入错误代码

### 预期结果
- 显示 MFA 验证失败错误
- 登录失败

### 预期数据状态（需事件桥接）
```sql
SELECT event_type, failure_reason FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_mfa'
-- ⚠️ 仅在事件兼容入口已配置时有效
```

### 故障排查

| 症状 | 原因 | 解决方案 |
|------|------|---------|
| UI 显示 MFA 错误但 `login_events` 无新记录 | 事件兼容入口未配置 | 检查 auth9-core 日志确认事件系统正常运行 |
| auth9-core 日志无 "Recorded login event" | 事件记录异常 | 检查 auth9-core 日志排查事件记录链路 |

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
