# SCIM 2.0 Provisioning - Token 管理（管理 API）

**模块**: Provisioning (SCIM 2.0)
**测试范围**: SCIM Bearer Token 的创建、列表、吊销，通过 JWT 保护的管理 API 操作
**场景数**: 5
**优先级**: 高

---

## 背景说明

SCIM 2.0 Provisioning 功能允许企业 IdP（如 Okta、Azure AD）通过 SCIM 协议自动同步用户和组到 Auth9。每个 SCIM 端点绑定到一个 Enterprise SSO Connector，使用独立的 Bearer Token 鉴权（非 JWT）。

管理 API 端点（JWT 保护）：

- `POST /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/tokens` — 生成 Token
- `GET /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/tokens` — 列出 Token
- `DELETE /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/tokens/{id}` — 吊销 Token

Token 格式：`scim_{base64_44chars}`，存储时使用 SHA-256 hash，API 仅返回前缀 `token_prefix` 用于辨识。

---

## 数据库表结构参考

### scim_tokens 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 |
| connector_id | CHAR(36) | 关联 SSO Connector |
| token_hash | VARCHAR(128) | SHA-256 hash |
| token_prefix | VARCHAR(12) | 前 8 字符，用于 UI 展示 |
| description | VARCHAR(255) | 描述 |
| expires_at | TIMESTAMP | 过期时间（NULL 表示永不过期） |
| last_used_at | TIMESTAMP | 最后使用时间 |
| revoked_at | TIMESTAMP | 吊销时间 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：创建 SCIM Token（含描述和过期时间）

### 初始状态
- 已有 Tenant 和 Enterprise SSO Connector
- 已获取 Admin JWT Token

### 目的
验证通过管理 API 成功创建 SCIM Bearer Token，返回完整 token（仅展示一次）和 token 元数据

### 测试操作流程

**API 测试**:
```bash
TENANT_ID="{tenant_id}"
CONNECTOR_ID="{connector_id}"

# 创建 SCIM Token
curl -s -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Okta SCIM integration",
    "expires_in_days": 90
  }'
```

### 预期结果
- HTTP 201 Created
- 响应包含 `token` 字段，格式为 `scim_` 前缀 + 44 字符 base64
- 响应包含 `id`、`tenant_id`、`connector_id`、`token_prefix`、`description`、`expires_at`、`created_at`
- `token_prefix` 为 token 前 8 字符
- `expires_at` 为当前时间 + 90 天
- `description` 为 `"Okta SCIM integration"`

### 预期数据状态
```sql
SELECT id, tenant_id, connector_id, token_prefix, description,
       expires_at IS NOT NULL as has_expiry, revoked_at
FROM scim_tokens
WHERE connector_id = '{connector_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: token_prefix 非空, description = 'Okta SCIM integration', has_expiry = 1, revoked_at = NULL
```

---

## 场景 2：创建无过期时间的 SCIM Token

### 初始状态
- 已有 Tenant 和 Enterprise SSO Connector

### 目的
验证创建不过期的 SCIM Token（`expires_in_days` 为空）

### 测试操作流程

**API 测试**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Permanent token"
  }'
```

### 预期结果
- HTTP 201 Created
- `expires_at` 为 `null`
- 其余字段正常返回

### 预期数据状态
```sql
SELECT expires_at FROM scim_tokens
WHERE connector_id = '{connector_id}' AND description = 'Permanent token';
-- 预期: expires_at = NULL
```

---

## 场景 3：列出 Connector 下的所有 SCIM Token

### 初始状态
- 场景 1、2 已创建两个 Token

### 目的
验证列出指定 Connector 下所有 Token，响应不包含 token_hash

### 测试操作流程

**API 测试**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200
- 返回 JSON 数组，包含之前创建的两个 Token
- 每个 Token 包含 `id`、`token_prefix`、`description`、`expires_at`、`created_at`
- **不包含** `token_hash` 和 `token`（原始 token 仅创建时返回一次）

### 预期数据状态
```sql
SELECT COUNT(*) as token_count FROM scim_tokens WHERE connector_id = '{connector_id}' AND revoked_at IS NULL;
-- 预期: token_count = 2
```

---

## 场景 4：吊销 SCIM Token

### 初始状态
- 已有至少一个 SCIM Token

### 目的
验证吊销（revoke）Token 后该 Token 无法再用于 SCIM 认证

### 测试操作流程

**API 测试**:
```bash
# 获取 token ID
TOKEN_ID="{scim_token_id}"

# 吊销 Token
curl -s -X DELETE \
  "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/tokens/$TOKEN_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- HTTP 204 No Content
- 该 Token 的 `revoked_at` 被设置为当前时间

### 预期数据状态
```sql
SELECT id, revoked_at FROM scim_tokens WHERE id = '{scim_token_id}';
-- 预期: revoked_at IS NOT NULL
```

---

## 场景 5：使用已吊销 Token 访问 SCIM 端点被拒绝

### 初始状态
- 场景 4 中已吊销一个 Token
- 记录该 Token 的原始值

### 目的
验证已吊销的 Token 无法通过 SCIM 鉴权中间件

### 测试操作流程

**API 测试**:
```bash
REVOKED_TOKEN="{revoked_scim_token}"

# 使用已吊销 Token 访问 SCIM 端点
curl -s "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer $REVOKED_TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- HTTP 401 Unauthorized
- 响应体为 SCIM 格式错误：
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
  "status": "401",
  "detail": "Invalid or expired SCIM token"
}
```

---

## 通用场景：SCIM Token 管理 API 鉴权

### 测试操作流程
1. 不携带 JWT Token 访问管理 API
2. 携带无效 JWT Token 访问管理 API
3. 使用无效 UUID 格式的 tenant_id 或 connector_id

### 预期结果
- 无 Token：HTTP 401
- 无效 Token：HTTP 401
- 无效 UUID：HTTP 400 Bad Request

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建 SCIM Token（含描述和过期时间） | ☐ | | | |
| 2 | 创建无过期时间的 SCIM Token | ☐ | | | |
| 3 | 列出 Connector 下的所有 SCIM Token | ☐ | | | |
| 4 | 吊销 SCIM Token | ☐ | | | |
| 5 | 使用已吊销 Token 访问 SCIM 端点被拒绝 | ☐ | | | |
| - | 通用：管理 API 鉴权 | ☐ | | | |
