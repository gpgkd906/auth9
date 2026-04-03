# MFA 强制配置重定向 (TOTP Setup Redirect)

**类型**: 安全 / 认证
**严重程度**: Medium
**影响范围**: auth9-core (required_actions service), auth9-portal (login flow, routes)
**前置依赖**: TOTP 基础功能已实现，required_actions 机制已存在

---

## 背景

当用户 `mfa_enabled=true` 但尚未配置 TOTP credential 时，登录后应自动重定向到 `/mfa/setup-totp` 进行 TOTP 注册。当前行为是用户直接进入 `/tenant/select` → `/dashboard`，跳过了 TOTP 配置步骤，导致 MFA 策略形同虚设。

---

## 期望行为

### R1: 登录流程检查 CONFIGURE_TOTP required action

登录认证成功后、签发 Token 前，检查用户是否存在 pending 的 `CONFIGURE_TOTP` required action。若用户 `mfa_enabled=true` 且无有效 TOTP credential，自动创建该 action。

**涉及文件**:
- `auth9-core/src/domains/identity/` — required_actions service，登录后检查逻辑

### R2: 重定向到 `/mfa/setup-totp`

Portal 检测到 `CONFIGURE_TOTP` pending action 后，将用户重定向到 TOTP 设置页面，而非继续正常的 tenant 选择流程。

**涉及文件**:
- `auth9-portal/app/routes/` — 登录回调处理、TOTP setup 页面路由

### R3: TOTP 配置完成后继续正常流程

用户成功配置 TOTP 后，清除 `CONFIGURE_TOTP` action，继续签发 Token 并进入正常的 tenant 选择 → dashboard 流程。

**涉及文件**:
- `auth9-portal/app/routes/` — TOTP setup 完成后的重定向逻辑

---

## 验证方法

### 手动验证

1. 为测试用户启用 MFA (`mfa_enabled=true`)
2. 删除该用户的 TOTP credential 记录
3. 使用该用户登录
4. 确认被重定向到 `/mfa/setup-totp` 页面
5. 完成 TOTP 配置后确认进入正常 dashboard 流程

### 代码验证

```bash
grep -r "CONFIGURE_TOTP\|mfa_enabled\|setup-totp" auth9-core/src/ auth9-portal/app/
cd auth9-core && cargo test mfa
cd auth9-portal && npm run test
```
