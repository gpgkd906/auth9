# SCIM 2.0 Provisioning - 鉴权安全与审计日志

**模块**: Provisioning (SCIM 2.0)
**测试范围**: SCIM Bearer Token 鉴权中间件安全性、Token 过期处理、审计日志查询
**场景数**: 5
**优先级**: 高

---

## 背景说明

SCIM 端点使用独立的鉴权中间件（`scim_auth_middleware`），与 JWT 认证体系完全隔离。中间件从 `Authorization: Bearer scim_xxx...` 提取 token，SHA-256 hash 后查表验证，并注入 `ScimRequestContext`（tenant_id, connector_id）到请求上下文。

所有 SCIM 操作（成功/失败）均记录到 `scim_provisioning_logs` 审计日志表。

**管理 API**：
- `GET /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/logs?offset=0&limit=50` — 分页查询审计日志

---

## 场景 1：无 Authorization Header 访问 SCIM 端点

### 初始状态
- SCIM 端点已注册

### 目的
验证不携带 Authorization header 时返回 SCIM 标准 401 错误

### 测试操作流程

**API 测试**:
```bash
# 不携带 Authorization header
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -w "\nHTTP: %{http_code}"

# 携带非 Bearer 格式 header
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Basic dXNlcjpwYXNz" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- 两种情况均返回 HTTP 401
- 响应体为 SCIM 标准错误格式：
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "401",
  "detail": "Missing or invalid Authorization header"
}
```

---

## 场景 2：使用无效/伪造 Token 访问 SCIM 端点

### 初始状态
- 无

### 目的
验证使用不存在或格式不正确的 Token 时，鉴权被拒绝

### 测试操作流程

**API 测试**:
```bash
# 伪造 token（不存在于数据库）
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer scim_this_is_a_fake_token_that_does_not_exist_in_db" \
  -w "\nHTTP: %{http_code}"

# 空 token
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer " \
  -w "\nHTTP: %{http_code}"

# JWT token（不是 SCIM token）
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer $TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- 所有情况均返回 HTTP 401
- 响应体：
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "401",
  "detail": "Invalid or expired SCIM token"
}
```
- JWT token 不被 SCIM 中间件接受（鉴权体系完全隔离）

---

## 场景 3：使用过期 Token 访问 SCIM 端点

### 初始状态
- 创建一个 `expires_in_days: 0`（或极短过期）的 SCIM Token
- 等待 token 过期（或直接在数据库中设置 `expires_at` 为过去时间）

### 目的
验证过期 token 被拒绝

### 测试操作流程

> **重要**: 创建 SCIM Token 使用的是管理 API (`/api/v1/tenants/*`)，需要 **Tenant Access Token**，
> 不能使用 Identity Token。使用 `gen-test-tokens.js tenant-owner` 生成：
> ```bash
> TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id $TENANT_ID)
> ```

**API 测试**:
```bash
# 生成 Tenant Owner Token（管理 API 需要）
TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id $TENANT_ID)

# 先创建一个 token
RESULT=$(curl -s -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"description": "Expire test", "expires_in_days": 1}')
EXPIRED_TOKEN=$(echo $RESULT | jq -r '.token')
EXPIRED_TOKEN_ID=$(echo $RESULT | jq -r '.id')

# 手动设置为过期
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "UPDATE scim_tokens SET expires_at = '2020-01-01 00:00:00' WHERE id = '$EXPIRED_TOKEN_ID';"

# 使用过期 token
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer $EXPIRED_TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- HTTP 401
- 响应包含 `"Invalid or expired SCIM token"`

---

## 场景 4：有效 Token 的 last_used_at 自动更新

### 初始状态
- 已有有效 SCIM Bearer Token（通过管理 API 创建，见场景 3 的 token 创建步骤）

### 目的
验证每次使用有效 Token 访问 SCIM 端点时，`last_used_at` 字段被自动更新

### 测试操作流程

**API 测试**:
```bash
# 步骤 1: 先通过管理 API 创建 SCIM Token
TENANT_ID="{tenant_id}"
CONNECTOR_ID="{connector_id}"
TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id $TENANT_ID)

RESULT=$(curl -s -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"description": "last_used_at test", "expires_in_days": 30}')
SCIM_TOKEN=$(echo $RESULT | jq -r '.token')
SCIM_TOKEN_ID=$(echo $RESULT | jq -r '.id')

# 步骤 2: 记录初始状态
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "SELECT last_used_at FROM scim_tokens WHERE id = '$SCIM_TOKEN_ID';"

# 步骤 3: 访问 SCIM 端点
curl -s "http://localhost:8080/api/v1/scim/v2/ServiceProviderConfig" \
  -H "Authorization: Bearer $SCIM_TOKEN" > /dev/null

# 步骤 4: 检查 last_used_at 更新
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "SELECT last_used_at FROM scim_tokens WHERE id = '$SCIM_TOKEN_ID';"
```

### 预期结果
- SCIM 请求成功（HTTP 200）
- `last_used_at` 从 NULL 或旧时间更新为当前时间

### 预期数据状态
```sql
SELECT last_used_at FROM scim_tokens WHERE id = '{scim_token_id}';
-- 预期: last_used_at 为最近时间（非 NULL）
```

---

## 场景 5：查询 SCIM 审计日志（分页）

### 初始状态
- 前序场景已产生多条 SCIM 操作日志
- **已有 Tenant Access Token（非 Identity Token）**

### 目的
验证管理 API 能分页查询 SCIM 操作审计日志

### 测试操作流程

**API 测试**:
```bash
TENANT_ID="{tenant_id}"
CONNECTOR_ID="{connector_id}"

# 生成 Tenant Owner Token（管理 API 需要 Tenant Access Token）
TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id $TENANT_ID)

# 查询日志（第一页）
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/logs?offset=0&limit=5" \
  -H "Authorization: Bearer $TOKEN"

# 查询日志（第二页）
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/logs?offset=5&limit=5" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200
- 响应结构：
```json
{
  "data": [
    {
      "id": "...",
      "tenant_id": "...",
      "connector_id": "...",
      "operation": "create",
      "resource_type": "User",
      "scim_resource_id": "...",
      "auth9_resource_id": "...",
      "status": "success",
      "response_status": 201,
      "created_at": "..."
    }
  ],
  "total": 10,
  "offset": 0,
  "limit": 5
}
```
- `total` 反映该 Connector 下的总日志数
- `data` 数组长度不超过 `limit`
- 第二页 offset=5 返回后续记录

### 预期数据状态
```sql
SELECT COUNT(*) as total_logs FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}';
-- 预期: total_logs >= 前序场景产生的操作数

SELECT DISTINCT operation FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}';
-- 预期: 包含 'create', 'patch', 'delete' 等操作类型
```

---

## 常见问题排查

| 症状 | 原因 | 修复方法 |
|------|------|----------|
| `FORBIDDEN: Identity token is only allowed for tenant selection and exchange` | 使用了 Identity Token（`gen-admin-token.sh`）访问管理 API | 使用 `node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id $TENANT_ID` 生成 Tenant Access Token |
| 创建 SCIM Token 返回 `FORBIDDEN` | 同上，创建 SCIM Token 的 API 也需要 Tenant Access Token | 同上 |
| SCIM Token 验证失败（手动插入 DB） | SCIM Token 通过 SHA-256 hash 验证，手动插入的 hash 不匹配 | 必须通过管理 API 创建 SCIM Token，API 返回原始 token 字符串 |
| 场景 4 无法获取有效 SCIM Token | 未通过管理 API 创建 token | 先用 Tenant Owner Token 调用 `POST .../scim/tokens` 创建，保存返回的 `.token` 值 |

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 无 Authorization Header 访问 SCIM 端点 | ☐ | | | |
| 2 | 使用无效/伪造 Token 访问 SCIM 端点 | ☐ | | | |
| 3 | 使用过期 Token 访问 SCIM 端点 | ☐ | | | |
| 4 | 有效 Token 的 last_used_at 自动更新 | ☐ | | | |
| 5 | 查询 SCIM 审计日志（分页） | ☐ | | | |
