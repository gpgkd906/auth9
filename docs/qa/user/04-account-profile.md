# 用户账户 - 个人资料与自更新 API

**模块**: 用户管理
**测试范围**: `GET/PUT /api/v1/users/me` 端点、Profile 页面编辑、自更新权限
**场景数**: 5
**优先级**: 高

---

## 背景说明

新增 `/api/v1/users/me` 端点，允许已认证用户获取和更新自己的个人资料（`display_name`、`avatar_url`）。同时修改了 `PUT /api/v1/users/:id` 端点，允许用户自更新而无需管理员权限。

端点：
- `GET /api/v1/users/me` — 获取当前用户信息
- `PUT /api/v1/users/me` — 更新当前用户的 display_name / avatar_url

---

## 数据库表结构参考

### users 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| identity_subject | VARCHAR(255) | 身份主体 ID |
| email | VARCHAR(255) | 邮箱（唯一） |
| display_name | VARCHAR(255) | 显示名称 |
| avatar_url | TEXT | 头像 URL |
| mfa_enabled | BOOLEAN | 是否启用 MFA |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 前置条件（浏览器登录场景）

> **种子用户登录限制**：
> - `testuser*@example.com` 系列用户在种子数据中**未设置密码**，无法直接浏览器登录
> - `admin@auth9.local` 要求 WebAuthn MFA，无法在普通浏览器环境完成认证
> - **推荐浏览器测试账号**：`mfa-user@auth9.local`，密码 `Auth9Dev!2026x`，配合 `reset-docker.sh` 输出的 TOTP secret 完成 MFA
> - **API 测试替代**：不涉及 UI 交互的场景（如场景 1、2、4），优先使用 Access Token 直接调用 API

---

## 场景 1：通过 API 获取当前用户信息

### 初始状态
- 用户已登录，持有有效的 Access Token

### 目的
验证 `GET /api/v1/users/me` 端点返回当前认证用户的完整信息

### 测试操作流程
1. 使用有效 Access Token 调用 API：
   ```bash
   curl -s -X GET http://localhost:8080/api/v1/users/me \
     -H "Authorization: Bearer {access_token}" | jq
   ```
2. 检查返回的用户数据字段

### 预期结果
- HTTP 200
- 返回 JSON 包含 `id`、`email`、`display_name`、`avatar_url`、`mfa_enabled`、`created_at` 等字段
- 返回数据与 JWT Token 中的 user_id 对应的用户一致

### 预期数据状态
```sql
SELECT id, email, display_name, avatar_url, mfa_enabled FROM users WHERE id = '{user_id}';
-- 预期: 返回结果与 API 响应一致
```

---

## 场景 2：未认证用户访问 /api/v1/users/me

### 初始状态
- 无有效 Access Token

### 目的
验证未认证请求被正确拒绝

### 测试操作流程
1. 不带 Authorization header 调用 API：
   ```bash
   curl -s -X GET http://localhost:8080/api/v1/users/me | jq
   ```
2. 使用过期或无效 token 调用：
   ```bash
   curl -s -X GET http://localhost:8080/api/v1/users/me \
     -H "Authorization: Bearer invalid-token" | jq
   ```

### 预期结果
- HTTP 401 Unauthorized
- 返回错误信息

---

## 场景 3：Account 入口可见性与 Profile 编辑显示名称和头像

### 初始状态
- 用户已登录并进入 `/dashboard/account` 页面
- 当前 display_name 为 `Test User`

### 目的
验证用户可以从侧边栏可见入口进入 Account Profile，并成功修改显示名称和头像 URL

### 测试操作流程
1. 在任意 Dashboard 页面，确认左侧边栏底部存在当前用户卡片入口
2. 点击用户卡片进入「Account / Profile」页面
3. 确认当前用户信息显示正确（头像、名称、邮箱、MFA 状态、加入日期）
4. 在「Display name」输入框中修改为 `Updated Name`
5. 在「Avatar URL」输入框中填写 `https://example.com/new-avatar.png`
6. 确认「Email」字段为只读状态（灰色禁用）
7. 点击「Save changes」

### 预期结果
- 显示成功提示 "Profile updated successfully"
- 页面上的头像预览更新为新 URL
- 侧边栏底部用户名更新为 `Updated Name`
- 侧边栏用户卡片入口可见且可点击进入 Profile 页面

### 预期数据状态
```sql
SELECT display_name, avatar_url, updated_at FROM users WHERE id = '{user_id}';
-- 预期: display_name = 'Updated Name', avatar_url = 'https://example.com/new-avatar.png'

-- 审计日志验证
SELECT action, resource_type, resource_id FROM audit_logs
WHERE resource_type = 'user' AND resource_id = '{user_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'user.update'
```

---

## 场景 4：自更新不需要管理员权限；他人资料不可更新

### 初始状态
- 普通用户（**member 角色**，非 admin/owner）已登录
- 用户持有有效 Tenant Access Token

### 步骤 0：验证 Token 角色为 member

**测试前必须确认 Token 携带的角色不包含 admin/owner：**

```bash
# 解码 tenant access token 的 payload（注意 token 是 JWT，取第二段 base64）
echo "{token}" | cut -d. -f2 | base64 -d 2>/dev/null | jq '.roles, .permissions'
# 预期: roles 中不含 "admin" 或 "owner"
# 若 roles 包含 "admin" 或 "owner"，则该用户有权限更新他人，测试结果为预期行为而非 Bug
```

> **重要**: `PUT /api/v1/users/{id}` 对 member 角色（无 `user:write` 等权限）
> 更新他人资料会返回 403。Admin/Owner 角色有权限更新同租户内其他用户。
> 若测试时发现 200 成功，首先检查 Token 角色是否为 admin/owner。

### 目的
1. 验证普通用户可以通过 `PUT /api/v1/users/me` 更新自己的资料，无需 admin 权限
2. 验证普通用户（member）无法通过 `PUT /api/v1/users/{other_id}` 更新他人资料

### 测试操作流程
1. 使用普通用户（member 角色）的 Tenant Access Token 调用自更新 API：
   ```bash
   curl -s -X PUT http://localhost:8080/api/v1/users/me \
     -H "Authorization: Bearer {member_user_token}" \
     -H "Content-Type: application/json" \
     -d '{"display_name": "Self Updated Name"}' | jq
   ```
2. 确认更新成功（HTTP 200）
3. 用同一 member token 尝试更新其他用户的资料：
   ```bash
   curl -s -X PUT http://localhost:8080/api/v1/users/{other_user_id} \
     -H "Authorization: Bearer {member_user_token}" \
     -H "Content-Type: application/json" \
     -d '{"display_name": "Hacked Name"}' | jq
   ```

### 预期结果
- 自更新：HTTP 200，display_name 被成功更新
- 更新他人：HTTP 403 Forbidden，操作被拒绝

> **故障排除**
>
> | 症状 | 原因 | 解决方案 |
> |------|------|---------|
> | 更新他人返回 200（而非 403）| Token 实际是 admin/owner 角色 | 执行步骤 0 验证 Token 角色 |
> | 更新他人返回 404（而非 403）| 目标用户不在当前租户 | 使用同一租户内的目标用户 ID |
>
> **权限检查说明**：`PUT /api/v1/users/{id}` 对非自身用户执行 `PolicyAction::UserManage` 权限检查，要求 `user:write`、`user:delete`、`user:*` 或 `rbac:*` 中的任一权限。`user:read` 单独不足以更新他人资料。如果测试中更新他人返回 200，首先确认 token 的 permissions 字段——`gen_tenant_access_token.js` 默认生成的 token 包含 `rbac:*,user:*`，会授予写权限。生成仅含 member 权限的 token：
> ```bash
> node gen_tenant_access_token.js "$USER_ID" "$TENANT_ID" "member" ""
> ```

### 预期数据状态
```sql
SELECT display_name FROM users WHERE id = '{current_user_id}';
-- 预期: 'Self Updated Name'

SELECT display_name FROM users WHERE id = '{other_user_id}';
-- 预期: 保持原值不变
```

---

## 场景 5：Profile 页面 API 失败处理

### 初始状态
- 用户已登录并进入 Profile 页面
- 后端 API 出现异常（如网络中断或服务不可用）

### 目的
验证 Profile 页面在 API 调用失败时的错误处理

### 测试操作流程
1. 导航至 `/dashboard/account`
2. 修改显示名称为有效值
3. 模拟 API 失败（停止后端服务或断开网络）
4. 点击「Save changes」

### 预期结果
- 按钮显示 "Saving..." 加载状态
- 失败后显示红色错误提示（允许文案为通用网络错误，例如 "Unable to connect to the server. Please try again later."）
- 页面不崩溃，不应跳转到 `/tenant/select?error=tenant_exchange_failed`
- 表单区域保持可见，用户可继续编辑或重试

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 通过 API 获取当前用户信息 | ☐ | | | |
| 2 | 未认证用户访问 /api/v1/users/me | ☐ | | | |
| 3 | 通过 Profile 页面编辑显示名称和头像 | ☐ | | | |
| 4 | 自更新不需要管理员权限 | ☐ | | | |
| 5 | Profile 页面 API 失败处理 | ☐ | | | |

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Normal user can update other users via PUT /api/v1/users/{id} (returns 200 instead of 403) | `gen_tenant_access_token.js` defaults to `role='admin'` and `permissions='rbac:*,user:*,...'`. The generated token has admin privileges regardless of the user's actual DB roles. | Pass explicit role and permissions: `node gen_tenant_access_token.js "$USER_ID" "$TENANT_ID" "member" ""` to simulate a non-admin user. |
| Self-update returns 403 | Token type is Identity Token instead of Tenant Access Token. The `PUT /api/v1/users/{id}` endpoint requires a Tenant Access Token for self-update (where `auth.user_id == id`). | Use `gen_tenant_access_token.js` to generate a Tenant Access Token with the correct user ID. |
