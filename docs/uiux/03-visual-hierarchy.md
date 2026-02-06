# UI/UX 测试 - 视觉层级与布局

**模块**: 视觉设计
**测试范围**: 视觉层级、间距规范、信息架构、布局清晰度
**场景数**: 4

---

## 视觉层级原则

### 层级系统
1. **主标题** (28px, 700) - Dashboard 页面标题
2. **页面标题** (24px, 600) - 子页面标题
3. **卡片标题** (16-17px, 600) - 卡片头部
4. **正文** (13-14px, 400) - 主要内容
5. **辅助文字** (11-13px, 500) - 标签、说明
6. **微小文字** (11-12px, 500) - 时间戳、元数据

### 间距规范
| 用途 | 值 | Tailwind |
|------|---|----------|
| 元素内间距 | 4px | `xs` |
| 紧凑间距 | 8px | `sm` |
| 默认间距 | 12px | `md` |
| 网格间隙 | 16px | `lg` (gap-4) |
| 卡片内边距 | 20px | `xl` (p-5) |
| 页面分区 | 24px | `2xl` (space-y-6) |

---

## 场景 1：Dashboard 页面视觉层级

### 初始状态
- 用户已登录
- 访问主 Dashboard (`/dashboard`)

### 目的
验证 Dashboard 页面的信息层级清晰，视觉流合理

### 测试操作流程
1. 访问 `/dashboard`
2. 从上到下扫视页面
3. 识别主要信息区块
4. 检查标题、统计卡片、列表的优先级

### 预期视觉效果

#### 页面结构（从上到下）
1. **主标题区域**：
   - 文字："Dashboard" 或 "欢迎回来"
   - 字号：28px，字重：700
   - 颜色：`--text-primary`
   - 位置：页面左上（侧边栏右侧）

2. **统计卡片区域**：
   - 4 列网格布局（`grid-cols-4 gap-4`）
   - 每张卡片包含：
     - 图标（左上，带彩色底色）
     - 标签（小号，`--text-secondary`）
     - 数值（大号 26-28px，字重 700）
     - 变化趋势（小号，绿色或红色）
   - 视觉权重：次高（仅次于主标题）

3. **内容卡片**：
   - 列表卡片（Recent Users, Services, Events）
   - 卡片标题：16-17px，字重 600
   - 表格或列表内容：13-14px
   - 视觉权重：中等

4. **辅助信息**：
   - 时间戳、状态 Badge
   - 11-12px，灰色
   - 视觉权重：最低

#### 信息密度
- 不过度拥挤：各区块间有 24px 间距
- 卡片内部：20px 内边距
- 表格单元格：横向 16px，纵向 12px

### 验证工具
```javascript
// 检查字号层级
function checkTypography() {
  const selectors = {
    'Dashboard Title': 'h1',
    'Card Title': '.card-title',
    'Body Text': 'td, p',
    'Small Text': '.text-xs, .text-sm',
  };
  
  Object.entries(selectors).forEach(([name, selector]) => {
    const el = document.querySelector(selector);
    if (el) {
      const styles = getComputedStyle(el);
      console.log(`${name}:`);
      console.log(`  Font size: ${styles.fontSize}`);
      console.log(`  Font weight: ${styles.fontWeight}`);
      console.log(`  Color: ${styles.color}`);
    }
  });
}

checkTypography();
```

---

## 场景 2：列表页面布局清晰度

### 初始状态
- 访问 Users 或 Tenants 列表页

### 目的
验证列表页面的布局合理，信息组织清晰

### 测试操作流程
1. 访问 `/dashboard/users`
2. 观察页面布局：
   - 页面标题和操作按钮
   - 搜索/筛选区域
   - 表格/卡片列表
   - 分页控件
3. 检查表格列宽分配

### 预期视觉效果

#### 页面头部
- **标题**：24px，字重 600，左对齐
- **操作按钮**：右对齐，与标题同一行
- 间距：标题与按钮之间自动填充（`justify-between`）
- 底部边距：24px（`mb-6`）

#### 表格布局
**表头**：
- 背景：无（透明或浅色）
- 文字：11px，字重 600，大写（`uppercase`），`letter-spacing: 0.04em`
- 颜色：`--text-secondary`
- 对齐：左对齐（数字列右对齐）

**表格行**：
- 高度：适中（不拥挤）
- 悬停：背景色变化（`hover:bg-gray-50` Light / `hover:bg-gray-800/50` Dark）
- 内边距：横向 16px，纵向 12px

**列宽分配**：
- Name/Email：较宽（flex-1）
- Status：固定宽度（100-120px）
- Actions：最窄（80-100px）
- 总宽度：充满容器（`w-full`）

#### 分页控件
- 位置：表格底部，右对齐
- 组件：上一页、页码、下一页
- 间距：与表格底部 16px

### 验证工具
```javascript
// 检查列宽分配
const table = document.querySelector('table');
const headers = table.querySelectorAll('th');
headers.forEach((th, index) => {
  const width = th.offsetWidth;
  console.log(`Column ${index + 1} (${th.textContent}): ${width}px`);
});

// 验证间距
const pageTitle = document.querySelector('h1');
const titleMargin = getComputedStyle(pageTitle).marginBottom;
console.log('Title bottom margin:', titleMargin); // 应为 24px
```

---

## 场景 3：表单布局可读性

### 初始状态
- 打开创建或编辑表单（如创建租户弹窗）

### 目的
验证表单布局清晰，标签与输入框关系明确

### 测试操作流程
1. 点击「创建租户」按钮
2. 观察表单弹窗
3. 检查标签与输入框的对齐和间距
4. 检查表单验证提示的位置

### 预期视觉效果

#### 表单结构
**弹窗容器**：
- 宽度：`max-w-md`（448px）
- 背景：玻璃效果
- 圆角：20px
- 内边距：24px

**表单字段**：
- 字段间距：16px（`space-y-4`）
- 标签：
  - 位置：输入框上方
  - 字号：13px，字重 500
  - 颜色：`--text-primary`
  - 底部间距：4-6px
- 输入框：
  - 高度：40px（`h-10`）
  - 圆角：12px
  - 边框：1px solid
  - 内边距：横向 12px

**按钮组**：
- 位置：表单底部
- 对齐：右对齐（`justify-end`）
- 间距：按钮之间 12px（`gap-3`）

#### 验证状态
**错误提示**：
- 位置：输入框下方 4px
- 字号：12px
- 颜色：`--accent-red`
- 图标：感叹号（可选）

**必填标识**：
- 红色星号 "*" 在标签后
- 字号与标签相同

### 验证工具
```javascript
// 检查表单间距
const formGroups = document.querySelectorAll('.form-group');
formGroups.forEach((group, index) => {
  const label = group.querySelector('label');
  const input = group.querySelector('input');
  
  if (label && input) {
    const labelBottom = label.getBoundingClientRect().bottom;
    const inputTop = input.getBoundingClientRect().top;
    const gap = inputTop - labelBottom;
    console.log(`Form group ${index + 1} label-input gap: ${gap}px`);
  }
});

// 验证输入框高度
const inputs = document.querySelectorAll('input[type="text"]');
inputs.forEach(input => {
  console.log('Input height:', input.offsetHeight); // 应为 40px
});
```

---

## 场景 4：卡片网格布局一致性

### 初始状态
- 访问包含多卡片布局的页面（如 Services、Settings）

### 目的
验证卡片网格布局整齐，间距一致

### 测试操作流程
1. 访问 `/dashboard/services`
2. 观察服务卡片网格
3. 测量卡片间距
4. 调整浏览器窗口宽度，观察响应式行为

### 预期视觉效果

#### 网格布局
**Desktop（>= 1024px）**：
- 3 列：`grid-cols-3`
- 间隙：16px（`gap-4`）
- 卡片宽度：自动填充（`auto-fit`）

**Tablet（768px - 1023px）**：
- 2 列：`md:grid-cols-2`
- 间隙：16px

**Mobile（< 768px）**：
- 1 列：`grid-cols-1`
- 间隙：12px（可选缩小）

#### 卡片内部布局
**统一规范**：
- 内边距：20px（`p-5`）
- 图标区域（如有）：顶部，居中
- 标题：16-17px，字重 600
- 描述：13px，`--text-secondary`
- 操作按钮：底部，右对齐

**对齐**：
- 所有卡片高度对齐（`items-start`）
- 标题基线对齐
- 按钮底部对齐

### 验证工具
```javascript
// 测量卡片间距
const cards = document.querySelectorAll('.grid > *');
if (cards.length >= 2) {
  const card1 = cards[0].getBoundingClientRect();
  const card2 = cards[1].getBoundingClientRect();
  
  // 横向间距
  const horizontalGap = card2.left - card1.right;
  console.log('Horizontal gap between cards:', horizontalGap);
  
  // 纵向间距（如有第二行）
  if (cards.length >= 4) {
    const card3 = cards[3].getBoundingClientRect();
    const verticalGap = card3.top - card1.bottom;
    console.log('Vertical gap between rows:', verticalGap);
  }
}

// 检查响应式断点
function checkResponsive() {
  const grid = document.querySelector('.grid');
  const cols = getComputedStyle(grid).gridTemplateColumns.split(' ').length;
  const width = window.innerWidth;
  console.log(`Window width: ${width}px, Grid columns: ${cols}`);
}

checkResponsive();
window.addEventListener('resize', checkResponsive);
```

---

## 常见问题排查

### 问题 1：文字层级不明显
**排查**：
```javascript
// 对比不同层级的字号和字重
const elements = [
  { selector: 'h1', name: 'Main Title' },
  { selector: 'h2', name: 'Page Title' },
  { selector: '.card-title', name: 'Card Title' },
  { selector: 'p', name: 'Body' },
];

elements.forEach(({ selector, name }) => {
  const el = document.querySelector(selector);
  if (el) {
    const styles = getComputedStyle(el);
    const size = parseInt(styles.fontSize);
    const weight = parseInt(styles.fontWeight);
    console.log(`${name}: ${size}px / ${weight}`);
  }
});
```

### 问题 2：间距不一致
**排查**：
- 使用浏览器 DevTools 的「Computed」面板
- 检查 `margin` 和 `padding` 值
- 查找硬编码的像素值（应使用 Tailwind 类名）

### 问题 3：卡片高度不对齐
**排查**：
```javascript
// 检查卡片高度差异
const cards = document.querySelectorAll('.grid > .card');
const heights = [...cards].map(card => card.offsetHeight);
console.log('Card heights:', heights);
console.log('Height variance:', Math.max(...heights) - Math.min(...heights));
```

---

## 截图说明

每个场景建议截图位置：
1. **场景 1**：Dashboard 全屏 + 标注层级（用箭头和数字）
2. **场景 2**：Users 列表全屏 + 表格特写
3. **场景 3**：创建表单弹窗 + 字段间距标注
4. **场景 4**：Services 卡片网格 + 间距测量（用尺标）
