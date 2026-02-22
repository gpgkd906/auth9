# UI/UX 测试 - 响应式布局

**模块**: 交互体验
**测试范围**: 响应式设计、移动端适配、触摸交互、断点行为
**场景数**: 4

---

## 响应式断点系统

### Tailwind 断点
| 断点 | 最小宽度 | 设备类型 | 布局特征 |
|------|---------|---------|---------|
| `sm` | 640px | 大屏手机 | 单列 → 双列 |
| `md` | 768px | 平板竖屏 | 侧边栏折叠 |
| `lg` | 1024px | 平板横屏 / 小笔记本 | 侧边栏展开 |
| `xl` | 1280px | 桌面 | 完整布局 |
| `2xl` | 1536px | 大屏 | 宽松布局 |

### 测试设备矩阵
| 设备 | 分辨率 | 断点 | 关键测试点 |
|------|--------|------|-----------|
| iPhone SE | 375×667 | < sm | 移动端导航、单列布局 |
| iPhone 14 Pro | 393×852 | < sm | 刘海适配、安全区 |
| iPad Mini | 768×1024 | md | 侧边栏折叠、双列 |
| iPad Pro | 1024×1366 | lg | 侧边栏展开、三列 |
| Laptop | 1366×768 | lg/xl | 完整桌面体验 |
| Desktop | 1920×1080 | xl/2xl | 宽屏适配 |

---

## 场景 1：侧边栏响应式行为

### 初始状态
- 桌面端登录 Dashboard

### 目的
验证侧边栏在不同屏幕尺寸下的折叠/展开行为

### 测试操作流程
1. 在桌面端（> 1024px）打开 Dashboard
2. 逐步缩小浏览器窗口宽度：
   - 1920px → 1024px（断点 lg）
   - 1024px → 768px（断点 md，侧边栏应折叠）
   - 768px → 375px（断点 sm，移动端布局）
3. 在移动端测试汉堡菜单功能
4. 展开/收起侧边栏，测试动画

### 预期视觉效果

#### 桌面端（>= 1024px）
- 侧边栏固定在左侧
- 宽度：240px（`w-60`）
- 圆角：右下角 24px
- 导航项完整显示（图标 + 文字）
- 无汉堡菜单按钮

#### 平板端（768px - 1023px）
- 侧边栏默认折叠（隐藏或仅显示图标）
- 主内容区域占满屏幕
- 左上角出现汉堡菜单按钮（≡）
- 点击后侧边栏从左侧滑入（overlay）

#### 移动端（< 768px）
- 侧边栏通过 `transform: translateX(-100%)` 隐藏（**非** `display: none`，以支持滑入动画）
- 汉堡菜单按钮必须可见
- 点击后全屏侧边栏（`width: 100vw`，覆盖整个视口）
- 背景遮罩（半透明黑色，`bg-black/50`）
- 点击遮罩关闭侧边栏

#### 动画效果
- 侧边栏滑入：`transform: translateX(-100%)` → `translateX(0)`
- 时长：300ms
- 缓动：`cubic-bezier(0.4, 0, 0.2, 1)`
- 遮罩淡入：`opacity: 0` → `opacity: 1`

### 验证工具

#### Chrome DevTools 响应式模式
```
1. 按 Ctrl+Shift+M 进入响应式模式
2. 选择设备预设或自定义尺寸
3. 测试断点:
   - 1024px (lg) - 侧边栏应展开
   - 1023px (< lg) - 侧边栏应折叠
   - 768px (md) - 汉堡菜单出现
   - 767px (< md) - 移动端布局
```

#### JavaScript 检查

> **注意**：侧边栏在移动端/平板端使用 `transform: translateX(-100%)` 隐藏（非 `display: none`），以支持平滑滑入动画。验证隐藏状态时应检查 `transform` 属性，而非 `display` 或 `visibility`。

```javascript
// 检查侧边栏状态
const sidebar = document.querySelector('.sidebar');
const sidebarStyles = getComputedStyle(sidebar);
const windowWidth = window.innerWidth;

console.log(`Window width: ${windowWidth}px`);
console.log('Sidebar transform:', sidebarStyles.transform);
console.log('Sidebar width:', sidebarStyles.width);

// 检查断点行为
if (windowWidth >= 1024) {
  // 桌面端: transform 应为 none 或 translateX(0)
  console.log('Desktop mode: Sidebar should be visible (transform: none)');
} else {
  // 移动端/平板端: 默认隐藏 (translateX(-100%))，打开时 translateX(0)
  const isHidden = sidebarStyles.transform.includes('matrix') &&
    sidebarStyles.transform !== 'none';
  console.log('Mobile/Tablet mode: Sidebar hidden via transform:', isHidden);
  if (windowWidth < 768) {
    console.log('Mobile: Sidebar should be full-width (100vw) when opened');
  }
}
```

---

## 场景 2：卡片网格响应式布局

### 初始状态
- Dashboard 或 Services 列表页

### 目的
验证卡片网格在不同屏幕自适应列数

### 测试操作流程
1. 访问 `/dashboard`（统计卡片）或 `/dashboard/services`
2. 在不同断点测试网格列数：
   - Desktop (>= 1024px)：4 列
   - Tablet (768px - 1023px)：2 列
   - Mobile (< 768px)：1 列
3. 检查卡片宽度和间距适配

### 预期视觉效果

#### 统计卡片网格
**Desktop (>= 1024px)**：
```css
grid-template-columns: repeat(4, 1fr);
gap: 16px;
```
- 4 列等宽
- 每张卡片约 23% 宽度（扣除间距）

**Tablet (768px - 1023px)**：
```css
grid-template-columns: repeat(2, 1fr);
gap: 16px;
```
- 2 列等宽
- 每张卡片约 48% 宽度

**Mobile (< 768px)**：
```css
grid-template-columns: 1fr;
gap: 12px;
```
- 单列布局
- 卡片占满容器宽度
- 间距缩小为 12px

#### 服务列表卡片
**响应式类名**：
```html
<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
  <!-- 卡片 -->
</div>
```

- Mobile: 1 列
- Tablet: 2 列
- Desktop: 3 列

### 验证工具
```javascript
// 检查网格列数
function checkGridColumns() {
  const grid = document.querySelector('.grid');
  if (!grid) return;
  
  const styles = getComputedStyle(grid);
  const columns = styles.gridTemplateColumns.split(' ').length;
  const gap = styles.gap;
  const width = window.innerWidth;
  
  console.log(`Screen width: ${width}px`);
  console.log(`Grid columns: ${columns}`);
  console.log(`Grid gap: ${gap}`);
  
  // 验证预期
  if (width >= 1024 && columns !== 4) {
    console.warn('Expected 4 columns on desktop!');
  } else if (width >= 768 && width < 1024 && columns !== 2) {
    console.warn('Expected 2 columns on tablet!');
  } else if (width < 768 && columns !== 1) {
    console.warn('Expected 1 column on mobile!');
  }
}

// 监听窗口大小变化
window.addEventListener('resize', checkGridColumns);
checkGridColumns();
```

---

## 场景 3：表格响应式处理

### 初始状态
- Users 或 Tenants 列表页

### 目的
验证数据表格在小屏幕上的可用性（横向滚动或卡片式）

### 测试操作流程
1. 访问 `/dashboard/users`
2. 在桌面端观察表格（完整显示）
3. 缩小到移动端
4. 检查表格处理方式：
   - 横向滚动（带滚动条）
   - 或转为卡片式列表

### 预期视觉效果

#### 桌面端（>= 768px）
- 标准表格布局
- 列宽自适应
- 所有列可见
- 无横向滚动

#### 移动端（< 768px）

**方案 A：横向滚动**：
```html
<div class="overflow-x-auto">
  <table class="min-w-full">
    <!-- 表格内容 -->
  </table>
</div>
```
- 容器可横向滚动
- 表格最小宽度 600px（`min-w-full` 或 `min-w-[600px]`）
- 滚动条清晰可见
- 列宽不压缩

**方案 B：卡片式列表**（推荐）：
```html
<!-- 桌面: 表格 -->
<table class="hidden md:table">...</table>

<!-- 移动: 卡片 -->
<div class="md:hidden space-y-4">
  <div class="card">
    <div class="font-semibold">John Doe</div>
    <div class="text-sm text-gray-500">john@example.com</div>
    <div><Badge>Active</Badge></div>
  </div>
</div>
```
- 每行数据转为卡片
- 垂直堆叠，间距 16px
- 关键信息突出显示
- 操作按钮底部对齐

### 验证工具
```javascript
// 检查表格处理方式
const table = document.querySelector('table');
const cardList = document.querySelector('.md\\:hidden');

if (window.innerWidth < 768) {
  if (table && getComputedStyle(table).display === 'none') {
    console.log('Mobile: Using card layout ✓');
    console.log('Cards visible:', cardList && getComputedStyle(cardList).display !== 'none');
  } else {
    console.log('Mobile: Using scrollable table');
    const container = table.closest('.overflow-x-auto');
    console.log('Scrollable container:', !!container);
  }
} else {
  console.log('Desktop: Using standard table');
}
```

---

## 场景 4：触摸交互优化

### 初始状态
- 使用真实移动设备或 Chrome DevTools 触摸模拟

### 目的
验证移动端触摸目标大小合适，交互流畅

### 测试操作流程
1. 在 Chrome DevTools 启用触摸模拟：
   - Ctrl+Shift+M → Device Mode
   - 勾选 "Toggle device toolbar"
2. 测试交互元素：
   - 按钮（应至少 44×44px）
   - 链接（应有足够点击区域）
   - 下拉菜单（触摸时展开，不悬停）
   - 表单输入框（聚焦时放大）
3. 测试滑动手势（侧边栏、下拉刷新）

### 预期视觉效果

#### 触摸目标尺寸
**按钮**：
- 最小高度：44px（`h-11` 或 `min-h-[44px]`）
- 最小宽度：44px（如图标按钮）
- 内边距：充足（`px-4 py-2` 至少）

**链接**：
- 行高：44px 或内边距扩大点击区域
- 相邻链接间距：至少 8px

**表单元素**：
- 输入框高度：44px（`h-11`）
- 下拉选择器：48px（`h-12`，稍大）
- 复选框/单选框：24×24px（`w-6 h-6`）

#### 触摸反馈
**按钮点击**：
- 触摸时显示涟漪效果或颜色变化
- 无延迟（`touch-action: manipulation`）
- 按下时缩小（`active:scale-95`）

**链接点击**：
- 触摸时背景高亮
- 长按显示系统菜单（正常行为）

#### 移动端专属优化
**输入框聚焦**：
- 移动端自动缩放（`user-scalable=yes`）
- 虚拟键盘推开内容（不遮挡）
- 焦点元素滚动到可见区域

**侧边栏**：
- 支持滑动手势打开/关闭（可选）
- 轻扫关闭（swipe to close）

### 验证工具

#### 触摸目标尺寸检查
```javascript
// 检查按钮尺寸
function checkTouchTargets() {
  const buttons = document.querySelectorAll('button, a[role="button"]');
  
  buttons.forEach((btn, index) => {
    const rect = btn.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;
    
    if (width < 44 || height < 44) {
      console.warn(`Button ${index + 1} too small:`, {
        width: Math.round(width),
        height: Math.round(height),
        text: btn.textContent.trim(),
      });
    }
  });
  
  console.log(`Checked ${buttons.length} touch targets`);
}

// 移动端执行
if (window.innerWidth < 768) {
  checkTouchTargets();
}
```

#### Lighthouse 移动端审计
```
1. 打开 DevTools Lighthouse
2. 选择 "Mobile" 模式
3. 勾选 "Performance" 和 "Accessibility"
4. 运行审计
5. 检查:
   - Tap targets are not sized appropriately (应通过)
   - Content is sized correctly for the viewport (应通过)
```

#### Viewport Meta 标签验证
```javascript
// 检查 viewport 配置
const viewport = document.querySelector('meta[name="viewport"]');
console.log('Viewport content:', viewport?.getAttribute('content'));
// 预期: width=device-width, initial-scale=1
```

---

## 常见问题排查

### 问题 1：断点不生效
**可能原因**：
- Tailwind 类名拼写错误（如 `md:gird-cols-2` → `md:grid-cols-2`）
- CSS 未编译/重新构建
- 浏览器缓存

**排查**：
```javascript
// 检查元素计算样式
const element = document.querySelector('.grid');
const styles = getComputedStyle(element);
console.log('Display:', styles.display);
console.log('Grid template columns:', styles.gridTemplateColumns);
```

### 问题 2：移动端字体太小
**排查**：
```javascript
// 检查最小字号
const body = document.body;
const fontSize = getComputedStyle(body).fontSize;
console.log('Body font size:', fontSize);
// 移动端应至少 14px
```

### 问题 3：触摸延迟 300ms
**解决方案**：
```css
/* 添加到全局样式 */
button, a, input, select {
  touch-action: manipulation;
}
```

### 问题 4：横向滚动溢出
**排查**：
```javascript
// 查找溢出元素
const elements = document.querySelectorAll('*');
const bodyWidth = document.body.clientWidth;

elements.forEach(el => {
  if (el.scrollWidth > bodyWidth) {
    console.warn('Element overflows:', el, {
      scrollWidth: el.scrollWidth,
      viewportWidth: bodyWidth,
    });
  }
});
```

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：侧边栏在 4 种尺寸下的状态（Desktop, Tablet, Mobile opened, Mobile closed）
2. **场景 2**：统计卡片网格在 3 种断点下的布局（4 列、2 列、1 列）
3. **场景 3**：表格在移动端的处理（横向滚动 vs 卡片式）
4. **场景 4**：触摸目标尺寸标注（用尺规或高亮）+ Lighthouse 审计结果
