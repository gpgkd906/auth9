# 认证流程 - OIDC 登录进阶 MFA 与登出页回归

**模块**: 认证流程
**测试范围**: OIDC 登录进阶页（TOTP 注册、认证器选择）与登出回归
**场景数**: 3
**优先级**: 中

---

## 背景说明

本文档承接 [01-oidc-login.md](./01-oidc-login.md) 的基础登录流程，补充以下进阶页面：

- `LoginConfigTotp.tsx` 承载的首次 TOTP 注册页
- `SelectAuthenticator.tsx` 承载的多认证方式选择页
- Sign out 后的登录态清理回归

这些场景仍属于「Sign in with password」主链路，但为满足“每文档不超过 5 个场景”的治理要求，单独拆分维护。

---

## 场景 1：MFA 首次配置（TOTP 注册）

### 初始状态
- 管理员已通过 Portal 为用户启用 MFA（`POST /api/v1/users/{id}/mfa`）
- 用户尚未完成 TOTP 注册（required action: `CONFIGURE_TOTP`）

### 目的
验证用户首次配置 TOTP 的完整流程。此流程由认证引擎在认证中强制触发，配置页面由 Auth9 品牌认证页自定义渲染（`LoginConfigTotp.tsx`），保持 Liquid Glass 品牌风格。

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

---

## 场景 2：认证器选择（多认证方式）

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
