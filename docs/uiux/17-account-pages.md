# UI/UX 测试 - Account 账户管理页

**模块**: 页面专项
**测试范围**: 个人资料编辑、密码修改、Passkey 管理、会话列表、身份关联
**场景数**: 6

---

## 场景 1：Account 子导航布局与活跃状态

### 目的
验证 Account 页面左侧子导航在不同屏幕尺寸下的布局一致性，以及活跃项的视觉反馈。

### 测试操作流程
1. 访问 `/dashboard/account`。
2. 检查左侧导航栏（Profile / Security / MFA / Passkeys / Sessions / Identities）的排列方式。
3. 点击每个导航项，验证活跃状态的视觉区分。
4. 缩小视口至 768px 以下，验证导航是否切换为水平滚动或折叠。

### 预期视觉效果
- **导航宽度**: 左侧导航固定宽度 `w-48`（192px），与内容区域之间有 `gap-6`（24px）间距。
- **活跃状态**: 活跃项使用 `--accent-blue` 文字色 + `--accent-blue-light` 背景色，圆角 `rounded-lg`（8px）。
- **非活跃项**: `--text-secondary` 文字色，hover 时 `--sidebar-item-hover` 背景色。
- **圆角**: 导航项 `rounded-xl`（12px）。
- **移动端**: 导航项水平排列或隐藏到下拉菜单，确保不会遮挡内容区域。
- **字体**: 导航项 14px，`font-medium`（500），行高 1.5。

---

## 场景 2：个人资料表单 Card 玻璃效果与头像显示

### 目的
验证 Profile 卡片正确应用 Liquid Glass 效果，头像显示与 fallback 逻辑正确。

### 测试操作流程
1. 访问 `/dashboard/account`（Profile 页）。
2. 检查 Card 组件的 `backdrop-filter`、`border-radius`、`box-shadow`。
3. 验证头像组件的圆形裁切和 fallback initials 显示。
4. 提交表单后验证成功/错误消息的显示。

### 预期视觉效果
- **Card 玻璃效果**: `backdrop-filter: blur(24px) saturate(180%)`，`border-radius: 20px`，`box-shadow: 0 8px 32px var(--glass-shadow)`。
- **头像**: 圆形（`border-radius: 50%`），尺寸 `w-16 h-16`（64px），无头像时显示邮箱/名称首字母。
- **表单字段**: Label 与 Input 间距 6px，字段之间 `space-y-4`（16px）。
- **成功消息**: 绿色背景（`--accent-green-light`），`--accent-green` 文字色，`rounded-lg`。
- **错误消息**: 红色背景（`--accent-red-light`），`--accent-red` 文字色，`role="alert"`。
- **提交按钮**: 禁用态透明度降低（`opacity: 0.5`），cursor 为 `not-allowed`。

---

## 场景 3：密码修改表单验证与反馈

### 目的
验证 Security 页面密码修改表单的输入验证、错误提示位置和视觉样式。

### 测试操作流程
1. 访问 `/dashboard/account/security`。
2. 不输入任何内容直接提交，验证浏览器原生验证。
3. 输入不匹配的新密码与确认密码，提交并观察错误提示。
4. 输入有效密码提交，观察成功反馈。

### 预期视觉效果
- **密码字段**: `type="password"`，Input 高度 `h-10`（40px），`border-radius: 12px`。
- **字段标签**: "Current Password" / "New Password" / "Confirm Password"，13px `font-medium`。
- **错误消息**: 表单顶部内联显示，红色背景 + `--accent-red` 文字，与第一个字段间距 `mb-4`（16px）。
- **成功消息**: 绿色背景 + `--accent-green` 文字。
- **按钮**: `variant="default"` 蓝色背景，提交中状态显示 loading 文本 + 禁用。

---

## 场景 4：Passkeys 列表与 WebAuthn 注册流程 UX

### 目的
验证 Passkeys 页面的列表展示、空状态设计和 WebAuthn 注册交互的视觉反馈。

### 测试操作流程
1. 访问 `/dashboard/account/passkeys`。
2. 若无 Passkey，验证空状态页面设计。
3. 点击"Register Passkey"按钮，观察浏览器 WebAuthn 弹窗期间页面的状态。
4. 注册成功后验证列表项的布局。

### 预期视觉效果
- **空状态**: 居中图标（`h-12 w-12`，`--text-tertiary` 色），标题 + 描述文字 + CTA 按钮，`text-center`。
- **CTA 按钮**: `variant="default"` 蓝色背景，`border-radius: 12px`。
- **Passkey 列表**: `divide-y divide-[var(--glass-border-subtle)]` 分隔线，每项使用 `flex` 布局。
- **列表项**: 左侧锁形图标（`LockClosedIcon`）+ 设备名称 + 创建时间，右侧删除按钮。
- **删除按钮**: `variant="ghost"` 或 `variant="destructive"`，hover 时红色强调。
- **注册中**: 按钮禁用 + loading 提示，防止重复点击。
- **信息卡片**: 底部说明卡片使用 `--accent-cyan-light` 背景，介绍 Passkey 用途。

---

## 场景 5：会话管理列表与当前会话高亮

### 目的
验证 Sessions 页面正确区分当前会话和其他会话，设备图标匹配准确。

### 测试操作流程
1. 访问 `/dashboard/account/sessions`。
2. 验证当前会话卡片的高亮样式。
3. 检查设备类型图标（桌面/移动端/未知）的正确显示。
4. 点击"Revoke"按钮验证确认流程。
5. 检查"Revoke All Other Sessions"批量操作按钮。

### 预期视觉效果
- **当前会话卡片**: 绿色左边框（`border-l-4 border-[var(--accent-green)]`）或绿色背景色调，标注 "Current Session" Badge。
- **Badge 样式**: `variant="success"` 绿色 pill（`border-radius: 100px`），文字 11-12px。
- **其他会话**: 标准 Card 样式，`divide-y` 分隔。
- **设备图标**: Desktop（DesktopIcon）/ Mobile（MobileIcon）/ Unknown（GlobeIcon），图标尺寸 `w-5 h-5`。
- **会话信息**: IP 地址 + 相对时间（"2 hours ago"），`--text-secondary` 色。
- **Revoke 按钮**: `variant="destructive"` 或 `variant="outline"` 红色文字。
- **批量撤销**: `variant="destructive"` 全宽按钮，位于列表底部，`mt-4`（16px）间距。
- **Tips 卡片**: 底部安全提示，使用 `--accent-orange-light` 背景（警告风格）。

---

## 场景 6：Account 页面深色模式适配

### 目的
验证所有 Account 子页面在深色模式下的颜色 token 切换正确性。

### 测试操作流程
1. 切换至深色模式。
2. 依次访问 Profile、Security、Passkeys、Sessions 页面。
3. 检查 Card 背景、输入框边框、文字颜色、图标颜色的 token 切换。

### 预期视觉效果
- **Card 背景**: `rgba(44, 44, 46, 0.65)`（深色 `--glass-bg`）。
- **输入框**: 边框 `rgba(255, 255, 255, 0.1)`（`--glass-border`），背景 `--bg-secondary`（`#1C1C1E`）。
- **主文字**: `#FFFFFF`（`--text-primary`），次要文字 `#98989D`（`--text-secondary`）。
- **成功/错误色**: 保持不变（`--accent-green` / `--accent-red`），但 light 背景切换为 dark 变体（`rgba(52, 199, 89, 0.2)` / `rgba(255, 59, 48, 0.2)`）。
- **分隔线**: `rgba(255, 255, 255, 0.05)`（`--glass-border-subtle`）。
- **过渡**: 主题切换时 `0.3s` 平滑过渡，无闪烁。

---

## 常见问题排查

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| `/dashboard/account` 返回 401 并重定向到登录页 | Session token 过期或环境未正确初始化 | 运行 `./scripts/reset-docker.sh` 重置环境后重新登录 |
| `/dashboard/account` 显示 401 但其他子页面（如 `/security`）可正常访问 | Account Profile 的 loader 调用 `userApi.getMe()` 失败，其他子页面可能使用不同的 API 调用 | 确认 auth9-core 服务正常运行（`curl http://localhost:8080/health`），然后重新登录 |
| 所有 Account 子页面均不可访问 | Session cookie 无效或 auth9-core 未运行 | 检查 Docker 服务状态，确保 auth9-core 和 Redis 正常运行 |
