# Portal MFA 自助管理页面

**模块**: auth (MFA Portal)
**测试范围**: Portal `/dashboard/account/mfa` 自助管理页面 — TOTP 设置/移除、恢复码生成、导航入口
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 Portal 新增 MFA 自助管理页面（`/dashboard/account/mfa`），允许已登录用户自主管理多因素认证。本文档覆盖 Portal UI 层面的功能验证。

后端 API 测试由 [24-mfa-totp-recovery.md](./24-mfa-totp-recovery.md) 覆盖，本文档聚焦于：
- Portal 页面入口可见性
- 页面状态展示正确性
- TOTP 内嵌设置流程
- 恢复码生成与弹窗展示
- TOTP 移除确认流程

**涉及端点**（均通过 Portal BFF 代理调用）：
- `GET /api/v1/mfa/status`
- `POST /api/v1/mfa/totp/enroll`
- `POST /api/v1/mfa/totp/enroll/verify`
- `DELETE /api/v1/mfa/totp`
- `POST /api/v1/mfa/recovery-codes/generate`

---

## 场景 1：MFA 页面入口可见性与初始状态

### 初始状态
- 用户已登录 Portal，进入 Dashboard
- 用户尚未启用 TOTP

### 目的
验证 MFA 页面在 Account 导航中可见，且未启用状态正确展示

### 测试操作流程
1. 进入「Account」页面（点击侧边栏用户头像或导航至 `/dashboard/account`）
2. 观察左侧导航项列表
3. 点击「MFA」导航项
4. 确认页面跳转至 `/dashboard/account/mfa`

### 预期结果
- Account 左侧导航中「MFA」位于「Security」和「Passkeys」之间
- 页面显示三个卡片区域：
  - **TOTP 验证器**：状态标签显示「Not set up」（灰色），可见「Set up TOTP」按钮
  - **恢复码**：显示提示文字「Set up TOTP first to use recovery codes.」
  - **通行密钥**：显示当前通行密钥状态，可见「Manage passkeys」链接

---

## 场景 2：TOTP 内嵌设置流程

### 初始状态
- 用户已登录，位于 `/dashboard/account/mfa`
- TOTP 未启用
- 用户已安装 Authenticator 应用（如 Google Authenticator）

### 目的
验证在 MFA 页面内完成 TOTP 设置（内嵌流程，无需离开页面）

### 步骤 0：Gate Check
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/mfa/status \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
# 预期: totp_enabled = false
```

### 测试操作流程
1. 在 TOTP 卡片中点击「Set up TOTP」按钮
2. 等待页面内展开设置区域（QR 码 + 验证码输入）
3. 确认 QR 码图片已渲染（200×200）
4. 可选：点击「Can't scan? Enter the key manually」查看手动密钥
5. 使用 Authenticator 应用扫描 QR 码
6. 在 6 位数 OTP 输入框中输入当前验证码
7. 确认自动提交（输入第 6 位后自动提交）

### 预期结果
- 点击按钮后，卡片内展开 QR 码区域（页面不跳转）
- QR 码清晰可扫描
- 手动密钥可复制，格式为 Base32 字符串
- 输入正确验证码后：
  - 显示成功消息「TOTP authenticator set up successfully」
  - TOTP 卡片状态标签变为「Enabled」（绿色）
  - 「Set up TOTP」按钮替换为「Remove TOTP」按钮
  - 恢复码卡片显示剩余数量和「Generate new codes」按钮

> **故障排除**
>
> | 症状 | 原因 | 解决方案 |
> |------|------|---------|
> | 输入正确 OTP 后显示「Invalid request」 | Setup token 已过期（Redis TTL 5 分钟） | 重新点击「Set up TOTP」生成新 QR 码；确保在 5 分钟内完成扫描和验证 |
> | 直接 API 调用成功但 UI 失败 | Portal session 过期或 accessToken 为空 | 刷新页面重新登录后再尝试；检查浏览器 Network 面板确认请求含 Authorization header |
> | OTP 代码总是错误 | 设备时间与服务器时间不同步 | 同步设备时间（NTP）；检查 `docker exec auth9-core date -u` 与本地时间差 |

### 预期数据状态
```sql
SELECT totp_enabled FROM mfa_settings
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local');
-- 预期: totp_enabled = 1
```

---

## 场景 3：恢复码生成与弹窗展示

### 初始状态
- 用户已登录，位于 `/dashboard/account/mfa`
- TOTP 已启用

### 目的
验证恢复码生成流程和弹窗展示

### 测试操作流程
1. 在「恢复码」卡片中点击「Generate new codes」按钮
2. 等待弹窗（Dialog）出现
3. 确认弹窗内容：
   - 标题「Recovery Codes Generated」
   - 提示信息（保存提醒）
   - 8 个恢复码（两列布局，等宽字体）
4. 点击「Copy all」按钮
5. 验证剪贴板内容（粘贴到文本编辑器）
6. 点击「I have saved these, close」关闭弹窗

### 预期结果
- 弹窗正确展示 8 个 10 位字母数字恢复码
- 「Copy all」点击后按钮文字变为「Copied!」（约 2 秒后恢复）
- 剪贴板内容为 8 个恢复码（每行一个）
- 关闭弹窗后，恢复码卡片显示「8 of 8 remaining」
- **关闭弹窗后无法再次查看这些恢复码**（后端只存储哈希）

### 预期数据状态
```sql
SELECT COUNT(*) AS code_count FROM recovery_codes
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
  AND used_at IS NULL;
-- 预期: code_count = 8
```

---

## 场景 4：TOTP 移除确认流程

### 初始状态
- 用户已登录，位于 `/dashboard/account/mfa`
- TOTP 已启用

### 目的
验证 TOTP 移除需要二次确认，移除后状态正确更新

### 测试操作流程
1. 在 TOTP 卡片中点击「Remove TOTP」按钮（红色文字）
2. 确认弹出确认对话框（AlertDialog）
3. 阅读确认内容（警告文字）
4. 点击「Cancel」取消
5. 确认 TOTP 未被移除（状态仍为 Enabled）
6. 再次点击「Remove TOTP」
7. 在确认对话框中点击「Remove TOTP」确认按钮（红色/危险样式）

### 预期结果
- 确认对话框显示标题「Remove TOTP Authenticator」
- 确认对话框显示警告信息
- 取消后 TOTP 状态不变
- 确认移除后：
  - 显示成功消息「TOTP authenticator removed」
  - TOTP 状态标签变为「Not set up」
  - 「Remove TOTP」替换为「Set up TOTP」
  - 恢复码卡片显示「Set up TOTP first to use recovery codes.」

### 预期数据状态
```sql
SELECT totp_enabled FROM mfa_settings
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local');
-- 预期: totp_enabled = 0 或记录不存在
```

---

## 场景 5：恢复码不足警告

### 初始状态
- 用户已登录，TOTP 已启用
- 恢复码剩余数量 < 3（通过多次使用恢复码或直接修改数据库）

### 步骤 0：准备恢复码不足状态

```sql
-- 模拟恢复码已使用（将部分恢复码标记为已用）
UPDATE recovery_codes SET used_at = NOW()
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
  AND used_at IS NULL
ORDER BY created_at ASC
LIMIT 6;

-- 验证剩余数量
SELECT COUNT(*) AS remaining FROM recovery_codes
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
  AND used_at IS NULL;
-- 预期: remaining = 2
```

### 目的
验证恢复码不足时显示警告

### 测试操作流程
1. 访问 `/dashboard/account/mfa`（或刷新页面）
2. 观察恢复码卡片区域

### 预期结果
- 恢复码卡片显示「2 of 8 remaining」
- 显示橙色警告横幅：提示恢复码不足，建议重新生成
- 「Generate new codes」按钮仍可用

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | MFA 页面入口可见性与初始状态 | ☐ | | | |
| 2 | TOTP 内嵌设置流程 | ☐ | | | |
| 3 | 恢复码生成与弹窗展示 | ☐ | | | |
| 4 | TOTP 移除确认流程 | ☐ | | | |
| 5 | 恢复码不足警告 | ☐ | | | |
