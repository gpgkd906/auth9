# UI/UX 测试 - Landing 页面交互元素

**模块**: 页面专项
**测试范围**: Landing 页面功能卡片悬停效果、动画交互、CTA 按钮可用性
**场景数**: 4
**关联 Ticket**: `docs/ticket/ui_landing-card-hover_scenario1_20260307_162702.md`

---

## 背景说明

Landing 页面（`/`）是产品的第一印象，包含：
- 顶部 Hero 区域（标题、描述、CTA 按钮）
- 功能介绍卡片区域（Feature Cards）
- 顶部 Navbar（Logo、导航链接、控件区、登录按钮）

所有交互元素必须在鼠标悬停时呈现明确、流畅的视觉反馈，不应出现元素消失或不可见状态。

---

## 场景 1：功能卡片入口可见性（入口可见性基准）

### 初始状态
- 用户未登录
- 访问 Landing 页面（`http://localhost:3000` 或 `http://localhost:5173`）

### 目的
验证页面加载完成后，所有功能介绍卡片在初始状态下完全可见，无需任何交互即可呈现。

### 测试操作流程
1. 打开 Landing 页面
2. 等待页面完全加载（所有动画稳定）
3. 逐一观察每个功能卡片（Feature Card）
4. 不移动鼠标，确认全部卡片处于可见状态

### 预期视觉效果
- 页面加载后所有卡片立即可见
- 卡片包含：图标、标题、描述文字
- 卡片应用玻璃质感样式：`backdrop-filter: blur(...)`, `background: var(--glass-bg)`
- 卡片 `opacity` 为 `1`，`visibility` 为 `visible`，`display` 不为 `none`

### 验证工具
```javascript
// 检查所有卡片的初始可见状态
const cards = document.querySelectorAll('.feature-card, [class*="card"], [class*="feature"]');
console.log(`Found ${cards.length} cards`);

cards.forEach((card, i) => {
  const styles = getComputedStyle(card);
  const rect = card.getBoundingClientRect();
  console.log(`Card ${i + 1}:`, {
    opacity: styles.opacity,
    visibility: styles.visibility,
    display: styles.display,
    height: rect.height,
    inViewport: rect.top < window.innerHeight,
  });
});
```

---

## 场景 2：功能卡片悬停效果（回归验证 Ticket #1）

### 初始状态
- Landing 页面已完全加载
- 至少两个功能介绍卡片在视口内可见

### 目的
验证鼠标悬停时卡片保持可见并呈现正确的悬停高亮效果，不得消失。

> ⚠️ **回归验证**: 此场景对应已知 Bug — 悬停时卡片整体消失（`opacity: 0` 或 `visibility: hidden` 被错误绑定到 `:hover`）。

### 测试操作流程
1. 将鼠标慢速移动到第一个功能卡片上
2. 保持悬停状态 3 秒，持续观察卡片是否处于可见状态
3. 移开鼠标，确认卡片恢复正常状态
4. 对剩余所有卡片重复上述操作
5. 快速扫过所有卡片（连续悬停），验证无闪烁消失

### 预期视觉效果

**悬停时（`:hover` 状态）**：
- 卡片保持完全可见，`opacity: 1`，`visibility: visible`
- 轻微上移效果：`transform: translateY(-4px)` 或类似
- 边框高亮：`border-color` 变为 `var(--accent-blue)` 或高亮色
- 阴影增强：`box-shadow` 更明显
- 背景轻微变化：玻璃效果透明度调整
- 过渡动画：`0.3s ease`，流畅无跳跃

**禁止出现**：
- ❌ 卡片整体消失（`opacity: 0`）
- ❌ 卡片不可见（`visibility: hidden`）
- ❌ 卡片从布局流中消失（`display: none`）
- ❌ 卡片内容被遮挡（父容器 `overflow: hidden` 与 `transform` 冲突）

### 验证工具
```javascript
// 模拟悬停检查（注意：完整验证需手动用鼠标操作）
const cards = document.querySelectorAll('.feature-card, [class*="card"]');

cards.forEach((card, i) => {
  // 检查 :hover CSS 规则是否存在危险属性
  const sheets = [...document.styleSheets];
  sheets.forEach(sheet => {
    try {
      [...sheet.cssRules].forEach(rule => {
        if (rule.selectorText?.includes(':hover') &&
            (rule.style?.opacity === '0' ||
             rule.style?.visibility === 'hidden' ||
             rule.style?.display === 'none')) {
          console.error('Dangerous hover rule found:', rule.cssText);
        }
      });
    } catch(e) {}
  });

  // 检查卡片的 hover 样式（手动悬停时运行）
  card.addEventListener('mouseenter', () => {
    const styles = getComputedStyle(card);
    console.log(`Card ${i + 1} on hover:`, {
      opacity: styles.opacity,         // Should be: 1
      visibility: styles.visibility,  // Should be: visible
      transform: styles.transform,     // Should be: translateY(-4px) or similar
    });
  });
});
```

---

## 场景 3：Navbar CTA 按钮与控件可点击性

### 初始状态
- Landing 页面已加载
- 桌面端（宽度 ≥ 1024px）

### 目的
验证顶部 Navbar 的所有控件（登录按钮、主题切换、语言切换）互不遮挡，均可独立点击。

### 测试操作流程
1. 定位右上角控件区域
2. 依次尝试点击：语言切换按钮、主题切换按钮、「Sign In / 登录」按钮
3. 确认每次点击响应正确，无被其他元素遮挡的情况
4. 使用 Chrome DevTools「Inspect Element」确认各元素无 z-index 遮挡

### 预期视觉效果
- 各按钮在视觉上独立、互不重叠
- 点击「Sign In」后跳转到登录页或触发登录弹窗
- 点击语言切换后语言切换生效
- 点击主题切换后主题切换生效
- 所有控件的点击区域（`pointer-events`）均正常，无穿透/遮挡

### 验证工具
```javascript
// 检查按钮元素是否相互遮挡
function checkOverlap(el1, el2) {
  const r1 = el1.getBoundingClientRect();
  const r2 = el2.getBoundingClientRect();
  return !(r2.left > r1.right || r2.right < r1.left ||
           r2.top > r1.bottom || r2.bottom < r1.top);
}

const signInBtn = document.querySelector('[href="/login"], button[class*="login"], a[class*="signin"]');
const themeToggle = document.querySelector('[aria-label*="mode"], [class*="theme"]');
const langToggle = document.querySelector('[class*="lang"], [class*="locale"]');

if (signInBtn && themeToggle) {
  console.log('SignIn & Theme overlap:', checkOverlap(signInBtn, themeToggle));
}
if (signInBtn && langToggle) {
  console.log('SignIn & Lang overlap:', checkOverlap(signInBtn, langToggle));
}
```

---

## 场景 4：Landing 页面动画与卡片动画不干扰内容可见性

### 初始状态
- Landing 页面刚刚完成加载（0~3 秒内）

### 目的
验证页面入场动画（Fade-in、Slide-up 等）完成后，所有内容正确过渡到完全可见状态，不遗留中间动画帧导致的不可见状态。

### 测试操作流程
1. 硬刷新 Landing 页面（Ctrl+Shift+R / Cmd+Shift+R）
2. 在页面加载开始的 1~3 秒内连续截图或录屏
3. 确认入场动画结束后所有卡片处于 `opacity: 1` 状态
4. 将网络条件设为 "Slow 3G"（Chrome DevTools → Network Throttling），重复验证
5. 在动画进行中将鼠标移上卡片，确认悬停效果正常叠加

### 预期视觉效果
- 入场动画（约 0.5s）结束后所有卡片 `opacity` 稳定为 `1`
- 入场动画期间（`opacity: 0 → 1`）不应响应悬停效果（可接受但非必须）
- 动画结束后（`animation-fill-mode: forwards` 或动画移除后），悬停状态正确触发

### 验证工具
```javascript
// 检查动画结束后的状态
const cards = document.querySelectorAll('.feature-card, [class*="card"]');

cards.forEach((card, i) => {
  card.addEventListener('animationend', () => {
    const styles = getComputedStyle(card);
    console.log(`Card ${i + 1} animation ended:`, {
      opacity: styles.opacity,         // Must be: 1
      animationFillMode: styles.animationFillMode,
    });
  });
});

// 立即检查当前状态
setTimeout(() => {
  cards.forEach((card, i) => {
    console.log(`Card ${i + 1} after 1s:`, getComputedStyle(card).opacity);
  });
}, 1000);
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 功能卡片入口可见性（初始状态） | ☐ | | | |
| 2 | 功能卡片悬停效果（回归 Ticket #1）| ☐ | | | **已知 Bug 回归项，必测** |
| 3 | Navbar CTA 按钮与控件可点击性 | ☐ | | | |
| 4 | 入场动画不干扰内容可见性 | ☐ | | | 慢网络条件下需重测 |

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：Landing 页面全局截图（初始状态）
2. **场景 2**：卡片悬停特写（需截图证明卡片可见 + 有高亮效果）
3. **场景 3**：Navbar 右上角控件区特写（各按钮间距清晰）
4. **场景 4**：动画进行中截图 + 动画结束后截图（对比 opacity 变化）
