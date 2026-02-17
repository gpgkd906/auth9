# 会话与安全 - OAuth State CSRF 校验

**模块**: 会话与安全
**测试范围**: Portal OAuth 登录流程中 state 参数的 CSRF 防护（cookie 存储、回调校验、清除）
**场景数**: 5
**优先级**: 高

---

## 背景说明

OAuth 2.0 规范要求客户端在授权请求中附带 `state` 参数，并在回调时校验一致性，防止 Login CSRF 攻击。

Auth9 Portal 的实现方式：
1. 用户点击「SSO 登录」时，Portal 生成 `state` 并存入 `oauth_state` httpOnly cookie（5 分钟有效）
2. 重定向到 Keycloak 授权端点，携带 `state` 参数
3. Keycloak 回调时，Portal 从 cookie 读取存储的 state，与 URL 中的 `state` 参数比对
4. 匹配成功 → 继续 token exchange；不匹配 → 拒绝，重定向到 `/login?error=state_mismatch`
5. 成功登录后清除 `oauth_state` cookie

涉及文件：
- `auth9-portal/app/services/session.server.ts` — `oauthStateCookie`、`serializeOAuthState()`、`getOAuthState()`、`clearOAuthStateCookie()`
- `auth9-portal/app/routes/login.tsx` — SSO 登录 action，设置 state cookie
- `auth9-portal/app/routes/auth.callback.tsx` — 回调 loader，校验并清除 state cookie

---

## 场景 1：正常 SSO 登录流程 — State 完整生命周期

### 初始状态
- 用户未登录
- 浏览器无 `oauth_state` cookie

### 目的
验证 SSO 登录的完整 state 生命周期：设置 → 传递 → 校验 → 清除

### 测试操作流程
1. 打开浏览器开发者工具 → Application → Cookies
2. 访问 `/login`
3. 输入企业邮箱地址（如 `user@acme.com`），点击「Continue with Enterprise SSO」
4. 观察浏览器跳转前的 Set-Cookie 响应头
5. 完成 Keycloak 登录
6. 观察回调 `/auth/callback` 的请求

### 预期结果
- 步骤 4：响应包含 `Set-Cookie: oauth_state=<encrypted_value>; Path=/; HttpOnly; SameSite=Lax`
- 步骤 5：Keycloak 授权 URL 包含 `state=<value>` 参数
- 步骤 6：回调成功后重定向到 `/dashboard`
- 回调响应包含两个 `Set-Cookie` 头：session cookie + `oauth_state=; Max-Age=0`（清除）
- 登录后浏览器中不再有 `oauth_state` cookie

---

## 场景 2：State 不匹配 — CSRF 攻击模拟

### 初始状态
- 用户未登录

### 目的
验证 state 参数不匹配时请求被拒绝

### 测试操作流程
1. 手动构造一个回调 URL，state 为伪造值：
   ```
   /auth/callback?code=valid-looking-code&state=forged-state-value
   ```
2. 在浏览器中直接访问该 URL（此时无 `oauth_state` cookie 或 cookie 中的值不匹配）
3. 观察页面行为

### 预期结果
- 重定向到 `/login?error=state_mismatch`
- 登录页面显示错误信息
- 不执行 token exchange（服务端日志包含 "OAuth state mismatch"）
- 浏览器中无新的 session cookie

---

## 场景 3：State Cookie 过期（5 分钟超时）

### 初始状态
- 用户点击 SSO 登录后，在 Keycloak 页面停留超过 5 分钟

### 目的
验证 state cookie 过期后回调被正确拒绝

### 测试操作流程
1. 访问 `/login`，点击「Continue with Enterprise SSO」
2. 跳转到 Keycloak 登录页后**等待超过 5 分钟**
3. 完成 Keycloak 登录
4. 观察回调行为

### 预期结果
- 回调重定向到 `/login?error=state_mismatch`（因 cookie 已过期，`getOAuthState()` 返回 `null`）
- 用户需要重新发起 SSO 登录

---

## 场景 4：无 State 参数的回调请求

### 初始状态
- 浏览器中有有效的 `oauth_state` cookie

### 目的
验证回调 URL 中缺少 state 参数时被拒绝

### 测试操作流程
1. 先通过正常流程设置 `oauth_state` cookie（点击 SSO 登录后不完成 Keycloak 认证）
2. 手动构造无 state 参数的回调 URL：
   ```
   /auth/callback?code=some-auth-code
   ```
3. 在浏览器中访问该 URL

### 预期结果
- 重定向到 `/login?error=state_mismatch`
- 日志记录 state mismatch（`hasStored: true, hasReceived: false`）

---

## 场景 5：Cookie 安全属性验证

### 初始状态
- 可访问 Portal 的 HTTP 和 HTTPS 环境

### 目的
验证 `oauth_state` cookie 的安全属性符合规范

### 测试操作流程
1. 在开发环境（HTTP）触发 SSO 登录，检查 `Set-Cookie` 响应头
2. 在生产环境（HTTPS）触发 SSO 登录，检查 `Set-Cookie` 响应头
3. 验证 cookie 属性

### 预期结果
- **共同属性**：
  - `Path=/`
  - `HttpOnly`（JavaScript 不可读取）
  - `SameSite=Lax`（允许 Keycloak 跨站 redirect 回带 cookie）
  - `Max-Age=300`（5 分钟）
- **开发环境**：无 `Secure` 标志（允许 HTTP）
- **生产环境**：包含 `Secure` 标志（仅 HTTPS）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 正常 SSO 登录 State 完整生命周期 | ☐ | | | |
| 2 | State 不匹配 — CSRF 攻击模拟 | ☐ | | | |
| 3 | State Cookie 过期（5 分钟超时） | ☐ | | | |
| 4 | 无 State 参数的回调请求 | ☐ | | | |
| 5 | Cookie 安全属性验证 | ☐ | | | |
