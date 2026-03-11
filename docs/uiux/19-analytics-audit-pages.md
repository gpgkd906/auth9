# UI/UX 测试 - Analytics 分析、Audit Logs 审计日志与 Security Alerts 安全告警页

**模块**: 页面专项
**测试范围**: 统计卡片趋势指标、日趋势图表、事件列表分页、审计日志表格、安全告警过滤与严重度色彩编码
**场景数**: 6

---

## 场景 1：Analytics 统计卡片与趋势指标

### 目的
验证 Analytics 页面顶部统计卡片的数值展示、趋势箭头和颜色编码一致性。

### 测试操作流程
1. 访问 `/dashboard/analytics`。
2. 检查统计卡片网格（总事件数、成功率、失败率、锁定账户等）。
3. 验证趋势指标（↑/↓/→）的颜色正确性。
4. 切换日期范围（7/14/30/90 天），观察数据刷新和卡片更新。

### 预期视觉效果
- **卡片网格**: `grid-cols-2 md:grid-cols-4`，`gap-4`（16px），每张卡片应用 `liquid-glass` 效果。
- **数值**: `font-size: 26-28px`，`font-weight: 700`，`font-variant-numeric: tabular-nums`（等宽数字对齐）。
- **趋势指标**:
  - 上升（正面）: `--accent-green` + "↑" 箭头。
  - 下降（负面）: `--accent-red` + "↓" 箭头。
  - 持平: `--text-secondary` + "→" 箭头。
- **日期选择器**: Button group 样式（7d / 14d / 30d / 90d），活跃项使用 `--accent-blue` 背景。
- **标签**: 13px `--text-secondary`，`letter-spacing: 0.04em`。
- **卡片 hover**: `transform: translateY(-2px)` + 增强阴影。

---

## 场景 2：日趋势堆叠柱状图渲染

### 目的
验证自定义 CSS 柱状图的颜色编码、响应式伸缩和数据可读性。

### 测试操作流程
1. 在 Analytics 页面定位日趋势图表区域。
2. 检查柱状图的颜色（成功=蓝色、失败=红色）。
3. 缩小视口至 768px，验证图表是否水平滚动或自适应。
4. Hover 某根柱子，验证 tooltip（如有）的信息展示。

### 预期视觉效果
- **图表容器**: Card 内，高度 `160px`，`overflow-x-auto` 允许水平滚动。
- **柱状图**: 成功区段 `--accent-blue` 色，失败区段 `--accent-red` 色，堆叠展示。
- **柱宽**: 固定或弹性宽度，柱间距 2-4px，整体居中。
- **X 轴标签**: 日期，11px `--text-tertiary`，不重叠（过多时隐藏部分）。
- **Y 轴**: 隐式（无显式刻度），通过柱高比例传达数量。
- **响应式**: 窄屏下柱状图保持最小宽度，容器水平滚动。
- **空数据**: 显示灰色占位文字 "No data for this period"。

---

## 场景 3：Analytics Events 事件列表过滤与分页

### 目的
验证事件列表页面的过滤器、事件类型 Badge 颜色编码和分页控件。

### 测试操作流程
1. 访问 Analytics → Events Tab。
2. 使用 Email 过滤器输入搜索。
3. 检查事件类型 Badge（success / failed / locked）的颜色。
4. 翻页并验证分页控件的样式和状态。

### 预期视觉效果
- **过滤器**: Input + 清除按钮（X 图标），Input 高度 `h-10`，`border-radius: 12px`。
- **事件 Badge**:
  - Success: `--accent-green` 文字 + `--accent-green-light` 背景（`variant="success"`）。
  - Failed: `--accent-red` 文字 + `--accent-red-light` 背景（`variant="danger"`）。
  - Locked: `--accent-orange` 文字 + `--accent-orange-light` 背景（`variant="warning"`）。
- **Badge 圆角**: `border-radius: 100px`（pill 形状）。
- **事件图标**: CheckCircledIcon（成功）、CrossCircledIcon（失败）、LockClosedIcon（锁定），与 Badge 颜色一致。
- **分页**: "Previous" / "Next" 按钮 `variant="outline"`，当前页禁用时 `opacity: 0.5`。
- **表格**: 标准 `Table` 组件，表头 11px 大写，行 hover `--sidebar-item-hover`。

---

## 场景 4：Audit Logs 审计日志表格排版

### 目的
验证审计日志页面的表格样式、列宽分配和时间戳格式化。

### 测试操作流程
1. 访问 `/dashboard/audit-logs`。
2. 检查表格列（Action / Resource / Actor / Time）的宽度和对齐。
3. 验证 `FormattedDate` 组件的日期格式遵循当前语言环境。
4. 翻页并验证无数据时的空状态。

### 预期视觉效果
- **表格容器**: Card 内，`overflow-x-auto` 响应式滚动。
- **表头**: `uppercase`，11px `font-weight: 600`，`letter-spacing: 0.04em`，`--text-tertiary` 色。
- **表格单元**: `px-4 py-3`（16px × 12px 内边距），`--text-primary` 色。
- **行分隔**: `divide-y divide-[var(--glass-border-subtle)]`。
- **Action 列**: 动作名称使用 `font-mono` 或 Badge 样式展示（如 "create", "delete", "update"）。
- **时间戳**: `FormattedDate` 组件，遵循当前 locale（zh-CN: "2026年3月11日 14:30" / en-US: "Mar 11, 2026 2:30 PM"）。
- **分页**: 与场景 3 相同的分页控件样式。
- **空状态**: 居中图标 + "No audit logs found" 文字，`--text-tertiary`。
- **列宽**: Action（20%）、Resource（25%）、Actor（30%）、Time（25%），表格最小宽度 `min-w-[600px]`。

---

## 场景 5：Security Alerts 严重度过滤与颜色编码

### 目的
验证安全告警页面的过滤器交互、告警卡片严重度颜色系统和已解决项样式。

### 测试操作流程
1. 访问 `/dashboard/security/alerts`。
2. 点击不同严重度过滤器（Critical / High / Medium / Low），验证 URL 参数变化。
3. 检查每张告警卡片的边框颜色与严重度对应。
4. 验证已解决告警的视觉弱化处理。
5. 无告警时检查空状态。

### 预期视觉效果
- **过滤器**: 按钮组（tag 样式），活跃项使用对应严重度颜色背景 + 白色文字，`border-radius: 100px`（pill）。
- **严重度颜色系统**:
  - Critical: `--accent-red`（`#FF3B30`），左边框 `border-l-4`。
  - High: `--accent-orange`（`#FF9500`），左边框 `border-l-4`。
  - Medium: 黄色调（`#FFCC00` 或 `--accent-orange` 浅色变体）。
  - Low: `--accent-blue`（`#007AFF`），左边框 `border-l-4`。
- **告警卡片**: Card 容器 + 左侧彩色边框，内含图标（ExclamationTriangleIcon）+ 标题 + 描述 + 时间。
- **已解决项**: `opacity: 0.6`，标注 CheckCircledIcon + "Resolved" Badge（`variant="success"`）。
- **操作按钮**: "Resolve" / "Dismiss"，使用 `variant="outline"` 或 `variant="ghost"`。
- **空状态**: 绿色 CheckCircledIcon（`h-12 w-12`）+ "All Clear" 标题 + 描述文字，居中展示。
- **推荐卡片**: 底部安全建议 Card，使用 `--accent-cyan-light` 背景。

---

## 场景 6：Analytics 与 Audit 页面深色模式适配

### 目的
验证数据可视化组件在深色模式下的可读性和颜色对比度。

### 测试操作流程
1. 切换至深色模式。
2. 检查柱状图在黑色背景上的辨识度。
3. 验证表格行 hover 在深色模式下的区分度。
4. 检查严重度颜色在深色背景上的对比度是否满足 WCAG AA（4.5:1）。

### 预期视觉效果
- **柱状图**: `--accent-blue` 和 `--accent-red` 在 `#000000` 背景上保持高对比度（两者均 > 4.5:1）。
- **表格 hover**: `rgba(255, 255, 255, 0.06)`（`--sidebar-item-hover` dark），与非 hover 行形成可察觉对比。
- **严重度 Badge**: 深色模式下使用 `0.2` 透明度背景变体（`rgba(255, 59, 48, 0.2)` 等）。
- **空状态图标**: `--text-tertiary`（`#636366`）在 `#000000` 背景上对比度为 3.9:1，图标尺寸 `h-12` 足够辨识。
- **日期选择器**: 活跃按钮 `--accent-blue` 在深色背景上清晰可辨。
