# SCIM 2.0 Provisioning - Bulk 操作与 Discovery 端点

**模块**: Provisioning (SCIM 2.0)
**测试范围**: SCIM Bulk 批量操作、ServiceProviderConfig / Schemas / ResourceTypes 发现端点
**场景数**: 5
**优先级**: 中

---

## 背景说明

SCIM 2.0 规范包含 Bulk 批量操作端点（RFC 7644 §3.7）和服务发现端点（RFC 7643 §5）。IdP 通过发现端点了解 Auth9 SCIM 服务的能力，通过 Bulk 端点高效执行批量用户/组变更。

**端点**：
- `POST /api/v1/scim/v2/Bulk` — 批量操作
- `GET /api/v1/scim/v2/ServiceProviderConfig` — 服务能力声明
- `GET /api/v1/scim/v2/Schemas` — Schema 定义
- `GET /api/v1/scim/v2/ResourceTypes` — 资源类型定义

**Bulk 限制**：
- 最大操作数：100
- 最大载荷大小：1 MB
- 支持 `failOnErrors` 计数控制

---

## 场景 1：ServiceProviderConfig 发现

### 初始状态
- 已有有效 SCIM Bearer Token

### 目的
验证 GET /ServiceProviderConfig 返回 Auth9 SCIM 服务的能力声明，IdP 据此配置同步行为

### 测试操作流程

**API 测试**:
```bash
SCIM_TOKEN="{scim_bearer_token}"

curl -s "http://localhost:8080/api/v1/scim/v2/ServiceProviderConfig" \
  -H "Authorization: Bearer $SCIM_TOKEN"
```

### 预期结果
- HTTP 200
- `Content-Type: application/scim+json;charset=utf-8`
- 响应结构：
```json
{
  "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
  "patch": {"supported": true},
  "bulk": {"supported": true, "maxOperations": 100, "maxPayloadSize": 1048576},
  "filter": {"supported": true, "maxResults": 200},
  "changePassword": {"supported": false},
  "sort": {"supported": false},
  "etag": {"supported": false},
  "authenticationSchemes": [
    {"name": "OAuth Bearer Token", "type": "oauthbearertoken", "primary": true}
  ]
}
```

---

## 场景 2：Schemas 端点

### 初始状态
- 已有有效 SCIM Bearer Token

### 目的
验证 GET /Schemas 返回 User 和 Group 两个 Schema 定义

### 测试操作流程

**API 测试**:
```bash
curl -s "http://localhost:8080/api/v1/scim/v2/Schemas" \
  -H "Authorization: Bearer $SCIM_TOKEN"
```

### 预期结果
- HTTP 200
- 返回包含两个 Schema 的数组：
  - User Schema: `id` = `"urn:ietf:params:scim:schemas:core:2.0:User"`，`attributes` 包含 `userName`（required）、`displayName`、`active`、`emails`
  - Group Schema: `id` = `"urn:ietf:params:scim:schemas:core:2.0:Group"`，`attributes` 包含 `displayName`（required）、`members`

---

## 场景 3：ResourceTypes 端点

### 初始状态
- 已有有效 SCIM Bearer Token

### 目的
验证 GET /ResourceTypes 返回 User 和 Group 资源类型定义

### 测试操作流程

**API 测试**:
```bash
curl -s "http://localhost:8080/api/v1/scim/v2/ResourceTypes" \
  -H "Authorization: Bearer $SCIM_TOKEN"
```

### 预期结果
- HTTP 200
- 返回数组包含两个 ResourceType：
  - `{"id": "User", "name": "User", "endpoint": "/Users", "schema": "urn:ietf:params:scim:schemas:core:2.0:User"}`
  - `{"id": "Group", "name": "Group", "endpoint": "/Groups", "schema": "urn:ietf:params:scim:schemas:core:2.0:Group"}`

---

## 场景 4：Bulk 批量创建用户

### 初始状态
- 已有有效 SCIM Bearer Token
- 目标用户 email 在系统中不存在

### 目的
验证 POST /Bulk 支持批量创建多个用户，各操作独立执行并返回结果

### 测试操作流程

**API 测试**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/scim/v2/Bulk" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d '{
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkRequest"],
    "Operations": [
      {
        "method": "POST",
        "path": "/Users",
        "bulkId": "user-1",
        "data": {
          "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
          "userName": "bulk-user1@example.com",
          "displayName": "Bulk User 1",
          "externalId": "bulk-ext-001",
          "active": true
        }
      },
      {
        "method": "POST",
        "path": "/Users",
        "bulkId": "user-2",
        "data": {
          "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
          "userName": "bulk-user2@example.com",
          "displayName": "Bulk User 2",
          "externalId": "bulk-ext-002",
          "active": true
        }
      }
    ]
  }'
```

### 预期结果
- HTTP 200
- 响应为 `ScimBulkResponse` 格式：
```json
{
  "schemas": ["urn:ietf:params:scim:api:messages:2.0:BulkResponse"],
  "Operations": [
    {"method": "POST", "bulkId": "user-1", "status": "201", "location": "..."},
    {"method": "POST", "bulkId": "user-2", "status": "201", "location": "..."}
  ]
}
```
- 每个操作的 `status` = `"201"`

### 预期数据状态
```sql
SELECT email, scim_external_id FROM users
WHERE email IN ('bulk-user1@example.com', 'bulk-user2@example.com');
-- 预期: 两条记录，scim_external_id 分别为 'bulk-ext-001'、'bulk-ext-002'

SELECT COUNT(*) as log_count FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}' AND operation = 'create' AND resource_type = 'User';
-- 预期: log_count 增加 2
```

---

## 场景 5：Bulk 混合操作（创建 + 删除）与 failOnErrors

### 初始状态
- 场景 4 已创建 bulk-user1 和 bulk-user2
- 记录 bulk-user1 的 UUID

### 目的
验证 Bulk 支持混合操作类型，以及 `failOnErrors` 参数控制失败容忍度

### 测试操作流程

**API 测试**:
```bash
BULK_USER1_ID="{bulk_user1_uuid}"

curl -s -X POST "http://localhost:8080/api/v1/scim/v2/Bulk" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d "{
    \"schemas\": [\"urn:ietf:params:scim:api:messages:2.0:BulkRequest\"],
    \"failOnErrors\": 1,
    \"Operations\": [
      {
        \"method\": \"DELETE\",
        \"path\": \"/Users/$BULK_USER1_ID\"
      },
      {
        \"method\": \"POST\",
        \"path\": \"/Users\",
        \"bulkId\": \"user-3\",
        \"data\": {
          \"schemas\": [\"urn:ietf:params:scim:schemas:core:2.0:User\"],
          \"userName\": \"bulk-user3@example.com\",
          \"displayName\": \"Bulk User 3\",
          \"active\": true
        }
      }
    ]
  }"
```

### 预期结果
- HTTP 200
- Operations 包含两个结果：
  - DELETE 操作 `status: "204"`
  - POST 操作 `status: "201"`
- `failOnErrors: 1` 表示允许最多 1 个错误后继续

### 预期数据状态
```sql
SELECT locked_until FROM users WHERE id = '{bulk_user1_uuid}';
-- 预期: locked_until IS NOT NULL (已停用)

SELECT email FROM users WHERE email = 'bulk-user3@example.com';
-- 预期: 记录存在
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | ServiceProviderConfig 发现 | ☐ | | | |
| 2 | Schemas 端点 | ☐ | | | |
| 3 | ResourceTypes 端点 | ☐ | | | |
| 4 | Bulk 批量创建用户 | ☐ | | | |
| 5 | Bulk 混合操作与 failOnErrors | ☐ | | | |
