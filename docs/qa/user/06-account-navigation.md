# 用户账户 - 导航与布局

**模块**: 用户管理
**测试范围**: Account 布局、侧边栏用户信息、Settings 导航清理、旧 URL 重定向
**场景数**: 5
**优先级**: 中

---

## 场景 1：侧边栏显示真实用户信息

### 初始状态
- 用户已登录（display_name 为 `Jane Smith`，邮箱为 `jane@example.com`）

### 目的
验证 Dashboard 侧边栏底部显示当前登录用户的真实信息，而非硬编码数据

### 测试操作流程
1. 登录后进入 Dashboard 任意页面
2. 查看左侧边栏底部的用户卡片区域
3. 确认显示内容

### 预期结果
- 显示真实的用户名 `Jane Smith`（非 "John Doe"）
- 显示真实邮箱 `jane@example.com`（非 "john@example.com"）
- 头像区域显示用户 avatar_url 对应图片，无头像时显示姓名首字母缩写 `JS`
- 点击用户卡片导航至 `/dashboard/account`

---

## 场景 2：Account 布局与导航

### 初始状态
- 用户已登录

### 目的
验证 Account 区域的布局和子导航功能

### 测试操作流程
1. 点击侧边栏底部用户卡片，或直接访问 `/dashboard/account`
2. 确认页面结构：
   - 页面标题 "Account"
   - 副标题 "Manage your personal account settings"
   - 左侧子导航包含 5 个项目
3. 依次点击每个导航项：
   - 「Profile」→ `/dashboard/account`
   - 「Security」→ `/dashboard/account/security`
   - 「Passkeys」→ `/dashboard/account/passkeys`
   - 「Sessions」→ `/dashboard/account/sessions`
   - 「Linked Identities」→ `/dashboard/account/identities`
4. 确认当前激活的导航项高亮显示（蓝色背景白色文字）

### 预期结果
- 所有 5 个导航项均可点击并正确跳转
- 当前页面对应的导航项呈高亮状态
- 每个子页面在 Account 布局内正确渲染（右侧内容区域）
- 浏览器标题显示 "Account - Auth9"

---

## 场景 3：Settings 导航已移除个人操作项

### 初始状态
- 用户已登录

### 目的
验证 Settings 导航中不再包含 Sessions、Passkeys 和 Change Password，仅保留系统级管理项

### 测试操作流程
1. 导航至 `/dashboard/settings`
2. 检查左侧子导航列表

### 预期结果
- 导航项应包含：
  - Organization
  - Login Branding
  - Email Provider
  - Email Templates
  - Password Policy（原 "Security"，仅保留密码策略管理）
  - Identity Providers
- 导航项中**不应**包含：
  - Sessions
  - Passkeys
  - Change Password / Security（个人安全相关）

---

## 场景 4：旧 Settings URL 自动重定向

### 初始状态
- 用户已登录

### 目的
验证访问旧的 Settings 子路径会自动重定向到 Account 对应页面，保持向后兼容

### 测试操作流程
1. 在浏览器地址栏直接输入 `/dashboard/settings/sessions`
2. 观察页面跳转
3. 在浏览器地址栏直接输入 `/dashboard/settings/passkeys`
4. 观察页面跳转

### 预期结果
- `/dashboard/settings/sessions` → 自动重定向到 `/dashboard/account/sessions`
- `/dashboard/settings/passkeys` → 自动重定向到 `/dashboard/account/passkeys`
- 重定向后页面正常加载，功能完整可用
- 地址栏 URL 更新为新路径

---

## 场景 5：Settings Security 页面仅保留密码策略

### 初始状态
- 管理员已登录
- 至少存在一个租户

### 目的
验证 Settings Security 页面已移除 Change Password 功能，仅保留 Password Policy 管理

### 测试操作流程
1. 导航至 `/dashboard/settings/security`（或通过 Settings 导航点击「Password Policy」）
2. 检查页面内容

### 预期结果
- 页面显示 Password Policy 配置卡片：
  - 租户选择下拉框
  - 最小长度设置
  - 大写/小写/数字/特殊字符要求开关
  - 密码过期天数
  - 历史密码检查数量
- 页面中**不应**出现：
  - "Change Password" 卡片
  - 当前密码 / 新密码 / 确认密码输入框

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 侧边栏显示真实用户信息 | ☐ | | | |
| 2 | Account 布局与导航 | ☐ | | | |
| 3 | Settings 导航已移除个人操作项 | ☐ | | | |
| 4 | 旧 Settings URL 自动重定向 | ☐ | | | |
| 5 | Settings Security 页面仅保留密码策略 | ☐ | | | |
