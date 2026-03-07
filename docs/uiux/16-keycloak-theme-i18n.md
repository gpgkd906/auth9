# UI/UX 测试 - Keycloak 主题国际化（i18n）

**模块**: 认证流程 / 国际化
**测试范围**: Keycloak 自定义主题在 zh-CN / en-US / ja 下的文案正确性、语言参数透传、认证页品牌一致性
**场景数**: 4
**关联 Ticket**: `docs/ticket/keycloak-theme_i18n-not-implemented_scenario1_20260307_162702.md`

---

## 背景说明

Auth9 使用自定义 Keycloak 主题（`auth9-keycloak-theme`）接管 Keycloak 的认证页面（登录、注册、MFA、错误页等）。

**已修复**（2026-03-07）：
- Keycloak 主题已实现 i18n（Keycloakify `withCustomTranslations` 支持 `en` / `zh-CN` / `ja`）
- Portal 认证跳转已附加 `ui_locales` 参数，Keycloak 认证页跟随 Portal 语言切换
- 所有硬编码文案已替换为 i18n 调用（`msgStr()`）

**Keycloak i18n 机制**：
- 通过 Keycloakify `i18nBuilder.withCustomTranslations` 定义自定义 key 翻译（`en` / `zh-CN` / `ja`）
- 内置 Keycloak message key 由 Keycloak 自身 i18n 处理
- 通过 URL 参数 `ui_locales` 控制语言

**Auth9 语言透传机制**：
- Portal 发起认证跳转时，在 Keycloak 授权 URL 中附加 `ui_locales={mapped_locale}` 参数
- 映射规则：`en-US` → `en`，`zh-CN` → `zh-CN`，`ja` → `ja`

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
7. 返回 Portal，将语言切换为「日本語」
8. 再次点击「パスワードでサインイン」跳转到认证页
9. 观察认证页文案是否变更为日语

### 预期视觉效果
- **中文状态下**：认证页标题、输入框 placeholder、按钮、错误提示均为中文
  - 例：「登录」「请输入邮箱」「继续」「忘记密码？」
- **英文状态下**：相同元素均为英文
  - 例：「Sign In」「Enter your email」「Continue」「Forgot password?」
- **日语状态下**：相同元素均为日语
  - 例：主题切换按钮 aria-label 为「ライトモード」「ダークモード」
  - 社交登录分隔文案为「または以下で続ける」
  - 返回登录链接为「← ログインに戻る」
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
   - 主题切换按钮 aria-label（「浅色模式」/「深色模式」）
   - 社交登录分隔文案（「或使用以下方式继续」）
3. 故意输入错误密码，触发错误提示
4. 检查错误提示是否为中文
5. 切换到日语，重复步骤 1-4，验证自定义 key 为日语

### 预期视觉效果

**中文文案覆盖**：

| 区域 | 预期中文内容 |
|------|------------|
| 页面标题 | 「登录到 {应用名}」或「欢迎回来」 |
| 邮箱输入框 | placeholder: 「请输入邮箱地址」 |
| 密码输入框 | placeholder: 「请输入密码」 |
| 提交按钮 | 「登录」 |
| 忘记密码 | 「忘记密码？」 |
| 登录失败提示 | 「用户名或密码错误」 |
| 社交登录分隔 | 「或使用以下方式继续」 |
| 主题切换 | aria-label: 「浅色模式」/「深色模式」 |

**日语文案覆盖（自定义 key）**：

| 区域 | 预期日语内容 |
|------|------------|
| 返回登录链接 | 「← ログインに戻る」 |
| OTP 设备选择 | 「OTPデバイスを選択」 |
| 社交登录分隔 | 「または以下で続ける」 |
| 已有账户提示 | 「アカウントをお持ちですか？」 |
| 主题切换 | aria-label: 「ライトモード」/「ダークモード」 |

**禁止出现**：
- ❌ 原始 i18n key（如 `loginTitleHtml`、`doLogIn`、`selectOtpDevice`）
- ❌ 语言不匹配的 fallback（如中文模式下出现英文 `Sign In`，日语模式下出现中文文案）
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
- Portal 分别设置为中文、英文和日语
- 可触发 MFA（如账号已启用 TOTP）

### 目的
验证 Keycloak 主题的所有页面（不只是登录页）均完成 i18n，包括 MFA 页、密码重置页、错误提示页。

### 测试操作流程
1. 使用已启用 MFA 的账号，在中文 Portal 下触发 MFA 流程
2. 观察 MFA 页面（输入 OTP 码的页面）的文案语言，特别是 OTP 设备选择标签
3. 在 Portal 设置英文，重复触发 MFA 流程
4. 在 Portal 设置日语，重复触发 MFA 流程
5. 在认证页输入错误密码 N 次，触发账号锁定或限流提示页
6. 观察这些附加页面在三种语言下的文案

### 预期视觉效果
- **MFA 页（中文）**：OTP 选择标签「选择验证设备」、验证按钮「验证」
- **MFA 页（英文）**：OTP 选择标签「Select OTP Device」
- **MFA 页（日语）**：OTP 选择标签「OTPデバイスを選択」
- **密码重置页**：返回登录链接在日语下为「← ログインに戻る」
- **错误/限流页**：三种语言下均显示对应翻译

所有 Keycloak 托管页面保持 Auth9 品牌样式，不泄漏 Keycloak 默认 UI。

---

## 场景 4：ui_locales 参数透传回归验证

### 初始状态
- Portal 分别切换语言为中文 / 英文 / 日语

### 目的
验证 Portal 在发起 Keycloak 认证跳转时，正确将当前语言作为 `ui_locales` 参数附加到授权 URL，Keycloak 据此渲染对应语言。含企业 SSO 发现请求的透传验证。

> ⚠️ **回归验证**: 此场景是 Keycloak i18n 实现的核心技术验证，修复后必须通过。

### 测试操作流程
1. 设置 Portal 语言为「English」
2. 在 Portal 点击「Sign in with password」
3. 在跳转到 Keycloak 之前（可通过 DevTools Network 或 URL 观察），检查请求 URL
4. 确认 URL 中包含 `ui_locales=en`（注意映射：`en-US` → `en`）
5. 切换到中文，重复上述步骤，确认 URL 包含 `ui_locales=zh-CN`
6. 切换到日语，重复上述步骤，确认 URL 包含 `ui_locales=ja`
7. 使用企业 SSO 登录入口，确认 SSO 发现请求也携带 `ui_locales` 参数

### 预期结果

**Network 请求（跳转前的 Authorization URL）**：
```
# 英文 Portal：en-US 映射为 en
ui_locales=en

# 中文 Portal
ui_locales=zh-CN

# 日语 Portal
ui_locales=ja
```

**企业 SSO 发现请求**：
```
GET /api/v1/enterprise-sso/discovery?...&ui_locales=ja
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
| 1 | 语言入口可见性 — 认证页三语跟随切换 | ☐ | | | 需验证日语跳转后文案正确 |
| 2 | 三语下认证页完整文案覆盖 | ☐ | | | 含自定义 key 日语验证 |
| 3 | MFA 页与错误页三语一致性 | ☐ | | | 需有启用 MFA 的测试账号 |
| 4 | ui_locales 三语参数透传 + 企业 SSO | ☐ | | | 含映射验证 en-US→en |

---

## 截图说明

1. **场景 1**：中文/英文 Portal × 中文/英文 Keycloak 认证页对比（2×2 截图组合）
2. **场景 2**：认证页所有文案区域截图（标注各区域）
3. **场景 3**：MFA 页中英文各一张
4. **场景 4**：Chrome DevTools Network 截图，标注 `ui_locales` 参数位置
