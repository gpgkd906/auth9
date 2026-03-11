# UI/UX 测试 - Dialog 弹窗与 Empty State 空状态跨页面一致性

**模块**: 交互体验（跨页面模式）
**测试范围**: Dialog/AlertDialog 玻璃效果一致性、焦点陷阱、Empty State 图标/文字/CTA 统一、表单提交状态
**场景数**: 5

---

## 场景 1：Dialog 组件 Liquid Glass 效果统一性

### 目的
验证所有 Dialog 弹窗（Create Tenant / Create User / Create Role / Edit Service / Webhook Secret 等）的玻璃效果参数完全一致。

### 测试操作流程
1. 依次打开以下 Dialog:
   - Tenants 页 → Create Tenant
   - Users 页 → Create User
   - Roles 页 → Create Role
   - Services 页 → Service Secret
   - Webhooks 页 → Create Webhook
2. 对每个 Dialog 使用 DevTools 检查以下 CSS 属性。
3. 在深色模式下重复检查。

### 预期视觉效果
- **遮罩层**: `rgba(0, 0, 0, 0.5)` 半透明黑色（Light/Dark 相同）。
- **Dialog 容器**:
  - `background: var(--glass-bg)`（Light: `rgba(255,255,255,0.72)` / Dark: `rgba(44,44,46,0.65)`）。
  - `backdrop-filter: blur(24px) saturate(180%)`。
  - `border: 1px solid var(--glass-border)`。
  - `border-radius: 20px`。
  - `box-shadow: 0 8px 32px var(--glass-shadow), inset 0 1px 0 var(--glass-highlight)`。
- **最大宽度**: `max-w-lg`（512px），居中展示。
- **内边距**: Header `p-6`，Content `px-6 pb-6`，Footer `px-6 pb-6`。
- **入场动画**: `scale(0.95) → scale(1)` + `opacity: 0 → 1`，200ms。

> **故障排除**: 如果某个 Dialog 的 `backdrop-filter` 值不同（如 `blur(20px)`），可能是自定义 className 覆盖了默认样式。应统一使用 `DialogContent` 基础组件的默认样式。

---

## 场景 2：Dialog 键盘交互与焦点管理

### 目的
验证所有 Dialog 的焦点陷阱（focus trap）、Escape 关闭和 Tab 循环。

### 测试操作流程
1. 用键盘打开 Dialog（Tab 到触发按钮 → Enter）。
2. 验证焦点自动移至 Dialog 内第一个可交互元素。
3. 连续按 Tab 验证焦点在 Dialog 内循环，不逃逸到背景。
4. 按 Escape 关闭 Dialog，验证焦点返回触发按钮。
5. 在 AlertDialog（删除确认）中重复测试。

### 预期视觉效果
- **焦点指示器**: `outline: 2px solid var(--accent-blue)`，`outline-offset: 2px`。
- **焦点陷阱**: Tab 键在 Dialog 内循环，Shift+Tab 反向循环。
- **首次焦点**: Dialog 打开后焦点移至第一个 Input（或 Close 按钮）。
- **Escape 关闭**: Dialog 关闭，焦点返回到打开 Dialog 的按钮。
- **AlertDialog 特殊**:
  - 焦点移至 "Cancel" 按钮（而非 "Confirm"），防止意外确认。
  - 点击遮罩层不关闭 AlertDialog（需明确点击 Cancel 或按 Escape）。
- **Screen reader**: Dialog 标题通过 `aria-labelledby` 关联，`role="dialog"`。

---

## 场景 3：Empty State 设计模式跨页面一致性

### 目的
验证所有空状态（无数据）页面使用统一的视觉模式：居中图标 + 标题 + 描述 + CTA。

### 测试操作流程
1. 准备空数据环境（无 Tenant / User / Service / Role / Webhook / Invitation / Passkey / Session / Alert）。
2. 依次访问以下页面，验证空状态展示:
   - `/dashboard/tenants` → 无租户
   - `/dashboard/users` → 无用户（搜索无结果）
   - `/dashboard/services` → 无服务
   - `/dashboard/roles` → 无角色
   - `/dashboard/audit-logs` → 无日志
   - `/dashboard/security/alerts` → 无告警
   - `/dashboard/account/passkeys` → 无 Passkey
   - `/dashboard/tenants/:id/webhooks` → 无 Webhook
   - `/dashboard/tenants/:id/invitations` → 无邀请

### 预期视觉效果
- **统一模式**: 所有空状态使用以下布局:
  ```
  [居中图标 h-12 w-12]
  [标题 16-17px font-semibold --text-primary]
  [描述 13-14px --text-secondary]
  [CTA 按钮 variant="default" --accent-blue]（如适用）
  ```
- **图标颜色**: `--text-tertiary`（Light: `#AEAEB2` / Dark: `#636366`）。
- **标题与描述间距**: `mt-2`（8px）。
- **描述与按钮间距**: `mt-4`（16px）。
- **容器**: `text-center py-12`（48px 上下内边距），居中于 Card 或页面内容区。
- **特殊: Security Alerts**: 空状态使用 `--accent-green` 图标（CheckCircledIcon），传达"一切正常"积极语义。
- **一致性检查项**:
  | 页面 | 图标 | CTA 文案 |
  |------|------|---------|
  | Tenants | BuildingIcon | "Create Tenant" |
  | Users | PersonIcon | "Create User"（或无 CTA，仅清除搜索）|
  | Services | GearIcon | "Create Service" |
  | Roles | LockClosedIcon | "Create Role" |
  | Audit Logs | ClipboardIcon | 无 CTA（数据自然产生）|
  | Alerts | CheckCircledIcon | 无 CTA |
  | Passkeys | KeyIcon/LockIcon | "Register Passkey" |
  | Webhooks | WebhookIcon | "Create Webhook" |
  | Invitations | EnvelopeIcon | "Send Invitation" |

---

## 场景 4：表单提交 Loading 状态与禁用交互

### 目的
验证所有表单（Dialog 内和页面内）在提交过程中的 Loading 状态和防重复提交机制。

### 测试操作流程
1. 在网络节流（Slow 3G）下提交 Create Tenant 表单。
2. 观察按钮状态变化。
3. 尝试在 Loading 中重复点击提交按钮。
4. 在其他页面（Create User / Change Password / Create Webhook）重复测试。

### 预期视觉效果
- **提交按钮 Loading 态**:
  - `disabled` 属性设为 `true`。
  - `opacity: 0.5` 或 `cursor: not-allowed`。
  - 文字替换为 i18n loading 文案（如 "Creating..." / "Saving..."）。
- **表单字段**: 提交中 Input/Select 应禁用（`disabled`），防止修改。
- **Cancel 按钮**: 提交中仍可点击（允许取消操作）或同时禁用（取决于实现）。
- **防重复**: `useNavigation().state === "submitting"` 检查，按钮自动禁用。
- **成功后**: Dialog 自动关闭（200ms 延迟），页面数据刷新（`revalidator`）。
- **失败后**: 按钮恢复可点击，错误消息内联展示（红色文字，`--accent-red`）。

---

## 场景 5：Destructive Action 确认弹窗统一性

### 目的
验证所有删除/撤销/不可逆操作使用统一的 AlertDialog 确认模式。

### 测试操作流程
1. 触发以下删除操作:
   - Delete Tenant → AlertDialog
   - Delete User → AlertDialog
   - Delete Service → AlertDialog
   - Delete Role → AlertDialog
   - Delete Webhook → AlertDialog
   - Revoke Invitation → AlertDialog
   - Revoke Session → AlertDialog（或 inline confirm）
2. 验证每个 AlertDialog 的视觉一致性。

### 预期视觉效果
- **AlertDialog 结构**:
  - Title: "Confirm [Action]" 或 "Delete [Resource]?"，17px `font-semibold`。
  - Description: 描述后果，14px `--text-secondary`，包含被操作对象名称。
  - Cancel 按钮: `variant="outline"`，左侧。
  - Confirm 按钮: `variant="destructive"`（红色背景 `--accent-red` + 白色文字），右侧。
- **按钮排列**: `flex gap-3 justify-end`，移动端 `flex-col-reverse` 堆叠（Confirm 在上）。
- **遮罩层**: 点击遮罩不触发确认（需明确操作）。
- **颜色一致性**: 所有 destructive 操作使用相同的 `--accent-red`（`#FF3B30`）。
- **确认文字**: 操作描述中包含资源名称（如 "Delete tenant **Acme Corp**?"），资源名加粗。
- **i18n**: 确认文字和按钮文案遵循当前语言环境。
