# Hosted Login 路由与 Branding 托管

**模块**: auth
**场景数**: 5

---

## 场景 1：`/login` 由 Portal 托管认证入口

### 步骤 0：Gate Check
1. 访问 `http://localhost:3000/login`
2. 确认地址栏保持在 Auth9 Portal 域名

### 测试步骤
1. 检查页面是否展示 Enterprise SSO、Password、Passkey、Email code（若启用）入口
2. 点击 Password 入口

### 预期结果
- 页面由 Portal 渲染
- 地址栏保持 `http://localhost:3000/login`
- Password 入口展开 Portal 内登录表单

---

## 场景 2：`/register`、`/forgot-password`、`/reset-password` 共享 branding 壳层

### 步骤 0：Gate Check
1. 若 `allow_registration=false`，先在 Branding 设置开启公开注册

### 测试步骤
1. 访问 `/register`
2. 访问 `/forgot-password`
3. 访问 `/reset-password?token=test-token`

### 预期结果
- 三个页面都显示 Auth9 Portal 的统一品牌壳层
- Logo/品牌缩写、标题区、背景和右上角语言/主题控件布局一致
- 页面由 Auth9 Portal 直接渲染品牌

---

## 场景 3：公开注册关闭时 `register` 回跳

### 测试步骤
1. 将 `allow_registration` 设置为 `false`
2. 访问 `http://localhost:3000/register`

### 预期结果
- 页面重定向到 `/login`
- 不展示注册表单

---

## 场景 4：`/mfa/verify` 路由可访问

> **注意**: `/mfa/verify` 路由需要 `mfa_session_token` 查询参数才能正常渲染。直接访问（无参数）会被重定向到 `/login`，这是**预期行为**。

### 测试步骤
1. 使用已启用 TOTP 的用户进行密码登录
2. 登录成功后应自动跳转到 `/mfa/verify?mfa_session_token=...&mfa_methods=totp`
3. 检查验证码输入框和继续按钮
4. 提交任意验证码

### 预期结果
- 页面由 Portal 渲染（需要有效的 `mfa_session_token`）
- 显示 TOTP 验证码输入框（OTP Input）和恢复码切换按钮
- 直接访问 `/mfa/verify`（无参数）→ 重定向到 `/login`

### 常见误报

| 现象 | 原因 | 解决方案 |
|------|------|----------|
| 直接访问重定向到 `/login` | 缺少 `mfa_session_token` 参数 | 通过正常登录流程触发 MFA 挑战 |
| 显示 "session expired" | MFA session token 已过期（TTL 有限） | 重新执行密码登录触发新的 MFA 挑战 |

---

## 场景 5：Portal 为唯一认证入口（注：Keycloak 已退役，无 fallback 概念）

### 测试步骤
1. 检查 Portal `/login` 的 Password 入口文案
2. 确认无任何外部认证页面的 fallback 跳转

### 预期结果
- Portal 是唯一认证入口
- UI 不包含任何外部认证引擎的 fallback 链路
- 所有认证流程由 Auth9 内置 OIDC 引擎处理
