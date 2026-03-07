# UI/UX 测试 - 错误消息用户体验与可读性

**模块**: 页面专项 / 交互体验
**测试范围**: 各场景下错误消息的人类可读性、本地化完整性、Toast 通知、表单内联验证
**场景数**: 5
**关联 Ticket**: `docs/ticket/ui_error-message-mapping_scenario1_20260307_162702.md`

---

## 背景说明

Auth9 Portal 的错误消息来自多个来源：
1. 前端表单验证（本地）
2. `auth9-core` REST API 响应的错误码（`error_code` 字段）
3. Keycloak 认证失败的错误描述

所有面向用户的错误消息必须满足：
- **人类可读**：不暴露原始错误码（如 `INVALID_CREDENTIALS`）或技术字符串
- **本地化一致**：与当前语言（`zh-CN` / `en-US` / `ja`）匹配
- **明确指导**：提示用户下一步应该做什么

### 错误显示载体
- **Toast 通知**：全局浮动提示（操作失败时）
- **表单内联错误**：红色文字显示在表单字段下方
- **全页错误**：错误边界页（500、404 等系统错误）
- **对话框/弹窗内错误**：Modal 内部的错误提示

---

## 场景 1：认证错误 — 友好的登录失败提示（入口可见性）

### 初始状态
- 用户未登录
- 访问 Auth9 Landing 页面或 `/login`

### 目的
验证登录失败时（密码错误、账号不存在等）显示的错误消息为人类可读的本地化文本，不暴露技术错误码。

> ⚠️ **回归验证**: 此场景对应已知问题 — 部分错误码未被映射为友好文本。

### 测试操作流程
1. 从 Landing 页面点击「Sign In / 登录」进入认证页
2. 输入正确格式的邮箱，但使用**错误的密码**
3. 点击登录/提交
4. 观察错误提示内容

**触发错误码（预期被映射）**：
- `INVALID_CREDENTIALS` → 中文：「邮箱或密码错误，请重试」/ 英文：「Invalid email or password, please try again」
- `ACCOUNT_LOCKED` → 「账号已被锁定，请联系管理员」

### 预期视觉效果
- 错误消息为自然语言句子
- 不出现英文大写下划线格式（如 `INVALID_CREDENTIALS`）
- 不出现 JSON 格式字符串（如 `{"error": "invalid_grant"}`）
- 不出现 HTTP 状态码作为唯一提示（如 `401` 或 `400`）
- 错误消息颜色为系统红色 `#FF3B30`

### 验证工具
```javascript
// 等待错误出现后运行
const errorEls = document.querySelectorAll(
  '[class*="error"], [class*="alert"], [role="alert"], .toast, [class*="Toast"]'
);

errorEls.forEach((el, i) => {
  const text = el.textContent?.trim();
  console.log(`Error ${i + 1}:`, text);

  // 检查是否包含技术性错误格式
  const isTechnical = /^[A-Z_]{5,}$/.test(text) ||
                      text.includes('{"') ||
                      /^\d{3}$/.test(text);
  if (isTechnical) {
    console.error(`⚠️ Technical error exposed to user: "${text}"`);
  }
});
```

---

## 场景 2：表单验证错误 — 内联错误的可读性与完整性

### 初始状态
- 已登录，进入任意包含表单的页面（用户创建、邀请发送、Provider 配置等）

### 目的
验证所有表单字段的内联验证错误消息为当前语言的友好文本，格式规范，位置直觉合理。

### 测试操作流程
1. 进入「Users」→「Create User」弹窗
2. 将必填字段留空，点击「创建 / Create」
3. 输入格式错误的邮箱（如 `notanemail`），点击提交
4. 输入过短的密码，点击提交
5. 进入「Invitations」→「Invite User」弹窗，留空邮箱提交

### 预期视觉效果

**内联错误格式**：
- 位置：字段输入框正下方，左对齐
- 颜色：`#FF3B30`（系统红色）
- 字号：比标签字号小 1~2 级（通常 12~13px）

**中文下的友好错误文本示例**：
- 邮箱格式错误：「请输入有效的邮箱地址」（不是 `Email format invalid`）
- 字段必填：「此字段不能为空」（不是 `required`）
- 密码过短：「密码长度至少为 8 位」（不是 `min length: 8`）

**英文下的对应示例**：
- 「Please enter a valid email address」
- 「This field is required」
- 「Password must be at least 8 characters」

### 验证工具
```javascript
// 提交后检查内联错误消息
const fieldErrors = document.querySelectorAll(
  '[class*="field-error"], [class*="input-error"], [class*="FormError"], p[class*="error"]'
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

## 场景 3：操作失败 Toast — 非技术性全局通知

### 初始状态
- 已登录，可以执行各类操作（创建、更新、删除）

### 目的
验证操作失败时（API 调用失败、权限不足、服务器错误）弹出的 Toast 通知内容人类可读、有指导意义。

> ⚠️ **回归验证**: 此场景对应已知问题 — 部分 API 错误码未映射为友好 Toast。

### 测试操作流程
1. 尝试触发以下操作失败场景：
   - 创建已存在 slugname 的 Tenant
   - 执行需要更高权限的操作（用低权限账号尝试）
   - 断开网络后执行操作
2. 每次触发后观察 Toast 通知内容
3. 切换语言后重复关键场景，验证 Toast 语言切换正确

### 预期视觉效果

**Toast 格式**：
- 位置：右上角或右下角固定浮层
- 错误 Toast：红色图标 + 深色或红色背景
- 文本：简洁的自然语言句子（1~2 行）
- 持续：3~5 秒自动消失，或有关闭按钮

**友好文本示例（中文）**：
- Slug 冲突：「该标识符已被使用，请更换一个」
- 权限不足：「您没有权限执行此操作」
- 网络错误：「网络连接失败，请稍后重试」

**禁止出现**：
- ❌ `SLUG_ALREADY_TAKEN`
- ❌ `403 Forbidden`
- ❌ `Internal Server Error`
- ❌ 原始 JSON 错误体

### 验证工具
```javascript
// 监听 Toast 出现
const observer = new MutationObserver((mutations) => {
  mutations.forEach(m => {
    m.addedNodes.forEach(node => {
      if (node.nodeType === 1) {
        const text = node.textContent?.trim();
        const isToast = node.className?.includes('toast') ||
                        node.getAttribute('role') === 'alert';
        if (isToast && text) {
          console.log('Toast appeared:', text);
          if (/[A-Z_]{6,}/.test(text) || text.match(/^\d{3}/)) {
            console.error('⚠️ Technical content in Toast:', text);
          }
        }
      }
    });
  });
});
observer.observe(document.body, { childList: true, subtree: true });
console.log('Toast observer active. Trigger an error to test.');
```

---

## 场景 4：网络与服务器级错误 — 系统错误页面的友好性

### 初始状态
- 通过 Chrome DevTools 模拟网络错误，或访问不存在的路由

### 目的
验证 404、500、网络断开等系统级错误下的展示，保持品牌一致且人类可读。

### 测试操作流程
1. 访问不存在的路由（如 `/dashboard/nonexistent-page-xyz`）
2. 在 Chrome DevTools → Network → Offline 模式下执行 API 操作
3. 停止 auth9-core 后访问需要 API 的页面

### 预期视觉效果

**404 页面**：
- 标题：「页面不存在 / Page Not Found」（非 `404`）
- 保持 Auth9 品牌样式，非浏览器默认 404

**网络错误 Toast/提示**：
- 「网络连接失败，请检查网络后重试」

**500 / 服务不可用**：
- 「服务暂时不可用，请稍后重试」
- 不暴露 Stack Trace 或技术细节

### 验证工具
```javascript
console.log('Page title:', document.title);
console.log('H1:', document.querySelector('h1')?.textContent);
console.log('Error text:', document.querySelector('[class*="error"], [class*="not-found"]')?.textContent?.trim());
```

---

## 场景 5：三种语言下错误消息的完整覆盖一致性

### 初始状态
- 系统支持 `zh-CN`、`en-US` 和 `ja`

### 目的
验证所有已知错误场景在三种语言下均有对应的友好文本，不出现某种语言有映射、其他语言显示原始错误码的情况。

> ⚠️ **回归验证**: 此场景对应已知问题 — 部分错误消息未随语言切换。

### 测试操作流程
1. 切换到中文（`zh-CN`），触发场景 1~3 中的错误场景，记录中文错误文本
2. 切换到英文（`en-US`），触发相同错误场景，记录英文错误文本
3. 对比：中英文各自不出现技术性字符串，语言对应正确

### 预期视觉效果

| 错误场景 | 中文 | 英文 |
|---------|------|------|
| 密码错误 | 邮箱或密码错误，请重试 | Invalid email or password, please try again |
| 权限不足 | 您没有权限执行此操作 | You don't have permission to perform this action |
| 字段必填 | 此字段不能为空 | This field is required |
| 网络错误 | 网络连接失败，请稍后重试 | Network error, please try again later |
| 服务器错误 | 服务器错误，请稍后重试 | Server error, please try again later |

**禁止出现**：
- ❌ 中文 UI 下出现英文错误码
- ❌ 英文 UI 下出现中文错误文本
- ❌ 任何语言下出现技术性格式字符串

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 认证错误 — 登录失败友好提示（回归 Ticket #3）| ☐ | | | **已知问题回归项，必测** |
| 2 | 表单验证错误 — 内联错误可读性 | ☐ | | | 覆盖多个表单弹窗 |
| 3 | 操作失败 Toast — 非技术性通知（回归 Ticket #3）| ☐ | | | **已知问题回归项，必测** |
| 4 | 网络与服务器级错误 — 错误页面友好性 | ☐ | | | 需模拟网络错误 |
| 5 | 三种语言下错误覆盖一致性（回归 Ticket #3）| ☐ | | | **已知问题回归项，必测；需含日语验证** |

---

## 截图说明

1. **场景 1**：登录失败时的错误消息特写
2. **场景 2**：表单提交失败时各字段内联错误特写
3. **场景 3**：Toast 通知特写（含完整文案）
4. **场景 4**：404 页面全页截图
5. **场景 5**：同一错误场景的中英文对比截图（并排）
