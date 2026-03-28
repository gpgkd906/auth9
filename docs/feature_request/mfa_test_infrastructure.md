# MFA Test Infrastructure: Seed MFA-Enabled Users

**类型**: 测试基础设施
**严重程度**: High
**影响范围**: auth9-core (Seed Data), scripts/reset-docker.sh
**前置依赖**: 无（TOTP / Recovery Code 功能已实现）
**阻塞工单**: `auth_01-oidc-login_scenario3-4`, `auth_19-hosted-login-routes_scenario4`, `auth_24-mfa-totp-recovery_blocked`

---

## 背景

Auth9 已完整实现 TOTP 二次验证（`TotpService`）和恢复码（`RecoveryCodeService`），但当前的种子数据脚本（`seed_initial_data`）仅创建了一个未启用 MFA 的管理员用户（`admin@auth9.local`）。

这导致所有涉及 MFA 流程的 QA 测试工单被阻塞：

| 工单 | 场景 | 阻塞原因 |
|------|------|----------|
| `auth_01-oidc-login_scenario3-4` | OIDC 登录触发 MFA 挑战 | 无 MFA 用户，无法触发 MFA 步骤 |
| `auth_19-hosted-login-routes_scenario4` | Hosted Login MFA 验证页 | 无 TOTP 凭证，无法渲染 MFA 页面 |
| `auth_24-mfa-totp-recovery_blocked` | TOTP 恢复码验证 | 无预生成恢复码，无法测试恢复流程 |

目前手动测试需要先通过 UI 完成 TOTP 注册（扫码 → 验证 → 保存恢复码），才能执行后续 MFA 场景。自动化 QA 脚本无法完成此前置步骤，因为 TOTP secret 在注册时随机生成，脚本无法计算有效的验证码。

### 现有种子数据架构

种子数据在 `auth9-core/src/migration/mod.rs` 的 `seed_initial_data()` 中执行，由 `auth9-init` 容器在 Docker 启动时运行。当前仅创建：

- Platform Tenant + Demo Tenant
- Admin User（`admin@auth9.local`，密码登录，`mfa_enabled: false`）
- RBAC 角色与权限

缺失：MFA 用户、TOTP 凭证、恢复码。

---

## 期望行为

### R1: 种子数据创建 MFA 用户

在 `seed_initial_data()` 或新增的 `seed_mfa_test_user()` 函数中，创建一个预启用 MFA 的测试用户：

| 字段 | 值 |
|------|-----|
| email | `mfa-user@auth9.local` |
| display_name | `MFA Test User` |
| password | `SecurePass123!`（与 admin 用户一致） |
| mfa_enabled | `true` |

该用户应关联到 Platform Tenant 和 Demo Tenant，并分配基本角色（与 admin 用户类似流程）。

**涉及文件**:
- `auth9-core/src/migration/mod.rs` — 新增 `seed_mfa_test_user()` 函数，在 `seed_initial_data()` 末尾调用

### R2: 确定性 TOTP Secret

TOTP secret 必须是确定性的（硬编码或从固定种子派生），以便 QA 脚本在运行时计算有效的 TOTP 验证码。

建议方案：使用固定 Base32 secret，在种子数据中直接写入 credential 表：

```rust
/// 固定 TOTP secret，仅用于测试环境
/// QA 脚本可使用此 secret 计算有效的 TOTP 验证码
const MFA_TEST_TOTP_SECRET_BASE32: &str = "JBSWY3DPEHPK3PXP";  // 标准测试 secret
```

种子数据需要：
1. 使用 `EncryptionKey` 加密 secret 后存入 `credentials` 表（类型 `CredentialType::Totp`）
2. 数据格式需与 `TotpCredentialData` 兼容（`encrypted_secret`, `algorithm: SHA1`, `digits: 6`, `period: 30`）

QA 脚本计算验证码示例：

```javascript
// Node.js / Playwright
import { authenticator } from 'otplib';
const code = authenticator.generate('JBSWY3DPEHPK3PXP');
```

```bash
# CLI (oathtool)
oathtool --totp -b JBSWY3DPEHPK3PXP
```

**涉及文件**:
- `auth9-core/src/migration/mod.rs` — 在种子函数中插入 TOTP credential
- `.claude/skills/tools/` — QA 辅助脚本中记录 secret 常量

### R3: 预生成恢复码

为 MFA 测试用户预生成一组确定性恢复码，并将其 SHA-256 哈希存入 credential 表。

建议使用固定恢复码列表（8 组，与 `RECOVERY_CODE_COUNT` 一致）：

```rust
const MFA_TEST_RECOVERY_CODES: [&str; 8] = [
    "rc-test-0001",
    "rc-test-0002",
    "rc-test-0003",
    "rc-test-0004",
    "rc-test-0005",
    "rc-test-0006",
    "rc-test-0007",
    "rc-test-0008",
];
```

种子数据需要：
1. 对每个恢复码计算 SHA-256 哈希
2. 以 `RecoveryCodeData { code_hash, used: false }` 格式存入 `credentials` 表（类型 `CredentialType::RecoveryCode`）
3. 恢复码明文记录在 QA 文档中，供测试脚本使用

**涉及文件**:
- `auth9-core/src/migration/mod.rs` — 在种子函数中插入恢复码 credentials
- `docs/qa/` — QA 文档中记录明文恢复码

### R4: 集成 reset-docker.sh

确保 `scripts/reset-docker.sh` 执行环境重置后，MFA 测试用户及其凭证可用：

1. `seed_initial_data()` 已包含 MFA 用户种子逻辑（R1-R3），无需额外脚本
2. `reset-docker.sh` 输出中新增 MFA 用户信息提示：
   ```
   URLs:
     Portal:     http://localhost:3000  (admin@auth9.local / SecurePass123!)
     MFA User:   mfa-user@auth9.local / SecurePass123! (TOTP secret: JBSWY3DPEHPK3PXP)
     Demo:       http://localhost:3002  (SDK integration guide)
   ```
3. 种子逻辑需幂等（使用 `INSERT IGNORE` 或先检查是否存在），与现有种子数据保持一致

**涉及文件**:
- `scripts/reset-docker.sh` — 输出中新增 MFA 用户提示
- `auth9-core/src/migration/mod.rs` — 种子逻辑幂等处理

---

## 涉及文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `auth9-core/src/migration/mod.rs` | 修改 | 新增 `seed_mfa_test_user()` 函数 |
| `scripts/reset-docker.sh` | 修改 | 输出中新增 MFA 用户信息 |
| `.claude/skills/tools/` | 修改 | QA 辅助脚本记录 TOTP secret |
| `docs/qa/auth/01-oidc-login.md` | 修改 | 补充 MFA 测试用户前置条件 |

---

## 验证方法

### 代码验证

```bash
# 确认种子函数包含 MFA 用户创建逻辑
grep -r "mfa.test\|mfa_test\|mfa-user@auth9.local" auth9-core/src/migration/

# 确认 TOTP credential 种子
grep -r "JBSWY3DPEHPK3PXP\|MFA_TEST_TOTP" auth9-core/src/migration/

# 确认恢复码种子
grep -r "rc-test-000\|MFA_TEST_RECOVERY" auth9-core/src/migration/

# 运行后端编译确认无语法错误
cd auth9-core && cargo build
```

### 手动验证

1. 执行 `./scripts/reset-docker.sh` 完整重置环境
2. 确认输出中包含 MFA 用户信息（`mfa-user@auth9.local`）
3. 使用 `mfa-user@auth9.local / SecurePass123!` 登录 Portal，确认触发 MFA 挑战
4. 使用 `oathtool --totp -b JBSWY3DPEHPK3PXP` 生成验证码并完成 MFA 验证
5. 测试恢复码流程：使用 `rc-test-0001` 完成恢复码验证
6. 重新执行 `./scripts/reset-docker.sh`，确认 MFA 用户在重置后仍然可用（幂等性）

### QA 自动化验证

```bash
# 在 QA 脚本中计算 TOTP 验证码并提交
node -e "
  const { authenticator } = require('otplib');
  console.log(authenticator.generate('JBSWY3DPEHPK3PXP'));
"
```

确认以下被阻塞工单可以正常执行：
- `auth_01-oidc-login_scenario3-4` — OIDC 登录 MFA 挑战流程
- `auth_19-hosted-login-routes_scenario4` — Hosted Login MFA 页面渲染
- `auth_24-mfa-totp-recovery_blocked` — TOTP 恢复码验证流程
