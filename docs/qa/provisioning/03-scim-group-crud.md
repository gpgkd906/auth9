# SCIM 2.0 Provisioning - 组 CRUD 与 Group-Role 映射

**模块**: Provisioning (SCIM 2.0)
**测试范围**: SCIM Group 资源的创建、查询、列表、替换、增量更新（成员变更）、删除，以及 Group-Role 映射管理 API
**场景数**: 5
**优先级**: 高

---

## 背景说明

SCIM Group 资源映射到 Auth9 的 Role 体系。IdP 推送的 Group 通过 `scim_group_role_mappings` 表映射为 Auth9 Role，Group 的成员变更（add/remove members）会自动同步为用户角色分配。

**SCIM 协议端点**（Bearer Token 鉴权）：
- `POST /api/v1/scim/v2/Groups` — 创建组
- `GET /api/v1/scim/v2/Groups` — 列表
- `GET /api/v1/scim/v2/Groups/{id}` — 获取
- `PUT /api/v1/scim/v2/Groups/{id}` — 全量替换
- `PATCH /api/v1/scim/v2/Groups/{id}` — 增量更新（成员变更）
- `DELETE /api/v1/scim/v2/Groups/{id}` — 删除

**管理 API**（JWT 鉴权）：
- `GET /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/group-mappings` — 列出映射
- `PUT /api/v1/tenants/{tid}/sso/connectors/{cid}/scim/group-mappings` — 更新映射

---

## 数据库表结构参考

### scim_group_role_mappings 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 |
| connector_id | CHAR(36) | 关联 SSO Connector |
| scim_group_id | VARCHAR(255) | SCIM Group ID |
| scim_group_display_name | VARCHAR(255) | SCIM Group 显示名 |
| role_id | CHAR(36) | 映射的 Auth9 Role ID |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：SCIM 创建 Group

### 初始状态
- 已有有效 SCIM Bearer Token
- 系统中已有至少一个 SCIM 创建的用户（用于成员关联）

### 目的
验证通过 SCIM POST /Groups 创建组，并包含成员引用

### 测试操作流程

**API 测试**:
```bash
SCIM_TOKEN="{scim_bearer_token}"
USER_ID="{scim_user_id}"

curl -s -X POST "http://localhost:8080/api/v1/scim/v2/Groups" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d "{
    \"schemas\": [\"urn:ietf:params:scim:schemas:core:2.0:Group\"],
    \"displayName\": \"Engineering\",
    \"members\": [
      {\"value\": \"$USER_ID\"}
    ]
  }"
```

### 预期结果
- HTTP 201 Created
- `Content-Type: application/scim+json;charset=utf-8`
- 响应包含 `id`、`displayName`（"Engineering"）、`members` 数组、`meta`
- `meta.resourceType` = `"Group"`

### 预期数据状态
```sql
SELECT scim_group_id, scim_group_display_name, role_id
FROM scim_group_role_mappings
WHERE connector_id = '{connector_id}' AND scim_group_display_name = 'Engineering';
-- 预期: 存在映射记录，role_id 指向自动创建的 Auth9 Role

SELECT operation, resource_type, status FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}' AND resource_type = 'Group'
ORDER BY created_at DESC LIMIT 1;
-- 预期: operation = 'create', status = 'success'
```

---

## 场景 2：SCIM PATCH Group — 添加/移除成员

### 初始状态
- 场景 1 已创建 Engineering Group
- 已有第二个 SCIM 用户

### 目的
验证 PATCH /Groups/{id} 支持 add/remove members 操作，自动同步 Auth9 角色分配

### 测试操作流程

**API 测试**:
```bash
GROUP_ID="{scim_group_id}"
NEW_USER_ID="{second_user_id}"

# 添加成员
curl -s -X PATCH "http://localhost:8080/api/v1/scim/v2/Groups/$GROUP_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d "{
    \"schemas\": [\"urn:ietf:params:scim:api:messages:2.0:PatchOp\"],
    \"Operations\": [
      {\"op\": \"add\", \"path\": \"members\", \"value\": [{\"value\": \"$NEW_USER_ID\"}]}
    ]
  }"

# 移除成员
curl -s -X PATCH "http://localhost:8080/api/v1/scim/v2/Groups/$GROUP_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -H "Content-Type: application/scim+json" \
  -d "{
    \"schemas\": [\"urn:ietf:params:scim:api:messages:2.0:PatchOp\"],
    \"Operations\": [
      {\"op\": \"remove\", \"path\": \"members\", \"value\": [{\"value\": \"$NEW_USER_ID\"}]}
    ]
  }"
```

### 预期结果
- 添加成员：HTTP 200，`members` 数组包含新成员
- 移除成员：HTTP 200，`members` 数组不再包含该成员
- 对应的 `user_tenant_roles` 记录应随之增减

### 预期数据状态
```sql
-- 添加后
SELECT utr.role_id FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{new_user_id}' AND utr.role_id = '{mapped_role_id}';
-- 预期: 存在记录

-- 移除后
-- 预期: 上述记录被删除
```

---

## 场景 3：SCIM 获取和列表 Group

### 初始状态
- 至少创建了一个 SCIM Group

### 目的
验证 GET /Groups/{id} 和 GET /Groups 返回正确的 SCIM 格式响应

### 测试操作流程

**API 测试**:
```bash
# 获取单个 Group
curl -s "http://localhost:8080/api/v1/scim/v2/Groups/$GROUP_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN"

# 列表所有 Groups
curl -s "http://localhost:8080/api/v1/scim/v2/Groups?startIndex=1&count=10" \
  -H "Authorization: Bearer $SCIM_TOKEN"
```

### 预期结果
- GET /Groups/{id}：HTTP 200，返回完整 ScimGroup 资源（含 members）
- GET /Groups：HTTP 200，返回 `ScimListResponse` 格式：
  ```json
  {
    "schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"],
    "totalResults": 1,
    "Resources": [{"displayName": "Engineering", ...}]
  }
  ```

---

## 场景 4：管理 API — 查看和更新 Group-Role 映射

### 初始状态
- 场景 1 已自动创建 Group-Role 映射
- 已有 Admin JWT Token

### 目的
验证管理员可以通过 JWT 保护的 API 查看和手动调整 SCIM Group → Auth9 Role 映射

### 测试操作流程

**API 测试**:
```bash
TENANT_ID="{tenant_id}"
CONNECTOR_ID="{connector_id}"
ROLE_ID="{custom_role_id}"

# 列出映射
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/group-mappings" \
  -H "Authorization: Bearer $TOKEN"

# 更新映射（全量替换）
curl -s -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/$CONNECTOR_ID/scim/group-mappings" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"mappings\": [
      {
        \"scim_group_id\": \"eng-group-001\",
        \"scim_group_display_name\": \"Engineering\",
        \"role_id\": \"$ROLE_ID\"
      }
    ]
  }"
```

### 预期结果
- GET：HTTP 200，返回当前 Connector 下所有 Group-Role 映射数组
- PUT：HTTP 200，返回更新后的映射数组
- 旧映射被删除，新映射被创建

### 预期数据状态
```sql
SELECT scim_group_id, scim_group_display_name, role_id
FROM scim_group_role_mappings WHERE connector_id = '{connector_id}';
-- 预期: 仅包含 PUT 请求中指定的映射
```

---

## 场景 5：SCIM DELETE Group

### 初始状态
- 已有 SCIM 创建的 Group

### 目的
验证 DELETE /Groups/{id} 移除 Group-Role 映射

### 测试操作流程

**API 测试**:
```bash
curl -s -X DELETE "http://localhost:8080/api/v1/scim/v2/Groups/$GROUP_ID" \
  -H "Authorization: Bearer $SCIM_TOKEN" \
  -w "\nHTTP: %{http_code}"
```

### 预期结果
- HTTP 204 No Content

### 预期数据状态
```sql
SELECT COUNT(*) as mapping_count FROM scim_group_role_mappings
WHERE connector_id = '{connector_id}' AND scim_group_id = '{scim_group_id}';
-- 预期: mapping_count = 0

SELECT operation, resource_type, status FROM scim_provisioning_logs
WHERE connector_id = '{connector_id}' AND resource_type = 'Group'
ORDER BY created_at DESC LIMIT 1;
-- 预期: operation = 'delete', status = 'success'
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | SCIM 创建 Group | ☐ | | | |
| 2 | SCIM PATCH Group — 添加/移除成员 | ☐ | | | |
| 3 | SCIM 获取和列表 Group | ☐ | | | |
| 4 | 管理 API — 查看和更新 Group-Role 映射 | ☐ | | | |
| 5 | SCIM DELETE Group | ☐ | | | |
