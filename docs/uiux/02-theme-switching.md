# UI/UX 测试 - 主题切换

**模块**: 视觉设计
**测试范围**: 明暗模式切换、主题持久化、过渡效果
**场景数**: 4

---

## 主题系统概述

### 主题切换机制
- 位置：右上角固定按钮
- 状态保存：`localStorage` (`auth9-theme`)
- 实现方式：`data-theme` 属性切换
- 过渡动画：`0.4s ease`

### 主题 Token 对比
| Token | Light | Dark |
|-------|-------|------|
| `--bg-primary` | `#F2F2F7` | `#000000` |
| `--bg-secondary` | `#FFFFFF` | `#1C1C1E` |
| `--glass-bg` | `rgba(255,255,255,0.72)` | `rgba(44,44,46,0.65)` |
| `--text-primary` | `#1D1D1F` | `#FFFFFF` |
| `--text-secondary` | `#86868B` | `#98989D` |

---

## 场景 1：主题切换按钮功能

### 初始状态
- 用户已登录 Dashboard
- 系统默认主题（根据 localStorage 或系统偏好）

### 目的
验证主题切换按钮正确工作，切换流畅

### 测试操作流程
1. 定位右上角主题切换按钮（太阳/月亮图标）
2. 点击切换按钮
3. 观察页面颜色变化
4. 再次点击，切换回原主题
5. 刷新页面，验证主题持久化

### 预期视觉效果
**切换按钮外观**：
- 位置：`fixed top-6 right-6`（右上角）
- 容器：玻璃效果背景，圆角 12px
- 两个图标按钮：SunIcon（Light）和 MoonIcon（Dark）
- 激活状态：蓝色淡背景 + 蓝色图标
- 非激活状态：透明背景 + 灰色图标

**切换过渡效果**：
- 背景色平滑过渡（0.4s ease）
- 文字颜色平滑过渡（0.3s ease）
- 玻璃背景和边框同步变化
- 无闪烁或跳跃

**持久化**：
- 刷新页面后保持用户选择的主题
- 新标签页打开时继承主题设置

### 验证工具
```javascript
// 检查当前主题
const theme = document.documentElement.getAttribute('data-theme');
console.log('Current theme:', theme || 'light');

// 检查 localStorage
console.log('Stored theme:', localStorage.getItem('auth9-theme'));

// 验证按钮状态
const lightBtn = document.querySelector('button[aria-label="Light mode"]');
const darkBtn = document.querySelector('button[aria-label="Dark mode"]');
console.log('Light button active:', lightBtn.classList.contains('active'));
console.log('Dark button active:', darkBtn.classList.contains('active'));

// 验证过渡时间
const body = document.body;
const transition = getComputedStyle(body).transition;
console.log('Body transition:', transition);
// 预期包含: background 0.4s ease
```

---

## 场景 2：明暗模式下所有组件适配

### 初始状态
- Light 模式

### 目的
验证切换到 Dark 模式后，所有 UI 组件正确适配

### 测试操作流程
1. 在 Light 模式下浏览多个页面：
   - Dashboard (`/dashboard`)
   - Tenants (`/dashboard/tenants`)
   - Users (`/dashboard/users`)
   - Settings (`/dashboard/settings`)
2. 切换到 Dark 模式
3. 重新浏览相同页面
4. 对比所有组件的视觉呈现

### 预期视觉效果

#### Dashboard 统计卡片
**Light 模式**：
- 玻璃背景：白色半透明
- 边框：浅灰色
- 文字：深色 `#1D1D1F`
- 图标底色：彩色淡背景（蓝色、绿色、橙色、紫色）

**Dark 模式**：
- 玻璃背景：深灰半透明 `rgba(44, 44, 46, 0.65)`
- 边框：浅色 `rgba(255, 255, 255, 0.1)`
- 文字：白色 `#FFFFFF`
- 图标底色：彩色更深的背景（提高对比度）

#### 侧边栏导航
**Light 模式**：
- 背景：`rgba(255, 255, 255, 0.7)`
- 边框：`rgba(0, 0, 0, 0.08)`
- 悬停：`rgba(0, 0, 0, 0.04)`

**Dark 模式**：
- 背景：`rgba(28, 28, 30, 0.75)`
- 边框：`rgba(255, 255, 255, 0.08)`
- 悬停：`rgba(255, 255, 255, 0.06)`

#### 表单输入框
**Light 模式**：
- 背景：白色 `#FFFFFF`
- 边框：`#E5E5EA`
- Placeholder：`#AEAEB2`

**Dark 模式**：
- 背景：深灰 `#1C1C1E`
- 边框：`rgba(255, 255, 255, 0.1)`
- Placeholder：`#636366`

#### Badge 状态
**颜色不变**：
- Success: `#34C759`（绿色）
- Warning: `#FF9500`（橙色）
- Danger: `#FF3B30`（红色）
- Info: `#32ADE6`（青色）

**背景适配**：
- Light: `rgba(颜色, 0.12)`
- Dark: `rgba(颜色, 0.2)`（更浓）

### 验证工具
```javascript
// 对比两种模式下的关键 Token
function compareThemes() {
  const tokens = [
    '--bg-primary',
    '--glass-bg',
    '--text-primary',
    '--sidebar-bg',
  ];
  
  tokens.forEach(token => {
    const value = getComputedStyle(document.documentElement)
      .getPropertyValue(token).trim();
    console.log(`${token}: ${value}`);
  });
}

// Light 模式下运行
console.log('=== Light Mode ===');
compareThemes();

// 切换到 Dark 模式后运行
console.log('=== Dark Mode ===');
compareThemes();
```

---

## 场景 3：动态背景渐变适配

### 初始状态
- Dashboard 页面

### 目的
验证页面背景的动画渐变在明暗模式下都美观

### 测试操作流程
1. 观察页面背景渐变（默认模式）
2. 等待 10 秒，观察渐变动画
3. 切换主题
4. 再次观察背景渐变
5. 验证动画持续性和流畅性

### 预期视觉效果

#### Light 模式渐变
**颜色组成**：
- 桃色：`rgba(255, 204, 128, 0.3)`
- 粉色：`rgba(255, 182, 193, 0.25)`
- 天蓝：`rgba(173, 216, 230, 0.3)`
- 薰衣草：`rgba(221, 160, 221, 0.25)`
- 杏色：`rgba(255, 218, 185, 0.3)`

**视觉特征**：
- 柔和、温暖的色调
- 透明度 25-30%
- 与白色玻璃卡片和谐搭配

#### Dark 模式渐变
**颜色组成**：
- 靛蓝：`rgba(99, 102, 241, 0.15)`
- 紫色：`rgba(168, 85, 247, 0.12)`
- 青绿：`rgba(6, 182, 212, 0.1)`
- 粉红：`rgba(236, 72, 153, 0.08)`
- 天青：`rgba(34, 211, 238, 0.12)`

**视觉特征**：
- 冷色调、科技感
- 透明度 8-15%（更低）
- 与深色玻璃卡片形成对比

#### 动画效果
- 时长：20 秒
- 缓动：`ease-in-out`
- 循环：`infinite alternate`
- 变换：`translate(5%, 5%) scale(1.1)`
- 主题切换时动画不中断

### 验证工具
```javascript
// 检查背景渐变
const backdrop = document.querySelector('.page-backdrop');
if (backdrop) {
  const before = getComputedStyle(backdrop, '::before');
  console.log('Background gradient:', before.background);
  console.log('Animation:', before.animation);
  // 预期: backdrop-shift 20s ease-in-out infinite alternate
}

// 验证动画关键帧
console.log('Animation state:', backdrop.getAnimations()[0]);
```

---

## 场景 4：主题切换无障碍性

### 初始状态
- Dashboard 页面

### 目的
验证主题切换符合无障碍要求，支持键盘操作和屏幕阅读器

### 测试操作流程
1. 使用 Tab 键导航到主题切换按钮
2. 验证焦点状态可见
3. 使用 Enter 或 Space 切换主题
4. 使用屏幕阅读器（如 NVDA、VoiceOver）
5. 验证按钮的 ARIA 标签

### 预期视觉效果

#### 键盘导航
- Tab 键可以聚焦到主题切换按钮
- 焦点状态：蓝色外轮廓（`outline: 2px solid #007AFF, outline-offset: 2px`）
- Enter 或 Space 可以触发切换

#### ARIA 属性
```html
<button
  aria-label="Switch to dark mode"
  role="button"
  tabindex="0"
>
  <MoonIcon aria-hidden="true" />
</button>
```

#### 屏幕阅读器
- Light 模式按钮：读作 "Switch to light mode, button"
- Dark 模式按钮：读作 "Switch to dark mode, button"
- 激活状态有明确反馈："Light mode active" / "Dark mode active"

### 验证工具
```javascript
// 检查 ARIA 属性
const themeToggle = document.querySelector('[aria-label*="mode"]');
console.log('ARIA label:', themeToggle.getAttribute('aria-label'));
console.log('Role:', themeToggle.getAttribute('role'));
console.log('Tabindex:', themeToggle.getAttribute('tabindex'));

// 验证焦点样式
themeToggle.focus();
const focusStyles = getComputedStyle(themeToggle, ':focus-visible');
console.log('Focus outline:', focusStyles.outline);
console.log('Outline offset:', focusStyles.outlineOffset);
```

**Lighthouse 可访问性测试**：
```
1. 打开 Chrome DevTools
2. 进入 Lighthouse 面板
3. 选择 "Accessibility" 类别
4. 运行审计
5. 验证主题切换按钮通过所有 a11y 检查
```

---

## 常见问题排查

### 问题 1：切换后部分元素颜色未更新
**可能原因**：
- 使用了硬编码颜色而非 CSS 变量
- 过渡动画未正确配置

**排查**：
```javascript
// 查找硬编码颜色
const elements = document.querySelectorAll('*');
elements.forEach(el => {
  const styles = getComputedStyle(el);
  if (styles.color === 'rgb(0, 0, 0)' || styles.color === '#000000') {
    console.warn('Hardcoded color found:', el);
  }
});
```

### 问题 2：主题未持久化
**排查**：
```javascript
// 检查 localStorage 写入
console.log('Stored theme:', localStorage.getItem('auth9-theme'));

// 手动设置
localStorage.setItem('auth9-theme', 'dark');
location.reload();
```

### 问题 3：动画在 prefers-reduced-motion 下未禁用
**排查**：
```javascript
// 检查用户偏好
const prefersReducedMotion = window.matchMedia(
  '(prefers-reduced-motion: reduce)'
).matches;
console.log('Prefers reduced motion:', prefersReducedMotion);

// 验证动画是否禁用
if (prefersReducedMotion) {
  const backdrop = document.querySelector('.page-backdrop');
  const animation = getComputedStyle(backdrop, '::before').animation;
  console.log('Animation (should be minimal):', animation);
}
```

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：主题切换按钮特写 + 切换过程（3 帧）
2. **场景 2**：同一页面明暗模式对比（并排或上下）
3. **场景 3**：背景渐变特写（Light vs Dark）
4. **场景 4**：焦点状态截图 + 屏幕阅读器输出
