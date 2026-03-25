# Action Claims 注入加固测试

**模块**: Action Claims 安全
**测试范围**: 保留 claim denylist、命名空间前缀、跨租户隔离、Token Exchange 注入
**场景数**: 4
**关联 FR**: FR-006

---

## 前置条件

### 步骤 0: Gate Check

1. 确认 auth9-core 已部署最新代码（包含 FR-006 变更）
2. 准备 Admin API Token:

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

3. 确认测试用户存在且属于至少一个租户:

```sql
SELECT u.id, u.email, tu.tenant_id
FROM users u
JOIN tenant_users tu ON tu.user_id = u.id
WHERE u.email = 'test@example.com'
LIMIT 1;
-- 预期: 至少 1 行
```

4. 确认租户有 Service（非平台级，tenant_id 非 NULL）:

```sql
SELECT s.id, s.name, s.tenant_id, c.client_id
FROM services s
JOIN clients c ON c.service_id = s.id
WHERE s.tenant_id IS NOT NULL
LIMIT 1;
-- 记录 tenant_id, client_id 用于后续测试
```

---

## 场景 1: Happy Path — Action Claims 通过 Token Exchange 注入 Tenant Access Token

**目标**: 验证 action 设置的 custom claim 经 token exchange 后出现在 Tenant Access Token 中，且带命名空间前缀。

### 步骤

1. 在租户 Service 上创建 post-login Action:

```bash
# 获取 service_id (租户拥有的 Service)
SERVICE_ID=$(curl -sf -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services \
  | jq -r '.data[] | select(.tenant_id != null) | .id' | head -1)

TENANT_ID=$(curl -sf -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/$SERVICE_ID \
  | jq -r '.data.tenant_id')

# 创建 action
curl -sf -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "FR006 Claims Test",
    "trigger": "post-login",
    "script": "context.claims = context.claims || {}; context.claims.department = \"engineering\"; context.claims.level = 5; context;",
    "enabled": true,
    "execution_order": 100
  }' | jq .
```

2. 通过 Hosted Login 获取 Identity Token:

```bash
IDENTITY_TOKEN=$(curl -sf -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "Test1234!"}' \ # pragma: allowlist secret
  | jq -r '.access_token // .identity_token // .token')
```

3. 验证 Identity Token **不含** action claims:

```bash
# 解码 JWT payload (base64)
echo "$IDENTITY_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 预期: 不含 "department"、"level"、"https://auth9.dev/" 任何 key
```

4. 执行 Token Exchange:

```bash
CLIENT_ID=$(curl -sf -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/$SERVICE_ID \
  | jq -r '.data.clients[0].client_id // empty')

EXCHANGE_RESP=$(curl -sf -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"tenant_id\": \"$TENANT_ID\", \"service_id\": \"$CLIENT_ID\"}")

TENANT_TOKEN=$(echo "$EXCHANGE_RESP" | jq -r '.access_token')
```

5. 验证 Tenant Access Token 包含命名空间化的 claims:

```bash
echo "$TENANT_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 预期:
#   "https://auth9.dev/department": "engineering"
#   "https://auth9.dev/level": 5
#   不含裸 "department" 或 "level" key
```

### 预期结果

- Identity Token: `extra` 为 null 或不存在
- Tenant Access Token: 包含 `https://auth9.dev/department` 和 `https://auth9.dev/level`

### 清理

```bash
# 删除测试 action (获取 action_id 从创建响应)
# curl -X DELETE http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID \
#   -H "Authorization: Bearer $TOKEN"
```

---

## 场景 2: Denylist — 保留 Claim 被过滤

**目标**: 验证 Action 返回的 JWT 保留字段（`sub`, `iss`, `aud`）不会出现在 Tenant Access Token 中。

### 步骤

1. 创建尝试覆盖保留字段的 Action:

```bash
curl -sf -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "FR006 Denylist Test",
    "trigger": "post-login",
    "script": "context.claims = { sub: \"evil-user\", iss: \"evil-issuer\", aud: \"evil-aud\", tenant_id: \"hijacked\", roles: [\"super-admin\"], safe_field: \"allowed\" }; context;",
    "enabled": true,
    "execution_order": 200
  }' | jq .
```

2. 登录 + Token Exchange（同场景 1 步骤 2-4）

3. 验证 Tenant Access Token:

```bash
echo "$TENANT_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 预期:
#   sub: 原始用户 ID（非 "evil-user"）
#   iss: 系统签发者（非 "evil-issuer"）
#   aud: 实际 client_id（非 "evil-aud"）
#   tenant_id: 实际租户 ID（非 "hijacked"）
#   roles: 不含 "super-admin"（除非用户实际拥有该角色）
#   "https://auth9.dev/safe_field": "allowed"（唯一通过的 claim）
```

### 预期结果

- 所有保留字段（sub, iss, aud, tenant_id, roles）保持原值不被覆盖
- 仅 `https://auth9.dev/safe_field` 出现在 token 中

---

## 场景 3: 跨租户隔离 — Identity Token 不含 Action Claims

**目标**: 验证多租户用户登录后 Identity Token 不包含任何 tenant-specific action claims。

### 步骤

1. 确认测试用户属于多个租户:

```sql
SELECT tu.tenant_id, t.name
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = (SELECT id FROM users WHERE email = 'test@example.com');
-- 需要 >= 2 行（如果只有 1 个租户，先添加第二个）
```

2. 在两个不同租户的 Service 上分别创建 post-login Action（各设置不同 claim）

3. 登录获取 Identity Token

4. 解码 Identity Token:

```bash
echo "$IDENTITY_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 预期: 不含任何 https://auth9.dev/ 前缀的 claim
# 只有标准字段: sub, email, name, iss, aud, token_type, iat, exp
```

### 预期结果

- Identity Token payload 仅包含标准 JWT 字段
- 不含任何租户 A 或租户 B 的 action claims
- 两个租户的 claims 不会被合并

---

## 场景 4: 向后兼容 — 无 Action 的租户 Token Exchange 正常

**目标**: 验证没有配置 post-login Action 的租户，token exchange 流程正常且 token 不包含 extra 字段。

### 步骤

1. 确认目标租户没有任何 post-login Action:

```sql
SELECT a.id, a.name, a.trigger
FROM actions a
JOIN services s ON a.service_id = s.id
WHERE s.tenant_id = '<TARGET_TENANT_ID>'
  AND a.trigger = 'post-login'
  AND a.enabled = true;
-- 预期: 0 行
```

2. 登录 + Token Exchange

3. 验证 Tenant Access Token 结构:

```bash
echo "$TENANT_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
# 预期: 标准字段 + roles + permissions，无 https://auth9.dev/ 前缀的 key
# JSON 中不应包含 "extra" 或空的 flatten 字段
```

### 预期结果

- Token exchange 返回 200
- Tenant Access Token 结构与 FR-006 之前一致（无额外字段）
- 响应时间无显著增加（action 执行在无 action 时应为快速路径）
