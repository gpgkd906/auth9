# 密码最大年龄强制执行 - 登录时密码过期拦截

**模块**: 集成测试 / 认证流程
**测试范围**: `max_age_days` 密码策略在登录时的强制执行，包括非 MFA 和 MFA 路径
**场景数**: 5
**优先级**: 高

---

## 背景说明

当租户密码策略中设置 `max_age_days > 0` 时，登录流程会在密码验证成功后检查 `password_changed_at` 是否超过允许天数。若密码过期，系统自动创建 `UPDATE_PASSWORD` required action，将用户重定向至 `/force-update-password` 页面强制修改密码后才能完成登录。

端点：
- `PUT /api/v1/tenants/{id}/password-policy` — 设置密码策略
- `POST /api/v1/hosted-login/password` — 密码登录（检查密码年龄）
- `POST /api/v1/mfa/challenge/totp` — MFA 验证后也检查密码年龄
- `GET /api/v1/hosted-login/pending-actions` — 查询 pending actions

---

## 数据库表结构参考

### users 表（相关字段）
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| password_changed_at | TIMESTAMP NULL | 密码最后修改时间 |

### pending_actions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 关联用户 identity_subject |
| action_type | VARCHAR(64) | 动作类型（如 `update_password`） |
| status | VARCHAR(16) | 状态：pending / completed / cancelled |

---

## 步骤 0：Gate Check

```bash
# 生成管理员 Token
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 获取测试租户 ID
TENANT_ID=$(curl -sf http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

# 获取测试用户信息
USER_EMAIL="user@example.com"
```

---

## 场景 1：密码过期用户登录被拦截并重定向到密码修改页面

### 初始状态
- 租户密码策略 `max_age_days: 1`
- 测试用户 `password_changed_at` 被手动设为 2 天前

### 目的
验证密码过期后登录流程创建 `UPDATE_PASSWORD` action 并在响应中返回 pending_actions

### 测试操作流程
1. 设置密码策略：
   ```bash
   curl -s -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"max_age_days": 1}' | jq .
   ```
2. 手动将用户 `password_changed_at` 设为过去：
   ```sql
   UPDATE users SET password_changed_at = DATE_SUB(NOW(), INTERVAL 2 DAY)
   WHERE email = 'user@example.com';
   ```
3. 使用该用户尝试密码登录：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "TestPassword123!"}' | jq .
   ```

### 预期结果
- 步骤 1: 返回 200，策略包含 `max_age_days: 1`
- 步骤 3: 返回 200，响应中包含 `pending_actions` 数组，第一个元素：
  - `action_type: "update_password"`
  - `redirect_url: "/force-update-password"`

### 预期数据状态
```sql
SELECT action_type, status FROM pending_actions
WHERE user_id = '{identity_subject}' AND action_type = 'update_password' AND status = 'pending';
-- 预期: 1 行，status = 'pending'
```

---

## 场景 2：密码未过期用户正常登录

### 初始状态
- 租户密码策略 `max_age_days: 90`
- 测试用户 `password_changed_at` 为当前时间

### 目的
验证密码未过期时登录正常完成，无 pending_actions

### 测试操作流程
1. 设置密码策略：
   ```bash
   curl -s -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"max_age_days": 90}' | jq .
   ```
2. 更新用户 `password_changed_at` 为当前时间：
   ```sql
   UPDATE users SET password_changed_at = NOW() WHERE email = 'user@example.com';
   ```
3. 清除已有 pending actions：
   ```sql
   DELETE FROM pending_actions WHERE user_id = '{identity_subject}' AND action_type = 'update_password';
   ```
4. 使用该用户登录：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "TestPassword123!"}' | jq .
   ```

### 预期结果
- 步骤 4: 返回 200，响应中 `pending_actions` 为空或不存在
- 返回有效的 `access_token`

### 预期数据状态
```sql
SELECT COUNT(*) FROM pending_actions
WHERE user_id = '{identity_subject}' AND action_type = 'update_password' AND status = 'pending';
-- 预期: 0
```

---

## 场景 3：`max_age_days=0` 时不启用密码年龄检查

### 初始状态
- 租户密码策略 `max_age_days: 0`（默认值，禁用）
- 测试用户 `password_changed_at` 为 NULL 或很久以前

### 目的
验证 `max_age_days=0` 时无论密码多老都不触发过期拦截

### 测试操作流程
1. 设置密码策略：
   ```bash
   curl -s -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"max_age_days": 0}' | jq .
   ```
2. 将用户 `password_changed_at` 设为 NULL：
   ```sql
   UPDATE users SET password_changed_at = NULL WHERE email = 'user@example.com';
   ```
3. 清除已有 pending actions：
   ```sql
   DELETE FROM pending_actions WHERE user_id = '{identity_subject}' AND action_type = 'update_password';
   ```
4. 使用该用户登录：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "TestPassword123!"}' | jq .
   ```

### 预期结果
- 步骤 4: 返回 200，无 `pending_actions`
- 正常返回 `access_token`

### 预期数据状态
```sql
SELECT COUNT(*) FROM pending_actions
WHERE user_id = '{identity_subject}' AND action_type = 'update_password' AND status = 'pending';
-- 预期: 0
```

---

## 场景 4：`password_changed_at` 为 NULL 且 `max_age_days > 0` 时视为过期

### 初始状态
- 租户密码策略 `max_age_days: 30`
- 测试用户 `password_changed_at = NULL`（历史用户，密码修改时间未记录）

### 目的
验证 `password_changed_at` 为 NULL 时，只要 `max_age_days > 0` 即视为过期

### 测试操作流程
1. 设置密码策略：
   ```bash
   curl -s -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy" \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"max_age_days": 30}' | jq .
   ```
2. 将用户 `password_changed_at` 设为 NULL：
   ```sql
   UPDATE users SET password_changed_at = NULL WHERE email = 'user@example.com';
   ```
3. 清除已有 pending actions：
   ```sql
   DELETE FROM pending_actions WHERE user_id = '{identity_subject}' AND action_type = 'update_password';
   ```
4. 使用该用户登录：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "TestPassword123!"}' | jq .
   ```

### 预期结果
- 步骤 4: 返回 200，包含 `pending_actions`，action_type = `update_password`

### 预期数据状态
```sql
SELECT action_type, status FROM pending_actions
WHERE user_id = '{identity_subject}' AND action_type = 'update_password' AND status = 'pending';
-- 预期: 1 行
```

---

## 场景 5：密码过期用户通过强制密码修改后成功登录

### 初始状态
- 租户密码策略 `max_age_days: 1`
- 测试用户密码已过期（`password_changed_at` 为 2 天前）
- 用户已通过场景 1 获取到带 `UPDATE_PASSWORD` pending action 的 access_token

### 目的
验证用户通过 `/force-update-password` 页面修改密码后，action 被完成，`password_changed_at` 更新，再次登录无拦截

### 测试操作流程
1. 使用场景 1 获得的 access_token 调用强制修改密码：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/force-change-password \
     -H "Authorization: Bearer $ACCESS_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"new_password": "NewSecurePass456!"}' | jq .
   ```
2. 完成 pending action：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/complete-action \
     -H "Authorization: Bearer $ACCESS_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"action_id": "{action_id}"}' | jq .
   ```
3. 使用新密码重新登录：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
     -H "Content-Type: application/json" \
     -d '{"email": "user@example.com", "password": "NewSecurePass456!"}' | jq .
   ```

### 预期结果
- 步骤 1: 返回 200，密码修改成功
- 步骤 2: 返回 200，action 标记为 completed
- 步骤 3: 返回 200，无 `pending_actions`，正常获取 `access_token`

### 预期数据状态
```sql
-- 验证 password_changed_at 已更新为当前时间
SELECT password_changed_at FROM users WHERE email = 'user@example.com';
-- 预期: 接近当前时间

-- 验证 action 已完成
SELECT status FROM pending_actions
WHERE user_id = '{identity_subject}' AND action_type = 'update_password'
ORDER BY created_at DESC LIMIT 1;
-- 预期: completed
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 密码过期用户登录被拦截 | ☐ | | | |
| 2 | 密码未过期用户正常登录 | ☐ | | | |
| 3 | max_age_days=0 不启用检查 | ☐ | | | |
| 4 | password_changed_at 为 NULL 视为过期 | ☐ | | | |
| 5 | 强制修改密码后成功登录 | ☐ | | | |
