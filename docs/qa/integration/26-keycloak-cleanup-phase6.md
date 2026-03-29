# 集成测试：Keycloak 遗留清理 Phase 6 — 数据库 Schema 最终清理

| 项目 | 内容 |
|------|------|
| **模块** | integration / database-migration |
| **测试范围** | Phase 6 数据库 Schema 清理：DROP 旧列、COALESCE 回退移除、`backend_client_id` 迁移、源码命名中性化 |
| **场景数** | 5 |
| **优先级** | 高 |

## 背景

Phase 6 是 Keycloak 遗留清理的最终阶段。此阶段执行以下变更：
1. 数据库迁移 `20260321000001_drop_keycloak_columns.sql`：添加 `backend_client_id` 列并 DROP 4 个旧列
2. 移除所有 SQL 查询中的 `COALESCE` 回退逻辑
3. 源码中 `keycloak_client_id` 重命名为 `backend_client_id`
4. 移除废弃的 `/api/v1/keycloak/events` 路由

### 涉及 API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/users` | 用户列表（验证 `identity_subject` 直接查询） |
| GET | `/api/v1/users/{id}` | 用户详情 |
| POST | `/api/v1/tenants/{tenant_id}/users` | 创建用户（验证不再写入 `keycloak_id`） |
| GET | `/api/v1/tenants/{tenant_id}/sessions` | 会话列表（验证 `provider_session_id` 直接查询） |
| GET | `/api/v1/tenants/{tenant_id}/saml-apps` | SAML 应用列表（验证 `backend_client_id` 字段） |
| POST | `/api/v1/identity/events` | 身份事件接收（新路径） |
| POST | `/api/v1/keycloak/events` | 旧路径（应返回 404） |

---

## 场景 1：用户 CRUD 不再依赖 keycloak_id 列

**初始状态**: 系统已运行 Phase 6 迁移，`users.keycloak_id` 列已被 DROP

**目的**: 验证用户创建、查询、搜索功能在旧列移除后仍正常工作

### 步骤 0：Gate Check

```bash
# 确认迁移已执行，keycloak_id 列不存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE users;" 2>/dev/null | grep keycloak_id
# 预期：无输出（列不存在）

# 确认 identity_subject 列存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE users;" 2>/dev/null | grep identity_subject
# 预期：有输出
```

### 操作 1：创建用户

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/users" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user": {
      "email": "phase6-test@example.com",
      "display_name": "Phase6 Test"
    }
  }'
```

**预期结果**: HTTP 201，返回用户对象包含 `identity_subject` 字段

### 操作 2：查询用户列表

```bash
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/users" \
  -H "Authorization: Bearer $TOKEN" | jq '.data[0].identity_subject'
```

**预期结果**: 返回非空的 `identity_subject` 值

### 操作 3：搜索用户

```bash
curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/users?search=phase6" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期结果**: 返回 >= 1

### 预期数据状态

```sql
-- 验证新创建用户的 identity_subject 已填充
SELECT id, identity_subject, email FROM users WHERE email = 'phase6-test@example.com';
-- 预期: identity_subject IS NOT NULL
```

---

## 场景 2：会话管理不再依赖 keycloak_session_id 列

**初始状态**: 系统已运行 Phase 6 迁移，`sessions.keycloak_session_id` 列已被 DROP

**目的**: 验证会话查询在旧列移除后仍正常工作

### 步骤 0：Gate Check

```bash
# 确认 keycloak_session_id 列不存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE sessions;" 2>/dev/null | grep keycloak_session_id
# 预期：无输出

# 确认 provider_session_id 列存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE sessions;" 2>/dev/null | grep provider_session_id
# 预期：有输出
```

### 操作 1：登录获取会话

通过 Hosted Login 登录后，查询活跃会话：

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
USER_ID=$(curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/users?search=admin" \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s "http://localhost:8080/api/v1/users/$USER_ID/sessions" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期结果**: 返回 >= 0（会话列表正常返回，无 SQL 错误）

---

## 场景 3：SAML 应用使用 backend_client_id 字段

**初始状态**: 系统已运行 Phase 6 迁移，`saml_applications.keycloak_client_id` 已替换为 `backend_client_id`

**目的**: 验证 SAML 应用 CRUD 使用新字段名

### 步骤 0：Gate Check

```bash
# 确认 backend_client_id 列存在，keycloak_client_id 不存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE saml_applications;" 2>/dev/null | grep backend_client_id
# 预期：有输出

docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE saml_applications;" 2>/dev/null | grep keycloak_client_id
# 预期：无输出
```

### 操作 1：查询 SAML 应用列表

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/saml-apps" \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期结果**: HTTP 200，响应中 SAML 应用对象包含 `backend_client_id` 字段（不包含 `keycloak_client_id`）

---

## 场景 4：Enterprise SSO 连接器不再依赖 keycloak_alias 列

**初始状态**: 系统已运行 Phase 6 迁移，`enterprise_sso_connectors.keycloak_alias` 列已被 DROP

**目的**: 验证 Enterprise SSO 连接器查询和创建功能正常

### 步骤 0：Gate Check

```bash
# 确认 keycloak_alias 列不存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE enterprise_sso_connectors;" 2>/dev/null | grep keycloak_alias
# 预期：无输出

# 确认 provider_alias 列存在
docker exec auth9-tidb mysql -u root -P 4000 -e "DESCRIBE enterprise_sso_connectors;" 2>/dev/null | grep provider_alias
# 预期：有输出
```

### 操作 1：查询 SSO 连接器列表

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期结果**: HTTP 200，连接器对象包含 `provider_alias` 字段

---

## 场景 5：废弃的 /api/v1/keycloak/events 路由已移除

**初始状态**: Phase 6 已移除旧的 `/api/v1/keycloak/events` 路由别名

**目的**: 验证旧路由返回 404，新路由正常工作

### 操作 1：请求旧路由

```bash
curl -s -o /dev/null -w "%{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -d '{"type": "LOGIN", "time": 1704067200000}'
```

**预期结果**: HTTP 404（路由不存在）

### 操作 2：请求新路由

```bash
curl -s -o /dev/null -w "%{http_code}" -X POST "http://localhost:8080/api/v1/identity/events" \
  -H "Content-Type: application/json" \
  -d '{"type": "LOGIN", "time": 1704067200000}'
```

**预期结果**: HTTP 401（未提供签名，但路由存在）或 HTTP 200/204（如未配置 webhook secret）

---

## Token 类型说明

> **租户级端点（`/api/v1/tenants/{tenant_id}/...`）需要 Tenant Access Token。**
> `gen-admin-token.sh` 生成的 Identity Token 可用于场景 1-2 的用户和会话查询，但场景 3-4 的 SAML 和 SSO 端点可能需要 Tenant Access Token。
> 如果遇到 `403: "Identity token is only allowed for tenant selection and exchange"`，请使用 `gen-test-tokens.js tenant-owner` 生成 Access Token。

---

## 清单

| # | 场景 | 类型 | 状态 |
|---|------|------|------|
| 1 | 用户 CRUD 不再依赖 keycloak_id 列 | 正常流程 | ✅ |
| 2 | 会话管理不再依赖 keycloak_session_id 列 | 正常流程 | ✅ |
| 3 | SAML 应用使用 backend_client_id 字段 | 字段迁移 | ✅ |
| 4 | Enterprise SSO 不再依赖 keycloak_alias 列 | 正常流程 | ✅ |
| 5 | 废弃路由已移除 | 兼容性 | ✅ |
