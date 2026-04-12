# MFA CONFIGURE_TOTP Pending Action on Login

**类型**: 功能增强
**严重程度**: High
**影响范围**: auth9-core (Backend - hosted_login flow)
**前置依赖**: 无
**被依赖**: 无

---

## 背景

当用户 `mfa_enabled=true` 但未配置 TOTP 凭证时，hosted-login 密码端点应返回 `pending_actions` 包含 `CONFIGURE_TOTP`，引导用户先配置 TOTP。

当前行为：login flow 中 adaptive MFA 引擎先于 pending actions 检查执行。当 MFA 被判定为 Required，flow 在 `hosted_login.rs:380-424` 提前返回 `MfaChallengeResponse { mfa_required: true, ... }`，永远不会到达 `check_post_login_actions()`（line 427+）。

### 期望行为

- **R1**: 当 `mfa_enabled=true` 且用户无 TOTP credential 时，login 返回 `pending_actions` 包含 `{ action_type: "CONFIGURE_TOTP", redirect_url: "/mfa/setup-totp" }`
- **R2**: 只有当用户已有 TOTP credential 时，才返回 `mfa_required: true` 要求验证
- **R3**: `check_post_login_actions` 中已有 CONFIGURE_TOTP 自动创建逻辑（`required_actions.rs:111-128`），只需在 MFA decision 前检测此场景

### 涉及文件

| 文件 | 修改内容 |
|------|----------|
| `auth9-core/src/domains/identity/api/hosted_login.rs` | 在 MFA decision 前检查 `mfa_enabled && !has_mfa_credential`，bypass MFA challenge |
| `auth9-core/src/domains/identity/service/required_actions.rs` | 已有 CONFIGURE_TOTP 逻辑，无需修改 |
| `auth9-core/src/domains/identity/service/adaptive_mfa.rs` | 可选：在 MFA evaluation 中加入 credential 存在性检查 |

### 验证方法

1. 创建 `mfa_enabled=true` 用户，不添加 TOTP credential
2. `POST /api/v1/hosted-login/password` 登录
3. 验证返回 `pending_actions` 包含 `CONFIGURE_TOTP`（而非 `mfa_required: true`）
4. 添加 TOTP credential 后重新登录
5. 验证返回 `mfa_required: true`

---
*Created from ticket: auth_36-mfa-enforcement-redirect_scenario1*
