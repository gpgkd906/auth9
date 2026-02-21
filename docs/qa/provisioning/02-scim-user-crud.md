# SCIM 2.0 Provisioning - 用户 CRUD（SCIM 协议端点）

**模块**: Provisioning (SCIM 2.0)
**测试范围**: SCIM User 资源的创建、查询、列表、全量替换、增量更新、删除（停用）
**场景数**: 5
**优先级**: 高

---

## 背景说明

SCIM 协议端点使用独立的 SCIM Bearer Token 鉴权（非 JWT），所有响应使用 `Content-Type: application/scim+json`。

端点：
- `POST /api/v1/scim/v2/Users` — 创建用户
- `GET /api/v1/scim/v2/Users` — 列表（支持 filter）
- `GET /api/v1/scim/v2/Users/{id}` — 获取单个用户
- `PUT /api/v1/scim/v2/Users/{id}` — 全量替换
- `PATCH /api/v1/scim/v2/Users/{id}` — 增量更新（PatchOp）
- `DELETE /api/v1/scim/v2/Users/{id}` — 停用用户

**属性映射**:
| SCIM Path | Auth9 Field |
|-----------|-------------|
| `userName` | `email` |
| `displayName` | `display_name` |
| `name.givenName` + `name.familyName` | `display_name`（拼接） |
| `externalId` | `scim_external_id` |
| `active` | `locked_until`（false → 锁定） |
| `photos[type eq "photo"].value` | `avatar_url` |

---

## 数据库表结构参考

### users 表（SCIM 相关字段）
| 字段 | 类型 | 说明 |
|------|------|------|
| scim_external_id | VARCHAR(255) | SCIM externalId（IdP 侧的用户 ID） |
| scim_provisioned_by | CHAR(36) | 创建该用户的 SSO Connector ID |

### scim_provisioning_logs 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 |
| connector_id | CHAR(36) | 关联 SSO Connector |
| operation | VARCHAR(20) | 操作类型（create/update/delete 等） |
| resource_type | VARCHAR(20) | 资源类型（User/Group） |
| scim_resource_id | VARCHAR(255) | SCIM 资源 ID |
| auth9_resource_id | CHAR(36) | Auth9 用户/角色 ID |
| status | VARCHAR(10) | 状态（success/error） |
| error_detail | TEXT | 错误详情 |
| response_status | INT | HTTP 响应状态码 |

---

## 场景 1：SCIM 创建用户

### 初始状态
- 已有有效 SCIM Bearer Token（场景 01-token-management 创建）
- 目标 email 在系统中不存在

### 目的
验证通过 SCIM POST /Users 创建用户，用户被关联到正确的 tenant 并设置 SCIM 追踪字段

### 测试操作流程

**API 测试**:
```bash
SCIM_TOKEN="{scim_bearer_token}"

curl -s -X POST "http://localhost:8080/api/v1/scim/v2/Users" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "jane.doe@example.com",
    "externalId": "okta-user-001",
    "displayName": "Jane Doe",
    "name": {
      "givenName": "Jane",
      "familyName": "Doe"
    },
    "emails": [
      {"value": "jane.doe@example.com", "type": "work", "primary": true}
    ],
    "active": true
  }'
```

### 预期结果
- HTTP 201 Created
- `Content-Type: application/scim+json;charset=utf-8`
- 响应包含 SCIM User 资源：`id`（Auth9 UUID）、`userName`、`displayName`、`externalId`、`active`、`meta`
- `meta.resourceType` = `"User"`

### 预期数据状态
```sql
SELECT id, email, display_name, scim_external_id, scim_provisioned_by
FROM users WHERE email = 'jane.doe@example.com';
-- 预期: scim_external_id = 'okta-user-001', scim_provisioned_by = '{connector_id}'

SELECT operation, resource_type, status FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: operation = 'create', resource_type = 'User', status = 'success'
```

---

## 场景 2：SCIM 获取用户与列表查询（filter）

### 初始状态
- 场景 1 已创建用户 jane.doe@example.com

### 目的
验证 GET /Users/{id} 返回 SCIM 格式的用户信息，以及 GET /Users?filter 过滤功能

### 测试操作流程

**API 测试**:
```bash
USER_ID="{scim_user_id}"

# 获取单个用户
curl -s "http://localhost:8080/api/v1/scim/v2/Users/$USER_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN"

# 按 userName 过滤
curl -s "http://localhost:8080/api/v1/scim/v2/Users?filter=userName%20eq%20%22jane.doe%40example.com%22" \
  -H "Authorization: Bearer $SCIM_TOKEN"

# 列表（带分页）
curl -s "http://localhost:8080/api/v1/scim/v2/Users?startIndex=1&count=10" \
  -H "Authorization: Bearer $SCIM_TOKEN"
```

### 预期结果
- GET /Users/{id}：HTTP 200，返回完整 SCIM User 资源
- GET /Users?filter：HTTP 200，返回 `ScimListResponse` 格式：
  ```json
  {
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
    "totalResults": 1,
    "startIndex": 1,
    "itemsPerPage": 10,
    "Resources": [{ "userName": "jane.doe@example.com", ... }]
  }
  ```
- 列表分页：`totalResults` 反映总数，`Resources` 长度不超过 `count`

---

## 场景 3：SCIM PATCH 用户（增量更新）

### 初始状态
- 场景 1 已创建用户

### 目的
验证 PATCH /Users/{id} 支持 RFC 7644 PatchOp 格式的增量更新

### 测试操作流程

**API 测试**:
```bash
# 更新 displayName
curl -s -X PATCH "http://localhost:8080/api/v1/scim/v2/Users/$USER_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
    "Operations": [
      {"op": "replace", "path": "displayName", "value": "Jane D. Smith"},
      {"op": "replace", "path": "active", "value": false}
    ]
  }'
```

### 预期结果
- HTTP 200
- 响应中 `displayName` = `"Jane D. Smith"`
- 响应中 `active` = `false`

### 预期数据状态
```sql
SELECT display_name, locked_until FROM users WHERE id = '{user_id}';
-- 预期: display_name = 'Jane D. Smith', locked_until IS NOT NULL (用户已锁定)

SELECT operation, status FROM scim_provisioning_logs
WHERE auth9_resource_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: operation = 'patch', status = 'success'
```

---

## 场景 4：SCIM PUT 全量替换用户

### 初始状态
- 已有 SCIM 创建的用户

### 目的
验证 PUT /Users/{id} 全量替换用户信息

### 测试操作流程

**API 测试**:
```bash
curl -s -X PUT "http://localhost:8080/api/v1/scim/v2/Users/$USER_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
    "userName": "jane.doe@example.com",
    "externalId": "okta-user-001",
    "displayName": "Jane Doe Updated",
    "active": true
  }'
```

### 预期结果
- HTTP 200
- 响应中 `displayName` = `"Jane Doe Updated"`
- `active` = `true`（用户重新激活）

### 预期数据状态
```sql
SELECT display_name, locked_until FROM users WHERE id = '{user_id}';
-- 预期: display_name = 'Jane Doe Updated', locked_until IS NULL (用户已解锁)
```

---

## 场景 5：SCIM DELETE 用户（软删除/停用）

### 初始状态
- 已有 SCIM 创建的用户

### 目的
验证 DELETE /Users/{id} 将用户停用（设置 locked_until），而非物理删除

### 测试操作流程

**API 测试**:
```bash
curl -s -X DELETE "http://localhost:8080/api/v1/scim/v2/Users/$USER_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- HTTP 204 No Content
- 用户未从数据库中删除，但被锁定

### 预期数据状态
```sql
SELECT id, email, locked_until FROM users WHERE id = '{user_id}';
-- 预期: 记录存在, locked_until IS NOT NULL

SELECT operation, resource_type, status FROM scim_provisioning_logs
WHERE auth9_resource_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: operation = 'delete', resource_type = 'User', status = 'success'
```

---

## 通用场景：SCIM 协议错误响应格式

### 测试操作流程
1. 不携带 Authorization header 访问 SCIM 端点
2. GET /Users/{invalid-uuid}
3. POST /Users 重复 userName（已存在的 email）
4. GET /Users/{不存在的 UUID}

### 预期结果
- 无认证：HTTP 401，SCIM 错误格式 `{"schemas":["urn:ietf:params:scim:api:messages:2.0:Error"],"status":"401",...}`
- 无效 UUID：HTTP 400，`scimType: "invalidValue"`
- 重复用户：HTTP 409，`scimType: "uniqueness"`
- 不存在：HTTP 404

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | SCIM 创建用户 | ☐ | | | |
| 2 | SCIM 获取用户与列表查询（filter） | ☐ | | | |
| 3 | SCIM PATCH 用户（增量更新） | ☐ | | | |
| 4 | SCIM PUT 全量替换用户 | ☐ | | | |
| 5 | SCIM DELETE 用户（软删除/停用） | ☐ | | | |
| - | 通用：SCIM 协议错误响应格式 | ☐ | | | |
