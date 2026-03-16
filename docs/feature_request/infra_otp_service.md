# OTP 通用服务层 (OtpService)

**类型**: 基础设施 / 重构
**严重程度**: Medium
**影响范围**: auth9-core (Backend)
**前置依赖**: 无
**被依赖**: `auth_email_otp.md`, `auth_sms_otp.md`

---

## 背景

Email OTP 和 SMS OTP 两个功能在核心逻辑上高度重合：

1. 生成 6 位密码学安全随机验证码
2. 将验证码存入 Redis（带 TTL）
3. 通过某种通道（邮件/短信）发送给用户
4. 用户提交验证码 → 从 Redis 取出比对 → 一次性消费
5. 速率限制（发送冷却期、24 小时发送上限、失败锁定）
6. 防枚举（无论用户是否存在，返回相同响应）

如果 Email OTP 和 SMS OTP 各自独立实现这些逻辑，将产生大量重复代码且难以维护。本 FR 要求在实现任何 OTP 认证方式之前，先建立通用的 OTP 服务层。

---

## 期望行为

### R1: OtpChannel trait

定义 OTP 发送通道的抽象接口：

```rust
/// OTP 发送通道标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OtpChannelType {
    Email,
    Sms,
}

/// OTP 发送通道 trait
#[async_trait]
pub trait OtpChannel: Send + Sync {
    /// 通道类型
    fn channel_type(&self) -> OtpChannelType;

    /// 发送验证码到目标地址
    /// - `destination`: 邮箱地址或手机号
    /// - `code`: 6 位验证码
    /// - `ttl_minutes`: 验证码有效期（分钟）
    async fn send_code(
        &self,
        destination: &str,
        code: &str,
        ttl_minutes: u32,
    ) -> Result<()>;
}
```

Email 通道实现封装现有 `EmailService` + `EmailTemplate::EmailMfa`；SMS 通道实现封装 `SmsService`（见 `infra_sms_provider.md`）。

**涉及文件**:
- `auth9-core/src/domains/identity/service/otp/channel.rs` — trait 定义
- `auth9-core/src/domains/identity/service/otp/email_channel.rs` — Email 实现
- `auth9-core/src/domains/identity/service/otp/sms_channel.rs` — SMS 实现（SMS Provider 就绪后）

### R2: OTP 生成与存储

提供验证码生成和 Redis 存储的通用实现：

```rust
pub struct OtpManager<C: CacheOperations> {
    cache: Arc<C>,
}

impl<C: CacheOperations> OtpManager<C> {
    /// 生成 6 位密码学安全随机数字验证码
    pub fn generate_code() -> String;

    /// 存储 OTP 到 Redis
    /// key: `otp:{channel}:{destination}`, TTL 由调用方指定
    pub async fn store(
        &self,
        channel: OtpChannelType,
        destination: &str,
        code: &str,
        ttl_secs: u64,
    ) -> Result<()>;

    /// 验证并消费 OTP（一次性使用）
    /// 成功返回 Ok(())，失败返回 Err 并记录失败次数
    pub async fn verify_and_consume(
        &self,
        channel: OtpChannelType,
        destination: &str,
        code: &str,
    ) -> Result<()>;
}
```

**Redis key 设计**:

| Key | 用途 | TTL |
|-----|------|-----|
| `otp:{channel}:{destination}` | 存储验证码 | 由通道决定（Email 10min, SMS 5min） |
| `otp_cooldown:{channel}:{destination}` | 发送冷却期 | 由通道决定（Email 60s, SMS 120s） |
| `otp_daily:{channel}:{destination}` | 24 小时发送计数 | 24h |
| `otp_fail:{channel}:{destination}` | 连续失败计数 | 由通道决定（Email 30min, SMS 30min） |

**涉及文件**:
- `auth9-core/src/domains/identity/service/otp/manager.rs` — OtpManager 实现
- `auth9-core/src/cache/mod.rs` — CacheOperations trait 新增 OTP 相关方法

### R3: CacheOperations 扩展

在现有 `CacheOperations` trait 中新增 OTP 相关的缓存方法：

```rust
// 新增到 CacheOperations trait
async fn store_otp(&self, key: &str, code: &str, ttl_secs: u64) -> Result<()>;
async fn get_otp(&self, key: &str) -> Result<Option<String>>;
async fn remove_otp(&self, key: &str) -> Result<()>;
async fn increment_counter(&self, key: &str, ttl_secs: u64) -> Result<u64>;
async fn get_counter(&self, key: &str) -> Result<u64>;
async fn set_flag(&self, key: &str, ttl_secs: u64) -> Result<bool>; // 返回是否已存在
```

同步更新 `CacheManager`（Redis 实现）和 `NoOpCacheManager`（测试用）。

**涉及文件**:
- `auth9-core/src/cache/operations.rs` — trait 定义
- `auth9-core/src/cache/manager.rs` + `manager_ops.rs` — Redis 实现
- `auth9-core/src/cache/noop.rs` + `noop_ops.rs` — NoOp 实现

### R4: 速率限制器

通用的 OTP 速率限制逻辑：

```rust
pub struct OtpRateLimitConfig {
    /// 发送冷却期（秒）
    pub cooldown_secs: u64,
    /// 24 小时内最大发送次数
    pub daily_max: u64,
    /// 最大连续失败次数
    pub max_failures: u64,
    /// 失败锁定时间（秒）
    pub lockout_secs: u64,
}

impl OtpRateLimitConfig {
    pub fn email_defaults() -> Self {
        Self {
            cooldown_secs: 60,
            daily_max: 10,
            max_failures: 5,
            lockout_secs: 900, // 15 min
        }
    }

    pub fn sms_defaults() -> Self {
        Self {
            cooldown_secs: 120,
            daily_max: 5,
            max_failures: 3,
            lockout_secs: 1800, // 30 min
        }
    }
}
```

速率限制检查在 `OtpManager` 的 `store` 和 `verify_and_consume` 中统一执行，调用方无需关心。

**涉及文件**:
- `auth9-core/src/domains/identity/service/otp/rate_limit.rs` — 配置 + 限制逻辑

### R5: 单元测试覆盖

- `generate_code()`: 验证 6 位数字、不同调用产生不同结果
- `store` + `verify_and_consume`: 正确码通过、错误码拒绝、过期码拒绝、已消费码拒绝
- 速率限制: 冷却期内重发被拒、超日上限被拒、连续失败锁定
- 使用 `NoOpCacheManager` 或 mock CacheOperations 测试

---

## 模块结构

```
auth9-core/src/domains/identity/service/otp/
├── mod.rs              # 模块导出
├── channel.rs          # OtpChannel trait + OtpChannelType
├── email_channel.rs    # EmailOtpChannel (封装 EmailService)
├── sms_channel.rs      # SmsOtpChannel (封装 SmsService, 后续实现)
├── manager.rs          # OtpManager (生成、存储、验证)
└── rate_limit.rs       # OtpRateLimitConfig + 限制逻辑
```

---

## 验证方法

```bash
# 搜索 OTP 服务实现
grep -r "OtpManager\|OtpChannel\|OtpRateLimitConfig" auth9-core/src/

# 运行 OTP 相关测试
cd auth9-core && cargo test otp
```

---

## 参考

- 现有 Email 基础设施: `auth9-core/src/email/` — EmailProvider trait 设计参考
- 现有缓存层: `auth9-core/src/cache/` — CacheOperations trait 扩展点
- Email OTP FR: `docs/feature_request/auth_email_otp.md`
- SMS OTP FR: `docs/feature_request/auth_sms_otp.md`
