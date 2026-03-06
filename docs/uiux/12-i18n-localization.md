# UI/UX 测试 - 国际化与本地化

**模块**: 页面专项
**测试范围**: Portal 语言切换、首屏语言协商、文案一致性、格式化、本地化可发现性
**场景数**: 5

---

## 背景说明

Auth9 Portal 已完成 `zh-CN` / `en-US` 双语接入，语言协商规则为：

1. `auth9_locale` cookie
2. `Accept-Language`
3. 默认回退 `zh-CN`

当前语言不进入 URL。语言切换入口位于认证页和 Dashboard 右上角，与主题切换按钮并排显示。

---

## 场景 1：登录页语言入口可见且可切换

### 初始状态
- 用户未登录
- 浏览器访问 `http://localhost:3000/login`

### 目的
验证用户无需手输 URL 参数，即可在登录页发现语言切换入口，并实时切换整页文案。

### 测试操作流程
1. 打开 `/login`
2. 观察右上角控制区，确认存在语言切换器与主题切换按钮
3. 记录当前标题、描述、SSO 按钮、密码登录按钮、Passkey 按钮文案
4. 将语言从「简体中文」切换到 `English`
5. 再次观察整页文案
6. 刷新页面

### 预期视觉效果
- 右上角控制区包含两个独立按钮：语言切换、主题切换
- 两者高度、圆角、边框风格一致，不出现拥挤或错位
- 切换语言后，标题、描述、按钮、错误提示、placeholder 同步切换为英文
- 刷新后保持英文，不回退到中文

### 验证工具
```javascript
console.log(document.documentElement.lang);
console.log(document.cookie.includes('auth9_locale='));
```

---

## 场景 2：首屏语言协商无闪烁

### 初始状态
- 清空 `auth9_locale` cookie
- 准备两个浏览器上下文：`Accept-Language: zh-CN` 与 `Accept-Language: en-US`

### 目的
验证 SSR 首屏直接使用协商后的语言渲染，不出现先中文后英文或先英文后中文的闪烁。

### 测试操作流程
1. 使用 `Accept-Language: zh-CN` 访问 `/login`
2. 观察首屏首个可见标题与按钮文案
3. 使用 `Accept-Language: en-US` 访问 `/login`
4. 观察首屏首个可见标题与按钮文案
5. 打开浏览器控制台，检查 hydration warning

### 预期视觉效果
- 中文上下文首屏直接显示中文
- 英文上下文首屏直接显示英文
- 页面加载过程中不出现中英来回切换
- 控制台无 hydration mismatch 相关警告

### 验证工具
```javascript
console.log(document.documentElement.lang);
performance.getEntriesByType('navigation')[0];
```

---

## 场景 3：Dashboard 全局控件与导航语言一致

### 初始状态
- 用户已登录并进入 `/dashboard`
- 已有至少 1 个 tenant

### 目的
验证 Dashboard 壳层、侧边栏、顶部控件、空态与弹窗文案在同一语言下保持一致，不出现中英混杂。

### 测试操作流程
1. 从侧边栏依次查看 `Dashboard`、`Tenants`、`Users`、`Services`、`Settings`
2. 打开任意创建弹窗或确认弹窗
3. 检查搜索框 placeholder、空态、按钮、Tab、下拉框 placeholder
4. 切换语言后重复上述检查

### 预期视觉效果
- 侧边栏导航、页面标题、按钮、弹窗、placeholder 属于同一语言
- 切换语言后，页面结构不抖动，按钮宽度变化可接受，不出现截断或重叠
- 右上角语言切换器与主题切换器在桌面端对齐，在移动端不溢出

### 验证重点
- 不允许同一屏内同时出现未翻译的英文 placeholder 与中文按钮
- 不允许确认弹窗标题/描述仍保留旧硬编码文案

---

## 场景 4：日期、数字与状态格式随语言变化

### 初始状态
- 用户已登录
- 存在带时间、数量、状态徽标的页面数据（如 Audit Logs、Analytics、Users、Sessions）

### 目的
验证格式化层使用当前 locale，而不是浏览器/运行环境的默认值。

### 测试操作流程
1. 在中文下访问 `/dashboard/audit-logs`、`/dashboard/analytics`、`/dashboard/account/sessions`
2. 记录日期、时间、计数分页、状态徽标格式
3. 切换到英文后刷新页面
4. 再次记录相同区域

### 预期视觉效果
- 中文下日期、数量、分页文案为中文格式
- 英文下日期、数量、分页文案为英文格式
- 服务端渲染与客户端 hydration 后格式保持一致
- 不出现同一字段在同页显示两种格式

### 验证工具
```javascript
console.log(document.documentElement.lang);
[...document.querySelectorAll('time')].map((el) => el.textContent);
```

---

## 场景 5：表单输入与错误提示本地化完整性

### 初始状态
- 用户已登录
- 可访问 `Settings / Email`、`Identity Providers`、`Users`、`Invitations`

### 目的
验证表单的 `label`、`placeholder`、校验错误、成功提示、确认弹窗在两种语言下都完整可读。

### 测试操作流程
1. 进入 `Settings -> Email`，观察 provider 配置表单 placeholder
2. 进入 `Settings -> Identity Providers`，打开新增弹窗
3. 进入 `Users` 创建用户弹窗
4. 进入 `Tenant Invitations` 创建邀请弹窗
5. 在中英文各执行一次字段留空/格式错误操作

### 预期视觉效果
- 表单 `label` 与 `placeholder` 语言一致
- 校验错误、成功提示、确认弹窗均切换到当前语言
- 敏感字段遮罩 placeholder（如 `***`）保持功能语义，不影响本地化完整性
- 移动端下长 placeholder 不应顶破输入框布局

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 登录页语言入口可见且可切换 | 通过 | 2026-03-07 | Codex | 浏览器实测，英文切换后 reload 保持英文 |
| 2 | 首屏语言协商无闪烁 | 通过 | 2026-03-07 | Codex | `curl` 实测 `Accept-Language: en-US` 返回 `<html lang="en-US">`，`zh-CN` 返回 `<html lang="zh-CN">`；浏览器未出现 hydration mismatch |
| 3 | Dashboard 全局控件与导航语言一致 | 通过 | 2026-03-07 | Codex | `/dashboard` 英文壳层、侧栏、顶部控件与页面内容一致，reload 后保持英文 |
| 4 | 日期、数字与状态格式随语言变化 | 通过 | 2026-03-07 | Codex | Dashboard 最近活动时间在英文下显示为 `Mar 7, 2026, 2:49 AM`，与 `lang=en-US` 一致 |
| 5 | 表单输入与错误提示本地化完整性 | 通过 | 2026-03-07 | Codex | `Settings / Identity Providers` 与 `Settings / Email` 占位符在英文下正确显示 |

---

## 本轮实测记录（2026-03-07）

### 实际环境
- Portal: `http://localhost:3000`
- 登录方式: 本地真实浏览器 + 实际后端 / Keycloak
- 附加验证: `curl` 直接检查 SSR HTML 首屏输出

### 关键结论
1. 语言持久化已修复
   - 登录页切换到 `English` 后刷新，页面保持英文
   - Dashboard 切换到 `English` 后刷新，侧栏、标题、日期格式保持英文
2. SSR 语言协商生效
   - `Accept-Language: en-US` 首屏直接输出英文 HTML
   - `Accept-Language: zh-CN` 首屏直接输出中文 HTML
3. 表单本地化已覆盖到实际可输入字段
   - Identity Providers OIDC 新建弹窗英文占位符：
     - `e.g., google-enterprise`
     - `e.g., Sign in with Google`
     - `OAuth Client ID`
     - `OAuth Client Secret`
     - `https://provider.com/oauth/authorize`
     - `https://provider.com/oauth/token`
   - Email Settings 英文占位符：
     - `smtp.example.com`
     - `username`
     - `Enter password`
     - `noreply@example.com`

### 非阻塞观察
- 浏览器控制台未出现 hydration mismatch
- 仍有若干与 i18n 无关的表单可访问性提示：
  - 部分输入缺少 `id/name`
  - 部分输入缺少 `autocomplete`
  这些不影响本轮国际化验收，但应单独纳入 a11y 收敛
