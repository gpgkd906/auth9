# UI/UX 测试 - Keycloak 主题国际化（i18n）

**模块**: 认证流程 / 国际化
**测试范围**: Keycloak 自定义主题在 zh-CN / en-US 下的文案正确性、语言参数透传、认证页品牌一致性
**场景数**: 4
**关联 Ticket**: `docs/ticket/keycloak-theme_i18n-not-implemented_scenario1_20260307_162702.md`

---

## 背景说明

Auth9 使用自定义 Keycloak 主题（`auth9-keycloak-theme`）接管 Keycloak 的认证页面（登录、注册、MFA、错误页等）。

**当前已知缺陷**：
- Keycloak 主题未实现 i18n，所有认证页文案固定为单一语言（英文）
- 用户在 Portal 中切换语言后，跳转到 Keycloak 认证页时，语言不跟随切换

**Keycloak i18n 机制**：
- 主题目录 `messages/messages_{locale}.properties` 文件定义每种语言的文案
- `theme.properties` 中声明 `locales=en,zh-CN`
- 通过 URL 参数 `ui_locales=zh-CN` 或 `Accept-Language` 控制语言

**Auth9 语言透传机制**（Fix 后应实现）：
- Portal 发起认证跳转时，在 Keycloak 授权 URL 中附加 `ui_locales={current_locale}` 参数

---

## 场景 1：语言入口可见性 — 认证页语言切换入口（入口可见性）

### 初始状态
- 用户在 Portal `/login` 页面
- 当前语言为中文（`zh-CN`）

### 目的
验证 Keycloak 认证页面在语言参数正确透传时，显示与 Portal 一致的语言文案，且认证页内有可见的语言切换入口（或跟随 Portal 语言自动切换）。

### 测试操作流程
1. 在 Portal 将语言设置为「简体中文」
2. 点击「使用密码登录」跳转到 Keycloak 认证页
3. 观察认证页的所有文案语言
4. 返回 Portal，将语言切换为「English」
5. 再次点击「Sign in with password」跳转到认证页
6. 观察认证页文案是否变更为英文

### 预期视觉效果
- **中文状态下**：认证页标题、输入框 placeholder、按钮、错误提示均为中文
  - 例：「登录」「请输入邮箱」「继续」「忘记密码？」
- **英文状态下**：相同元素均为英文
  - 例：「Sign In」「Enter your email」「Continue」「Forgot password?」
- 认证页保持 Auth9 品牌样式（不暴露 Keycloak 默认 UI）

### 验证工具（在认证页运行）
```javascript
// 在 Keycloak 认证页检查文案语言
console.log('Page language:', document.documentElement.lang);
console.log('URL ui_locales param:', new URLSearchParams(location.search).get('ui_locales'));
console.log('Submit button text:', document.querySelector('[type="submit"]')?.value);
console.log('Email placeholder:', document.querySelector('[type="email"]')?.placeholder);
```

---

## 场景 2：中文语言下认证页完整文案覆盖

### 初始状态
- Portal 当前语言为「简体中文」
- Docker 环境运行正常，Keycloak 使用最新 auth9 主题

### 目的
验证 Keycloak 认证页的所有文案区域（包括错误提示）在中文模式下均有完整的中文翻译，无英文 fallback 或原始 i18n key 泄漏。

### 测试操作流程
1. 设置 Portal 语言为中文，跳转到 Keycloak 认证页
2. 逐一检查以下区域的文案：
   - 页面标题区域
   - 邮箱/用户名输入框 placeholder
   - 密码输入框 placeholder
   - 「登录」提交按钮
   - 「忘记密码」链接
   - 「注册」链接（如有）
   - 社交登录按钮（如「使用 GitHub 登录」）
3. 故意输入错误密码，触发错误提示
4. 检查错误提示是否为中文

### 预期视觉效果

| 区域 | 预期中文内容 |
|------|------------|
| 页面标题 | 「登录到 {应用名}」或「欢迎回来」 |
| 邮箱输入框 | placeholder: 「请输入邮箱地址」 |
| 密码输入框 | placeholder: 「请输入密码」 |
| 提交按钮 | 「登录」 |
| 忘记密码 | 「忘记密码？」 |
| 登录失败提示 | 「用户名或密码错误」 |

**禁止出现**：
- ❌ 原始 i18n key（如 `loginTitleHtml`、`doLogIn`）
- ❌ 英文 fallback（如 `Sign In`、`Password`）
- ❌ 空白文字区域（key 未翻译导致显示空字符串）

### 验证工具
```javascript
// 检查是否有未翻译的 key（通常为驼峰或点分格式）
const allText = document.body.innerText;
const untranslatedPattern = /\b[a-z]+[A-Z][a-zA-Z]+\b/g;  // camelCase patterns
const matches = allText.match(untranslatedPattern);
if (matches) {
  console.warn('Possible untranslated keys found:', matches);
}
```

---

## 场景 3：错误页面与 MFA 页面的语言一致性

### 初始状态
- Portal 分别设置为中文和英文
- 可触发 MFA（如账号已启用 TOTP）

### 目的
验证 Keycloak 主题的所有页面（不只是登录页）均完成 i18n，包括 MFA 页、密码重置页、错误提示页。

### 测试操作流程
1. 使用已启用 MFA 的账号，在中文 Portal 下触发 MFA 流程
2. 观察 MFA 页面（输入 OTP 码的页面）的文案语言
3. 在 Portal 设置英文，重复触发 MFA 流程
4. 在认证页输入错误密码 N 次，触发账号锁定或限流提示页
5. 观察这些附加页面的文案语言

### 预期视觉效果
- **MFA 页（中文）**：「请输入验证器 App 中显示的验证码」「验证」
- **MFA 页（英文）**：「Enter the verification code from your authenticator app」「Verify」
- **错误/限流页（中文）**：「您的账号已被临时锁定，请稍后再试」
- **错误/限流页（英文）**：「Your account has been temporarily locked. Please try again later.」

所有 Keycloak 托管页面保持 Auth9 品牌样式，不泄漏 Keycloak 默认 UI。

---

## 场景 4：ui_locales 参数透传回归验证

### 初始状态
- Portal 分别切换语言为中文 / 英文

### 目的
验证 Portal 在发起 Keycloak 认证跳转时，正确将当前语言作为 `ui_locales` 参数附加到授权 URL，Keycloak 据此渲染对应语言。

> ⚠️ **回归验证**: 此场景是 Keycloak i18n 实现的核心技术验证，修复后必须通过。

### 测试操作流程
1. 设置 Portal 语言为「English」
2. 在 Portal 点击「Sign in with password」
3. 在跳转到 Keycloak 之前（可通过 DevTools Network 或 URL 观察），检查请求 URL
4. 确认 URL 中包含 `ui_locales=en` 或 `ui_locales=en-US`
5. 切换到中文，重复上述步骤，确认 URL 包含 `ui_locales=zh-CN`

### 预期结果

**Network 请求（跳转前的 Authorization URL）**：
```
https://auth.example.com/realms/auth9/protocol/openid-connect/auth?
  client_id=...
  &redirect_uri=...
  &ui_locales=en-US    ← 应在英文 Portal 下出现
```

```
https://auth.example.com/realms/auth9/protocol/openid-connect/auth?
  client_id=...
  &redirect_uri=...
  &ui_locales=zh-CN    ← 应在中文 Portal 下出现
```

### 验证工具
```javascript
// 在 Portal /login 页面，检查登录按钮的跳转 URL（如果是静态链接）
const loginBtn = document.querySelector('a[href*="keycloak"], a[href*="auth"], [class*="login-btn"]');
if (loginBtn) {
  const href = loginBtn.getAttribute('href');
  console.log('Login URL:', href);
  console.log('Has ui_locales:', href?.includes('ui_locales'));
}

// 若是动态请求，通过 Network 面板观察 302 Redirect 目标 URL
// Chrome DevTools → Network → 点击登录按钮 → 查找 302 请求 → 查看 Location header
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 语言入口可见性 — 认证页语言跟随切换（入口可见性）| ☐ | | | |
| 2 | 中文下认证页完整文案覆盖（回归 Ticket #4）| ☐ | | | **已知 Bug 回归项，必测** |
| 3 | MFA 页与错误页语言一致性 | ☐ | | | 需有启用 MFA 的测试账号 |
| 4 | ui_locales 参数透传回归验证（回归 Ticket #4）| ☐ | | | **已知 Bug 回归项，必测** |

---

## 截图说明

1. **场景 1**：中文/英文 Portal × 中文/英文 Keycloak 认证页对比（2×2 截图组合）
2. **场景 2**：认证页所有文案区域截图（标注各区域）
3. **场景 3**：MFA 页中英文各一张
4. **场景 4**：Chrome DevTools Network 截图，标注 `ui_locales` 参数位置
