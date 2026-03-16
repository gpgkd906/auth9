# 认证流程 - Email OTP 无密码登录

**模块**: 认证流程
**测试范围**: Email OTP 发送/验证端点、Portal 登录 UI 入口、租户级开关、防枚举
**场景数**: 5
**优先级**: 高

---

## 背景说明

Email OTP 是 Auth9 的无密码登录方式：用户输入邮箱 → 收到 6 位验证码 → 输入验证码完成登录。

**Portal 登录页认证方式**（`/login`）：

| 按钮 | 认证方式 | 流程 |
|------|---------|------|
| **Continue with Enterprise SSO** | 企业 SSO | 输入邮箱 → 域名发现 → IdP |
| **Sign in with password** | 密码登录 | → Auth9 品牌认证页 → 用户名+密码 |
| **Sign in with email code** | Email OTP | → `/auth/email-otp` → 输入邮箱 → 收到验证码 → 输入验证码 |
| **Sign in with passkey** | Passkey | WebAuthn API → 无密码认证 |

> **注意**: 「Sign in with email code」按钮仅在系统品牌设置中 `email_otp_enabled = true` 时显示。

**API 端点**（公开，不需要认证）：

- `POST /api/v1/auth/email-otp/send` — 发送验证码
- `POST /api/v1/auth/email-otp/verify` — 验证并签发 Identity Token

**速率限制**（由 OtpManager 基础设施层控制）：
- 冷却期：60 秒
- 日发送上限：10 次 / 24 小时
- 失败锁定：5 次失败后锁定 15 分钟

**开关控制**：通过 `BrandingConfig.email_otp_enabled` 字段控制，默认 `false`。

---

## 场景 1：登录页 Email OTP 入口可见性

### 初始状态
- 用户未登录
- 系统品牌设置 `email_otp_enabled = false`（默认）

### 目的
验证 Email OTP 登录入口仅在 `email_otp_enabled = true` 时显示

### 测试操作流程

#### 步骤 0: 验证环境状态
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

# 确认 Portal 运行中
curl -sf http://localhost:3000/login -o /dev/null -w "%{http_code}"
# 预期: 200
```

1. 访问 Portal 登录页 `http://localhost:3000/login`
2. 观察登录页按钮列表，确认 **不存在** 「Sign in with email code」按钮
3. 通过 API 启用 Email OTP：
   ```bash
   # 获取当前品牌设置
   TOKEN={admin_tenant_access_token}
   CURRENT=$(curl -s http://localhost:8080/api/v1/system/branding \
     -H "Authorization: Bearer $TOKEN" | jq '.data')

   # 更新设置，启用 email_otp_enabled
   curl -s -X PUT http://localhost:8080/api/v1/system/branding \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d "{\"config\": $(echo $CURRENT | jq '. + {email_otp_enabled: true}')}" | jq .
   ```
4. 刷新 Portal 登录页 `http://localhost:3000/login`
5. 观察登录页按钮列表

### 预期结果
- 步骤 2：登录页不显示「Sign in with email code」按钮
- 步骤 5：登录页显示「Sign in with email code」按钮（位于密码登录按钮和 Passkey 按钮之间）
- 点击该按钮跳转到 `/auth/email-otp` 页面

---

## 场景 2：Email OTP 发送验证码

### 初始状态
- `email_otp_enabled = true`（场景 1 已启用）
- 用户邮箱已在系统中注册
- 邮件服务已配置

### 目的
验证发送验证码端点正常工作，返回统一响应

### 测试操作流程

#### Portal UI 流程
1. 从 Portal 登录页 `http://localhost:3000/login` 点击「Sign in with email code」
2. 进入 `/auth/email-otp` 页面，看到邮箱输入框
3. 输入已注册的邮箱地址
4. 点击「Send verification code」

#### API 流程
```bash
# 发送验证码
curl -s -X POST http://localhost:8080/api/v1/auth/email-otp/send \
  -H "Content-Type: application/json" \
  -d '{"email": "{registered_email}"}' | jq .
```

### 预期结果
- Portal UI：页面切换到验证码输入阶段，显示邮箱地址和 6 位数字输入框
- API 响应（200）：
  ```json
  {
    "message": "If this email is registered, a verification code has been sent.",
    "expires_in_seconds": 600
  }
  ```
- 用户邮箱收到包含 6 位数字验证码的邮件（使用 EmailMfa 模板）
- Portal UI 显示「Resend code」按钮，初始 60 秒冷却倒计时

---

## 场景 3：Email OTP 验证并登录

### 初始状态
- `email_otp_enabled = true`
- 已发送验证码（场景 2 完成）
- 用户知道正确的 6 位验证码

### 目的
验证正确验证码成功认证并签发 Identity Token

### 测试操作流程

#### Portal UI 流程
1. 在验证码输入页面，输入收到的 6 位数字验证码
2. 点击「Verify」

#### API 流程
```bash
# 验证码验证
curl -s -X POST http://localhost:8080/api/v1/auth/email-otp/verify \
  -H "Content-Type: application/json" \
  -d '{"email": "{registered_email}", "code": "{otp_code}"}' | jq .
```

### 预期结果
- Portal UI：验证成功后自动跳转到 `/tenant/select`（多租户）或 `/dashboard`（单租户）
- API 响应（200）：
  ```json
  {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 3600
  }
  ```
- 返回的 `access_token` 是 Identity Token（可用于后续 tenant token exchange）

### 预期数据状态
```sql
SELECT id, user_id, ip_address, created_at FROM sessions
WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在新会话记录
```

---

## 场景 4：防枚举与错误处理

### 初始状态
- `email_otp_enabled = true`
- 系统中不存在 `nonexistent@example.com`

### 目的
验证未注册邮箱的防枚举响应、错误验证码处理、开关关闭时的 404

### 测试操作流程

#### 4a: 未注册邮箱发送验证码（防枚举）
```bash
curl -s -X POST http://localhost:8080/api/v1/auth/email-otp/send \
  -H "Content-Type: application/json" \
  -d '{"email": "nonexistent@example.com"}' | jq .
```

#### 4b: 错误验证码
```bash
curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://localhost:8080/api/v1/auth/email-otp/verify \
  -H "Content-Type: application/json" \
  -d '{"email": "{registered_email}", "code": "000000"}'
```

#### 4c: 无效邮箱格式
```bash
curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://localhost:8080/api/v1/auth/email-otp/send \
  -H "Content-Type: application/json" \
  -d '{"email": "not-an-email"}'
```

#### 4d: 关闭开关后端点返回 404
```bash
# 先关闭开关
TOKEN={admin_tenant_access_token}
CURRENT=$(curl -s http://localhost:8080/api/v1/system/branding \
  -H "Authorization: Bearer $TOKEN" | jq '.data')

curl -s -X PUT http://localhost:8080/api/v1/system/branding \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"config\": $(echo $CURRENT | jq '. + {email_otp_enabled: false}')}" > /dev/null

# 尝试发送验证码
curl -s -w "\nHTTP_STATUS:%{http_code}" -X POST http://localhost:8080/api/v1/auth/email-otp/send \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'
```

### 预期结果
- **4a**: 返回 200，响应与已注册邮箱完全相同（`"If this email is registered..."`），不泄露邮箱是否存在
- **4b**: 返回 400，`"Invalid or expired verification code."`
- **4c**: 返回 400，`"Invalid email address."`
- **4d**: 返回 404，`"Not found"`（send 和 verify 端点均返回 404）

---

## 场景 5：后端单元测试与编译验证

### 初始状态
- auth9-core 和 auth9-portal 代码可编译

### 目的
验证 Email OTP 相关代码编译通过、Lint 无警告、现有测试无回归

### 测试操作流程
1. 后端编译与 Lint：
   ```bash
   cd auth9-core && cargo clippy 2>&1
   ```
2. 后端 OTP 单元测试：
   ```bash
   cd auth9-core && cargo test otp -- --nocapture 2>&1
   ```
3. 后端品牌配置测试：
   ```bash
   cd auth9-core && cargo test branding -- --nocapture 2>&1
   ```
4. 前端类型检查：
   ```bash
   cd auth9-portal && npm run typecheck 2>&1
   ```
5. 前端单元测试：
   ```bash
   cd auth9-portal && npm run test 2>&1
   ```

### 预期结果
- `cargo clippy` 无错误无警告
- OTP 相关 14+ 个测试全部通过
- 品牌配置测试全部通过（含 `email_otp_enabled` 字段）
- `npm run typecheck` 无类型错误
- 前端全部 1260+ 个测试通过

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 登录页 Email OTP 入口可见性 | ✅ PASS | 2026-03-16 | QA Test | 开关关闭不显示，开启后按钮出现在密码和通行密钥之间，点击跳转 /auth/email-otp |
| 2 | Email OTP 发送验证码 | ✅ PASS | 2026-03-16 | QA Test | API 200 + Mailpit 收到 6 位验证码邮件；Portal UI 切换到验证码输入阶段 |
| 3 | Email OTP 验证并登录 | ✅ PASS | 2026-03-16 | QA Test | API 返回 Identity Token (token_type=identity)；Portal UI 跳转 /tenant/select；DB 会话已创建 |
| 4 | 防枚举与错误处理 | ✅ PASS | 2026-03-16 | QA Test | 4a 未注册邮箱同响应；4b 错误码 400；4c 无效邮箱 400；4d 关闭开关 404 |
| 5 | 后端单元测试与编译验证 | ✅ PASS | 2026-03-16 | QA Test | clippy clean；18 OTP 测试通过；58 branding 测试通过；typecheck clean；1260 前端测试通过 |
