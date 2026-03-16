# Email OTP 登录认证

**类型**: 新功能
**严重程度**: Medium
**影响范围**: auth9-core (Backend), auth9-portal (Frontend)
**前置依赖**: `infra_otp_service.md`

---

## 背景

Auth9 当前支持密码、TOTP、Passkey、Enterprise SSO、社交登录等多种认证方式，但缺少基于邮件验证码的无密码登录（Email OTP）。

Email OTP 是一种轻量级无密码认证方案：用户输入邮箱后，系统发送 6 位数字验证码到邮箱，用户在页面输入验证码即可完成登录。相比 Magic Link，Email OTP 不需要用户切换到邮箱客户端点击链接，体验更流畅。

### 现有基础设施

Auth9 已具备实现 Email OTP 的大部分基础设施：

- **邮件发送服务**: `EmailService` 支持 SMTP、AWS SES、Oracle 三种 Provider
- **邮件模板**: `EmailTemplate::EmailMfa` 模板已定义，包含 `verification_code` 变量
- **速率限制**: 已有暴力破解保护机制（Keycloak `bruteForceProtected`）
- **Redis 缓存**: 可用于存储 OTP 及其过期时间

### 缺失部分

- 无独立的 Email OTP 登录流程（当前 `EmailMfa` 模板仅作为 MFA 第二因素设计，未用于独立登录）
- 无 OTP 生成、验证、速率限制的服务层逻辑
- Portal 无 Email OTP 登录入口 UI

---

## 期望行为

### R1: OTP 发送端点

新增 `POST /api/v1/auth/email-otp/send`（公开端点），使用 `OtpManager`（来自 `infra_otp_service.md`）+ `EmailOtpChannel`：

- 接受 `email` 参数
- 通过 `OtpManager` 生成验证码、检查速率限制、存储到 Redis（TTL 10 分钟）
- 通过 `EmailOtpChannel` 使用 `EmailTemplate::EmailMfa` 模板发送验证码邮件
- 速率限制使用 `OtpRateLimitConfig::email_defaults()`（60s 冷却、10 次/24h、5 次失败锁定 15min）
- 无论邮箱是否存在于系统中，均返回相同响应（防枚举）

**请求**:
```json
{
  "email": "user@example.com"
}
```

**响应**（统一）:
```json
{
  "message": "If this email is registered, a verification code has been sent.",
  "expires_in_seconds": 600
}
```

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/` — 新增 `email_otp.rs` handler
- `auth9-core/src/domains/identity/service/otp/email_channel.rs` — Email 通道实现
- `auth9-core/src/server/mod.rs` — 注册公开路由

### R2: OTP 验证端点

新增 `POST /api/v1/auth/email-otp/verify`（公开端点）：

- 接受 `email` + `code` 参数
- 通过 `OtpManager::verify_and_consume()` 验证（一次性使用、失败计数、锁定均由 OtpManager 处理）
- 验证成功后：
  - 查找用户，如用户不存在则返回认证失败
  - 签发 Identity Token（与密码登录后相同的 token）
  - 返回 token 响应，后续进入正常的 tenant 选择 → token exchange 流程
- 验证失败返回通用错误消息（不泄露用户是否存在）

**请求**:
```json
{
  "email": "user@example.com",
  "code": "123456"
}
```

**成功响应**:
```json
{
  "access_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/email_otp.rs` — verify handler

### R3: Portal Email OTP 登录 UI

在 Portal 登录页 (`/login`) 添加 "Sign in with email code" 入口：

1. 用户点击后展示邮箱输入框
2. 输入邮箱并提交 → 调用 send 端点
3. 跳转到验证码输入页面（6 位数字输入框，自动 focus，支持粘贴）
4. 输入验证码 → 调用 verify 端点
5. 验证成功 → 进入 tenant 选择 → dashboard
6. 显示"重新发送验证码"按钮（60 秒冷却倒计时）

**涉及文件**:
- `auth9-portal/app/routes/login.tsx` — 添加 Email OTP 入口按钮
- `auth9-portal/app/routes/` — 新增 `auth.email-otp.tsx` 页面（邮箱输入 + 验证码输入）
- `auth9-portal/app/services/api.ts` — 添加 `sendEmailOtp()` 和 `verifyEmailOtp()` 方法

### R4: 租户级开关

通过系统设置控制是否启用 Email OTP 登录：

- `auth_methods.email_otp.enabled`: boolean，默认 `false`
- Portal 登录页根据此设置决定是否显示 Email OTP 入口
- send/verify 端点在未启用时返回 `404`

**涉及文件**:
- `auth9-core/src/models/system_settings.rs` — 添加配置项
- `auth9-core/src/domains/identity/api/auth/email_otp.rs` — 检查开关

### R5: 单元测试覆盖

- handler 层：send/verify 端点的请求/响应测试
- EmailOtpChannel：mock EmailService 验证邮件发送
- OTP 核心逻辑（由 `infra_otp_service.md` 的 OtpManager 测试覆盖）
- 邮箱枚举防护：不存在的邮箱返回相同响应

---

## 验证方法

### 代码验证

```bash
# 搜索 Email OTP 相关实现
grep -r "email_otp\|EmailOtp" auth9-core/src/ auth9-portal/app/

# 运行后端测试
cd auth9-core && cargo test email_otp

# 运行前端测试
cd auth9-portal && npm run test
```

### 手动验证

1. 在系统设置中启用 Email OTP
2. 访问 Portal 登录页，确认出现 "Sign in with email code" 按钮
3. 输入已注册邮箱，确认收到 6 位验证码邮件
4. 输入正确验证码，确认成功登录并进入 tenant 选择页
5. 测试错误验证码、过期验证码、重发冷却期等边界情况
6. 输入未注册邮箱，确认返回相同响应（无信息泄露）

---

## 实现顺序

1. `infra_otp_service.md` — OTP 通用服务层（前置依赖）
2. **本 FR** — Email OTP 登录（首个 OTP 消费者，验证 OtpService 可用性）

---

## 参考

- OTP 通用服务层: `docs/feature_request/infra_otp_service.md`（前置依赖）
- 现有邮件模板: `auth9-core/src/email/templates/mod.rs` — `EmailTemplate::EmailMfa`
- 现有邮件服务: `auth9-core/src/domains/platform/service/email.rs`
- 密码重置流程（参考 token 生成/验证模式）: `auth9-core/src/domains/identity/service/password.rs`
- Auth0 Passwordless: https://auth0.com/docs/authenticate/passwordless
