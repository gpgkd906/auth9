# Auth - Required Actions 与登录后跳转

**模块**: Auth
**测试范围**: Required Actions API 端点、登录后 pending action 跳转、Portal 页面渲染
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 自管 required actions。登录成功后，如果用户有 pending actions，Auth9 决定跳转到对应流程页面。

端点：
- `GET /api/v1/hosted-login/pending-actions` — 列出 pending actions（需认证）
- `POST /api/v1/hosted-login/complete-action` — 完成一个 action（需认证）

Action 类型与 Portal 页面映射：
| Action Type | Portal 页面 | 用途 |
|-------------|------------|------|
| `verify_email` | `/verify-email` | 邮箱验证 |
| `update_password` | `/force-update-password` | 强制更新密码 |
| `complete_profile` | `/complete-profile` | 补充 profile |
| `CONFIGURE_TOTP` | `/mfa/setup-totp` | MFA 强制配置（mfa_enabled=true 且无 MFA 凭证时自动创建） |

登录后跳转逻辑（`login.tsx` action）：
```
密码登录成功 → 检查 pending_actions
  → 有 pending actions → redirect 到第一个 action 的 redirect_url?action_id=xxx
  → 无 pending actions → redirect 到 /tenant/select
```

---

## 场景 1：获取 Pending Actions 列表

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中：`curl -sf http://localhost:8080/health`
- Required Actions 功能由 auth9-oidc 引擎提供（注：`IDENTITY_BACKEND` 标志已移除，auth9_oidc 是唯一后端）
- 已获取有效 identity token（`$TOKEN`）

### 初始状态
- 用户已登录，持有有效 access token

### 目的
验证已认证用户可获取自己的 pending actions 列表

### 测试操作流程
1. 获取 API token：
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

2. 查询 pending actions：
```bash
curl -s http://localhost:8080/api/v1/hosted-login/pending-actions \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体为数组（可能为空或包含 action 对象）：
```json
[]
```
或：
```json
[
  {
    "id": "act-xxx",
    "action_type": "update_password",
    "redirect_url": "/force-update-password"
  }
]
```

---

## 场景 2：完成 Pending Action

### 步骤 0（Gate Check）
- 已获取有效 identity token（`$TOKEN`）
- 数据库中存在该用户的 pending action

### 初始状态
- 用户存在一个 `status = 'pending'` 的 action 记录

### 目的
验证通过 API 可标记一个 pending action 为已完成

### 测试操作流程
1. 先创建一个 pending action（通过数据库或登录时自动触发）：
```sql
INSERT INTO auth9.pending_actions (id, user_id, action_type, status, metadata, created_at)
VALUES (
  UUID(),
  (SELECT identity_subject FROM auth9.users WHERE email = 'qa-user@example.com'),
  'complete_profile',
  'pending',
  '{}',
  NOW()
);
```

2. 获取 action ID：
```sql
SELECT id, action_type, status FROM auth9.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'qa-user@example.com')
AND status = 'pending';
```

3. 完成 action：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/complete-action \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"action_id": "<ACTION_ID>"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体：
```json
{
  "message": "Action completed successfully."
}
```

### 预期数据状态
```sql
-- Action 状态已更新
SELECT id, action_type, status, completed_at
FROM auth9.pending_actions
WHERE id = '<ACTION_ID>';
-- 预期: status = 'completed', completed_at IS NOT NULL

-- 验证审计日志
SELECT action, resource_type, created_at
FROM auth9.audit_logs
WHERE action = 'required_action.completed'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 行
```

---

## 场景 3：Force Update Password 页面 — 未认证跳转

### 初始状态
- 用户未登录（无 session）

### 目的
验证 `/force-update-password` 页面在未认证时重定向到登录页

### 测试操作流程
1. 在无痕窗口中直接访问：
```
http://localhost:3000/force-update-password?action_id=test-123
```

### 预期结果
- 页面重定向到 `/login`
- 未显示强制更新密码表单

> **故障排除**
>
> | 症状 | 原因 | 解决方案 |
> |------|------|---------|
> | 页面显示密码表单而非重定向 | 浏览器中已有有效 `auth9_session` cookie（之前登录未完全退出） | **必须使用无痕窗口**或在测试前手动清除 `auth9_session` cookie |
> | 页面显示密码表单（无痕窗口） | 不应出现；若发生请提交 bug | 检查 Portal 日志确认 `getAccessToken()` 返回值 |

---

## 场景 4：Complete Profile 页面渲染与提交

### 步骤 0（Gate Check）
- 用户已登录 Portal（有有效 session）——**需要一个有密码凭证且无 MFA 的测试用户**。默认 Docker 环境中 `admin@auth9.local` 可能启用了 MFA，OIDC 创建的用户无密码。如果无法通过密码登录 Portal，此场景应标记为 BLOCKED 而非 FAILED
- 存在 `complete_profile` 类型的 pending action

### 初始状态
- 用户已登录
- `pending_actions` 表中存在 `complete_profile` action

### 目的
验证 Complete Profile 页面可正确渲染并提交 display_name

### 测试操作流程
1. 在已登录状态下访问 `/complete-profile?action_id=<ACTION_ID>`
2. 确认页面显示：
   - 品牌 Logo 或默认 BrandMark
   - 标题文本（补充 profile 相关）
   - Display Name 输入框
   - 提交按钮
3. 输入 Display Name：`QA Test User`
4. 点击提交按钮

### 预期结果
- 页面正常渲染，由 Auth9 Portal 托管
- 提交成功后重定向到 `/tenant/select`
- `users.display_name` 已更新为 `QA Test User`
- Pending action 标记为 `completed`

### 预期数据状态
```sql
-- 用户 display_name 已更新
SELECT display_name FROM auth9.users WHERE email = 'qa-user@example.com';
-- 预期: 'QA Test User'

-- Action 已完成
SELECT status, completed_at FROM auth9.pending_actions
WHERE id = '<ACTION_ID>';
-- 预期: status = 'completed'
```

---

## 场景 5：登录后 Pending Action 自动跳转

### 步骤 0（Gate Check）
- Auth9 Core 和 Portal 服务均运行中
- **测试用户必须拥有密码凭证**（通过 `reset-docker.sh` 创建，或使用 `admin@auth9.local`）
- 用户存在 pending action

> **重要 — `pending_actions.user_id` 列存的是 `identity_subject` 而非 `users.id`**
> 切勿直接 INSERT 一个 UUID 到 user_id 列。必须使用 `(SELECT identity_subject FROM users WHERE email='...')` 子查询，否则登录后的 `check_post_login_actions` 查不到该 action，导致测试假阴性（看到登录跳转到 `/tenant/select` 而非 `/force-update-password`）。

### 初始状态
- 系统中存在有密码凭证的测试用户（如 `admin@auth9.local` / `Auth9Dev!2026x`）
- 该用户有一个 `update_password` 类型的 pending action

> **注意**: `qa-user@example.com` 默认通过 OIDC 创建，**没有密码凭证**，无法用于密码登录场景。请使用 `admin@auth9.local` 或在 `reset-docker.sh` 中为 qa-user 设置密码。

### 目的
验证密码登录成功后，如果用户有 pending actions，自动跳转到 action 页面而非 tenant select

### 测试操作流程
1. 确保存在 pending action：
```sql
INSERT INTO auth9.pending_actions (id, user_id, action_type, status, metadata, created_at)
VALUES (
  UUID(),
  (SELECT identity_subject FROM auth9.users WHERE email = 'qa-user@example.com'),
  'update_password',
  'pending',
  '{}',
  NOW()
);
```

2. 在 Portal 登录页 (`http://localhost:3000/login`) 使用用户凭证登录

3. 观察登录后的跳转行为

### 预期结果
- 登录成功后**不**跳转到 `/tenant/select`
- 而是跳转到 `/force-update-password?action_id=<action_id>`
- 强制更新密码页面正确渲染
- 页面由 Auth9 域名托管（`localhost:3000`），由 Auth9 Portal 渲染
- 完成密码更新后，跳转到 `/tenant/select`
