# Hosted Login 路由与 Branding 托管

**模块**: auth
**场景数**: 5

---

## 前提条件

- 全栈环境运行中（Docker + auth9-core on :8080 + auth9-portal on :3000）
- **测试用户必须已有密码凭据（password credential）**：运行 `./scripts/reset-docker.sh` 重置环境并种子化测试数据，确保测试用户在 `credentials` 表中有 `credential_type = 'password'` 的记录。场景 4（MFA verify 路由）需要用户先完成密码登录以触发 MFA 挑战，若用户无密码凭据则无法进入该流程。

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

> **重要**: `/register` 页面读取的是 **服务级别** branding（通过 `GET /api/v1/public/branding?client_id=auth9-portal`），而非系统级别 branding。
> 必须通过服务 branding API 关闭注册，而非 `PUT /api/v1/system/branding`。

### 步骤 0：Gate Check
1. 获取 Portal 所属 service 的 ID：
   ```sql
   SELECT s.id FROM services s JOIN clients c ON c.service_id = s.id WHERE c.client_id = 'auth9-portal' LIMIT 1;
   ```
2. 确认该 service 有 branding 且 `allow_registration=true`：
   ```sql
   SELECT JSON_EXTRACT(config, '$.allow_registration') FROM service_branding WHERE service_id = '<service_id>';
   ```

### 测试步骤
1. 通过 **服务 branding API** 关闭注册（需要管理员 Token）：
   ```bash
   curl -X PUT "http://localhost:8080/api/v1/services/<service_id>/branding" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"config":{"primary_color":"#007AFF","secondary_color":"#5856D6","background_color":"#F5F5F7","text_color":"#1D1D1F","allow_registration":false,"email_otp_enabled":false}}'
   ```
2. 访问 `http://localhost:3000/register`

### 预期结果
- 页面重定向到 `/login`
- 不展示注册表单

### 测试后恢复
将 `allow_registration` 恢复为 `true`（避免影响后续场景）。

### 常见误报

| 现象 | 原因 | 解决方案 |
|------|------|----------|
| 注册页面仍然显示 | 通过系统 branding API 关闭了注册 | 必须通过服务 branding API 关闭（服务级别覆盖系统级别） |

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
