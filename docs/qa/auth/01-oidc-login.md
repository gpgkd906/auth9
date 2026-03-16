# 认证流程 - OIDC 登录测试

**模块**: 认证流程
**测试范围**: OIDC 标准登录流程
**场景数**: 7

---

> **语言说明**: Portal 支持三种语言（`zh-CN` / `en-US` / `ja`），默认为 `zh-CN`。如需按英文按钮/标题执行本用例，请先在页面右上角切换到 `English`，或预置 `auth9_locale=en-US` cookie；如需日语，切换到 `日本語` 或预置 `auth9_locale=ja`。若未切换，测试时应以中文文案为准。

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
| **Sign in with password** | 密码登录 | → Auth9 品牌认证页（由 auth9-keycloak-theme 承载）→ 输入用户名+密码 |
| **Sign in with email code** | Email OTP | → `/auth/email-otp` → 输入邮箱 → 输入 6 位验证码（需启用 `email_otp_enabled`，见 [17-email-otp-login.md](./17-email-otp-login.md)） |
| **Sign in with passkey** | Passkey | WebAuthn API → 无密码认证（不经过 Keycloak） |

**本文档测试的是「Sign in with password」路径**，即通过 Auth9 品牌认证页进行用户名+密码认证。

**登录流程中的页面归属**：
- Portal `/login` 页面 → 认证方式选择入口（Auth9 Portal 提供）
- 用户名密码/注册/MFA 页面 → 由托管认证页承载，使用 auth9-keycloak-theme 自定义外观
- Tenant 选择页面 `/tenant/select` 与 Dashboard/管理页面 → 由 Auth9 Portal（React Router 7）提供

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
3. 跳转到 Auth9 品牌认证页（由 auth9-keycloak-theme 承载）
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

### 初始状态
- 用户在底层认证主体中存在
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
- **Keycloak 事件桥接已部署**：MFA 失败事件由 Keycloak 产生，需通过 ext-event-http SPI 插件（p2-inc/keycloak-events）以 Webhook 方式推送到 auth9-core 的 `POST /api/v1/keycloak/events` 端点才能写入 `login_events` 表。
  - SPI 插件通过 `auth9-keycloak-events-builder` 构建并部署到 Keycloak providers 目录
  - seeder 在 `KEYCLOAK_WEBHOOK_SECRET` 配置时自动注册 `ext-event-http` 事件监听器
- **注意**：auth9-core 的 OIDC 回调仅记录**成功登录**事件（`record_successful_login`）。失败事件（密码错误、MFA 失败）发生在 Keycloak 侧，回调不会被触发，因此必须依赖事件桥接。
- 如果事件桥接未部署，本场景的 UI 行为测试仍然有效，但 `login_events` 数据库断言将不适用。

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
-- ⚠️ 仅在 Keycloak 事件桥接已部署时有效
```

### 故障排查

| 症状 | 原因 | 解决方案 |
|------|------|---------|
| UI 显示 MFA 错误但 `login_events` 无新记录 | Keycloak ext-event-http SPI 未部署 | 确认 `keycloak-events-*.jar` 已部署到 providers 目录，检查 Keycloak 启动日志是否加载 SPI |
| auth9-core 日志无 "Recorded login event" | Webhook 未到达 auth9-core | 检查 `KEYCLOAK_WEBHOOK_SECRET` 是否配置，确认 Keycloak realm Events → Event Listeners 包含 `ext-event-http` |

---

## 场景 5：MFA 首次配置（TOTP 注册）

### 初始状态
- 管理员已通过 Portal 为用户启用 MFA（`POST /api/v1/users/{id}/mfa`）
- 用户尚未完成 TOTP 注册（Keycloak required action: `CONFIGURE_TOTP`）

### 目的
验证用户首次配置 TOTP 的完整流程。此流程由 Keycloak 在认证中强制触发，配置页面由 auth9-keycloak-theme 自定义渲染（`LoginConfigTotp.tsx`），保持 Liquid Glass 品牌风格。

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」
2. 在 Auth9 品牌认证页输入用户名和密码
3. 自动跳转到 TOTP 配置页面（QR 码页面）
4. 验证页面保持 Auth9 品牌风格（Liquid Glass），**非** Keycloak 默认 PatternFly UI
5. 页面显示三步引导：
   - Step 1: 安装 authenticator 应用（如 FreeOTP, Google Authenticator）
   - Step 2: 扫描 QR 码（或点击「Unable to scan?」切换手动输入密钥模式）
   - Step 3: 输入验证码
6. 使用 authenticator 应用扫描 QR 码
7. 输入 6 位 TOTP 验证码
8. 输入设备名称（可选）
9. 点击提交
10. 验证成功，进入后续登录流程

### 预期结果
- TOTP 配置页面使用 Auth9 品牌风格（Liquid Glass 毛玻璃卡片、渐变背景）
- QR 码正常显示，可被 authenticator 应用识别
- 手动输入密钥模式可正常切换
- 验证码输入后成功完成 TOTP 注册
- 后续登录正常进入 MFA 验证页（场景 3 的 `LoginOtp` 页面）

### 品牌一致性检查
- ☐ 页面背景为 Liquid Glass 渐变效果
- ☐ QR 码容器使用圆角白色背景
- ☐ 步骤编号为蓝色圆形气泡
- ☐ 输入框使用 Glass Input 组件
- ☐ 按钮为蓝色主题按钮
- ☐ 不出现 Keycloak 默认 PatternFly 样式

---

## 场景 6：认证器选择（多认证方式）

### 初始状态
- 用户配置了多种认证方式（如 TOTP + WebAuthn/Passkey）
- Keycloak authentication flow 包含多个 authenticator

### 目的
验证多认证方式选择页面。此页面由 auth9-keycloak-theme 自定义渲染（`SelectAuthenticator.tsx`），在用户有多种认证选项时触发。

### 测试操作流程
1. 使用配置了多种认证方式的账号登录
2. 输入密码后，跳转到认证器选择页面
3. 验证页面保持 Auth9 品牌风格
4. 页面显示可用的认证方式列表（带图标、名称、描述）
5. 点击选择一种认证方式
6. 跳转到对应的认证页面

### 预期结果
- 选择页面使用 Auth9 品牌风格
- 每个认证方式显示为卡片式选项，带图标和描述
- 悬停有视觉反馈（蓝色边框、轻微上移）
- 点击后正确跳转到对应认证流程

---

## 场景 7：登出流程

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
| 5 | MFA 首次配置（TOTP 注册） | ☐ | | | 需管理员先启用 MFA |
| 6 | 认证器选择（多认证方式） | ☐ | | | 需多种认证方式 |
| 7 | 登出流程 | ☐ | | | |
