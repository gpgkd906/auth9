# Passkeys (WebAuthn) - 注册与管理

**模块**: Passkeys
**测试范围**: 原生 WebAuthn Passkey 注册、列表、删除
**场景数**: 5
**优先级**: 高

---

## 背景说明

Passkeys 已从 Keycloak 代理模式迁移到原生 WebAuthn 实现。注册流程完全在 Auth9 内完成：前端调用浏览器 WebAuthn API，后端使用 `webauthn-rs` 处理挑战和验证，凭据存储在 TiDB `webauthn_credentials` 表中。

注册端点（需要认证）：
- `POST /api/v1/users/me/passkeys/register/start` — 获取 `CreationChallengeResponse`
- `POST /api/v1/users/me/passkeys/register/complete` — 提交浏览器验证结果

管理端点（需要认证）：
- `GET /api/v1/users/me/passkeys` — 列出用户的 Passkeys
- `DELETE /api/v1/users/me/passkeys/{credential_id}` — 删除指定 Passkey

---

## 数据库表结构参考

### webauthn_credentials 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 所属用户 ID |
| credential_id | VARCHAR(512) | WebAuthn 凭据 ID（UNIQUE） |
| credential_data | JSON | 序列化的 webauthn-rs Passkey 结构 |
| user_label | VARCHAR(255) | 用户自定义名称 |
| aaguid | VARCHAR(64) | 认证器标识 |
| created_at | TIMESTAMP | 创建时间 |
| last_used_at | TIMESTAMP | 最后使用时间 |

---

## 场景 1：查看 Passkeys 列表（无 Passkey）

### 初始状态
- 用户已登录
- 用户没有注册任何 Passkey

### 目的
验证空状态页面正确显示

### 测试操作流程
1. 进入「Account」→「Passkeys」

### 预期结果
- 显示空状态提示「No passkeys yet」
- 显示说明文字「Add a passkey to sign in faster and more securely.」
- 显示「Add your first passkey」按钮
- 显示「About Passkeys」信息卡片，包含「More secure」「Fast & easy」「Works everywhere」
- 顶部显示「Add passkey」按钮

### 预期数据状态
```sql
SELECT COUNT(*) FROM webauthn_credentials WHERE user_id = '{user_id}';
-- 预期: 0
```

---

## 场景 2：注册新 Passkey（原生 WebAuthn）

### 初始状态
- 用户已登录
- 用户的设备支持 WebAuthn（如 Touch ID、Windows Hello）
- 浏览器支持 `navigator.credentials.create()` API

### 目的
验证原生 WebAuthn Passkey 注册流程（不跳转 Keycloak）

### 测试操作流程
1. 进入「Account」→「Passkeys」
2. 点击「Add passkey」按钮
3. 按钮变为「Registering...」状态（禁用）
4. 设备弹出生物识别/PIN 验证对话框
5. 完成生物识别或 PIN 验证

### 预期结果
- 点击后**不跳转到 Keycloak**，页面保持不变
- 设备弹出 WebAuthn 验证请求（Touch ID / Windows Hello / Security Key）
- 验证成功后显示绿色提示「Passkey registered successfully!」
- Passkey 列表自动刷新，显示新注册的 Passkey
- 新 Passkey 显示类型标签（Passwordless）和创建日期
- 按钮恢复为「Add passkey」可用状态

### 预期数据状态
```sql
SELECT id, user_id, credential_id, user_label, aaguid, created_at
FROM webauthn_credentials
WHERE user_id = '{user_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在新记录，credential_id 非空，credential_data 为有效 JSON
```

---

## 场景 3：查看已注册的 Passkeys

### 初始状态
- 用户已登录
- 用户已注册 1 个或多个 Passkeys

### 目的
验证 Passkey 列表正确显示（包含 TiDB 原生凭据和可能的 Keycloak 遗留凭据）

### 测试操作流程
1. 进入「Account」→「Passkeys」

### 预期结果
- 显示所有已注册的 Passkeys
- 每个 Passkey 显示：
  - 名称（用户自定义或默认「Passkey」）
  - 类型标签（Passwordless / Two-Factor / 原始类型字符串）
  - 添加日期（格式：Month Day, Year）
  - 「Remove」按钮
- 顶部显示「Add passkey」按钮
- 如有加载错误，显示红色错误提示

### 预期数据状态
```sql
SELECT id, user_label, credential_id,
       JSON_EXTRACT(credential_data, '$.cred.type_') as cred_type,
       created_at
FROM webauthn_credentials
WHERE user_id = '{user_id}';
-- 预期: 返回列表与页面显示一致
```

---

## 场景 4：删除 Passkey

### 初始状态
- 用户已登录
- 用户已注册至少 1 个 Passkey

### 目的
验证 Passkey 删除功能

### 测试操作流程
1. 进入「Account」→「Passkeys」
2. 找到要删除的 Passkey
3. 点击该 Passkey 行右侧的「Remove」按钮

### 预期结果
- 显示成功提示消息
- 该 Passkey 从列表中消失
- 如果删除后列表为空，显示空状态
- 该 Passkey 不能再用于登录

### 预期数据状态
```sql
SELECT COUNT(*) FROM webauthn_credentials
WHERE id = '{credential_id}' AND user_id = '{user_id}';
-- 预期: 0（记录已删除）
```

---

## 场景 5：注册 Passkey 失败（取消或超时）

### 初始状态
- 用户已登录
- 设备支持 WebAuthn

### 目的
验证注册取消或失败时的错误处理

### 测试操作流程
1. 进入「Account」→「Passkeys」
2. 点击「Add passkey」按钮
3. 在设备弹出 WebAuthn 对话框时**点击取消**（或等待超时）

### 预期结果
- 显示红色错误提示「Registration was cancelled or timed out.」
- 按钮恢复为「Add passkey」可用状态
- 不创建新的 Passkey 记录
- 页面保持正常，可以重新尝试注册

### 预期数据状态
```sql
SELECT COUNT(*) FROM webauthn_credentials WHERE user_id = '{user_id}';
-- 预期: 数量与操作前相同（无新增）
```

---

## 通用场景：认证状态检查

### 测试操作流程
1. 在未登录状态下直接访问 `/dashboard/account/passkeys`

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 Passkeys 管理内容
- 登录后可正常访问 Passkeys 页面

---

## Agent 自动化测试：Playwright MCP 工具 + CDP 虚拟认证器

本文档的场景 1-4 可由 AI Agent 通过 Playwright MCP 工具直接执行。核心原理：通过 `browser_run_code` 调用 CDP `WebAuthn.addVirtualAuthenticator` 创建虚拟认证器，拦截浏览器的 `navigator.credentials.create()` 调用，无需物理硬件。

> **前提条件**: 全栈环境运行中（Docker + auth9-core on :8080 + auth9-portal on :3000）。Playwright MCP 使用 Chromium（CDP 虚拟认证器仅支持 Chromium）。

### 步骤 0：初始化虚拟认证器

> 这是所有后续步骤的前提。虚拟认证器的生命周期绑定到浏览器 Tab，一旦创建，该 Tab 内所有 WebAuthn 请求都会被虚拟认证器自动接管。

调用 **`browser_run_code`**:
```javascript
async (page) => {
  const client = await page.context().newCDPSession(page);
  await client.send('WebAuthn.enable');
  const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return { authenticatorId };
}
```

### 步骤 1：SSO 登录

1. 调用 **`browser_navigate`**: `http://localhost:3000/login`
2. 调用 **`browser_snapshot`** 查看页面，找到「Sign in with SSO」按钮
3. 调用 **`browser_click`** 点击 SSO 按钮
4. 页面跳转到 Keycloak，调用 **`browser_snapshot`** 查看登录表单
5. 调用 **`browser_fill_form`** 填写 username=`e2e-test-user`, password=`Test123!`
6. 调用 **`browser_click`** 点击 Keycloak 的「Sign In」按钮
7. 调用 **`browser_snapshot`** 确认跳转到 Dashboard

### 步骤 2：场景 1 — 验证空状态

1. 调用 **`browser_navigate`**: `http://localhost:3000/dashboard/account/passkeys`
2. 调用 **`browser_snapshot`** 查看页面
3. **验证**（从 snapshot 中确认）：
   - 页面包含文字「No passkeys yet」
   - 页面包含文字「Add a passkey to sign in faster and more securely.」
   - 存在「Add your first passkey」按钮
   - 存在「About Passkeys」区域，包含「More secure」「Fast & easy」「Works everywhere」

### 步骤 3：场景 2 — 注册 Passkey

> 因为步骤 0 已创建虚拟认证器且 `automaticPresenceSimulation: true`，点击注册按钮后虚拟认证器会自动完成生物识别验证，无需额外操作。

1. 调用 **`browser_snapshot`** 确认当前在 Passkeys 页面
2. 调用 **`browser_click`** 点击「Add passkey」按钮
3. 等待约 2 秒（虚拟认证器自动完成注册仪式）
4. 调用 **`browser_snapshot`** 查看结果
5. **验证**（从 snapshot 中确认）：
   - 显示绿色提示「Passkey registered successfully!」
   - 列表中出现新 Passkey（显示类型标签如「Passwordless」和创建日期）
   - 「Add passkey」按钮恢复可用

### 步骤 4：场景 3 — 查看已注册的 Passkeys

1. 调用 **`browser_snapshot`** 查看当前列表（注册后会自动刷新）
2. **验证**（从 snapshot 中确认）：
   - 列表中至少有 1 个 Passkey
   - 显示名称（默认「Passkey」）
   - 显示类型标签（Passwordless / Two-Factor）
   - 显示添加日期
   - 每个 Passkey 有「Remove」按钮

### 步骤 5：场景 4 — 删除 Passkey

1. 调用 **`browser_click`** 点击「Remove」按钮
2. 等待约 1 秒
3. 调用 **`browser_snapshot`** 查看结果
4. **验证**（从 snapshot 中确认）：
   - Passkey 已从列表消失
   - 显示空状态「No passkeys yet」（如果是最后一个）

### 场景 5（取消注册）— 不可自动化

虚拟认证器的设计目的是自动成功完成操作，无法模拟用户点击取消。此场景需手动测试。

### 注意事项

- **虚拟认证器生命周期**: 绑定到当前浏览器 Tab。如果 `browser_navigate` 跳转到其他域再回来，认证器仍然有效。但如果关闭 Tab 或切换 Tab，需要重新创建。
- **`automaticPresenceSimulation: true`**: 虚拟认证器会自动响应所有 WebAuthn 请求，Agent 只需正常点击按钮即可。
- **单次 `browser_run_code` 限制**: 步骤 0 的虚拟认证器创建必须通过 `browser_run_code`，因为这是 CDP 调用。后续的页面交互可以使用常规 MCP 工具（`browser_click`、`browser_snapshot` 等）。

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看 Passkeys 列表（无 Passkey） | ☐ | | | |
| 2 | 注册新 Passkey（原生 WebAuthn） | ☐ | | | 不应跳转 Keycloak |
| 3 | 查看已注册的 Passkeys | ☐ | | | |
| 4 | 删除 Passkey | ☐ | | | |
| 5 | 注册 Passkey 失败（取消或超时） | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
