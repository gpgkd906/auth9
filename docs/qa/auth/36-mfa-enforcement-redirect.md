# Auth - MFA 强制配置重定向

**模块**: Auth
**测试范围**: 登录时自动创建 CONFIGURE_TOTP required action、Portal 重定向到 TOTP 设置页、配置完成后正常流程恢复
**场景数**: 4
**优先级**: 高

---

## 背景说明

当管理员为用户启用 MFA（`mfa_enabled=true`）但用户尚未配置任何 MFA 凭证（TOTP / WebAuthn）时，登录流程应自动创建 `CONFIGURE_TOTP` pending action，强制用户在进入 tenant 选择 / dashboard 前完成 TOTP 配置。

核心逻辑位于 `check_post_login_actions()`：
- 条件：`mfa_enabled=true && !has_mfa_credential && 无已存在的 CONFIGURE_TOTP action`
- 结果：自动创建 `CONFIGURE_TOTP` action，返回在 `pending_actions` 中

---

> **前置条件**: 测试用户 `mfa-user@auth9.local` 需通过 `./scripts/reset-docker.sh` 预置。

## 场景 1：MFA 启用 + 无凭证 — 登录返回 CONFIGURE_TOTP Action [DEFERRED - pending FR: auth_mfa_configure_totp_on_login]

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中：`curl -sf http://localhost:8080/health`
- 已获取管理员 token（`$TOKEN`）
- 存在一个测试用户，`mfa_enabled=true`，且无 TOTP/WebAuthn 凭证

### 初始状态
- 测试用户 `mfa-user@auth9.local` 已存在
- 用户 `mfa_enabled=true`
- 用户无 TOTP/WebAuthn 凭证记录

### 目的
验证登录成功后，API 响应的 `pending_actions` 中包含 `CONFIGURE_TOTP` action

### 测试操作流程

1. 获取管理员 token：
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

2. 准备测试用户（确保 mfa_enabled=true 且无 TOTP 凭证）：
```bash
# 查找用户
USER_ID=$(curl -s http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" | jq -r '.items[] | select(.email=="mfa-user@auth9.local") | .id')

# 启用 MFA
curl -s -X PUT "http://localhost:8080/api/v1/users/${USER_ID}/mfa" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"mfa_enabled": true}' | jq .

# 删除现有 TOTP 凭证（如有）
```

3. 使用测试用户密码登录：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "mfa-user@auth9.local", "password": "Test1234!"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体包含 `pending_actions` 数组，其中有一个 `CONFIGURE_TOTP` action：
```json
{
  "access_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "pending_actions": [
    {
      "id": "<ACTION_ID>",
      "action_type": "CONFIGURE_TOTP",
      "redirect_url": "/mfa/setup-totp"
    }
  ]
}
```

### 预期数据状态
```sql
SELECT id, action_type, status FROM auth9_oidc.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'mfa-user@auth9.local')
AND action_type = 'CONFIGURE_TOTP'
AND status = 'pending';
-- 预期: 1 行
```

---

## 场景 2：TOTP 配置完成后恢复正常流程

### 步骤 0（Gate Check）
- 场景 1 已通过（用户有 CONFIGURE_TOTP pending action）
- 用户已登录并持有有效 identity token

### 初始状态
- 用户有 `CONFIGURE_TOTP` pending action
- 用户已获得 identity token（来自场景 1 的登录响应）

### 目的
验证用户完成 TOTP 配置后，action 被标记为 completed，再次登录不再出现 CONFIGURE_TOTP

### 测试操作流程

1. 使用场景 1 返回的 identity token，启动 TOTP 注册：
```bash
IDENTITY_TOKEN="<场景1的access_token>"
curl -s -X POST http://localhost:8080/api/v1/mfa/totp/enroll \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password": "Test1234!"}' | jq .
```

2. 记录 `setup_token` 和 `secret`，使用 TOTP 工具生成验证码后完成注册：
```bash
# 使用返回的 setup_token 和生成的 OTP code
curl -s -X POST http://localhost:8080/api/v1/mfa/totp/enroll/verify \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"setup_token": "<SETUP_TOKEN>", "code": "<OTP_CODE>"}' | jq .
```

3. 完成 pending action：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/complete-action \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"action_id": "<ACTION_ID>"}' | jq .
```

4. 再次登录，验证不再有 pending actions：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "mfa-user@auth9.local", "password": "Test1234!"}' | jq .
```

### 预期结果
- 步骤 2：TOTP 注册成功，返回 200
- 步骤 3：Action 完成成功，返回 200
- 步骤 4：再次登录时，因 TOTP 已配置，触发 MFA 验证（`mfa_required: true`），而非 CONFIGURE_TOTP pending action

### 预期数据状态
```sql
-- Action 已完成
SELECT id, action_type, status, completed_at FROM auth9_oidc.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'mfa-user@auth9.local')
AND action_type = 'CONFIGURE_TOTP';
-- 预期: status = 'completed', completed_at IS NOT NULL

-- TOTP 凭证已创建
SELECT id, credential_type, is_active FROM auth9_oidc.credentials
WHERE user_id = (SELECT id FROM auth9.users WHERE email = 'mfa-user@auth9.local')
AND credential_type = 'totp';
-- 预期: 1 行, is_active = 1
```

---

## 场景 3：MFA 启用 + TOTP 已配置 — 无 CONFIGURE_TOTP Action

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中
- 存在一个 `mfa_enabled=true` 且已配置 TOTP 的用户

### 初始状态
- 测试用户已启用 MFA 且已有 TOTP 凭证（场景 2 完成后的状态）

### 目的
验证已配置 TOTP 的用户登录时不会生成 CONFIGURE_TOTP pending action

### 测试操作流程
1. 使用已配置 TOTP 的用户登录：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "mfa-user@auth9.local", "password": "Test1234!"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体为 MFA challenge（`mfa_required: true`），而非包含 CONFIGURE_TOTP 的 pending_actions
```json
{
  "mfa_required": true,
  "mfa_session_token": "...",
  "mfa_methods": ["totp"],
  "expires_in": 300
}
```

### 预期数据状态
```sql
SELECT id, action_type, status FROM auth9_oidc.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'mfa-user@auth9.local')
AND action_type = 'CONFIGURE_TOTP'
AND status = 'pending';
-- 预期: 0 行（无新建的 pending action）
```

---

## 场景 4：MFA 未启用 + 无 TOTP — 无 CONFIGURE_TOTP Action

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中
- 存在一个 `mfa_enabled=false` 且无 TOTP 凭证的普通用户

### 初始状态
- 用户 `qa-user@example.com`，`mfa_enabled=false`，无 TOTP 凭证

### 目的
验证 MFA 未启用的用户登录时不会生成 CONFIGURE_TOTP pending action

### 测试操作流程
1. 使用普通用户登录：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com", "password": "Test1234!"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体包含 `access_token`，`pending_actions` 为空或不包含 `CONFIGURE_TOTP`：
```json
{
  "access_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "pending_actions": []
}
```

---

## 检查清单

- [ ] MFA 启用 + 无 MFA 凭证 → 登录返回 CONFIGURE_TOTP pending action
- [ ] TOTP 配置完成后 → action 标记 completed，再次登录触发 MFA 验证而非配置
- [ ] MFA 启用 + TOTP 已配置 → 不生成新的 CONFIGURE_TOTP action
- [ ] MFA 未启用 → 不生成 CONFIGURE_TOTP action
- [ ] 已存在 CONFIGURE_TOTP pending action 时 → 不重复创建
