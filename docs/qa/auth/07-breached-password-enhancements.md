# 泄露密码增强（Breach Check Enhancements）

**模块**: auth (Password Security)
**测试范围**: 登录时异步泄露检查（D1）、租户级 PasswordPolicy 配置（D2）、Warn 模式 `password_warning` 响应（D3）
**场景数**: 5
**优先级**: 高
**前置文档**: [32-breached-password-detection.md](./32-breached-password-detection.md)（基础 HIBP 拦截）

---

## 背景说明

在基础泄露密码检测（doc 32）的基础上，本增强功能引入三项能力：

**D1 — Login-time async breach check**:
- 用户通过密码登录成功后，系统异步调用 HIBP API 检查当前密码
- 若检测到泄露，自动创建 `update_password` 类型的 required action
- 用户下次登录时被引导至强制更新密码页面

**D2 — 租户级 PasswordPolicy 配置**:
- `breach_check_mode`: `block` | `warn` | `disabled`（默认 `block`）
- `min_breach_count`: 最小泄露次数阈值（HIBP 返回的 count >= 此值才触发）
- `breach_check_on_login`: 是否在登录时执行异步泄露检查（boolean）

**D3 — Warn 模式**:
- 当 `breach_check_mode = "warn"` 时，注册/修改密码/重置密码使用泄露密码**不被拦截**
- 操作成功，但响应中附带 `password_warning` 字段提示用户

**受影响端点**:
- `PUT /api/v1/tenants/{id}/password-policy` — 更新密码策略（含 breach 字段）
- `POST /api/v1/users` — 注册（warn 模式返回 `password_warning`）
- `POST /api/v1/auth/reset-password` — 重置密码（warn 模式返回 `password_warning`）
- `POST /api/v1/users/me/password` — 修改密码（warn 模式返回 `password_warning`）
- `POST /api/v1/hosted-login/password` — 登录（触发异步 breach check）

**测试用密码**:
- 已知泄露：`password`（HIBP 中出现 3.8M+ 次）
- 已知泄露：`123456`（HIBP 中出现 40M+ 次）
- 安全密码：`Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!`（极长随机字符串，不太可能出现在 HIBP 中）

---

## 场景 1：租户级 PasswordPolicy breach 字段配置（D2）

### 步骤 0（Gate Check）
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

# 获取管理员 Token
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $TOKEN | head -c 20
# 预期: 输出 JWT 前缀

# 获取测试租户 ID
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')
echo $TENANT_ID
# 预期: 非空 UUID
```

### 初始状态
- Auth9 Core 运行中
- 存在至少一个租户

### 目的
验证 `PUT /api/v1/tenants/{id}/password-policy` 端点正确接受并持久化 breach 相关字段（`breach_check_mode`、`min_breach_count`、`breach_check_on_login`）

### 测试操作流程

#### API 测试
1. 设置 PasswordPolicy 为 warn 模式，min_breach_count=10，启用登录检查：
```bash
curl -s -w "\n%{http_code}" -X PUT \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "warn",
    "min_breach_count": 10,
    "breach_check_on_login": true
  }'
```

2. 读取 PasswordPolicy 验证持久化：
```bash
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" | jq .
```

3. 设置为 disabled 模式：
```bash
curl -s -w "\n%{http_code}" -X PUT \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "disabled",
    "min_breach_count": 1,
    "breach_check_on_login": false
  }'
```

4. 设置非法 `breach_check_mode` 值：
```bash
curl -s -w "\n%{http_code}" -X PUT \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "invalid_mode"
  }'
```

### 预期结果
- 步骤 1：HTTP **200**，响应包含 `breach_check_mode: "warn"`, `min_breach_count: 10`, `breach_check_on_login: true`
- 步骤 2：GET 返回的字段值与步骤 1 设置一致
- 步骤 3：HTTP **200**，`breach_check_mode: "disabled"`
- 步骤 4：HTTP **400** 或 **422**，提示 `breach_check_mode` 值无效

### 预期数据状态
```sql
SELECT breach_check_mode, min_breach_count, breach_check_on_login
FROM password_policies
WHERE tenant_id = '<TENANT_ID>';
-- 预期: breach_check_mode = 'disabled', min_breach_count = 1, breach_check_on_login = 0
```

---

## 场景 2：Warn 模式 — 注册使用泄露密码返回 password_warning（D3）

> **⚠️ 关键前置条件：`POST /api/v1/users` 必须包含 `tenant_id`**
>
> 创建用户时请求体**必须**包含 `tenant_id` 字段，以便系统使用该租户的 PasswordPolicy。
> 如果不提供 `tenant_id`，系统使用默认 `PasswordPolicy`（`breach_check_mode: "block"`），
> 即使你已将租户策略设为 `warn`，注册泄露密码仍会返回 422 而非 201。

### 步骤 0（Gate Check）
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .

TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 获取测试租户 ID
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

# 将 breach_check_mode 设置为 warn
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"breach_check_mode": "warn", "min_breach_count": 1}'
# 预期: HTTP 200
```

### 初始状态
- 租户 PasswordPolicy `breach_check_mode = "warn"`
- HIBP API 可达

### 目的
验证 warn 模式下，使用泄露密码注册**成功**，但响应中包含 `password_warning` 字段

### 测试操作流程

#### API 测试
1. 使用泄露密码 `password` 注册用户：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "warn-mode-reg@example.com",
    "password": "password", <!-- pragma: allowlist secret -->
    "name": "Warn Mode Registration Test",
    "tenant_id": "'"$TENANT_ID"'"
  }'
# pragma: allowlist secret
```

2. 使用安全密码注册用户（对照组）：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "warn-mode-safe@example.com",
    "password": "Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!", <!-- pragma: allowlist secret -->
    "name": "Warn Mode Safe Test",
    "tenant_id": "'"$TENANT_ID"'"
  }'
# pragma: allowlist secret
```

### 预期结果
- 步骤 1：HTTP **201**（或 200），用户创建成功。响应体包含：
```json
{
  "id": "...",
  "email": "warn-mode-reg@example.com",
  "password_warning": "This password has been found in a data breach. Consider changing it." <!-- pragma: allowlist secret -->
}
```
- 步骤 2：HTTP **201**（或 200），用户创建成功。响应体**不包含** `password_warning` 字段

### 预期数据状态
```sql
SELECT id, email FROM users
WHERE email IN ('warn-mode-reg@example.com', 'warn-mode-safe@example.com');
-- 预期: 2 条记录（两个用户均创建成功）

-- 清理
DELETE FROM users WHERE email IN ('warn-mode-reg@example.com', 'warn-mode-safe@example.com');
```

---

## 场景 3：Warn 模式 — 修改密码与重置密码返回 password_warning（D3）

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 确认租户 breach_check_mode = "warn"
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" | jq '.breach_check_mode'
# 预期: "warn"（如不是，先执行场景 1 步骤 1 设置）
```

### 初始状态
- 租户 PasswordPolicy `breach_check_mode = "warn"`
- 已存在有密码凭证的测试用户
- HIBP API 可达

### 目的
验证 warn 模式下，修改密码（`POST /api/v1/users/me/password`）和重置密码（`POST /api/v1/auth/reset-password`）使用泄露密码时操作成功但返回 `password_warning`

### 测试操作流程

#### API 测试 — 修改密码
1. 使用泄露密码修改密码：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!",
    "new_password": "password"
  }'
# pragma: allowlist secret
```

#### API 测试 — 重置密码
2. 从数据库获取 reset token：
```sql
SELECT token, user_id, expires_at FROM password_reset_tokens
ORDER BY created_at DESC LIMIT 1;
-- 记录 {reset_token}
```

3. 使用泄露密码重置：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token": "{reset_token}",
    "new_password": "123456"
  }'
# pragma: allowlist secret
```

### 预期结果
- 步骤 1：HTTP **200**，密码修改成功，响应包含 `"password_warning": "This password has been found in a data breach. Consider changing it."`
- 步骤 3：HTTP **200**，密码重置成功，响应包含 `"password_warning"` 字段

> **与 block 模式对比**: 在 `breach_check_mode = "block"` 下，同样的操作会返回 HTTP 422 并拒绝修改。

---

## 场景 4：Login-time async breach check 创建 required action（D1）

> **⚠️ 关键前置条件：准备测试用户时 `POST /api/v1/users` 必须包含 `tenant_id`**
>
> 如果通过 API 创建测试用户（例如在 `disabled` 模式下创建使用泄露密码的用户），请求体**必须**包含 `tenant_id`。
> 否则系统使用默认 `PasswordPolicy`（`breach_check_mode: "block"`），创建会被拦截返回 422。

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 确认租户配置 breach_check_on_login = true
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "block",
    "min_breach_count": 1,
    "breach_check_on_login": true
  }'
# 预期: HTTP 200

# 准备测试用户：创建使用泄露密码的用户（需先 disabled 模式创建，再改回 block）
# 或使用一个已知密码为 "password" 的已有测试用户
```

### 初始状态
- 租户 `breach_check_on_login = true`
- 存在一个密码为已知泄露密码（如 `password`）的测试用户
- 该用户当前无 `update_password` 类型的 pending action
- HIBP API 可达

### 目的
验证用户通过密码登录成功后，系统异步检查 HIBP 并在发现泄露时创建 `update_password` required action

### 测试操作流程

#### 准备工作
1. 确认用户无现有 `update_password` action：
```sql
SELECT id, action_type, status FROM auth9_oidc.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'breach-login-test@example.com')
AND action_type = 'update_password' AND status = 'pending';
-- 预期: 0 行
```

#### API 测试
2. 使用泄露密码通过 hosted login 端点登录：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{
    "email": "breach-login-test@example.com",
    "password": "password"
  }'
# pragma: allowlist secret
```

3. 等待 2-3 秒（异步检查需要时间）后查询 pending actions：
```bash
sleep 3
curl -s http://localhost:8080/api/v1/hosted-login/pending-actions \
  -H "Authorization: Bearer $LOGIN_TOKEN" | jq .
```

### 预期结果
- 步骤 2：HTTP **200**，登录成功（登录本身不被 breach check 阻止）
- 步骤 3：pending actions 列表中出现 `update_password` 类型的 action：
```json
[
  {
    "id": "act-xxx",
    "action_type": "update_password",
    "redirect_url": "/force-update-password"
  }
]
```

### 预期数据状态
```sql
-- 验证 pending action 已创建
SELECT id, action_type, status, created_at
FROM auth9_oidc.pending_actions
WHERE user_id = (SELECT identity_subject FROM auth9.users WHERE email = 'breach-login-test@example.com')
AND action_type = 'update_password' AND status = 'pending';
-- 预期: 1 行，status = 'pending'
```

---

## 场景 5：min_breach_count 阈值过滤与 disabled 模式（D2）

> **⚠️ 关键前置条件：`POST /api/v1/users` 必须包含 `tenant_id`**
>
> 本场景的用户创建请求已包含 `tenant_id`（见下方 curl 示例）。如果遗漏该字段，系统使用默认 `PasswordPolicy`（`breach_check_mode: "block"`），
> 导致在高阈值或 disabled 模式下仍然返回 422，产生误报。

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')
```

### 初始状态
- Auth9 Core 运行中
- HIBP API 可达

### 目的
验证 `min_breach_count` 阈值正确过滤低频泄露密码，以及 `disabled` 模式完全跳过 HIBP 检查

### 测试操作流程

#### API 测试 — 高阈值放行
1. 设置 `min_breach_count` 为极高值（block 模式）：
```bash
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "block",
    "min_breach_count": 999999999
  }'
```

2. 使用常见泄露密码注册（HIBP count 低于阈值，密码需满足策略要求）：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "threshold-test@example.com",
    "password": "Password123!!",
    "name": "Threshold Test",
    "tenant_id": "'"$TENANT_ID"'"
  }'
# pragma: allowlist secret
```

#### API 测试 — disabled 模式
3. 设置 `breach_check_mode = "disabled"`：
```bash
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "breach_check_mode": "disabled"
  }'
```

4. 使用泄露密码注册（密码需满足策略最低要求：12+ 字符、大小写、符号）：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "disabled-mode-test@example.com",
    "password": "Password123!!",
    "name": "Disabled Mode Test",
    "tenant_id": "'"$TENANT_ID"'"
  }'
# pragma: allowlist secret
```
> **注意**: 密码 `123456` 不满足默认密码策略（12字符、大小写、符号），会导致 422 验证错误而非 breach check。必须使用满足策略但仍在 HIBP 泄露列表中的密码。

### 预期结果
- 步骤 2：HTTP **201**（或 200），因 `password` 的 HIBP count (~3.8M) 低于 999999999 阈值，不触发拦截
- 步骤 4：HTTP **201**（或 200），`disabled` 模式完全跳过 HIBP 检查，泄露密码不受任何影响

### 预期数据状态
```sql
SELECT id, email FROM users
WHERE email IN ('threshold-test@example.com', 'disabled-mode-test@example.com');
-- 预期: 2 条记录（两个用户均创建成功）

-- 恢复租户策略为默认 block 模式
-- UPDATE password_policies SET breach_check_mode = 'block', min_breach_count = 1 WHERE tenant_id = '<TENANT_ID>';

-- 清理测试用户
DELETE FROM users WHERE email IN ('threshold-test@example.com', 'disabled-mode-test@example.com');
```

---

## 故障排除

| 现象 | 原因 | 解决方法 |
|------|------|----------|
| Warn 模式下 `POST /api/v1/users` 返回 422 而非 201 | 请求体未包含 `tenant_id`，系统使用默认 PasswordPolicy（`breach_check_mode: "block"`） | 在请求体中添加 `"tenant_id": "<TENANT_ID>"`，确保使用目标租户的密码策略 |
| Disabled 模式下 `POST /api/v1/users` 返回 422 | 同上，缺少 `tenant_id` 导致使用默认策略 | 同上 |
| 高 `min_breach_count` 阈值未生效 | 同上，缺少 `tenant_id` 导致使用默认策略 | 同上 |

---

## 检查清单

| # | 场景 | 覆盖特性 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|----------|------|----------|----------|------|
| 1 | 租户级 PasswordPolicy breach 字段配置 | D2 | ☐ | | | |
| 2 | Warn 模式 — 注册返回 password_warning | D3 | ☐ | | | |
| 3 | Warn 模式 — 修改/重置密码返回 password_warning | D3 | ☐ | | | |
| 4 | Login-time async breach check 创建 required action | D1 | ☐ | | | 依赖 HIBP 外网 |
| 5 | min_breach_count 阈值过滤与 disabled 模式 | D2 | ☐ | | | |
