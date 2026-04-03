# 认证流程 - OIDC 登录进阶 MFA 与登出页回归

**模块**: 认证流程
**测试范围**: OIDC 登录进阶页（TOTP 注册、认证器选择）与登出回归
**场景数**: 3
**优先级**: 中

---

## 背景说明

本文档承接 [01-oidc-login.md](./01-oidc-login.md) 的基础登录流程，补充以下进阶页面：

- `/mfa/setup-totp` 路由（`mfa.setup-totp.tsx`）承载的首次 TOTP 注册页
- `SelectAuthenticator.tsx` 承载的多认证方式选择页
- Sign out 后的登录态清理回归

这些场景仍属于「Sign in with password」主链路，但为满足”每文档不超过 5 个场景”的治理要求，单独拆分维护。

> **Browser Session Persistence (Playwright)**
> Playwright CLI headless browser may not persist cookies between page navigations in ephemeral contexts.
> The portal sets a `_session` cookie that must survive across redirects (login -> password -> MFA setup/verify -> /tenant/select -> /dashboard).
> If multi-step flows fail with unexpected redirects back to `/login`, ensure you are using a **persistent browser context**:
> - Use `--save-storage` / `--load-storage` to persist cookies across Playwright CLI invocations
> - Or maintain a **single `BrowserContext`** for the entire multi-step flow (do not create a new context per step)
> - Ephemeral (incognito-like) contexts will lose the `_session` cookie and break post-login navigation

---

## 场景 1：MFA 首次配置（TOTP 注册）

> **[DEFERRED - pending FR: mfa_enforcement_redirect.md]** MFA enforcement redirect flow is not yet implemented.

### 前置条件（防止误报）

> **⚠️ 种子数据中 `mfa-user@auth9.local` 的 TOTP 状态说明**:
> `mfa-user@auth9.local` 在种子数据中已有 TOTP 凭据已配置的状态。如果需要测试**首次 TOTP 注册**流程，需要：
> - (a) 在测试前手动删除该用户的 TOTP 凭据（通过数据库或 API），或
> - (b) 创建一个新的测试用户，设置 `mfa_enabled=1` 但不配置 TOTP 凭据
>
> **前置条件**: 种子数据中 `mfa-user@auth9.local` 已配置 TOTP 凭据。若需测试首次 TOTP 注册流程，需先通过 API 删除该用户的 TOTP 凭据：`DELETE /api/v1/users/{user_id}/credentials/totp`
>
> 如果直接使用 `mfa-user@auth9.local` 登录，系统会跳过 TOTP 注册页直接进入 TOTP 验证页（因为 TOTP 已配置），这不是 bug。

> **MFA 测试用户**: 使用 `./scripts/reset-docker.sh` 种子化的 MFA 测试用户 `mfa-user@auth9.local`，该用户已预配置为"MFA 已启用但 TOTP 未配置"状态，适合直接测试首次 TOTP 注册流程。
>
> **启用 MFA 的 API 需要 Tenant Access Token**: `POST /api/v1/users/{id}/mfa` 是租户管理端点，不接受 Identity Token。如果使用 Identity Token 调用会返回 `403: "Identity token is only allowed for tenant selection and exchange"`。必须使用 Tenant Access Token：
> ```bash
> # 生成 Tenant Access Token
> TENANT_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner | grep 'Bearer' | awk '{print $2}')
> # 使用 Tenant Access Token 启用 MFA
> curl -X POST http://localhost:8080/api/v1/users/{id}/mfa \
>   -H "Authorization: Bearer $TENANT_TOKEN" \
>   -H "Content-Type: application/json"
> ```
>
> | 症状 | 原因 | 解决方法 |
> |------|------|----------|
> | 无 "MFA enabled but TOTP not configured" 状态的用户 | 环境未种子化 MFA 测试用户 | 运行 `./scripts/reset-docker.sh`，使用 `mfa-user@auth9.local` |
> | 403 "Identity token is only allowed for tenant selection and exchange" | 使用 Identity Token 调用 MFA 启用 API | 改用 Tenant Access Token（见上方命令） |

### 初始状态
- 管理员已通过 Portal 为用户启用 MFA（`POST /api/v1/users/{id}/mfa`）——**需使用 Tenant Access Token**
- 用户尚未完成 TOTP 注册（required action: `CONFIGURE_TOTP`）
- **推荐使用种子化的 MFA 测试用户 `mfa-user@auth9.local`**（运行 `./scripts/reset-docker.sh` 后自动创建）
- **用户必须能够成功完成密码登录**（依赖 `auth9-core init` 已正确种子化 admin 密码凭据）

### 目的
验证用户首次配置 TOTP 的完整流程。此流程由认证引擎在认证中强制触发，配置页面由 Auth9 品牌认证页自定义渲染（`/mfa/setup-totp` 路由，对应 `mfa.setup-totp.tsx`），保持 Liquid Glass 品牌风格。

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」
2. 在 Auth9 品牌认证页输入用户名和密码
3. 自动跳转到 TOTP 配置页面（QR 码页面）
4. 验证页面保持 Auth9 品牌风格（Liquid Glass），**非**原生认证 UI 的默认样式
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
- 后续登录正常进入 MFA 验证页（见 [01-oidc-login.md](./01-oidc-login.md) 场景 3）

### 品牌一致性检查
- ☐ 页面背景为 Liquid Glass 渐变效果
- ☐ QR 码容器使用圆角白色背景
- ☐ 步骤编号为蓝色圆形气泡
- ☐ 输入框使用 Glass Input 组件
- ☐ 按钮为蓝色主题按钮
- ☐ 不出现原生认证 UI 的默认样式

> **故障排除**
>
> | 症状 | 原因 | 解决方案 |
> |------|------|---------|
> | `ApiResponseError: Invalid or expired token` on `/mfa/setup-totp` | 用户未完成密码登录，或 session token 已过期 | 确保先完成密码登录流程（依赖 admin 凭据正确种子化），然后再访问 TOTP 页面 |
> | 页面显示通用错误 | Portal 调用 `totpEnrollStart` API 时 token 无效 | 运行 `./scripts/reset-docker.sh` 重置环境后重新登录 |
> | 密码登录失败（前置步骤） | `auth9-core init` 未能种子化 admin 密码凭据 | 检查 auth9-init 容器日志，确认 "Admin password credential set" 消息 |

---

## 场景 2：认证器选择（多认证方式）

> **⚠️ 种子数据说明**: 默认种子数据中**没有**配置了多种认证方式（TOTP + WebAuthn）的用户。
> 测试此场景需要手动准备：
> 1. 为测试用户配置 TOTP 凭据（通过场景 1 的 TOTP 注册流程完成）
> 2. 再为同一用户配置 WebAuthn/Passkey 凭据（通过浏览器 WebAuthn API 注册）
> 3. 只有同一用户同时拥有多种认证方式时，认证器选择页面才会被触发
>
> 如果未完成上述准备，登录后会直接进入唯一已配置的认证方式页面（如仅 TOTP），不会出现认证器选择页。这不是 bug。

### 初始状态
- 用户配置了多种认证方式（如 TOTP + WebAuthn/Passkey）
- 认证流程包含多个 authenticator

### 目的
验证多认证方式选择页面。此页面由 Auth9 品牌认证页自定义渲染（`SelectAuthenticator.tsx`），在用户有多种认证选项时触发。

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

## 场景 3：登出流程

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
| 1 | MFA 首次配置（TOTP 注册） | ☐ | | | 需管理员先启用 MFA |
| 2 | 认证器选择（多认证方式） | ☐ | | | 需多种认证方式 |
| 3 | 登出流程 | ☐ | | | |
