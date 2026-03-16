# SMS OTP 登录认证

**类型**: 新功能
**严重程度**: Medium
**影响范围**: auth9-core (Backend), auth9-portal (Frontend)
**前置依赖**: `infra_otp_service.md`, `infra_sms_provider.md`

---

## 背景

Auth9 当前所有认证方式均依赖邮箱或专用设备（Authenticator App、Passkey），缺少基于手机短信的认证能力。SMS OTP 是许多场景（特别是移动端用户、无邮箱用户、B2C 场景）的关键认证方式。

### 与 Email OTP 的关系

SMS OTP 在架构上与 Email OTP 高度相似（生成验证码 → 发送 → 验证 → 签发 token），主要差异在于：

| 维度 | Email OTP | SMS OTP |
|------|-----------|---------|
| 发送通道 | EmailService (SMTP/SES) | SmsService (新增) |
| 用户标识 | email | phone_number |
| 成本 | 接近零 | 按条计费 |
| 安全性 | 较高（需访问邮箱） | 较低（SIM swap、SS7 攻击） |
| 速率限制 | 宽松 | 严格（成本 + 滥用防护） |

### 前置条件

- **用户模型需支持手机号**: 当前 `users` 表需新增 `phone_number` 字段
- **SMS Provider 基础设施**: 需新增短信发送服务（项目中尚无任何 SMS 相关代码）
- **建议先实现 Email OTP**: SMS OTP 可复用 Email OTP 的验证流程和 Redis 存储模式

---

## 期望行为

### R1: 用户模型扩展

> **注**: SMS Provider 基础设施已拆分为独立 FR，见 `infra_sms_provider.md`。
> OTP 生成/验证/速率限制的通用逻辑见 `infra_otp_service.md`。

在 `users` 表添加手机号字段：

```sql
ALTER TABLE users ADD COLUMN phone_number VARCHAR(20) NULL;
ALTER TABLE users ADD COLUMN phone_verified BOOLEAN NOT NULL DEFAULT FALSE;
CREATE UNIQUE INDEX idx_users_phone_number ON users(phone_number);
```

- `phone_number` 存储 E.164 格式（如 `+8613800138000`）
- `phone_verified` 标记手机号是否已验证
- 唯一索引防止重复绑定

**涉及文件**:
- `auth9-core/migrations/` — 新增迁移文件
- `auth9-core/src/models/user.rs` — User 模型添加字段
- `auth9-core/src/repository/user.rs` — Repository 查询支持按 phone_number 查找

### R2: OTP 发送端点

新增 `POST /api/v1/auth/sms-otp/send`（公开端点），使用 `OtpManager`（来自 `infra_otp_service.md`）+ `SmsOtpChannel`：

- 接受 `phone_number` 参数（E.164 格式）
- 通过 `OtpManager` 生成验证码、检查速率限制、存储到 Redis（TTL 5 分钟）
- 通过 `SmsOtpChannel` 发送短信，内容：`"Your Auth9 verification code is: {code}. Valid for 5 minutes."`
- 速率限制使用 `OtpRateLimitConfig::sms_defaults()`（120s 冷却、5 次/24h、3 次失败锁定 30min）
- 防枚举：无论号码是否注册，均返回相同响应

**请求**:
```json
{
  "phone_number": "+8613800138000"
}
```

**响应**（统一）:
```json
{
  "message": "If this phone number is registered, a verification code has been sent.",
  "expires_in_seconds": 300
}
```

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/` — 新增 `sms_otp.rs` handler
- `auth9-core/src/domains/identity/service/otp/sms_channel.rs` — SMS 通道实现

### R3: OTP 验证端点

新增 `POST /api/v1/auth/sms-otp/verify`（公开端点）：

- 接受 `phone_number` + `code` 参数
- 通过 `OtpManager::verify_and_consume()` 验证（一次性使用、失败计数、锁定）
- 验证成功后签发 Identity Token

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/sms_otp.rs` — verify handler

### R4: Portal SMS OTP 登录 UI

在 Portal 登录页添加 "Sign in with phone" 入口：

1. 用户点击后展示手机号输入框（带国际区号选择器）
2. 输入手机号并提交 → 调用 send 端点
3. 跳转到验证码输入页（6 位数字，120 秒重发冷却倒计时）
4. 验证成功 → tenant 选择 → dashboard

**涉及文件**:
- `auth9-portal/app/routes/login.tsx` — 添加 SMS OTP 入口按钮
- `auth9-portal/app/routes/` — 新增 `auth.sms-otp.tsx` 页面
- `auth9-portal/app/services/api.ts` — 添加 `sendSmsOtp()` 和 `verifySmsOtp()` 方法

### R5: 租户级开关

- `auth_methods.sms_otp.enabled`: boolean，默认 `false`
- Portal 登录页根据此设置决定是否显示 SMS OTP 入口
- send/verify 端点在未启用时返回 `404`
- SMS Provider 配置由 `infra_sms_provider.md` 管理

**涉及文件**:
- `auth9-core/src/models/system_settings.rs` — 添加开关项

### R6: 单元测试覆盖

- handler 层：send/verify 端点的请求/响应测试
- 手机号格式验证（复用 `infra_sms_provider.md` 的 E.164 校验）
- OTP 核心逻辑（由 `infra_otp_service.md` 的 OtpManager 测试覆盖）
- SmsProvider mock：使用 `MockSmsProvider` 测试，不调用真实 SMS API
- 防枚举：未注册号码返回相同响应

---

## 安全考量

SMS OTP 存在已知的安全风险，需在文档和 UI 中向管理员说明：

1. **SIM Swap 攻击**: 攻击者可通过社会工程向运营商申请 SIM 卡转移
2. **SS7 协议漏洞**: 短信可被拦截（国家级攻击能力）
3. **短信嗅探**: 部分地区 2G 网络短信未加密

**缓解措施**:
- SMS OTP **不应作为唯一认证因素**用于高安全场景
- 建议管理员将 SMS OTP 定位为便捷登录方式，而非安全方式
- 高权限操作（如修改密码、管理 API Key）仍需 TOTP 或 Passkey
- 在 Portal 设置页面添加安全风险提示

---

## 验证方法

### 代码验证

```bash
# 搜索 SMS OTP 相关实现
grep -r "sms_otp\|SmsOtp\|SmsProvider" auth9-core/src/ auth9-portal/app/

# 运行后端测试
cd auth9-core && cargo test sms

# 运行前端测试
cd auth9-portal && npm run test
```

### 手动验证

1. 配置 SMS Provider（如 Twilio Test Credentials）
2. 在系统设置中启用 SMS OTP
3. 确保用户已绑定手机号
4. 访问登录页，使用 "Sign in with phone" 流程
5. 验证收到短信验证码并成功登录
6. 测试速率限制、错误码、过期码等边界情况

---

## 实现顺序

本 FR 在依赖链中位于最后，推荐按以下顺序实施：

1. `infra_otp_service.md` — OTP 通用服务层
2. `auth_email_otp.md` — Email OTP（验证 OtpService 可用性）
3. `infra_sms_provider.md` — SMS Provider 基础设施
4. **本 FR** — SMS OTP 登录（复用以上三者）

---

## 参考

- OTP 通用服务层: `docs/feature_request/infra_otp_service.md`
- SMS Provider 基础设施: `docs/feature_request/infra_sms_provider.md`
- Email OTP FR: `docs/feature_request/auth_email_otp.md`（姊妹功能）
- NIST 800-63B: https://pages.nist.gov/800-63-3/sp800-63b.html（SMS OTP 安全限制）
