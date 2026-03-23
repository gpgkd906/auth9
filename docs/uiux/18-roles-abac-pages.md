# UI/UX 测试 - Roles 角色管理与 ABAC 策略页

**模块**: 页面专项
**测试范围**: 角色 Tab 切换、权限复选框、层级树视图、ABAC 策略编辑器、模拟引擎
**场景数**: 6

---

## 场景 1：Roles 页面 Tabs 切换与玻璃效果一致性

### 目的
验证 Roles 页面三个 Tab（Roles / Permissions / Hierarchy）的切换动效和内容区玻璃效果一致性。

### 测试操作流程
1. 访问 `/dashboard/roles`。
2. 依次点击三个 Tab，验证切换动效。
3. 检查每个 Tab 内容区的 Card 玻璃效果。
4. 缩小视口至 768px，验证 Tab 标签是否溢出。

### 预期视觉效果
- **Tab 容器**: `TabsList` 使用 `--glass-bg` 背景，`border-radius: 10px`（small elements），内边距均匀。
- **活跃 Tab**: `--accent-blue` 文字 + 白色/浅色背景（light mode）或深色高亮背景（dark mode）。
- **非活跃 Tab**: `--text-secondary` 文字色，hover 时 `--sidebar-item-hover` 背景。
- **切换动效**: Tab 内容区 `opacity` 过渡 0.2s，无布局跳动（height 保持稳定或 smooth transition）。
- **移动端**: Tab 标签保持水平排列不换行，必要时水平滚动（`overflow-x-auto`）。
- **内容区 Card**: 每个 Tab 内的 Card 均应用标准 `liquid-glass` 效果。

---

## 场景 2：角色列表 CRUD Dialog 布局

### 目的
验证创建/编辑角色的 Dialog 组件遵循 Liquid Glass 设计，表单字段排列紧凑。

### 测试操作流程
1. 点击"Create Role"按钮，打开创建 Dialog。
2. 检查 Dialog 的背景模糊、圆角、阴影。
3. 检查 Parent Role 控件是否为项目统一 Selector 组件。
4. 展开 Parent Role 下拉菜单，检查选项面板样式与选中态。
5. 打开某个既有角色的编辑 Dialog，验证 Parent Role 默认值与当前继承关系一致。
4. 在移动端验证 Dialog 宽度自适应。

### 预期视觉效果
- **Dialog 背景**: `backdrop-filter: blur(24px)`，遮罩层 `rgba(0, 0, 0, 0.5)` 半透明。
- **Dialog 容器**: `border-radius: 20px`，`--glass-bg` 背景，最大宽度 `max-w-lg`（512px）。
- **表单字段**: Name（Input）、Description（Input）、Parent Role（Select），间距 `space-y-4`。
- **Select Trigger**: 高度 40px（`h-10`），圆角 10px，边框/背景与项目 Select 风格一致。
- **Select 下拉**: Popover 面板使用 glass 背景与 `rounded-[14px]`；下拉项 `rounded-[8px]`，选中态带勾选图标；编辑场景中排除当前角色自身。
- **底部按钮**: Cancel（`variant="outline"`）+ Create（`variant="default"`），`gap-3`（12px），右对齐。
- **移动端**: Dialog 宽度 `calc(100% - 32px)`，底部按钮 `flex-col-reverse` 堆叠。

---

## 场景 3：权限管理复选框交互与视觉反馈

### 目的
验证 Permissions Tab 中复选框切换的视觉反馈和乐观更新 UX。

### 测试操作流程
1. 切换至 Permissions Tab。
2. 展开某个角色的权限列表。
3. 勾选/取消勾选权限项，观察即时反馈。
4. 验证权限数量 Badge 的更新。

### 预期视觉效果
- **复选框**: `Checkbox` 组件，选中时 `--accent-blue` 背景 + 白色对勾，`border-radius: 4px`。
- **乐观更新**: 勾选后立即显示选中状态，无需等待服务器响应（失败时自动回滚）。
- **权限项**: 每项高度 `min-h-[40px]`（40px），标签 14px，描述 12px `--text-secondary`。
- **权限计数 Badge**: 角色名称旁显示已分配权限数，`variant="secondary"` pill 样式。
- **Table 样式**: 表头 11px 大写，`letter-spacing: 0.04em`，`--text-tertiary` 色。
- **hover 行**: `--sidebar-item-hover` 背景色过渡。

---

## 场景 4：角色层级树视图缩进与连接线

### 目的
验证 Hierarchy Tab 中角色继承树的视觉层级清晰、连接线正确。

### 测试操作流程
1. 切换至 Hierarchy Tab。
2. 检查父子角色之间的缩进层级。
3. 验证孤儿角色（无父角色）的独立展示。
4. 在深色模式下验证连接线可见性。

### 预期视觉效果
- **缩进**: 每级缩进 `pl-6`（24px），树形结构使用 `border-l` 连接线。
- **连接线颜色**: `--glass-border-subtle`（Light: `rgba(0,0,0,0.06)` / Dark: `rgba(255,255,255,0.05)`）。
- **角色节点**: Card 样式容器，显示角色名 + 权限数 + 子角色数。
- **孤儿角色**: 标注特殊 Badge（如 `variant="warning"` "Orphaned"），单独分组在底部。
- **深色模式**: 连接线在 `#000000` 背景上需有足够对比度，可能需要 `rgba(255,255,255,0.1)` 以上。
- **展开/折叠**: 节点可折叠，折叠图标旋转动画 0.2s。

---

## 场景 5：ABAC 策略编辑器代码区域样式

### 目的
验证 ABAC 页面的 JSON 编辑区域使用等宽字体，且与 Liquid Glass 风格协调。

### 测试操作流程
1. 访问 `/dashboard/abac`。
2. 检查策略 JSON 编辑区域（Textarea）的字体和样式。
3. 输入非法 JSON，验证错误提示。
4. 点击 Publish，验证模式选择（enforce/shadow）的 UI。

### 预期视觉效果
- **代码区域**: `font-family: monospace`，`font-size: 13px`（`text-xs`），`line-height: 1.5`。
- **Textarea**: `border-radius: 12px`，`--glass-border` 边框，最小高度 200px，可自由调整高度。
- **JSON 错误**: 红色文字提示（`--accent-red`），位于 Textarea 下方，间距 `mt-2`（8px）。
- **版本列表**: Card 内的 `divide-y` 列表，每项显示版本号 + 模式（Badge）+ 时间戳。
- **模式 Badge**: enforce = `variant="danger"` 红色，shadow = `variant="warning"` 橙色。
- **Publish 按钮**: `variant="default"` 蓝色，旁边有 Select 选择发布模式。

---

## 场景 6：ABAC 模拟引擎输入与结果展示

### 目的
验证模拟引擎的四个 JSON 输入区域布局合理，结果展示清晰易读。

### 测试操作流程
1. 滚动至 Simulation 区域。
2. 验证四个 JSON 输入框（Subject / Resource / Request / Environment）的网格布局。
3. 输入测试数据并运行模拟。
4. 验证决策结果（Allow/Deny）的颜色编码。

### 预期视觉效果
- **输入网格**: `grid-cols-1 md:grid-cols-2`，四个 Textarea 均匀分布，`gap-4`（16px）。
- **输入标签**: 每个区域上方有 Label（"Subject", "Resource" 等），13px `font-medium`。
- **运行按钮**: `variant="default"` 蓝色，居中或右对齐，`mt-4`。
- **结果展示**:
  - Allow: `--accent-green` 文字 + `--accent-green-light` 背景，显示匹配的规则 ID。
  - Deny: `--accent-red` 文字 + `--accent-red-light` 背景。
- **匹配规则**: 列表展示匹配的规则 ID，使用 `font-mono` 等宽字体，12px。
- **空结果**: 灰色文字 "No matching rules"，`--text-tertiary` 色。

---

## 常见问题排查

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| `/dashboard/roles` 返回 500 错误 | Session token 过期或 auth9-core API 返回错误 | 运行 `./scripts/reset-docker.sh` 重置环境后重新登录 |
| Roles 页面加载缓慢或超时 | Roles 页面 loader 需对每个 service 并发请求 roles + permissions，服务数量多时 API 调用量大 | 确认 auth9-core 服务正常运行，检查网络延迟 |
| Tab 内容为空但无错误 | 当前租户下无 service 或 service 无 roles/permissions 配置 | 确认数据库中存在 services 及其关联的 roles |
