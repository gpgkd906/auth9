# UI/UX 测试 - 全局控件布局与位置合理性

**模块**: 视觉设计 / 交互体验 / 页面专项
**测试范围**: 主题切换与语言切换控件在 Landing 页 Navbar 和 Dashboard Sidemenu 中的位置合理性、无重叠性
**场景数**: 5
**关联 Ticket**: `docs/ticket/ui_theme-lang-switcher-overlap_scenario1_20260307_162702.md`

---

## 背景说明

主题切换（Theme Toggle）和语言切换（Language Switcher）是贯穿整个 Auth9 Portal 的全局控件。当前已知缺陷：

1. **Landing 页面**：控件与「Sign In / 登录」按钮发生视觉重叠
2. **Dashboard 页面**：控件与侧边栏导航按钮发生重叠

本文档用于验证修复后的布局方案，以及对三种解决方案方向的视觉合理性评估。

### 全局控件定义
- **主题切换（Theme Toggle）**：在 Light / Dark 模式之间切换
- **语言切换（Language Switcher）**：在 `zh-CN` / `en-US` / `ja` 之间切换（三语支持）
- 两者应始终并排出现，保持相同的视觉风格

---

## 场景 1：全局控件入口可见性（两个页面均需验证）

### 初始状态
- 用户未登录，访问 Landing 页面 `http://localhost:3000`

### 目的
验证主题切换与语言切换控件在 Landing 页面和 Dashboard 页面中均有清晰可见的固定入口。

### 测试操作流程
1. 打开 Landing 页面，定位主题/语言控件
2. 记录其位置（右上角、Navbar 内等）
3. 登录并进入 Dashboard
4. 定位主题/语言控件位置（顶部 Header 右侧或侧边栏底部）
5. 确认两个页面的控件均可被发现，且位置直觉合理

### 预期视觉效果
- **Landing 页面**：控件位于顶部 Navbar 右侧，「Sign In」按钮左侧，两者之间有明确间距（最小 `8px`）
- **Dashboard 页面**：控件位于顶部 Header 右侧（用户头像左侧）或 Sidemenu 底部固定区域，与导航按钮物理隔离
- 两个页面的控件外观风格一致（相同的圆角、边框、图标）

### 验证工具
```javascript
// 检查控件位置
const theme = document.querySelector('[aria-label*="mode"], [class*="ThemeToggle"], [class*="theme-toggle"]');
const lang = document.querySelector('[class*="LanguageSwitcher"], [class*="lang-switch"], [class*="locale"]');

if (theme) {
  const rect = theme.getBoundingClientRect();
  console.log('Theme toggle position:', { top: rect.top, right: rect.right, left: rect.left });
}
if (lang) {
  const rect = lang.getBoundingClientRect();
  console.log('Language switcher position:', { top: rect.top, right: rect.right, left: rect.left });
}
```

---

## 场景 2：Landing 页面 — 控件与「Sign In」按钮无重叠（回归验证 Ticket #2）

### 初始状态
- 用户未登录
- Landing 页面已完全加载
- 桌面端（宽度 ≥ 1024px）

### 目的
验证主题/语言切换控件与「Sign In / 登录」按钮在 Landing Navbar 中无视觉重叠，均可独立点击。

> ⚠️ **回归验证**: 此场景对应已知 Bug — 控件与登录按钮重叠。

### 测试操作流程
1. 打开 Landing 页面
2. 观察 Navbar 右上角区域
3. 确认可同时看到：语言切换 + 主题切换 + 「Sign In」按钮
4. 依次点击三者，确认各自响应正确
5. 缩小浏览器窗口到 1280px 宽，确认仍无重叠
6. 缩小到 1024px，验证布局是否优雅折叠或保持可用

### 预期视觉效果

**元素排列顺序（从左到右）**：
```
[Logo] ··· [导航链接] ··· [语言切换] [主题切换] [Sign In 按钮]
```

**间距要求**：
- 语言切换 ↔ 主题切换：`4px ~ 8px`
- 主题切换 ↔ Sign In 按钮：`12px ~ 16px`
- Sign In 按钮右侧距 Navbar 边缘：`≥ 16px`

**禁止出现**：
- ❌ 控件叠在 Sign In 按钮上方（z-index 遮挡）
- ❌ 控件与 Sign In 按钮位置互换
- ❌ 控件溢出 Navbar 可视范围

### 验证工具
```javascript
// 检查重叠
function checkOverlap(el1, el2) {
  if (!el1 || !el2) return false;
  const r1 = el1.getBoundingClientRect();
  const r2 = el2.getBoundingClientRect();
  return !(r2.left > r1.right || r2.right < r1.left ||
           r2.top > r1.bottom || r2.bottom < r1.top);
}

const signInBtn = document.querySelector('a[href*="login"], button[class*="signin"], a[class*="login"]');
const themeToggle = document.querySelector('[aria-label*="mode"], [class*="ThemeToggle"]');
const langToggle = document.querySelector('[class*="LanguageSwitcher"], [class*="locale"]');

console.log('SignIn & Theme overlap:', checkOverlap(signInBtn, themeToggle));  // Should be: false
console.log('SignIn & Lang overlap:', checkOverlap(signInBtn, langToggle));    // Should be: false
console.log('Theme & Lang overlap:', checkOverlap(themeToggle, langToggle));   // Should be: false

// 检查各元素 pointer-events（确认可点击性）
[signInBtn, themeToggle, langToggle].forEach((el, i) => {
  if (el) {
    const styles = getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    console.log(`Element ${i}:`, {
      pointerEvents: styles.pointerEvents,
      left: rect.left,
      right: rect.right,
      top: rect.top,
    });
  }
});
```

---

## 场景 3：Dashboard — 控件与 Sidemenu 导航按钮无重叠（回归验证 Ticket #2）

### 初始状态
- 用户已登录，进入 Dashboard（`/dashboard`）
- 侧边栏（Sidemenu）处于展开状态

### 目的
验证主题/语言切换控件在 Dashboard 中不与侧边栏导航按钮重叠，布局清晰、合理。

> ⚠️ **回归验证**: 此场景对应已知 Bug — 控件与 Sidemenu 按钮重叠。

### 测试操作流程
1. 登录后进入 `/dashboard`
2. 观察侧边栏（左侧）区域
3. 观察顶部 Header 区域
4. 确认主题/语言控件位于 Sidemenu **外部**（顶部 Header 或 Sidemenu 底部独立区域）
5. 点击各侧边栏导航项（Dashboard、Tenants、Users 等），确认导航正常且控件不干扰点击区域
6. 同时点击主题/语言控件，确认正常响应

### 预期视觉效果

**方案 A（推荐）— 控件位于顶部 Header**：
```
[ Sidemenu ] | [页面标题] ··· [语言切换] [主题切换] [用户头像/菜单]
```

**方案 B — 控件位于 Sidemenu 底部**：
```
[ Nav Item 1 ]
[ Nav Item 2 ]
[ Nav Item N ]
─── 分割线 ───
[ 语言切换 ] [ 主题切换 ]
```

**禁止出现**：
- ❌ 控件叠在 Sidemenu 导航按钮区域
- ❌ 控件使用 `position: fixed` 且坐标与 Sidemenu 重叠
- ❌ 点击 Sidemenu 导航时意外触发控件

### 验证工具
```javascript
// Dashboard 中检查侧边栏与控件的位置关系
const sidebar = document.querySelector('nav, aside, [class*="sidebar"], [class*="Sidemenu"]');
const themeToggle = document.querySelector('[aria-label*="mode"], [class*="ThemeToggle"]');
const langToggle = document.querySelector('[class*="LanguageSwitcher"]');

if (sidebar) {
  const sidebarRect = sidebar.getBoundingClientRect();
  console.log('Sidebar bounds:', { right: sidebarRect.right, bottom: sidebarRect.bottom });

  [themeToggle, langToggle].forEach((el, i) => {
    if (el) {
      const rect = el.getBoundingClientRect();
      const isInsideSidebar = rect.left < sidebarRect.right && rect.top < sidebarRect.bottom;
      console.log(`Control ${i} inside sidebar zone:`, isInsideSidebar); // Should be: false (if Header) or isolated (if Sidemenu bottom)
    }
  });
}
```

---

## 场景 4：响应式布局 — 控件在各断点下的位置合理性

### 初始状态
- 分别测试：Landing 页面、Dashboard 页面
- 用 Chrome DevTools 响应式模式模拟以下宽度：
  - 桌面：1920px、1440px、1280px
  - 笔记本：1024px
  - 平板：768px

### 目的
验证主题/语言控件在不同宽度下始终有合理的布局，不溢出、不遮挡其他功能入口。

### 测试操作流程
1. 在 Chrome DevTools 中开启响应式模式（Ctrl+Shift+M）
2. 依次设置宽度：`1920 → 1440 → 1280 → 1024 → 768`
3. 在每个断点观察：Landing Navbar 中控件与 Sign In 按钮的关系
4. 切换到 Dashboard，在相同断点观察控件位置
5. 在 768px（平板/移动端）下，确认控件有可访问的折叠方案（如汉堡菜单内、悬浮按钮等）

### 预期视觉效果
- **1920px ~ 1280px**：控件与相邻元素有明确间距，无重叠
- **1024px**：控件仍可见，可能尺寸略小，但不消失也不重叠
- **768px 以下**：允许控件折叠到菜单内，但必须有明确的可发现入口

### 验证工具
```javascript
// 检查当前视口宽度下的控件位置
function auditControlsAtBreakpoint() {
  const width = window.innerWidth;
  const controls = document.querySelectorAll('[aria-label*="mode"], [class*="ThemeToggle"], [class*="LanguageSwitcher"], [class*="lang-switch"]');
  console.log(`\n=== Viewport Width: ${width}px ===`);
  controls.forEach((el, i) => {
    const rect = el.getBoundingClientRect();
    const styles = getComputedStyle(el);
    console.log(`Control ${i + 1}:`, {
      visible: styles.display !== 'none' && styles.visibility !== 'hidden',
      left: rect.left, right: rect.right,
      width: rect.width, height: rect.height,
    });
  });
}
auditControlsAtBreakpoint();
```

---

## 场景 5：控件功能在各页面上下文中均正常工作

### 初始状态
- 分别访问 Landing 页面（未登录）和 Dashboard（已登录）

### 目的
验证无论控件位于何处（Navbar / Header / Sidemenu 底部），其**功能本身**均正常工作，切换后状态持久化。

### 测试操作流程
1. 在 Landing 页面切换主题（Light → Dark）
2. 登录并进入 Dashboard，确认主题保持（持久化）
3. 在 Dashboard 切换语言
4. 导航到其他 Dashboard 页面，确认语言设置保持
5. 登出后回到 Landing 页面，确认语言和主题均保持上次设置

### 预期视觉效果
- 主题切换后，`localStorage.getItem('auth9-theme')` 为对应值
- 语言切换后，`document.cookie` 包含 `auth9_locale=...`
- 页面内导航（不刷新）和刷新后均保持设置
- Landing → Dashboard 跨页面跳转后，设置不丢失

### 验证工具
```javascript
// 检查持久化状态
console.log('Theme:', localStorage.getItem('auth9-theme'));
console.log('Locale cookie:', document.cookie.split(';').find(c => c.includes('auth9_locale')));
console.log('HTML lang:', document.documentElement.lang);
console.log('Data theme:', document.documentElement.getAttribute('data-theme'));
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 全局控件入口可见性（两个页面） | ☐ | | | |
| 2 | Landing 页 — 控件与 Sign In 按钮无重叠（回归 Ticket #2）| ☐ | | | **已知 Bug 回归项，必测** |
| 3 | Dashboard — 控件与 Sidemenu 无重叠（回归 Ticket #2）| ☐ | | | **已知 Bug 回归项，必测** |
| 4 | 响应式布局各断点合理性 | ☐ | | | 重点测试 1024px 和 768px |
| 5 | 控件功能在各上下文中正常工作 | ☐ | | | 包含跨页面持久化验证 |

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：Landing Navbar 全局截图 + Dashboard Header/Sidemenu 全局截图
2. **场景 2**：Landing Navbar 右上角控件区特写（证明无重叠）
3. **场景 3**：Dashboard Sidemenu + 控件区特写（证明无重叠）
4. **场景 4**：各断点全页截图（标注宽度）
5. **场景 5**：切换前后 localStorage/cookie 截图
