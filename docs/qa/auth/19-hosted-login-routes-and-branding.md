# Hosted Login 路由与 Branding 托管

**模块**: auth
**场景数**: 5

---

## 场景 1：`/login` 由 Portal 托管认证入口

### 步骤 0：Gate Check
1. 访问 `http://localhost:3000/login`
2. 确认地址栏保持在 Auth9 Portal 域名，而不是 `Keycloak /realms/...`

### 测试步骤
1. 检查页面是否展示 Enterprise SSO、Password、Passkey、Email code（若启用）入口
2. 点击 Password 入口

### 预期结果
- 页面由 Portal 渲染
- 地址栏保持 `http://localhost:3000/login`
- Password 入口先展开 Portal 内 fallback 说明，不应默认直接跳到 Keycloak

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
- 页面不依赖 Keycloak theme 二次拉取品牌再渲染

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

### 测试步骤
1. 访问 `http://localhost:3000/mfa/verify`
2. 检查验证码输入框和继续按钮
3. 提交任意验证码

### 预期结果
- 页面由 Portal 渲染，不跳到 Keycloak
- 显示 Hosted MFA 的占位说明
- 当前阶段返回后续接入提示，而不是 Keycloak 原生页面

---

## 场景 5：Keycloak theme 仅作为 fallback

### 测试步骤
1. 阅读 `auth9-keycloak-theme/README.md`
2. 检查 Portal `/login` 的 Password 入口文案

### 预期结果
- README 明确 Portal 是默认认证入口
- Keycloak theme 被标记为 rollback / compatibility fallback
- UI 和文档都不再把 Keycloak 登录页描述为默认主入口
