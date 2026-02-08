# 用户账户 - 安全与身份管理

**模块**: 用户管理
**测试范围**: Account 下的修改密码、Passkeys 管理、会话管理、关联身份管理
**场景数**: 5
**优先级**: 高

---

## 背景说明

个人安全相关功能从 Settings 迁移至 Account 区域（`/dashboard/account/*`）。包括：
- `/dashboard/account/security` — 修改密码
- `/dashboard/account/passkeys` — Passkey 管理
- `/dashboard/account/sessions` — 会话管理
- `/dashboard/account/identities` — 关联身份管理（新增）

关联身份 API：
- `GET /api/v1/users/me/linked-identities` — 获取当前用户的关联身份列表
- `DELETE /api/v1/identity-providers/linked/{id}` — 解除关联身份

---

## 场景 1：Account 页面修改密码

### 初始状态
- 用户已登录
- 当前密码为 `OldPass123!`

### 目的
验证在 Account Security 页面可以成功修改密码

### 测试操作流程
1. 导航至 `/dashboard/account/security`
2. 确认页面标题为 "Change Password"
3. 填写：
   - Current password：`OldPass123!`
   - New password：`NewPass456!`
   - Confirm new password：`NewPass456!`
4. 点击「Change password」

### 预期结果
- 按钮显示 "Changing..." 加载状态
- 成功后显示绿色提示 "Password changed successfully"
- 使用旧密码 `OldPass123!` 无法登录
- 使用新密码 `NewPass456!` 可以正常登录

---

## 场景 2：修改密码表单验证

### 初始状态
- 用户已登录并进入 `/dashboard/account/security`

### 目的
验证密码修改表单的前端验证逻辑

### 测试操作流程
1. **空字段验证**：不填写任何字段，直接点击「Change password」
2. **密码过短**：填写新密码为 `Short1`（少于 8 字符），点击提交
3. **密码不匹配**：
   - Current password：`CurrentPass!`
   - New password：`NewPass456!`
   - Confirm new password：`DifferentPass!`
   - 点击「Change password」
4. **错误的当前密码**：
   - Current password：`WrongPassword!`
   - New password：`NewPass456!`
   - Confirm new password：`NewPass456!`
   - 点击「Change password」

### 预期结果
- 空字段：显示 "All password fields are required"
- 密码过短：显示 "New password must be at least 8 characters"
- 密码不匹配：显示 "New passwords do not match"
- 错误的当前密码：API 返回错误，显示对应错误信息

---

## 场景 3：Account Passkeys 页面管理

### 初始状态
- 用户已登录
- 用户已注册至少一个 Passkey

### 目的
验证 Passkeys 页面在 Account 区域正常工作

### 测试操作流程
1. 导航至 `/dashboard/account/passkeys`
2. 确认页面显示：
   - 页面标题 "Passkeys"
   - 描述文字 "Passkeys are a secure, passwordless way to sign in..."
   - 已有 Passkey 列表（显示名称、类型标签「Passwordless」或「Two-Factor」、添加日期）
   - 「Add passkey」按钮
   - "About Passkeys" 信息卡片
3. 点击已有 Passkey 的「Remove」按钮
4. 确认 Passkey 已被删除

### 预期结果
- Passkey 列表正确显示所有已注册的 Passkey
- 删除后显示成功提示 "Passkey deleted"
- 列表中已不包含被删除的 Passkey
- 如果所有 Passkey 已删除，显示空状态 "No passkeys yet"

---

## 场景 4：Account Sessions 页面管理

### 初始状态
- 用户在多个设备/浏览器登录，存在至少 2 个活跃会话

### 目的
验证 Sessions 页面在 Account 区域正常工作

### 测试操作流程
1. 导航至 `/dashboard/account/sessions`
2. 确认页面显示：
   - "Current Session" 卡片（带绿色 "Current" 标签、设备图标、IP 地址、位置信息、最后活跃时间）
   - "Other Sessions" 卡片（列出其他设备的会话信息）
   - "Security Tips" 卡片
3. 在 Other Sessions 中，点击某个会话的「Revoke」按钮
4. 验证该会话已被撤销
5. 如有多个其他会话，点击「Sign out all」按钮

### 预期结果
- 单个撤销：页面刷新后该会话从列表中消失，被撤销设备需要重新登录
- 全部撤销：Other Sessions 列表清空，显示 "No other active sessions"
- 当前会话不受影响

---

## 场景 5：Account 关联身份管理

### 初始状态
- 用户已登录
- 用户通过社交登录（如 Google、GitHub）关联了至少一个外部身份

### 目的
验证 Linked Identities 页面的展示和解除关联功能

### 测试操作流程
1. 导航至 `/dashboard/account/identities`
2. 确认页面显示：
   - 页面标题 "Linked Identities"
   - 描述 "External accounts connected to your Auth9 account..."
   - 已关联身份列表（Provider 图标、名称、外部邮箱、关联日期）
3. 点击某个关联身份的「Unlink」按钮
4. 确认关联身份已被解除
5. 若无关联身份，确认显示空状态 "No linked identities"

### 预期结果
- 关联身份列表正确展示（Google 显示 "G"，GitHub 显示 "GH"，Microsoft 显示 "MS" 等图标缩写）
- 解除关联后显示绿色提示 "Identity unlinked successfully"
- 列表中已不包含被解除的身份
- 解除后，用户下次登录时无法使用该社交登录方式

### 预期数据状态
```sql
SELECT id, provider_type, provider_alias, external_email, linked_at
FROM linked_identities WHERE user_id = '{user_id}';
-- 预期: 被解除的关联身份不在结果中
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Account 页面修改密码 | ☐ | | | |
| 2 | 修改密码表单验证 | ☐ | | | |
| 3 | Account Passkeys 页面管理 | ☐ | | | |
| 4 | Account Sessions 页面管理 | ☐ | | | |
| 5 | Account 关联身份管理 | ☐ | | | |
