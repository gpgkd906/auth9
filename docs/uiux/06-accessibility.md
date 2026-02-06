# UI/UX 测试 - 可访问性 (Accessibility)

**模块**: 可访问性
**测试范围**: WCAG 2.1 AA 合规、键盘导航、屏幕阅读器、颜色对比度
**场景数**: 5

---

## 可访问性标准

### WCAG 2.1 Level AA 要求
| 类别 | 标准 | Auth9 目标 |
|------|------|-----------|
| 颜色对比度 | 正文 4.5:1, 大文本 3:1 | 7:1+ (主文本) |
| 键盘导航 | 所有功能可键盘操作 | 100% 支持 |
| 焦点可见性 | 焦点指示明确 | 2px 蓝色轮廓 |
| ARIA 标签 | 语义化 HTML + ARIA | 完整支持 |
| 表单标签 | 所有输入有关联标签 | 100% 关联 |

### 辅助技术支持
- **屏幕阅读器**: NVDA (Windows), JAWS, VoiceOver (macOS/iOS), TalkBack (Android)
- **键盘导航**: Tab, Shift+Tab, Enter, Space, Arrow keys
- **浏览器**: Chrome, Firefox, Safari (最新版本)

---

## 场景 1：键盘导航完整性

### 初始状态
- Dashboard 页面
- 鼠标不可用（仅键盘）

### 目的
验证所有交互元素可通过键盘访问和操作

### 测试操作流程
1. 刷新页面，Tab 键开始导航
2. 按 Tab 键遍历所有可交互元素：
   - 主题切换按钮
   - 侧边栏导航项
   - 统计卡片（如可点击）
   - 表格操作按钮
   - 表单输入框
3. 验证焦点顺序逻辑
4. 使用 Enter/Space 激活按钮和链接
5. 使用 Shift+Tab 反向导航

### 预期视觉效果

#### 焦点指示器
**样式规范**：
```css
:focus-visible {
  outline: 2px solid var(--accent-blue); /* #007AFF */
  outline-offset: 2px;
  border-radius: inherit; /* 继承元素圆角 */
}
```

**表现**：
- 蓝色外轮廓清晰可见
- 与元素边缘有 2px 间距
- 圆角与元素一致
- 不被其他元素遮挡

#### 焦点顺序
**逻辑顺序**（从上到下，从左到右）：
1. 主题切换按钮（右上角）
2. 侧边栏导航项（按显示顺序）
3. 主内容区域：
   - 页面标题
   - 统计卡片（第 1-4 张）
   - 内容卡片内的交互元素
   - 表格行操作按钮
4. 分页控件

#### 键盘操作
| 元素 | 操作 | 效果 |
|------|------|------|
| 按钮 | Enter / Space | 触发点击 |
| 链接 | Enter | 导航 |
| 下拉菜单 | Enter 打开, Arrow keys 选择, Enter 确认 | 选择选项 |
| 对话框 | Esc | 关闭 |
| 复选框 | Space | 切换选中 |

### 验证工具

#### 键盘导航测试
```
1. 不使用鼠标，仅用键盘完成任务:
   - 从 Dashboard 导航到 Users
   - 创建新租户（打开弹窗，填写表单，提交）
   - 关闭弹窗（Esc 键）
2. 记录遇到的无法键盘访问的元素
```

#### JavaScript 检查
```javascript
// 检查 tabindex
function checkTabIndex() {
  const interactiveElements = document.querySelectorAll(
    'button, a, input, select, textarea, [tabindex]'
  );
  
  interactiveElements.forEach((el, index) => {
    const tabIndex = el.getAttribute('tabindex');
    if (tabIndex && parseInt(tabIndex) < 0 && tabIndex !== '-1') {
      console.warn(`Element ${index + 1} has unusual tabindex:`, tabIndex, el);
    }
  });
  
  console.log(`Checked ${interactiveElements.length} interactive elements`);
}

checkTabIndex();
```

#### 焦点陷阱检测
```javascript
// 检查对话框是否有焦点陷阱
function checkFocusTrap(dialog) {
  const focusableElements = dialog.querySelectorAll(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  );
  
  console.log('Focusable elements in dialog:', focusableElements.length);
  
  if (focusableElements.length === 0) {
    console.error('Dialog has no focusable elements!');
  }
  
  // 验证焦点循环
  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];
  console.log('First focusable:', firstElement);
  console.log('Last focusable:', lastElement);
}
```

---

## 场景 2：颜色对比度验证

### 初始状态
- Dashboard 任意页面
- 分别测试 Light 和 Dark 模式

### 目的
验证所有文本和 UI 元素符合 WCAG AA 对比度要求

### 测试操作流程
1. 使用 Chrome DevTools 或在线工具测量对比度
2. 检查关键文本颜色组合：
   - 主文本 vs 玻璃背景
   - 次要文本 vs 背景
   - 链接 vs 背景
   - 按钮文本 vs 按钮背景
   - Placeholder vs 输入框背景
3. 切换到 Dark 模式重复测试

### 预期视觉效果

#### Light 模式对比度
| 文本类型 | 前景色 | 背景色 | 对比度 | 标准 |
|---------|-------|--------|--------|------|
| 主文本 | `#1D1D1F` | `rgba(255,255,255,0.72)` | 7:1+ | ✓ AA |
| 次要文本 | `#86868B` | `rgba(255,255,255,0.72)` | 4.5:1+ | ✓ AA |
| 链接 | `#007AFF` | `rgba(255,255,255,0.72)` | 4.5:1+ | ✓ AA |
| 按钮文本 | `#FFFFFF` | `#007AFF` | 7:1+ | ✓ AAA |
| Placeholder | `#AEAEB2` | `#FFFFFF` | 3:1+ | ✓ AA (大文本) |

#### Dark 模式对比度
| 文本类型 | 前景色 | 背景色 | 对比度 | 标准 |
|---------|-------|--------|--------|------|
| 主文本 | `#FFFFFF` | `rgba(44,44,46,0.65)` | 7:1+ | ✓ AA |
| 次要文本 | `#98989D` | `rgba(44,44,46,0.65)` | 4.5:1+ | ✓ AA |
| 链接 | `#007AFF` | `rgba(44,44,46,0.65)` | 4.5:1+ | ✓ AA |

### 验证工具

#### Chrome DevTools 对比度检查
```
1. 右键点击文本元素 → 检查
2. Styles 面板中找到 color 属性
3. 点击颜色方块，查看对比度信息
4. DevTools 会显示:
   - 对比度数值 (如 7.12)
   - AA / AAA 通过状态（✓ 或 ✗）
```

#### 在线工具
- **WebAIM Contrast Checker**: https://webaim.org/resources/contrastchecker/
- **Contrast Ratio**: https://contrast-ratio.com/

#### JavaScript 检查
```javascript
// 获取元素的实际对比度
function getContrastRatio(element) {
  const styles = getComputedStyle(element);
  const fgColor = styles.color;
  const bgColor = styles.backgroundColor;
  
  console.log('Foreground:', fgColor);
  console.log('Background:', bgColor);
  
  // 注意: 需要递归获取实际背景色（如背景透明）
  // 可使用第三方库如 'color' 或 Chrome DevTools API
}

const mainText = document.querySelector('p');
getContrastRatio(mainText);
```

#### Lighthouse 可访问性审计
```
1. DevTools Lighthouse 面板
2. 选择 "Accessibility" 类别
3. 运行审计
4. 检查 "Background and foreground colors have a sufficient contrast ratio"
5. 查看未通过的元素列表
```

---

## 场景 3：屏幕阅读器兼容性

### 初始状态
- 启动屏幕阅读器（NVDA / VoiceOver / TalkBack）
- Dashboard 页面

### 目的
验证页面结构清晰，所有内容可被屏幕阅读器正确朗读

### 测试操作流程

#### Windows (NVDA)
```
1. 启动 NVDA (Ctrl+Alt+N)
2. 访问 Auth9 Portal
3. 使用快捷键导航:
   - H: 下一个标题
   - B: 下一个按钮
   - K: 下一个链接
   - F: 下一个表单字段
4. 听取元素描述和状态
```

#### macOS (VoiceOver)
```
1. 启动 VoiceOver (Cmd+F5)
2. 使用 VoiceOver 键 (VO) + 方向键导航
3. VO+A: 读取整个页面
4. VO+右箭头: 下一个元素
5. VO+Shift+下箭头: 进入容器
```

### 预期语音输出

#### 页面标题
```
"Auth9 Dashboard - Dashboard, heading level 1"
```

#### 统计卡片
```
"Total Tenants, 12, heading level 2"
"Active Users, 1,847, +12% from last month"
```

#### 导航链接
```
"Dashboard, link, current page"
"Users, link"
"Tenants, link"
```

#### 按钮
```
"Create Tenant, button"
"Delete, button, danger action"
```

#### 表单字段
```
"Email, edit, required"
"Password, password, edit, required"
"Remember me, checkbox, not checked"
```

#### ARIA 标签要求

**语义化 HTML**：
```html
<h1>Dashboard</h1>
<nav aria-label="Main navigation">
  <a href="/dashboard" aria-current="page">Dashboard</a>
</nav>
<main>
  <section aria-label="Statistics">
    <!-- 统计卡片 -->
  </section>
</main>
```

**ARIA 属性**：
```html
<!-- 按钮 -->
<button aria-label="Switch to dark mode">
  <MoonIcon aria-hidden="true" />
</button>

<!-- 对话框 -->
<div role="dialog" aria-labelledby="dialog-title" aria-modal="true">
  <h2 id="dialog-title">Create Tenant</h2>
</div>

<!-- 状态 Badge -->
<span role="status" aria-live="polite">Active</span>

<!-- 装饰性图标 -->
<svg aria-hidden="true">...</svg>
```

### 验证工具

#### ARIA Validator
```javascript
// 检查必需的 ARIA 属性
function validateAria() {
  // 对话框
  const dialogs = document.querySelectorAll('[role="dialog"]');
  dialogs.forEach(dialog => {
    const hasLabel = dialog.hasAttribute('aria-labelledby') || 
                     dialog.hasAttribute('aria-label');
    const isModal = dialog.hasAttribute('aria-modal');
    console.log('Dialog ARIA:', { hasLabel, isModal });
  });
  
  // 按钮
  const iconButtons = document.querySelectorAll('button:not(:has(span, div))');
  iconButtons.forEach(btn => {
    const hasLabel = btn.hasAttribute('aria-label') || btn.textContent.trim();
    if (!hasLabel) {
      console.warn('Icon button without label:', btn);
    }
  });
}

validateAria();
```

---

## 场景 4：表单可访问性

### 初始状态
- 创建租户表单

### 目的
验证表单字段有清晰标签，错误提示可被辅助技术识别

### 测试操作流程
1. 打开创建租户表单
2. 使用 Tab 键导航到各字段
3. 检查每个字段的标签关联
4. 提交空表单，触发验证错误
5. 验证错误提示可被屏幕阅读器读取

### 预期视觉效果

#### 表单字段标签
```html
<div class="form-group">
  <label for="tenant-name">
    Tenant Name
    <span class="text-red-500" aria-label="required">*</span>
  </label>
  <input
    id="tenant-name"
    name="name"
    type="text"
    required
    aria-required="true"
    aria-describedby="name-help"
  />
  <small id="name-help" class="text-gray-500">
    A friendly name for your tenant
  </small>
</div>
```

**关键点**：
- `<label for="id">` 与 `<input id="id">` 关联
- 必填字段有 `required` 和 `aria-required="true"`
- 帮助文本用 `aria-describedby` 关联
- 必填星号有 `aria-label="required"`

#### 错误提示
```html
<input
  id="tenant-name"
  aria-invalid="true"
  aria-errormessage="name-error"
/>
<div id="name-error" role="alert" class="text-red-500">
  Tenant name is required
</div>
```

**关键点**：
- 错误状态：`aria-invalid="true"`
- 错误信息：`aria-errormessage` 关联
- 错误容器：`role="alert"` 自动朗读

### 验证工具

#### 标签关联检查
```javascript
function checkFormLabels() {
  const inputs = document.querySelectorAll('input, select, textarea');
  
  inputs.forEach(input => {
    const id = input.id;
    const label = document.querySelector(`label[for="${id}"]`);
    const ariaLabel = input.getAttribute('aria-label');
    const ariaLabelledBy = input.getAttribute('aria-labelledby');
    
    const hasLabel = label || ariaLabel || ariaLabelledBy;
    
    if (!hasLabel) {
      console.warn('Input without label:', input);
    } else {
      console.log('Input has label:', input.name || input.id);
    }
  });
}

checkFormLabels();
```

---

## 场景 5：跳过链接和地标

### 初始状态
- Dashboard 页面

### 目的
验证提供「跳过导航」链接，使用 ARIA 地标

### 测试操作流程
1. 刷新页面
2. 按 Tab 键（第一个焦点应是「跳过导航」链接）
3. 按 Enter 跳转到主内容
4. 使用屏幕阅读器的地标导航（R 键，NVDA）

### 预期视觉效果

#### 跳过链接
```html
<a href="#main-content" class="skip-link">
  Skip to main content
</a>
```

**样式**（默认隐藏，聚焦时显示）：
```css
.skip-link {
  position: absolute;
  top: -40px;
  left: 0;
  background: var(--accent-blue);
  color: white;
  padding: 8px 16px;
  text-decoration: none;
  z-index: 100;
}

.skip-link:focus {
  top: 0;
}
```

#### ARIA 地标
```html
<body>
  <a href="#main-content" class="skip-link">Skip to main content</a>
  
  <aside aria-label="Sidebar navigation">
    <nav aria-label="Main navigation">
      <!-- 导航链接 -->
    </nav>
  </aside>
  
  <main id="main-content" tabindex="-1">
    <h1>Dashboard</h1>
    <!-- 主内容 -->
  </main>
  
  <footer>
    <!-- 页脚 -->
  </footer>
</body>
```

**地标类型**：
- `<header>` 或 `role="banner"`: 页面头部
- `<nav>` 或 `role="navigation"`: 导航区域
- `<main>` 或 `role="main"`: 主内容（唯一）
- `<aside>` 或 `role="complementary"`: 侧边栏
- `<footer>` 或 `role="contentinfo"`: 页脚

### 验证工具

#### 地标检查
```javascript
function checkLandmarks() {
  const landmarks = [
    { selector: 'header, [role="banner"]', name: 'Banner' },
    { selector: 'nav, [role="navigation"]', name: 'Navigation' },
    { selector: 'main, [role="main"]', name: 'Main' },
    { selector: 'aside, [role="complementary"]', name: 'Complementary' },
    { selector: 'footer, [role="contentinfo"]', name: 'Content Info' },
  ];
  
  landmarks.forEach(({ selector, name }) => {
    const elements = document.querySelectorAll(selector);
    console.log(`${name}:`, elements.length);
    if (elements.length === 0) {
      console.warn(`No ${name} landmark found`);
    }
  });
}

checkLandmarks();
```

---

## 常见问题排查

### 问题 1：焦点指示器不可见
**排查**：
```javascript
const button = document.querySelector('button');
button.focus();
const outline = getComputedStyle(button, ':focus-visible').outline;
console.log('Focus outline:', outline);
// 应为 "2px solid rgb(0, 122, 255)"
```

### 问题 2：表单字段无标签
**自动修复**：
- 为每个 `<input>` 添加 `id`
- 创建 `<label for="id">`
- 或添加 `aria-label` 属性

### 问题 3：图标按钮无描述
**修复**：
```html
<!-- 错误 -->
<button><Icon /></button>

<!-- 正确 -->
<button aria-label="Delete user">
  <Icon aria-hidden="true" />
</button>
```

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：焦点指示器示例（多个元素）+ Tab 顺序标注
2. **场景 2**：对比度检查工具截图（DevTools + Lighthouse）
3. **场景 3**：屏幕阅读器界面（NVDA/VoiceOver）+ 语音输出文本
4. **场景 4**：表单错误状态 + ARIA 属性标注
5. **场景 5**：跳过链接聚焦状态 + 地标结构图
