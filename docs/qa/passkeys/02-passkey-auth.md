# Passkeys (WebAuthn) - Passkey 登录认证

**模块**: Passkeys
**测试范围**: 使用 Passkey 在登录页面进行无密码认证
**场景数**: 5
**优先级**: 高

---

## 背景说明

登录页面新增「Sign in with passkey」按钮，与 SSO 登录并列。用户可使用已注册的 Passkey 进行 discoverable authentication（浏览器自动展示可用 Passkey 列表），无需输入用户名/密码。

认证流程：
1. 前端调用 `POST /api/v1/auth/webauthn/authenticate/start`（公开端点，无需认证）
2. 前端调用 `navigator.credentials.get()` 显示浏览器 Passkey 选择器
3. 用户选择 Passkey 并完成生物识别验证
4. 前端调用 `POST /api/v1/auth/webauthn/authenticate/complete`
5. 后端验证断言、创建 Session、签发 Identity Token
6. 前端通过 Form POST 将 Token 存入 Session Cookie，重定向到 `/tenant/select`（单 tenant 账号可自动进入 Dashboard）

签发的 Identity Token 与 Keycloak OIDC 登录签发的格式相同（含 `sub`, `email`, `name`, `sid`, `iss`, `aud` claims），后续 Token Exchange 和会话管理与 SSO 登录完全一致。

---

## 场景 1：登录页面显示 Passkey 登录按钮

### 初始状态
- 用户已登出
- 访问 `/login?error=access_denied` 或 `/login?passkey=true`

### 目的
验证登录页面在 error 或 passkey 模式下正确显示 SSO 和 Passkey 两种登录方式

### 测试操作流程
1. 访问 `/login?passkey=true`

### 预期结果
- 显示 Auth9 Logo「A9」
- 显示标题「Sign In」
- 显示「Sign in with SSO」按钮
- 显示分割线「or」
- 显示「Sign in with passkey」按钮（带锁图标）
- 两个按钮均可点击

---

## 场景 2：使用 Passkey 成功登录

### 初始状态
- 用户已注册至少 1 个 Passkey
- 用户已登出
- 设备上存在已注册的 Passkey

### 目的
验证完整的 Passkey 认证流程：从点击按钮到成功登录跳转

### 测试操作流程
1. 访问 `/login?passkey=true`
2. 点击「Sign in with passkey」按钮
3. 按钮变为「Verifying...」状态（禁用）
4. 浏览器弹出 Passkey 选择器（显示可用的 Passkey 列表）
5. 选择一个 Passkey
6. 完成生物识别/PIN 验证

### 预期结果
- 浏览器弹出系统级 Passkey 选择对话框
- 验证成功后跳转到 `/tenant/select` 或直接 `/dashboard`（取决于 tenant 数量）
- 若进入 `/tenant/select`，选择 tenant 后进入 Dashboard，页面正常加载且用户信息正确
- 用户的 Session 状态与 SSO 登录一致

### 预期数据状态
```sql
-- 验证 Session 已创建
SELECT id, user_id, ip_address, user_agent, created_at
FROM sessions
WHERE user_id = '{user_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在新 Session 记录

-- 验证 Passkey last_used_at 已更新
SELECT id, last_used_at FROM webauthn_credentials
WHERE user_id = '{user_id}'
ORDER BY last_used_at DESC LIMIT 1;
-- 预期: last_used_at 为当前时间附近
```

---

## 场景 3：Passkey 认证取消

### 初始状态
- 用户已登出
- 访问 `/login?passkey=true`

### 目的
验证用户取消 Passkey 认证时的错误处理

### 测试操作流程
1. 访问 `/login?passkey=true`
2. 点击「Sign in with passkey」按钮
3. 在浏览器弹出 Passkey 选择器时**点击取消**

### 预期结果
- 显示红色错误提示「Authentication was cancelled or timed out.」
- 按钮恢复为「Sign in with passkey」可用状态
- 「Sign in with SSO」按钮也恢复为可用状态
- 用户可以重新尝试 Passkey 登录或改用 SSO

---

## 场景 4：错误页面显示 Passkey 登录选项

### 初始状态
- 用户访问 `/login?error=access_denied`（SSO 登录失败后重定向）

### 目的
验证 SSO 登录失败后，用户可以选择 Passkey 作为替代登录方式

### 测试操作流程
1. 模拟 SSO 登录失败：访问 `/login?error=access_denied`

### 预期结果
- 显示标题「Sign In Failed」
- 显示错误描述「Access was denied. Please try again or contact your administrator.」
- 显示「Sign in with SSO」按钮（可重新尝试 SSO）
- 显示「Sign in with passkey」按钮（可改用 Passkey）
- 两种登录方式均可正常使用

---

## 场景 5：默认自动跳转 SSO（无 error/passkey 参数）

### 初始状态
- 用户已登出
- 直接访问 `/login`（不带任何查询参数）

### 目的
验证默认行为保持不变：自动重定向到 SSO 认证

### 测试操作流程
1. 在浏览器地址栏输入 `/login` 并回车

### 预期结果
- 页面自动 302 重定向到 Keycloak `/api/v1/auth/authorize` 端点
- 不显示登录页面 UI
- 重定向 URL 包含 `response_type=code`、`scope=openid email profile`、`state=` 参数
- 用户正常进入 Keycloak SSO 流程

---

## 通用场景：Passkey 登录后 Token 有效性

### 测试操作流程
1. 使用 Passkey 成功登录
2. 若进入 `/tenant/select`，先选择 tenant 并完成 token exchange
3. 进入 Dashboard，正常使用各功能（查看租户、用户、设置等）
4. 检查 Token Exchange 是否正常工作

### 预期结果
- Passkey 登录获得的 Identity Token 可正常用于 Token Exchange
- 所有需要认证的 API 调用正常工作
- Session 管理（列表、撤销）正常工作
- 与 SSO 登录的行为完全一致

---

## Agent 自动化测试：Playwright MCP 工具 + CDP 虚拟认证器

本文档的场景 1、2、4、5 可由 AI Agent 通过 Playwright MCP 工具执行。场景 2（Passkey 登录）需要先注册凭据，因此必须在**同一个 `browser_run_code` 调用**中完成「注册 + 登出 + Passkey 登录」的完整流程（虚拟认证器的凭据存储在 CDP 会话内存中，跨调用会丢失）。

> **前提条件**: 全栈环境运行中（Docker + auth9-core on :8080 + auth9-portal on :3000）。

### 步骤 0：初始化虚拟认证器

调用 **`browser_run_code`**:
```javascript
async (page) => {
  const client = await page.context().newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,       // 必须为 true，Passkey 登录使用 discoverable credential
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return { authenticatorId };
}
```

### 步骤 1：场景 1 — 登录页面 UI 验证

1. 调用 **`browser_navigate`**: `http://localhost:3000/login?passkey=true`
2. 调用 **`browser_snapshot`** 查看页面
3. **验证**（从 snapshot 中确认）：
   - 显示 Auth9 Logo「A9」
   - 显示标题「Sign In」
   - 存在「Sign in with SSO」按钮
   - 存在分割线「or」
   - 存在「Sign in with passkey」按钮（带锁图标）

### 步骤 2：场景 4 — 错误页面 Passkey 选项

1. 调用 **`browser_navigate`**: `http://localhost:3000/login?error=access_denied`
2. 调用 **`browser_snapshot`** 查看页面
3. **验证**（从 snapshot 中确认）：
   - 显示标题「Sign In Failed」
   - 显示错误描述「Access was denied...」
   - 同时存在「Sign in with SSO」和「Sign in with passkey」按钮

### 步骤 3：场景 5 — 默认 SSO 重定向

1. 调用 **`browser_navigate`**: `http://localhost:3000/login`（不带参数）
2. 调用 **`browser_snapshot`** 查看页面
3. **验证**: 页面已自动跳转到 Keycloak SSO 页面（URL 包含 `/realms/auth9` 或 `/api/v1/auth/authorize`）

### 步骤 4：场景 2 — Passkey 完整登录流程（核心）

> 此步骤必须在**单个 `browser_run_code` 调用**中完成全部操作，因为虚拟认证器的凭据不跨调用保留。

调用 **`browser_run_code`**:
```javascript
async (page) => {
  // ===== 1. 创建虚拟认证器 =====
  const client = await page.context().newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal',
      hasResidentKey: true, hasUserVerification: true,
      isUserVerified: true, automaticPresenceSimulation: true,
    },
  });

  // ===== 2. SSO 登录 =====
  await page.goto('http://localhost:3000/login');
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/, { timeout: 10000 });
  await page.getByLabel(/username/i).fill('e2e-test-user');
  await page.getByLabel(/password/i).fill('Test123!');
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL(/localhost:3000/, { timeout: 15000 });

  // ===== 3. 注册 Passkey（为 Passkey 登录准备凭据） =====
  await page.goto('http://localhost:3000/dashboard/account/passkeys');
  await page.waitForSelector('text=Passkeys', { timeout: 5000 });
  await page.getByRole('button', { name: /Add passkey/i }).first().click();
  // 虚拟认证器自动完成注册
  await page.waitForSelector('text=Passkey registered successfully', { timeout: 10000 });
  const registered = true;

  // ===== 4. 登出 =====
  await page.goto('http://localhost:3000/logout');
  await page.waitForTimeout(2000);

  // ===== 5. Passkey 登录 =====
  await page.goto('http://localhost:3000/login?passkey=true');
  await page.waitForSelector('text=Sign in with passkey', { timeout: 5000 });
  await page.getByRole('button', { name: /Sign in with passkey/i }).click();
  // 虚拟认证器自动完成 discoverable authentication
  await page.waitForURL(/\/(tenant\/select|dashboard)/, { timeout: 15000 });

  // 多租户账号会先停在 tenant 选择页，选择后再进入 dashboard
  if (page.url().includes('/tenant/select')) {
    const firstTenantButton = page.locator('button').filter({ hasText: /continue|select|进入|选择/i }).first();
    if (await firstTenantButton.count()) {
      await firstTenantButton.click();
      await page.waitForURL(/\/dashboard/, { timeout: 15000 });
    }
  }

  return {
    registered,
    loginSuccess: page.url().includes('/dashboard') || page.url().includes('/tenant/select'),
    finalUrl: page.url(),
  };
}
```

调用完成后，调用 **`browser_snapshot`** 确认已到达 `/tenant/select` 或 Dashboard；若在 `/tenant/select`，执行选择后再确认 Dashboard。

### 场景 3（认证取消）— 不可自动化

虚拟认证器无法模拟用户点击取消，此场景需手动测试。

### 注意事项

- **`hasResidentKey: true` 是必须的**：Passkey 登录使用 discoverable authentication（`navigator.credentials.get()` 不指定 `allowCredentials`），虚拟认证器必须支持 resident key 才能响应。
- **注册和登录必须同一调用**：虚拟认证器的凭据存储在 CDP 会话内存中，每次 `browser_run_code` 创建新 CDP session 时凭据不保留。
- **场景 1、4、5 可独立执行**：这些场景只验证 UI 和重定向行为，不涉及 WebAuthn 交互，可以用普通 MCP 工具（`browser_navigate` + `browser_snapshot`）完成。

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 登录页面显示 Passkey 登录按钮 | ☐ | | | |
| 2 | 使用 Passkey 成功登录 | ☐ | | | 核心流程 |
| 3 | Passkey 认证取消 | ☐ | | | |
| 4 | 错误页面显示 Passkey 登录选项 | ☐ | | | |
| 5 | 默认自动跳转 SSO | ☐ | | | 回归测试 |
| 6 | Passkey 登录后 Token 有效性 | ☐ | | | |
