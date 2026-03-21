# UI/UX 测试 - 错误消息用户体验与可读性

**模块**: 页面专项 / 交互体验
**测试范围**: 各场景下错误消息的人类可读性、本地化完整性、内联错误展示、表单验证
**场景数**: 5
**关联 Feature Request**: `docs/feature_request/ui_error-message-mapping_scenario1_20260307_162702.md`

---

## 背景说明

Auth9 Portal 的错误消息来自多个来源：
1. 前端表单验证（本地）
2. `auth9-core` REST API 响应的错误码（`{ error: "error_code", message: "..." }` 格式）
3. Auth9 OIDC Engine 认证失败的错误描述

### 错误映射架构（两层映射）

所有 API 错误在 route action/loader 的 catch 块中通过 `mapApiError()` 统一映射为本地化文本，UI 层直接渲染已翻译的字符串。

```
API Response { error: "error_code", message: "..." }
    ↓
ApiResponseError (保留 code + message)
    ↓
mapApiError(error, locale)
    ├── 已知 error code → API_ERROR_CODE_MAP → i18n key → 本地化文本
    ├── "validation" code → formatErrorMessage() → 字段级格式化
    └── 未知 / 普通 Error → formatErrorMessage() 子字符串匹配
    ↓
actionData.error（已本地化的友好文本）
    ↓
UI 内联展示（红色文字 / role="alert"）
```

**第一层**：`API_ERROR_CODE_MAP` 精确匹配 16 种后端错误码到 `apiErrors.*` i18n key：

| 后端 error code | i18n key | zh-CN | en-US |
|---|---|---|---|
| `not_found` | `apiErrors.notFound` | 请求的资源不存在。 | The requested resource was not found. |
| `bad_request` | `apiErrors.badRequest` | 请求无效，请检查您的输入。 | The request is invalid. Please check your input. |
| `unauthorized` | `apiErrors.unauthorized` | 您的会话已过期，请重新登录。 | Your session has expired. Please sign in again. |
| `forbidden` | `apiErrors.forbidden` | 您没有权限执行此操作。 | You do not have permission to perform this action. |
| `conflict` | `apiErrors.conflict` | 具有相同标识的资源已存在。 | A resource with this identifier already exists. |
| `database_error` / `cache_error` / `internal_error` | `apiErrors.serverError` | 服务器发生错误，请稍后重试。 | A server error occurred. Please try again later. |
| `jwt_error` | `apiErrors.sessionExpired` | 您的会话已过期，请重新登录。 | Your session has expired. Please sign in again. |
| `auth_service_error` | `apiErrors.authServiceError` | 认证服务暂时不可用，请稍后重试。 | The authentication service is temporarily unavailable. |
| `rate_limited` | `apiErrors.rateLimited` | 请求过于频繁，请稍后再试。 | Too many requests. Please wait a moment and try again. |
| (unknown) | `apiErrors.unknown` | 发生未知错误，请重试。 | Something went wrong. Please try again. |

**第二层**：`formatErrorMessage()` 处理 `validation` 类错误的字段级细节（如 `"slug: invalid_slug"` → `"Slug: Slug 只能包含…"`）。

### 错误显示载体

当前系统使用以下方式展示错误：
- **内联错误区域**：表单/弹窗内红色文字（`text-[var(--accent-red)]`，`role="alert"`）
- **全页错误**：ErrorBoundary 渲染的错误页面（404、500 等）

> **注意**：虽然 `@radix-ui/react-toast` 已安装，但当前系统**未使用 Toast 通知**展示错误。所有操作错误通过内联区域展示。

---

## 场景 1：认证错误 — 友好的登录失败提示（入口可见性）

### 初始状态
- 用户未登录
- 访问 Auth9 Landing 页面或 `/login`

### 目的
验证登录失败时（密码错误、账号不存在等）显示的错误消息为人类可读的本地化文本，不暴露技术错误码。

### 测试操作流程
1. 从 Landing 页面点击「Sign In / 登录」进入认证页
2. 输入正确格式的邮箱，但使用**错误的密码**
3. 点击登录/提交
4. 观察错误提示内容
5. 切换语言后重复步骤 2~4

**触发错误码（预期被映射）**：
- OIDC `invalid_grant` → 认证页显示友好错误文本
- Portal 内部登录失败 → 通过 `mapApiError` 映射

### 预期视觉效果
- 错误消息为自然语言句子
- 不出现英文大写下划线格式（如 `INVALID_CREDENTIALS`）
- 不出现 JSON 格式字符串（如 `{"error": "invalid_grant"}`）
- 不出现 HTTP 状态码作为唯一提示（如 `401` 或 `400`）
- 错误消息颜色为系统红色 `var(--accent-red)` (`#FF3B30`)

### 验证工具
```javascript
const errorEls = document.querySelectorAll(
  '[class*="error"], [class*="alert"], [role="alert"]'
);

errorEls.forEach((el, i) => {
  const text = el.textContent?.trim();
  console.log(`Error ${i + 1}:`, text);

  const isTechnical = /^[A-Z_]{5,}$/.test(text) ||
                      text.includes('{"') ||
                      /^\d{3}$/.test(text);
  if (isTechnical) {
    console.error(`Technical error exposed to user: "${text}"`);
  }
});
```

---

## 场景 2：表单验证错误 — 内联错误的可读性与完整性

### 初始状态
- 已登录，进入任意包含表单的页面（用户创建、邀请发送、Provider 配置等）

### 目的
验证所有表单字段的内联验证错误消息为当前语言的友好文本，格式规范，位置直觉合理。验证 `validation` 类错误通过 `formatErrorMessage()` 正确格式化字段名和错误描述。

### 测试操作流程
1. 进入「Users」→「Create User」弹窗
2. 将必填字段留空，点击「创建 / Create」
3. 输入格式错误的邮箱（如 `notanemail`），点击提交
4. 输入过短的密码，点击提交
5. 进入「Tenants」→「Create Tenant」弹窗
6. 输入已存在的 slug，点击「创建 / Create」
7. 进入「Invitations」→「Invite User」弹窗，留空邮箱提交

### 预期视觉效果

**内联错误格式**：
- 位置：表单区域或弹窗底部，`role="alert"` 属性
- 颜色：`var(--accent-red)` / `#FF3B30`（系统红色）
- 字号：`text-sm`（14px）

**中文下的友好错误文本示例**（`mapApiError` + `formatErrorMessage` 输出）：
- 邮箱格式错误：「请输入有效的邮箱地址。」
- 字段必填：「此字段为必填项。」
- Slug 格式错误：「Slug 只能包含小写字母、数字和连字符，且不能以连字符开头或结尾。」
- 资源已存在：「具有相同标识的资源已存在。」

**英文下的对应示例**：
- 「Please enter a valid email address.」
- 「This field is required.」
- 「Slug can only contain lowercase letters, numbers, and hyphens...」
- 「A resource with this identifier already exists.」

### 验证工具
```javascript
const fieldErrors = document.querySelectorAll(
  'p[class*="accent-red"], [role="alert"]'
);

fieldErrors.forEach((el, i) => {
  const text = el.textContent?.trim();
  const styles = getComputedStyle(el);
  console.log(`Field Error ${i + 1}:`, {
    text,
    color: styles.color,
    visible: styles.display !== 'none',
  });
});
```

---

## 场景 3：操作失败 — 内联错误区域的人类可读性

### 初始状态
- 已登录，可以执行各类操作（创建、更新、删除）

### 目的
验证操作失败时（API 调用失败、权限不足、服务器错误）在表单/弹窗内显示的内联错误内容为经过 `mapApiError` 映射的本地化友好文本。

### 测试操作流程
1. 尝试触发以下操作失败场景：
   - 创建已存在 slug/name 的 Tenant → 预期：`apiErrors.conflict` 映射文本
   - 使用低权限 token 执行管理操作 → 预期：`apiErrors.forbidden` 映射文本
   - 停止 `auth9-core` 后执行操作 → 预期：网络错误或 `apiErrors.serverError` 映射文本
2. 每次触发后观察弹窗/表单内的红色内联错误区域
3. 切换语言后重复关键场景，验证内联错误语言切换正确

### 预期视觉效果

**内联错误格式**：
- 位置：弹窗/表单内，通常在提交按钮上方
- 颜色：红色 `var(--accent-red)` 文字
- 文本：简洁的自然语言句子

**中文友好文本示例**（实际 `apiErrors` i18n 输出）：
- Slug 冲突（`conflict`）：「具有相同标识的资源已存在。」
- 权限不足（`forbidden`）：「您没有权限执行此操作。」
- 服务器错误（`internal_error`）：「服务器发生错误，请稍后重试。」
- 认证服务异常（`auth_service_error`）：「认证服务暂时不可用，请稍后重试。」
- 请求频率过高（`rate_limited`）：「请求过于频繁，请稍后再试。」

**禁止出现**：
- `conflict`、`forbidden`、`internal_error` 等原始 error code
- `403 Forbidden`、`500 Internal Server Error` 等 HTTP 状态描述
- 原始 JSON 错误体
- 英文后端错误消息（在非英文 locale 下）

### 验证工具
```javascript
const observer = new MutationObserver((mutations) => {
  mutations.forEach(m => {
    m.addedNodes.forEach(node => {
      if (node.nodeType === 1) {
        const text = node.textContent?.trim();
        const isError = node.getAttribute('role') === 'alert' ||
                        node.className?.includes('accent-red');
        if (isError && text) {
          console.log('Error appeared:', text);
          if (/^[a-z_]{6,}$/.test(text) || text.match(/^\d{3}/)) {
            console.error('Technical content in error:', text);
          }
        }
      }
    });
  });
});
observer.observe(document.body, { childList: true, subtree: true });
console.log('Error observer active. Trigger an error to test.');
```

---

## 场景 4：网络与服务器级错误 — 系统错误页面的友好性

### 初始状态
- 通过 Chrome DevTools 模拟网络错误，或访问不存在的路由

### 目的
验证 404、500、网络断开等系统级错误下的展示，保持品牌一致且人类可读。ErrorBoundary 组件使用 `common.errors.*` i18n key 展示本地化错误页面。

### 测试操作流程
1. 访问不存在的路由（如 `/dashboard/nonexistent-page-xyz`）
2. 在 Chrome DevTools → Network → Offline 模式下执行 API 操作
3. 停止 auth9-core 后访问需要 API 的页面

### 预期视觉效果

**404 页面**：
- 标题：「页面不存在 / Page Not Found」（非 `404`）
- 保持 Auth9 品牌样式，非浏览器默认 404
- 提供「返回首页 / Go back home」导航链接

**网络错误内联提示**：
- 「发生未知错误，请重试。 / Something went wrong. Please try again.」

**500 / 服务不可用**：
- 「服务器发生错误，请稍后重试。 / A server error occurred. Please try again later.」
- 不暴露 Stack Trace 或技术细节

### 验证工具
```javascript
console.log('Page title:', document.title);
console.log('H1:', document.querySelector('h1')?.textContent);
console.log('Error text:', document.querySelector('[class*="error"], [class*="not-found"]')?.textContent?.trim());
```

---

## 场景 5：三种语言下错误消息映射的完整覆盖一致性

### 初始状态
- 系统支持 `zh-CN`、`en-US` 和 `ja`
- `apiErrors` 命名空间在三个 locale 文件中均已定义

### 目的
验证所有已知 API 错误码在三种语言下均有对应的本地化友好文本，不出现某种语言有映射、其他语言显示原始错误码的情况。

### 测试操作流程
1. 切换到中文（`zh-CN`），触发场景 1~3 中的错误场景，记录中文错误文本
2. 切换到英文（`en-US`），触发相同错误场景，记录英文错误文本
3. 切换到日语（`ja`），触发相同错误场景，记录日语错误文本
4. 对比：三种语言各自不出现技术性字符串，语言对应正确

### 预期视觉效果

| 错误场景 | zh-CN | en-US | ja |
|---------|-------|-------|-----|
| 资源不存在 | 请求的资源不存在。 | The requested resource was not found. | リクエストされたリソースが見つかりません。 |
| 权限不足 | 您没有权限执行此操作。 | You do not have permission to perform this action. | この操作を実行する権限がありません。 |
| 资源冲突 | 具有相同标识的资源已存在。 | A resource with this identifier already exists. | この識別子のリソースはすでに存在します。 |
| 服务器错误 | 服务器发生错误，请稍后重试。 | A server error occurred. Please try again later. | サーバーエラーが発生しました。しばらくしてから再度お試しください。 |
| 会话过期 | 您的会话已过期，请重新登录。 | Your session has expired. Please sign in again. | セッションの有効期限が切れました。再度ログインしてください。 |
| 请求频繁 | 请求过于频繁，请稍后再试。 | Too many requests. Please wait a moment and try again. | リクエストが多すぎます。しばらく待ってから再度お試しください。 |
| 字段必填 | 此字段为必填项。 | This field is required. | この項目は必須です。 |
| 未知错误 | 发生未知错误，请重试。 | Something went wrong. Please try again. | エラーが発生しました。再度お試しください。 |

**禁止出现**：
- 中文 UI 下出现英文错误码
- 英文 UI 下出现中文/日文错误文本
- 日语 UI 下出现其他语言错误文本
- 任何语言下出现技术性格式字符串（`not_found`、`conflict`、`403` 等）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 认证错误 — 登录失败友好提示 | ☐ | | | 需覆盖三种语言 |
| 2 | 表单验证错误 — 内联错误可读性 | ☐ | | | 覆盖多个表单弹窗，验证 formatErrorMessage 字段格式化 |
| 3 | 操作失败 — 内联错误区域人类可读性 | ☐ | | | 验证 mapApiError 对 16 种 error code 的映射 |
| 4 | 网络与服务器级错误 — 错误页面友好性 | ☐ | | | 需模拟网络错误，验证 ErrorBoundary 本地化 |
| 5 | 三种语言下错误映射覆盖一致性 | ☐ | | | 需含日语验证；对照 apiErrors i18n 表格逐项确认 |

---

## 截图说明

1. **场景 1**：登录失败时的错误消息特写（中/英/日三语）
2. **场景 2**：表单提交失败时各字段内联错误特写（含字段名本地化）
3. **场景 3**：操作失败时弹窗内联错误区域特写（不同 error code 对应的文案）
4. **场景 4**：404 页面全页截图
5. **场景 5**：同一错误场景的中英日三语对比截图（并排）

---

## 实现参考

- **错误映射函数**: `auth9-portal/app/lib/error-messages.ts` — `mapApiError()` + `formatErrorMessage()`
- **i18n 翻译**: `auth9-portal/app/i18n/locales/{en-US,zh-CN,ja}.ts` — `apiErrors.*` 命名空间
- **API 客户端**: `auth9-portal/app/services/api/client.ts` — `ApiResponseError` 类
- **后端错误定义**: `auth9-core/src/error/mod.rs` — `AppError` + `ErrorResponse`

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Auth9 login error shows "Invalid username or password." in English | This is the Auth9 hosted login page. Login errors are rendered by the Auth9 品牌认证页, not Portal's `mapApiError()`. | Auth9 login page i18n is controlled by the `ui_locales` parameter. |
| 404 page shows English text despite Chinese locale | Playwright browser defaults to `Accept-Language: en`. During client-side navigation, the root loader resolves locale from Accept-Language. SSR renders correctly (verified via `curl`). | Set `auth9_locale` cookie before testing, or configure Playwright's `locale` option in the test config. Verify SSR with `curl -s http://localhost:3000/nonexistent \| rg 'lang='`. |
| 404 page text not translated (suspected missing i18n) | ErrorBoundary in `root.tsx` correctly uses `translate(locale, ...)` for all 404 page text. Translations exist in all three locales: en-US "Page not found", zh-CN "页面不存在", ja "ページが見つかりません". This is NOT a missing translation bug. | Verify the `auth9_locale` cookie is set to the correct locale value. The root loader reads locale from this cookie (or falls back to `Accept-Language`). If the cookie is absent or set to `en-US`, English text is expected behavior. |
| Language switch on 404 page not working | The ErrorBoundary 404 page is minimal and does not include a language switcher. | Navigate away from the 404 page, switch language on a normal page, then return to the nonexistent URL. |
