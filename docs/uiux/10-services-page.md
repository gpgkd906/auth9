# UI/UX 测试 - Services 服务管理页

**模块**: 页面专项
**测试范围**: OIDC 注册表单、密钥显示区、状态标签适配、边距一致性
**场景数**: 5

---

## 场景 1：Client Secret 显示区域响应式

### 目的
验证生成的密钥在不同尺寸下的换行和按钮大小。

### 预期视觉效果
- **按钮尺寸**: 复制按钮在手机端应适当变大（44px 宽），且文字标签在极窄屏下可隐藏，仅留图标。
- **换行**: 密钥（Long UUID）应支持自动换行，不得撑破容器（`break-all`）。

---

## 2. 状态标签 (Active/Inactive) 的垂直边距

### 目的
验证状态圆点与文字在各种设备上的中心对齐。

---

## 3. 注册服务长表单的滚动体验

### 目的
验证移动端弹窗中的长表单，底部“Register”按钮是否常驻或有足够的底部边距。

### 预期视觉效果
- **Padding**: 弹窗底部应留有 24px 的安全边距，防止按钮贴边。

---

## 4. 字段标签后的“Optional”标注样式

### 目的
验证可选字段（如 Logo URL）的标注文字层级。

---

## 场景 5：Service 详情页 Actions / Branding Tab 布局

### 目的
验证 Service 详情页新增的「Actions」和「Branding」标签页在不同尺寸下的布局表现。

### 预期视觉效果
- **Tab 栏**: 4 个标签（Configuration、Integration、Actions、Branding）在宽屏下水平排列，窄屏下可横向滚动，不折行
- **Actions Tab**: Action 列表卡片与「New Action」按钮间距一致，空状态居中显示提示文字
- **Branding Tab**: 颜色选择器排列整齐（2 列网格），表单字段间距与其他 Tab 一致
- **「Reset to Default」按钮**: 使用 destructive 配色，与「Save Changes」按钮水平对齐

---

## 场景 6：网格布局切换时的按钮对齐与卡片高度

### 目的
验证从 3 列切换到 1 列时，每个 Service 卡片内的操作按钮位置是否一致，且高度整齐。

### 预期视觉效果
- **高度对齐**: 即使有的服务描述很长，有的很短，同一行的卡片底部按钮必须在同一水平高度对齐（使用 `flex-col` 和 `mt-auto`）。
- **文字截断**: 当服务名称超长时，应使用省略号（`truncate`）而非撑破卡片高度或导致布局错乱。

### 验证方法
- **目标元素**: 服务名称使用 `<p>` 标签（非 `<h3>`），带有 `truncate text-base font-semibold` 类名。
- **检查方式**: 在 DevTools 中选中服务名称的 `<p>` 元素，确认 `text-overflow: ellipsis`、`overflow: hidden`、`white-space: nowrap` 三个计算样式均已生效。
- **父容器**: 父 `<div>` 需要 `min-w-0 flex-1` 类名，以确保 flex 子项中 truncate 正常工作。
