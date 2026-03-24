# SDK - 核心管理 API 子客户端

**模块**: SDK
**测试范围**: Auth9Client 的 tenants / users / services / roles / permissions / rbac / invitations 子客户端
**场景数**: 5
**优先级**: 高

---

## 背景说明

### 子客户端架构

`@auth9/core` 的 `Auth9Client` 通过 getter 属性暴露 7 个子客户端，每个子客户端封装一组 REST API：

| 子客户端 | 方法数 | API 前缀 |
|---------|--------|---------|
| `client.tenants` | 8 | `/api/v1/tenants` |
| `client.users` | 13 | `/api/v1/users` |
| `client.services` | 10 | `/api/v1/services` |
| `client.roles` | 7 | `/api/v1/roles` + `/api/v1/services/{id}/roles` |
| `client.permissions` | 3 | `/api/v1/permissions` + `/api/v1/services/{id}/permissions` |
| `client.rbac` | 4 | `/api/v1/rbac` + `/api/v1/users/{id}/tenants/{id}/roles` |
| `client.invitations` | 8 | `/api/v1/invitations` + `/api/v1/tenants/{id}/invitations` |

### 前置条件

- auth9-core 运行中 (`http://localhost:8080/health`)
- 已获取有效的 **Tenant Access Token**（平台管理员）

> **⚠️ 重要: Token 类型说明**
> `gen-admin-token.sh` 生成的是 **Identity Token**（`token_type: "identity"`），只能用于 tenant-token exchange 和 userinfo 端点。
> 本文档中的所有 CRUD 操作（tenants / users / services / roles / permissions / invitations）需要使用 **Tenant Access Token**。
> 必须先用 Identity Token 换取 Tenant Access Token（见步骤 0）。

---

## 步骤 0：获取 Tenant Access Token

```bash
# 1. 获取 Identity Token
IDENTITY_TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $IDENTITY_TOKEN | head -c 20

# 2. 获取 tenant_id（使用 identity token 可以访问 userinfo 或直接从数据库查）
TENANT_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;" 2>/dev/null)
echo "Tenant: $TENANT_ID"

# 3. 用 Identity Token 换取 Tenant Access Token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"tenant_id\":\"$TENANT_ID\",\"service_id\":\"auth9-portal\"}" | jq -r '.access_token')
echo $TOKEN | head -c 20
```

**预期**: 最终 `$TOKEN` 为 Tenant Access Token（非空），后续步骤均使用此 token

---

## 场景 1：Tenants CRUD 全流程

> **⚠️ 重要**: Tenant Access Token 具有租户作用域。创建新租户后，GET/PUT/DELETE 该新租户时，如果 token 是为其他租户签发的，即使是平台管理员也会返回 403（`"Cannot access another tenant with a tenant-scoped token"`）。这是安全设计：TenantAccess token 始终限于签发时的租户上下文。测试 GET 新租户时，需使用场景中同一步骤创建+获取的方式（见步骤 2），或使用 Identity Token 代替。

### 步骤

1. **创建租户**

```bash
curl -s -X POST http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Test Tenant","slug":"sdk-test-tenant"}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.name` = "SDK Test Tenant"，`data.slug` = "sdk-test-tenant"，`data.status` = "active"

2. **获取租户**

```bash
TENANT_ID=$(curl -s -X POST http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Get Test","slug":"sdk-get-test"}' | jq -r '.data.id')

curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `data.id` = `$TENANT_ID`

3. **更新租户**

```bash
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Updated Tenant"}' | jq .
```

**预期**: 返回 `data.name` = "SDK Updated Tenant"

4. **列出租户**

```bash
curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 1

5. **删除租户**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/tenants/$TENANT_ID \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-Confirm-Destructive: true" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 场景 2：Users 子客户端 — 创建与租户关联

### 步骤

1. **创建用户**

```bash
curl -s -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"email":"sdk-test-user@auth9.dev","displayName":"SDK Test User"}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.email` = "sdk-test-user@auth9.dev"

2. **获取用户**

```bash
USER_ID=$(curl -s -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"email":"sdk-get-user@auth9.dev","displayName":"Get Test"}' | jq -r '.data.id')

curl -s http://localhost:8080/api/v1/users/$USER_ID \
  -H "Authorization: Bearer $TOKEN" | jq '.data.email'
```

**预期**: 返回 "sdk-get-user@auth9.dev"

3. **列出用户**

```bash
curl -s http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 1

4. **删除用户**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/users/$USER_ID \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 场景 3：Services 子客户端 — CRUD 与 Client 管理

### 步骤

1. **创建服务**

> **⚠️ 注意**: API 字段使用 **snake_case**（如 `redirect_uris`），非 camelCase。`client_id` 由服务端自动生成，无需提供。`redirect_uris` 为必填字段。

```bash
curl -s -X POST http://localhost:8080/api/v1/services \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Test Service","redirect_uris":["https://sdk-test.auth9.dev/callback"]}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.name` = "SDK Test Service"

2. **获取服务集成信息**

```bash
SVC_ID=$(curl -s -X POST http://localhost:8080/api/v1/services \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Integration Test","redirect_uris":["https://sdk-int.auth9.dev/callback"]}' | jq -r '.data.id')

curl -s http://localhost:8080/api/v1/services/$SVC_ID/integration \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回包含 `issuerUrl`、`authorizationEndpoint`、`tokenEndpoint` 的对象

3. **删除服务**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/services/$SVC_ID \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 场景 4：Roles & Permissions — RBAC 管理

### 步骤

> **⚠️ 注意**: API 字段使用 **snake_case**（如 `service_id`），非 camelCase。

1. **创建权限**

```bash
curl -s -X POST http://localhost:8080/api/v1/permissions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"service_id":"'$SVC_ID'","code":"sdk:test:read","name":"SDK Test Read"}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.code` = "sdk:test:read"

2. **创建角色**

```bash
curl -s -X POST http://localhost:8080/api/v1/roles \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"service_id":"'$SVC_ID'","name":"SDK Test Role"}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.name` = "SDK Test Role"

3. **列出服务的角色**

```bash
curl -s http://localhost:8080/api/v1/services/$SVC_ID/roles \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 1

---

## 场景 5：Invitations — 创建与撤销

### 步骤

1. **创建邀请**

> **⚠️ 注意**: `role_ids` 至少需要 1 个角色 ID（验证规则要求 `min = 1`）。先从场景 4 获取 `$ROLE_ID`。

```bash
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/invitations \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"email":"sdk-invite@auth9.dev","role_ids":["'$ROLE_ID'"]}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.status` = "pending"

2. **列出邀请**

```bash
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/invitations \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 1

3. **撤销邀请**

```bash
INV_ID=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/invitations \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s -X POST http://localhost:8080/api/v1/invitations/$INV_ID/revoke \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Tenants CRUD 全流程 | ☐ | | | |
| 2 | Users 子客户端 — 创建与租户关联 | ☐ | | | |
| 3 | Services 子客户端 — CRUD 与 Client 管理 | ☐ | | | |
| 4 | Roles & Permissions — RBAC 管理 | ☐ | | | |
| 5 | Invitations — 创建与撤销 | ☐ | | | |
