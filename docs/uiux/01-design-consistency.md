# UI/UX 测试 - 设计一致性

**模块**: 视觉设计
**测试范围**: Liquid Glass 设计系统一致性、玻璃质感、颜色系统
**场景数**: 5

---

## 设计系统参考

### Liquid Glass 核心特征
- 半透明背景（`backdrop-filter: blur(24px)`）
- 玻璃边框和高光效果
- 统一圆角（卡片 20px，侧边栏 24px）
- 双层阴影（外阴影 + 内高光）
- 渐变光照效果

---

## 场景 1：Dashboard 页面玻璃质感验证

### 初始状态
- 用户已登录 Auth9 Portal
- 访问主 Dashboard 页面 (`/dashboard`)

### 目的
验证 Dashboard 统计卡片应用了正确的 Liquid Glass 效果

### 测试操作流程
1. 访问 `/dashboard`
2. 观察 4 个统计卡片（Total Tenants, Active Users, Services, Events）
3. 打开 Chrome DevTools (F12)
4. 选中任一统计卡片元素
5. 检查 Computed 样式

### 预期视觉效果
**卡片外观**：
- 半透明白色背景（Light）或深灰背景（Dark）
- 能够透视到背景渐变色
- 边框为亮色细线（1px）
- 顶部有微弱的内发光高光
- 圆角 20px，柔和的外阴影

**悬停效果**：
- 鼠标悬停时卡片向上浮动 2px
- 背景透明度略微增加
- 阴影扩大，强度增强

### 验证工具
```javascript
// DevTools Console
const card = document.querySelector('.stat-card');
const styles = getComputedStyle(card);

// 验证 backdrop-filter
console.log(styles.backdropFilter); 
// 预期: blur(24px) saturate(180%)

// 验证背景色
console.log(styles.backgroundColor); 
// Light: rgba(255, 255, 255, 0.72)
// Dark: rgba(44, 44, 46, 0.65)

// 验证圆角
console.log(styles.borderRadius); 
// 预期: 20px

// 验证阴影
console.log(styles.boxShadow); 
// 应包含 8px 和 32px 阴影
```

---

## 场景 2：侧边栏导航玻璃效果

### 初始状态
- 已登录 Dashboard
- 侧边栏可见（**桌面端，视窗宽度 ≥ 1024px**）

### 目的
验证侧边栏应用了更强的模糊效果（40px blur）

### 测试操作流程
1. **确保浏览器宽度 ≥ 1024px**（侧边栏在桌面端为浮动卡片样式，移动端为全宽抽屉）
2. 检查左侧导航栏
3. 观察背景透视效果
4. 检查导航项的悬停和激活状态

### 预期视觉效果
**侧边栏容器**（桌面端 ≥ 1024px）：
- `backdrop-filter: blur(40px) saturate(180%)`（比卡片更强）
- 圆角 24px（比卡片更圆润）；**移动端 / 平板端（< 1024px）为 0px，因全宽抽屉布局**
- 底部右侧有圆角处理
- 与页面主背景有清晰视觉分离

**导航项状态**：
- 默认：无背景，文字灰色
- Hover：浅色背景（`rgba(0, 0, 0, 0.04)` Light / `rgba(255, 255, 255, 0.06)` Dark）
- Active：蓝色淡背景（`--accent-blue-light`），文字蓝色（`#007AFF`）

### 验证工具
```javascript
const sidebar = document.querySelector('.sidebar');
const styles = getComputedStyle(sidebar);

console.log(styles.backdropFilter); 
// 预期: blur(40px) saturate(180%)

console.log(styles.borderRadius);
// 预期: 24px（仅桌面端 ≥ 1024px；移动端为 0px）

// 检查激活项样式
const activeItem = document.querySelector('.sidebar-item.active');
const activeStyles = getComputedStyle(activeItem);
console.log(activeStyles.color); 
// 预期: rgb(0, 122, 255) (#007AFF)
```

---

## 场景 3：颜色系统 Token 一致性

### 初始状态
- Dashboard 任意页面

### 目的
验证所有页面使用统一的 CSS 变量颜色系统

### 测试操作流程
1. 打开 DevTools Console
2. 运行 CSS 变量验证脚本
3. 检查明暗模式切换前后的 Token 值

### 预期视觉效果
**Light 模式 Token**：
- `--bg-primary`: `#F2F2F7`
- `--glass-bg`: `rgba(255, 255, 255, 0.72)`
- `--text-primary`: `#1D1D1F`
- `--accent-blue`: `#007AFF`

**Dark 模式 Token**：
- `--bg-primary`: `#000000`
- `--glass-bg`: `rgba(44, 44, 46, 0.65)`
- `--text-primary`: `#FFFFFF`
- `--accent-blue`: `#007AFF`（不变）

### 验证工具
```javascript
const root = document.documentElement;
const getVar = (name) => getComputedStyle(root).getPropertyValue(name).trim();

// Light 模式验证
console.log('Light Mode Colors:');
console.log('--bg-primary:', getVar('--bg-primary'));
console.log('--glass-bg:', getVar('--glass-bg'));
console.log('--text-primary:', getVar('--text-primary'));
console.log('--accent-blue:', getVar('--accent-blue'));

// 切换到 Dark 模式后重新运行
// document.documentElement.setAttribute('data-theme', 'dark');
```

---

## 场景 4：按钮变体一致性

### 初始状态
- 任何包含多种按钮的页面（如创建租户表单）

### 目的
验证按钮变体（default, secondary, outline, glass, destructive）视觉样式正确

### 测试操作流程
1. 访问 `/dashboard/tenants` 并点击「创建租户」
2. 观察弹窗中的按钮样式
3. 测试悬停效果
4. 检查圆角和内边距

### 预期视觉效果
**按钮变体**：
| 变体 | 背景 | 文字 | 边框 | 用途 |
|------|------|------|------|------|
| `default` | 蓝色 `#007AFF` | 白色 | 无 | 主操作 |
| `secondary` | 玻璃效果 | 深色/白色 | 有 | 次要操作 |
| `outline` | 透明 | 蓝色 | 蓝色 | 三级操作 |
| `glass` | 完整玻璃效果 | 深色/白色 | 玻璃边框 | 特殊强调 |
| `destructive` | 红色 `#FF3B30` | 白色 | 无 | 危险操作 |

**统一规范**：
- 圆角：`12px`
- 内边距：`px-4 py-2`（16px 水平，8px 垂直）
- 悬停：向上浮动 1px（主按钮）或背景变化（次要按钮）

### 验证工具
```javascript
// 检查主按钮
const primaryBtn = document.querySelector('button[type="submit"]');
const styles = getComputedStyle(primaryBtn);
console.log('Background:', styles.backgroundColor); // rgb(0, 122, 255)
console.log('Border radius:', styles.borderRadius); // 12px
console.log('Padding:', styles.padding); // 8px 16px
```

---

## 场景 5：卡片组件一致性

### 初始状态
- 访问包含多张卡片的页面（如 Services 列表、Users 列表）

### 目的
验证所有卡片组件应用了统一的 Liquid Glass 样式

### 测试操作流程
1. 访问 `/dashboard/services`
2. 观察服务列表卡片
3. 访问 `/dashboard/users`
4. 观察用户列表卡片
5. 对比样式一致性

### 预期视觉效果
**卡片统一规范**：
- 背景：`var(--glass-bg)` 玻璃效果
- 圆角：`20px`
- 内边距：`20px`（CardHeader 和 CardContent）
- 边框：`1px solid var(--glass-border)`
- 阴影：`0 8px 32px var(--glass-shadow)` + 内高光
- 光照：左上角 135° 渐变叠加

**CardHeader**：
- 标题字体：16-17px，字重 600
- 描述字体：13px，颜色 `--text-secondary`
- 底部边框：`var(--glass-border-subtle)`

**CardContent**：
- 内容区域无额外背景
- 与 Header 间距：默认无（由内容决定）

### 验证工具
```javascript
// 检查多个卡片一致性
const cards = document.querySelectorAll('.liquid-glass');
cards.forEach((card, index) => {
  const styles = getComputedStyle(card);
  console.log(`Card ${index + 1}:`);
  console.log('  Border radius:', styles.borderRadius);
  console.log('  Backdrop filter:', styles.backdropFilter);
  console.log('  Box shadow:', styles.boxShadow);
});

// 验证一致性
const uniqueRadii = new Set([...cards].map(c => getComputedStyle(c).borderRadius));
console.log('Unique border-radius values:', uniqueRadii);
// 预期: 只有一个值 "20px"
```

---

## 常见问题排查

### 问题 1：玻璃效果不显示
**可能原因**：
- 浏览器不支持 `backdrop-filter`
- CSS 变量未正确加载
- 父元素没有背景内容

**排查**：
```javascript
// 检查浏览器支持
console.log('Supports backdrop-filter:', 
  CSS.supports('backdrop-filter', 'blur(24px)'));

// 检查 fallback
const card = document.querySelector('.liquid-glass');
if (!CSS.supports('backdrop-filter', 'blur(24px)')) {
  console.log('Fallback background:', 
    getComputedStyle(card).backgroundColor);
  // 应为 var(--bg-secondary) 纯色背景
}
```

### 问题 2：颜色对比度不足
**排查**：
- 使用 Chrome DevTools Lighthouse 运行可访问性审计
- 检查文字颜色是否符合 WCAG AA 标准（4.5:1）
- 验证半透明背景下的实际对比度

### 问题 3：动画卡顿
**排查**：
- 检查是否使用了 `transform` 和 `opacity`（GPU 加速）
- 避免动画 `backdrop-filter`（性能差）
- 使用 Performance 面板分析帧率

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：Dashboard 统计卡片全屏 + 单卡片特写
2. **场景 2**：侧边栏全高 + 激活项特写
3. **场景 3**：明暗模式对比（并排）
4. **场景 4**：按钮变体组合（表单弹窗）
5. **场景 5**：不同页面的卡片布局（Services vs Users）
