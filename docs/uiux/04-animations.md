# UI/UX 测试 - 动画与过渡效果

**模块**: 交互体验
**测试范围**: 动画流畅度、过渡效果、性能优化、用户偏好
**场景数**: 5

---

## 动画系统概述

### 动画时长标准
| 类型 | 时长 | 缓动函数 | 用途 |
|------|------|----------|------|
| 微交互 | 150ms | `ease-out` | 按钮点击、下拉展开 |
| 悬停效果 | 300ms | `cubic-bezier(0.4,0,0.2,1)` | 卡片悬停、链接高亮 |
| 主题切换 | 400ms | `ease` | 明暗模式切换 |
| 页面进入 | 500ms | `cubic-bezier(0.34,1.56,0.64,1)` | fadeInUp 动画 |
| 背景动画 | 20s | `ease-in-out` | 背景渐变循环 |

### 性能优化原则
- 优先使用 `transform` 和 `opacity`（GPU 加速）
- 避免动画 `backdrop-filter`、`box-shadow`（性能差）
- 使用 `will-change` 提示浏览器（谨慎使用）
- 支持 `prefers-reduced-motion`（无障碍）

---

## 场景 1：卡片悬停动画流畅度

### 初始状态
- 访问 Dashboard 页面
- 鼠标悬停在统计卡片上

### 目的
验证卡片悬停动画流畅，无卡顿

### 测试操作流程
1. 访问 `/dashboard`
2. 将鼠标缓慢移入统计卡片
3. 观察卡片向上浮动和阴影增强
4. 快速移入移出，测试动画响应
5. 打开 DevTools Performance 面板录制

### 预期视觉效果

#### 悬停变化
**位置变化**：
- 向上平移：`transform: translateY(-2px) !important`
- 时长：300ms
- 缓动：`cubic-bezier(0.4, 0, 0.2, 1)`

> **注意**：统计卡片使用 `fadeInUp` 动画（`animation-fill-mode: forwards`），`transform` 属性被动画锁定。悬停位移通过 `.liquid-glass:hover` 的 `transform: translateY(-2px) !important` 强制覆盖动画锁定值。验证时应检查 `getComputedStyle(card).transform`。

**背景变化**：
- 透明度增加：`--glass-bg` → `--glass-bg-hover`
- Light: `0.72` → `0.85`
- Dark: `0.65` → `0.75`

**阴影变化**：
- 扩散半径：`8px 32px` → `12px 40px`
- 颜色强度：`--glass-shadow` → `--glass-shadow-strong`

**流畅度要求**：
- 60 FPS（无掉帧）
- 移入移出对称（同样的时长和缓动）
- 快速交互不抖动

### 验证工具

#### CSS 检查
```javascript
const card = document.querySelector('.stat-card');
const styles = getComputedStyle(card);

console.log('Transition:', styles.transition);
// 预期: all 0.3s cubic-bezier(0.4, 0, 0.2, 1)

// 悬停后检查 transform 属性
// .liquid-glass:hover 使用 !important 覆盖 fadeInUp forwards 锁定值
card.dispatchEvent(new MouseEvent('mouseenter'));
setTimeout(() => {
  console.log('transform:', getComputedStyle(card).transform);
  // 预期: matrix(1, 0, 0, 1, 0, -2) — 即 translateY(-2px)
}, 400);
```

#### 常见误报排查
| 现象 | 原因 | 解决 |
|------|------|------|
| `transform` 悬停前后不变 | hover 样式未加载或被其他样式覆盖 | 确认 `.liquid-glass:hover` 的 `!important` 生效 |
| `transform` 为 `matrix(1,0,0,1,0,0)`（非悬停态） | 正常——`fadeInUp` 动画结束后 transform 为 `translateY(0)` | 确保在悬停状态下检查 |

#### Performance 分析
```
1. 打开 DevTools Performance 面板
2. 点击 Record
3. 悬停卡片 3-5 次
4. 停止录制
5. 检查 FPS 曲线（应保持 60 FPS）
6. 检查 Main 线程（应无长任务）
```

**预期结果**：
- FPS 曲线平稳在 60
- 无紫色「Long Task」标记
- GPU 内存使用稳定

---

## 场景 2：页面进入动画（fadeInUp）

### 初始状态
- 任意页面

### 目的
验证页面元素依次淡入并上移，错开时间优雅

### 测试操作流程
1. 访问 `/dashboard`（首次加载或刷新）
2. 观察元素出现顺序：
   - 主标题
   - 统计卡片（从左到右）
   - 内容卡片（从上到下）
3. 导航到其他页面，重复观察

### 预期视觉效果

#### 动画参数
**关键帧**：
```css
@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

**应用**：
- 时长：500ms
- 缓动：`cubic-bezier(0.34, 1.56, 0.64, 1)`（弹性）
- 填充模式：`forwards`（保持结束状态）

#### 错开延迟（Stagger）
- 统计卡片 1：0ms（`delay-1`）
- 统计卡片 2：50ms（`delay-2`）
- 统计卡片 3：100ms（`delay-3`）
- 统计卡片 4：150ms（`delay-4`）
- 内容卡片：200ms、250ms、300ms

**总时长**：约 800ms（最后一个元素完成）

#### 视觉效果
- 元素从下方 20px 处滑入
- 透明度从 0 渐变到 1
- 整体感觉：流畅、有序、不突兀

### 验证工具
```javascript
// 检查动画应用
const animatedElements = document.querySelectorAll('.animate-fade-in-up');
animatedElements.forEach((el, index) => {
  const styles = getComputedStyle(el);
  console.log(`Element ${index + 1}:`);
  console.log('  Animation:', styles.animation);
  console.log('  Animation delay:', styles.animationDelay);
});

// 预期输出示例:
// Element 1: fadeInUp 0.5s cubic-bezier(...) 0s forwards
// Element 2: fadeInUp 0.5s cubic-bezier(...) 0.05s forwards
// Element 3: fadeInUp 0.5s cubic-bezier(...) 0.1s forwards
```

---

## 场景 3：背景渐变动画性能

### 初始状态
- Dashboard 或任意页面

### 目的
验证背景渐变动画流畅，不影响主线程性能

### 测试操作流程
1. 访问 `/dashboard`
2. 保持页面打开 30 秒
3. 观察背景渐变缓慢移动
4. 同时进行其他操作（点击、滚动、输入）
5. 使用 Performance Monitor 监控

### 预期视觉效果

#### 动画效果
**关键帧**：
```css
@keyframes backdrop-shift {
  0% { transform: translate(0, 0) scale(1); }
  100% { transform: translate(5%, 5%) scale(1.1); }
}
```

**应用**：
- 时长：20s
- 缓动：`ease-in-out`
- 循环：`infinite alternate`（往返循环）
- 目标：`.page-backdrop::before` 伪元素

#### 性能要求
- 背景动画不影响前景交互
- FPS 保持 60（即使有其他操作）
- CPU 占用低（< 10%）
- GPU 内存稳定

### 验证工具

#### Performance Monitor
```
1. 打开 DevTools Command Menu (Ctrl+Shift+P)
2. 输入 "Performance Monitor"
3. 观察指标:
   - CPU usage: 应 < 10%
   - JS heap size: 稳定
   - DOM Nodes: 不增长
   - Layouts/sec: < 1
```

#### Animation Inspector
```javascript
// 获取动画对象
const backdrop = document.querySelector('.page-backdrop');
const animations = backdrop.getAnimations({ subtree: true });

animations.forEach(anim => {
  console.log('Animation:', anim.animationName);
  console.log('Duration:', anim.effect.getTiming().duration);
  console.log('Iterations:', anim.effect.getTiming().iterations);
  console.log('Current time:', anim.currentTime);
});

// 预期: backdrop-shift, 20000ms, Infinity, [变化中]
```

---

## 场景 4：主题切换过渡效果

### 初始状态
- 任意页面，Light 或 Dark 模式

### 目的
验证主题切换时颜色平滑过渡，无闪烁

### 测试操作流程
1. 点击右上角主题切换按钮
2. 观察页面颜色变化
3. 连续快速切换 3-5 次
4. 检查是否有元素延迟或闪烁

### 预期视觉效果

#### 过渡配置
**全局过渡**：
```css
body {
  transition: background 0.4s ease, color 0.3s ease;
}

.liquid-glass {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
```

**Token 变化**：
- 背景色：0.4s 平滑过渡
- 文字色：0.3s 平滑过渡
- 边框、阴影：0.3s 平滑过渡

#### 视觉效果
- 无任何元素瞬间跳变
- 颜色渐变自然（非线性）
- 所有元素同步变化（无延迟差）
- 背景渐变色平滑切换

#### 问题检测
**不应出现**：
- 白色闪屏
- 部分元素延迟 1-2 帧
- 文字颜色跳变
- 背景色先变，边框后变

### 验证工具
```javascript
// 检查过渡配置
const body = document.body;
const bodyTransition = getComputedStyle(body).transition;
console.log('Body transition:', bodyTransition);

const card = document.querySelector('.liquid-glass');
const cardTransition = getComputedStyle(card).transition;
console.log('Card transition:', cardTransition);

// 监控主题切换
document.documentElement.addEventListener('transitionend', (e) => {
  console.log('Transition ended:', e.propertyName, e.target);
});
```

---

## 场景 5：Reduced Motion 支持

### 初始状态
- 启用系统「减少动画」设置
  - Windows: 设置 → 轻松使用 → 显示 → 显示动画
  - macOS: 系统偏好设置 → 辅助功能 → 显示 → 减少动态效果
  - Linux: GNOME 设置 → 通用访问 → 减少动画

### 目的
验证启用「减少动画」后，动画最小化或禁用

### 测试操作流程
1. 启用系统「减少动画」设置
2. 刷新页面
3. 观察页面加载（不应有淡入动画）
4. 悬停卡片（不应有浮动动画）
5. 切换主题（应瞬间切换）

### 预期视觉效果

#### 禁用的动画
- ❌ fadeInUp 进入动画
- ❌ 卡片悬停浮动
- ❌ 背景渐变移动
- ❌ 按钮点击弹性

#### 保留的过渡
- ✅ 主题切换颜色过渡（极短，< 50ms）
- ✅ 下拉菜单展开（极短）
- ✅ 焦点状态变化（即时）

#### 实现方式
```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 验证工具
```javascript
// 检查用户偏好
const prefersReducedMotion = window.matchMedia(
  '(prefers-reduced-motion: reduce)'
).matches;

console.log('Prefers reduced motion:', prefersReducedMotion);

if (prefersReducedMotion) {
  // 验证动画被禁用
  const card = document.querySelector('.stat-card');
  const styles = getComputedStyle(card);
  const transitionDuration = styles.transitionDuration;
  console.log('Transition duration:', transitionDuration);
  // 预期: 0.01ms 或 0s
  
  const animations = card.getAnimations();
  console.log('Active animations:', animations.length);
  // 预期: 0（无动画）
}
```

---

## 常见问题排查

### 问题 1：动画卡顿
**可能原因**：
- 使用了非 GPU 加速属性（如 `top`, `left`, `width`）
- `backdrop-filter` 被动画化
- 浏览器 compositing layer 过多

**排查**：
```javascript
// 检查是否使用了 GPU 加速
const card = document.querySelector('.stat-card');
console.log('Will change:', getComputedStyle(card).willChange);
console.log('Transform:', getComputedStyle(card).transform);

// 使用 DevTools Layers 面板查看合成层
```

### 问题 2：动画不触发
**排查**：
```javascript
// 检查动画定义
const element = document.querySelector('.animate-fade-in-up');
const animations = element.getAnimations();
console.log('Animations:', animations);

if (animations.length === 0) {
  console.warn('No animations found! Check CSS rules.');
}
```

### 问题 3：过渡闪烁
**可能原因**：
- 多个 `transition` 属性冲突
- 初始值未设置（从 `auto` 过渡）

**排查**：
```javascript
const element = document.querySelector('.element');
console.log('Transition:', getComputedStyle(element).transition);
// 检查是否有 "all" 和具体属性混用
```

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：卡片悬停前后对比 + Performance 面板 FPS 曲线
2. **场景 2**：页面加载序列帧（0ms, 200ms, 500ms）
3. **场景 3**：Performance Monitor 截图 + 背景动画播放中
4. **场景 4**：主题切换中间帧（灰色过渡状态）
5. **场景 5**：Reduced motion 对比（启用前后）
