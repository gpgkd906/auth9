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
| keycloak_id | VARCHAR(255) | Keycloak 用户 ID |
| email | VARCHAR(255) | 邮箱（唯一） |
| display_name | VARCHAR(255) | 显示名称 |
| avatar_url | TEXT | 头像 URL |
| mfa_enabled | BOOLEAN | 是否启用 MFA |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

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

## 场景 3：通过 Profile 页面编辑显示名称和头像

### 初始状态
- 用户已登录并进入 `/dashboard/account` 页面
- 当前 display_name 为 `Test User`

### 目的
验证 Profile 页面可以成功修改用户的显示名称和头像 URL，并同步到 Keycloak

### 测试操作流程
1. 导航至 `/dashboard/account`
2. 确认当前用户信息显示正确（头像、名称、邮箱、MFA 状态、加入日期）
3. 在「Display name」输入框中修改为 `Updated Name`
4. 在「Avatar URL」输入框中填写 `https://example.com/new-avatar.png`
5. 确认「Email」字段为只读状态（灰色禁用）
6. 点击「Save changes」

### 预期结果
- 显示成功提示 "Profile updated successfully"
- 页面上的头像预览更新为新 URL
- 侧边栏底部用户名更新为 `Updated Name`

### 预期数据状态
```sql
SELECT display_name, avatar_url, updated_at FROM users WHERE id = '{user_id}';
-- 预期: display_name = 'Updated Name', avatar_url = 'https://example.com/new-avatar.png'

-- 审计日志验证
SELECT action, resource_type, resource_id FROM audit_logs
WHERE resource_type = 'user' AND resource_id = '{user_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'update'
```

---

## 场景 4：自更新不需要管理员权限

### 初始状态
- 普通用户已登录（非管理员角色）
- 用户持有有效 Access Token

### 目的
验证普通用户可以通过 `PUT /api/v1/users/me` 更新自己的资料，无需 admin 权限

### 测试操作流程
1. 使用普通用户的 Access Token 调用自更新 API：
   ```bash
   curl -s -X PUT http://localhost:8080/api/v1/users/me \
     -H "Authorization: Bearer {normal_user_token}" \
     -H "Content-Type: application/json" \
     -d '{"display_name": "Self Updated Name"}' | jq
   ```
2. 确认更新成功
3. 尝试用同一 token 更新其他用户：
   ```bash
   curl -s -X PUT http://localhost:8080/api/v1/users/{other_user_id} \
     -H "Authorization: Bearer {normal_user_token}" \
     -H "Content-Type: application/json" \
     -d '{"display_name": "Hacked Name"}' | jq
   ```

### 预期结果
- 自更新：HTTP 200，display_name 被成功更新
- 更新他人：HTTP 403 Forbidden，操作被拒绝

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
- 失败后显示红色错误提示 "Failed to update profile"
- 页面不崩溃，表单保持可编辑状态

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 通过 API 获取当前用户信息 | ☐ | | | |
| 2 | 未认证用户访问 /api/v1/users/me | ☐ | | | |
| 3 | 通过 Profile 页面编辑显示名称和头像 | ☐ | | | |
| 4 | 自更新不需要管理员权限 | ☐ | | | |
| 5 | Profile 页面 API 失败处理 | ☐ | | | |
