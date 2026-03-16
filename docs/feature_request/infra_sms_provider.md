# SMS Provider 基础设施

**类型**: 基础设施
**严重程度**: Medium
**影响范围**: auth9-core (Backend), auth9-portal (Frontend - 配置 UI)
**前置依赖**: 无
**被依赖**: `auth_sms_otp.md`

---

## 背景

Auth9 已有成熟的邮件发送基础设施（`EmailProvider` trait + SMTP/SES/Oracle 三种实现），但完全没有短信发送能力。SMS OTP 登录、短信通知、短信告警等功能都依赖 SMS Provider 基础设施。

本 FR 参照 `EmailProvider` 的设计模式，建立平行的 SMS 发送抽象层。

### 现有 EmailProvider 架构（参照）

```
auth9-core/src/email/
├── mod.rs          # 模块导出
├── provider.rs     # EmailProvider trait + EmailProviderError
├── smtp.rs         # SmtpEmailProvider
├── ses.rs          # SesEmailProvider
└── templates/      # 邮件模板
```

```rust
// 现有 EmailProvider trait（参照设计）
#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, message: &EmailMessage) -> Result<EmailSendResult, EmailProviderError>;
    async fn test_connection(&self) -> Result<(), EmailProviderError>;
    fn provider_name(&self) -> &'static str;
}
```

---

## 期望行为

### R1: SmsProvider trait

参照 `EmailProvider` 设计对等的 SMS 抽象：

```rust
/// SMS 发送错误类型
#[derive(Error, Debug)]
pub enum SmsProviderError {
    #[error("SMS provider not configured")]
    NotConfigured,

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Invalid phone number: {0}")]
    InvalidPhoneNumber(String),
}

/// SMS 发送结果
pub struct SmsSendResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub provider: String,
}

/// SMS Provider trait
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// 发送短信
    async fn send(
        &self,
        to: &str,       // E.164 格式，如 "+8613800138000"
        message: &str,
    ) -> Result<SmsSendResult, SmsProviderError>;

    /// 测试连接
    async fn test_connection(&self) -> Result<(), SmsProviderError>;

    /// Provider 名称
    fn provider_name(&self) -> &'static str;
}
```

**涉及文件**:
- `auth9-core/src/sms/mod.rs` — 模块导出
- `auth9-core/src/sms/provider.rs` — trait + error 定义

### R2: Twilio 实现（首选 Provider）

实现基于 Twilio REST API 的 SMS 发送：

```rust
pub struct TwilioSmsProvider {
    account_sid: String,
    auth_token: String,    // 通过 HTTP Basic Auth
    from_number: String,   // Twilio 分配的发送号码
    client: reqwest::Client,
}
```

**API 调用**: `POST https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json`

| 参数 | 值 |
|------|-----|
| `To` | 目标手机号 (E.164) |
| `From` | Twilio 发送号码 |
| `Body` | 短信内容 |
| Auth | HTTP Basic (account_sid:auth_token) |

**涉及文件**:
- `auth9-core/src/sms/twilio.rs` — Twilio 实现
- `auth9-core/Cargo.toml` — 无需新增依赖（复用已有 `reqwest`）

### R3: AWS SNS 实现（可选，第二优先）

实现基于 AWS SNS 的 SMS 发送：

```rust
pub struct SnsSmsProvider {
    client: aws_sdk_sns::Client,
    sender_id: Option<String>,
    message_type: String,  // "Transactional"
}
```

与现有 `SesEmailProvider` 共享 AWS SDK 配置，降低运维负担。

**涉及文件**:
- `auth9-core/src/sms/sns.rs` — SNS 实现
- `auth9-core/Cargo.toml` — 新增 `aws-sdk-sns` 依赖

### R4: SmsService（业务层封装）

参照 `EmailService` 设计，提供业务层封装：

```rust
pub struct SmsService<R: SystemSettingsRepository> {
    settings_repo: Arc<R>,
    provider_factory: Arc<dyn SmsProviderFactory>,
}

#[async_trait]
pub trait SmsProviderFactory: Send + Sync {
    async fn create(&self, config: &SmsProviderConfig) -> Result<Box<dyn SmsProvider>>;
}

impl<R: SystemSettingsRepository> SmsService<R> {
    /// 发送短信（从系统设置读取 Provider 配置）
    pub async fn send(&self, to: &str, message: &str) -> Result<SmsSendResult>;

    /// 测试 SMS 配置（连接性检查）
    pub async fn test_configuration(&self) -> Result<()>;
}
```

**涉及文件**:
- `auth9-core/src/domains/platform/service/sms.rs` — SmsService
- `auth9-core/src/state.rs` — AppState 注入 SmsService

### R5: 配置模型

在系统设置中新增 SMS Provider 配置：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmsProviderConfig {
    None,
    Twilio(TwilioConfig),
    Sns(SnsConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,     // 加密存储
    pub from_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnsConfig {
    pub region: String,
    pub sender_id: Option<String>,
    pub message_type: String,   // "Transactional" | "Promotional"
}
```

**涉及文件**:
- `auth9-core/src/models/system_settings.rs` — 配置枚举
- `auth9-core/src/models/sms.rs` — SMS 相关模型（SmsSendResult 等）

### R6: 手机号格式验证

提供 E.164 格式验证工具函数：

```rust
/// 验证 E.164 格式手机号
/// 格式: +{country_code}{subscriber_number}, 总长度 8-15 位数字
pub fn validate_e164(phone_number: &str) -> Result<(), ValidationError>;

/// 标准化手机号为 E.164 格式
pub fn normalize_phone_number(phone_number: &str) -> Result<String, ValidationError>;
```

规则：
- 必须以 `+` 开头
- `+` 后仅包含数字
- 总位数（不含 `+`）在 7-15 位之间
- 不做运营商级号码有效性验证（成本过高）

**涉及文件**:
- `auth9-core/src/models/phone.rs` — 手机号验证

### R7: Portal SMS 配置 UI

在管理后台系统设置中新增 SMS Provider 配置页面：

- Provider 选择（None / Twilio / AWS SNS）
- 对应的配置字段输入（含密码字段隐藏）
- "Test Connection" 按钮（调用 `test_configuration`）
- 配置保存

**涉及文件**:
- `auth9-portal/app/routes/dashboard.settings/` — SMS 配置组件
- `auth9-portal/app/services/api.ts` — SMS 配置 API 方法

### R8: 单元测试覆盖

- `TwilioSmsProvider`: 使用 `wiremock` mock Twilio API（参照 Keycloak 测试模式）
- `SmsService`: mock `SystemSettingsRepository` + mock `SmsProviderFactory`
- E.164 验证: 合法号码通过、非法格式拒绝、边界长度测试
- `MockSmsProvider`: 通过 `mockall` 自动生成，供 OTP 服务层测试使用

---

## 模块结构

```
auth9-core/src/sms/
├── mod.rs          # 模块导出
├── provider.rs     # SmsProvider trait + SmsProviderError + SmsSendResult
├── twilio.rs       # TwilioSmsProvider
└── sns.rs          # SnsSmsProvider (可选)

auth9-core/src/domains/platform/service/
└── sms.rs          # SmsService (业务层封装)

auth9-core/src/models/
├── sms.rs          # SmsProviderConfig, TwilioConfig, SnsConfig
└── phone.rs        # E.164 验证
```

---

## 验证方法

### 代码验证

```bash
# 搜索 SMS 相关实现
grep -r "SmsProvider\|SmsService\|TwilioSms" auth9-core/src/

# 运行 SMS 相关测试
cd auth9-core && cargo test sms

# E.164 验证测试
cd auth9-core && cargo test phone
```

### 手动验证

1. 在系统设置中配置 Twilio（使用 Twilio Test Credentials，不产生费用）
2. 点击 "Test Connection" 确认连接正常
3. 通过 SMS OTP 功能或直接调用 API 发送短信验证是否收到

---

## 参考

- 现有 Email 架构（完整参照）: `auth9-core/src/email/` — provider.rs, smtp.rs, ses.rs
- 现有 Email 业务层: `auth9-core/src/domains/platform/service/email.rs`
- Twilio REST API: https://www.twilio.com/docs/messaging/api/message-resource
- AWS SNS SMS: https://docs.aws.amazon.com/sns/latest/dg/sms_publish-to-phone.html
- E.164 格式: https://www.itu.int/rec/T-REC-E.164
- SMS OTP FR: `docs/feature_request/auth_sms_otp.md`
