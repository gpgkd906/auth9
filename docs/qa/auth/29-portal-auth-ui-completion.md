# Portal 认证 UI 补全

**模块**: Auth (Portal UI)
**测试范围**: Portal 密码登录表单、MFA 验证页面、TOTP 注册页面
**场景数**: 5
**优先级**: 高
**前置条件**: Docker 环境已启动，Portal (localhost:3000) 和 Auth9 Core (localhost:8080) 可访问

---

## 背景说明

Phase 5 Keycloak 退役后，Portal 需要补全认证 UI：密码登录表单、MFA 验证页面、TOTP 注册页面。后端 API 已完整实现，本文档验证 Portal 前端 UI 与后端的集成。

涉及路由：
- `/login` — 密码登录表单（R1）
- `/mfa/verify` — MFA 验证页面（R2）
- `/mfa/setup-totp` — TOTP 注册页面（R3）

涉及 API：
- `POST /api/v1/hosted-login/password` — 密码登录（可能返回 MFA 挑战）
- `POST /api/v1/mfa/challenge/totp` — TOTP 验证
- `POST /api/v1/mfa/challenge/recovery-code` — 恢复码验证
- `POST /api/v1/mfa/totp/enroll` — TOTP 注册开始
- `POST /api/v1/mfa/totp/enroll/verify` — TOTP 注册验证

---

## 场景 1：密码登录表单 — 正常登录

### 步骤 0（Gate Check）

- 确认测试用户 `test@example.com` 存在且未启用 MFA
- 确认 Portal 可访问: 浏览器访问 `http://localhost:3000/login`

### 步骤 1: 展开密码登录表单

1. 访问 `http://localhost:3000/login`
2. 点击 "Sign in with password" 按钮
3. **预期**: 展开一个包含邮箱和密码输入框的表单

### 步骤 2: 提交登录凭据

1. 在邮箱输入框填入 `test@example.com`
2. 在密码输入框填入有效密码
3. 点击 "Sign in" 按钮
4. **预期**: 成功跳转到 `/tenant/select` 页面

### 步骤 3: 验证邮箱预填充

1. 在上方 SSO 邮箱输入框中输入 `test@example.com`
2. 点击 "Sign in with password" 展开密码表单
3. **预期**: 密码表单的邮箱输入框已预填充 `test@example.com`

---

## 场景 2：密码登录表单 — 错误处理

### 步骤 0（Gate Check）

- 确认 Portal 可访问

### 步骤 1: 空表单提交

1. 展开密码登录表单
2. 不填写任何内容，点击 "Sign in"
3. **预期**: 浏览器原生验证阻止提交（`required` 属性）

### 步骤 2: 错误凭据

1. 填入 `test@example.com` 和错误密码
2. 点击 "Sign in"
3. **预期**: 页面显示错误消息（如 "Invalid email or password"），不跳转

---

## 场景 3：MFA 验证 — TOTP 验证流程

### 步骤 0（Gate Check）

- 确认测试用户已启用 TOTP（通过 API 或管理后台启用）
- 确认用户有有效 TOTP secret

### 步骤 1: 触发 MFA 挑战

1. 在 `/login` 页面使用启用了 TOTP 的用户密码登录
2. **预期**: 自动跳转到 `/mfa/verify?mfa_session_token=...&mfa_methods=totp`

### 步骤 2: 验证 MFA 页面 UI

1. **预期**: 页面标题为 "Two-factor authentication"（或对应语言翻译）
2. **预期**: 显示 6 位 OTP 输入框（独立数字框）
3. **预期**: 底部有 "Use a recovery code instead" 链接
4. **预期**: 底部有 "Back to sign in" 链接

### 步骤 3: 输入 TOTP 验证码

1. 输入正确的 6 位 TOTP 验证码
2. **预期**: 自动提交（输入第 6 位后自动提交表单）
3. **预期**: 成功跳转到 `/tenant/select`

### 步骤 4: 错误验证码

1. 重新触发 MFA 挑战
2. 输入错误的 6 位验证码
3. **预期**: 显示错误消息，输入框变为错误状态（红色边框）

---

## 场景 4：MFA 验证 — 恢复码流程

### 步骤 0（Gate Check）

- 确认测试用户已启用 TOTP 且有生成的 recovery codes
- 通过 API `POST /api/v1/mfa/recovery-codes/generate` 生成恢复码并记录

### 步骤 1: 切换到恢复码模式

1. 在 MFA 验证页面点击 "Use a recovery code instead"
2. **预期**: 页面切换为文本输入框，提示输入恢复码
3. **预期**: 显示 "Verify" 按钮
4. **预期**: 链接变为 "Use authenticator app"

### 步骤 2: 输入恢复码

1. 输入有效的恢复码
2. 点击 "Verify"
3. **预期**: 成功跳转到 `/tenant/select`

---

## 场景 5：TOTP 注册页面

### 步骤 0（Gate Check）

- 确认测试用户已认证（有有效 session）
- 确认用户尚未启用 TOTP: `GET /api/v1/mfa/status` 返回 `totp_enabled: false`
- 用户有 `CONFIGURE_TOTP` pending action（或直接访问 `/mfa/setup-totp`）

### 步骤 1: 访问 TOTP 注册页面

1. 带认证 session 访问 `/mfa/setup-totp`
2. **预期**: 页面显示 QR 码图片
3. **预期**: 页面显示 "Can't scan the code?" 链接
4. **预期**: 页面显示 6 位验证码输入框

### 步骤 2: 查看手动输入密钥

1. 点击 "Can't scan the code?"
2. **预期**: 展开显示 base32 编码的 secret 密钥
3. **预期**: 密钥可选择复制

### 步骤 3: 完成 TOTP 注册

1. 使用认证器应用扫描 QR 码
2. 输入认证器应用显示的 6 位验证码
3. **预期**: 自动提交并成功
4. **预期**: 跳转到 `/tenant/select`
5. **预期**: 用户 MFA 状态变为 enabled

### 步骤 4: 验证未认证访问

1. 清除 session cookies
2. 直接访问 `/mfa/setup-totp`
3. **预期**: 跳转到 `/login`（未认证用户无法访问）
