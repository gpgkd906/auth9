# SDK - 安全与企业功能 API 子客户端

**模块**: SDK
**测试范围**: Auth9Client 的 identityProviders / sso / saml / abac / sessions / webhooks / scim / tenantServices 子客户端
**场景数**: 5
**优先级**: 高

---

## 背景说明

### 子客户端架构

`@auth9/core` 的 `Auth9Client` Phase 2 新增 8 个子客户端，封装企业级安全功能 REST API：

| 子客户端 | 方法数 | API 前缀 |
|---------|--------|---------|
| `client.identityProviders` | 8 | `/api/v1/identity-providers` + `/api/v1/users/me/linked-identities` |
| `client.sso` | 5 | `/api/v1/tenants/{id}/sso/connectors` |
| `client.saml` | 8 | `/api/v1/tenants/{id}/saml-apps` |
| `client.abac` | 6 | `/api/v1/tenants/{id}/abac` |
| `client.sessions` | 4 | `/api/v1/users/me/sessions` + `/api/v1/admin/users/{id}/logout` |
| `client.webhooks` | 7 | `/api/v1/tenants/{id}/webhooks` |
| `client.scim` | 6 | `/api/v1/tenants/{id}/sso/connectors/{id}/scim` |
| `client.tenantServices` | 3 | `/api/v1/tenants/{id}/services` |

### 前置条件

- auth9-core 运行中 (`http://localhost:8080/health`)
- 已获取有效的 **Tenant Access Token**（平台管理员）
- 已有至少一个租户（用于 tenant-scoped 端点）

> **⚠️ 重要: API 字段命名规范**
> auth9 API 使用 **snake_case** 命名（如 `display_name`、`provider_id`、`service_id`），不使用 camelCase。
> 请求和响应的 JSON 字段均为 snake_case。

---

## 步骤 0：获取 Tenant Access Token 和租户 ID

```bash
# 1. 获取 Identity Token
IDENTITY_TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $IDENTITY_TOKEN | head -c 20

# 2. 获取 tenant_id
TENANT_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;" 2>/dev/null)
echo "Tenant: $TENANT_ID"

# 3. 用 Identity Token 换取 Tenant Access Token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"tenant_id\":\"$TENANT_ID\",\"service_id\":\"auth9-portal\"}" | jq -r '.access_token')
echo $TOKEN | head -c 20
```

**预期**: Token 非空，TENANT_ID 非空

---

## 场景 1：Identity Providers — CRUD 与模板查询

### 步骤

1. **获取 IdP 模板列表**

```bash
curl -s http://localhost:8080/api/v1/identity-providers/templates \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 0（模板列表可能为空，但端点应正常响应）

2. **创建 Identity Provider**

```bash
curl -s -X POST http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"alias":"sdk-test-google","display_name":"SDK Test Google","provider_id":"google","config":{"client_id":"test-client-id","client_secret":"test-client-secret"}}' | jq .  # pragma: allowlist secret
```

**预期**: 返回 `data.alias` = "sdk-test-google"，`data.provider_id` = "google"，`data.enabled` = true

3. **获取 Identity Provider**

```bash
curl -s http://localhost:8080/api/v1/identity-providers/sdk-test-google \
  -H "Authorization: Bearer $TOKEN" | jq '.data.alias'
```

**预期**: 返回 "sdk-test-google"

4. **更新 Identity Provider**

```bash
curl -s -X PUT http://localhost:8080/api/v1/identity-providers/sdk-test-google \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"SDK Updated Google","enabled":false}' | jq '.data.display_name'
```

**预期**: 返回 "SDK Updated Google"

5. **删除 Identity Provider**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/identity-providers/sdk-test-google \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 场景 2：Webhooks — CRUD 与测试触发

### 步骤

1. **创建 Webhook**

```bash
curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Test Webhook","url":"https://webhook-test.auth9.dev/hook","events":["user.created","user.updated"]}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.name` = "SDK Test Webhook"，`data.enabled` = true

2. **获取 Webhook**

```bash
WH_ID=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks/$WH_ID \
  -H "Authorization: Bearer $TOKEN" | jq '.data.name'
```

**预期**: 返回 "SDK Test Webhook"

3. **更新 Webhook**

```bash
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks/$WH_ID \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Updated Webhook","enabled":false}' | jq '.data.name'
```

**预期**: 返回 "SDK Updated Webhook"

4. **测试 Webhook**

```bash
curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks/$WH_ID/test \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期**: 返回包含 `success` 字段的对象

5. **删除 Webhook**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks/$WH_ID \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: HTTP 状态码 200 或 204

---

## 场景 3：ABAC Policies — 创建、发布与模拟

### 步骤

1. **创建 ABAC 策略**

```bash
curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/abac/policies \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Test Policy","rules":[{"effect":"allow","subjects":{"role":"admin"},"resources":{"type":"document"},"actions":["read","write"]}]}' | jq .
```

**预期**: 返回 `data.id` 非空，`data.name` = "SDK Test Policy"，`data.status` = "draft"

2. **列出策略**

```bash
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/abac/policies \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 1

3. **发布策略**

```bash
POLICY_VID=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/abac/policies \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].version_id // .data[0].versionId')

curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/abac/policies/$POLICY_VID/publish \
  -H "Authorization: Bearer $TOKEN" | jq '.data.status'
```

**预期**: 返回 "published"

4. **模拟策略评估**

```bash
curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/abac/simulate \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"subject":{"role":"admin"},"resource":{"type":"document"},"action":"read"}' | jq '.data'
```

**预期**: 返回包含 `allowed` 字段的对象

---

## 场景 4：Sessions — 查询与撤销

> **⚠️ 注意**: `/api/v1/users/me/sessions` 端点需要真实的浏览器会话（session cookie），不能仅通过 JWT Bearer token 访问。
> 使用 curl + JWT 调用这些端点会返回 401，这是预期行为。
> 要测试 session 端点，请通过浏览器登录 Portal 后在 DevTools 中操作，或使用 Playwright E2E 测试。

### 步骤

1. **列出当前用户会话**（需通过浏览器会话访问）

```bash
# 注意: 此端点需要真实 browser session，curl + JWT 会返回 401
curl -s http://localhost:8080/api/v1/users/me/sessions \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 若通过浏览器会话访问，返回数量 >= 0；若通过 curl + JWT 访问，返回 401（预期行为）

2. **撤销其他会话**（需通过浏览器会话访问）

```bash
# 注意: 此端点需要真实 browser session，curl + JWT 会返回 401
curl -s -X DELETE http://localhost:8080/api/v1/users/me/sessions \
  -H "Authorization: Bearer $TOKEN" -w "%{http_code}"
```

**预期**: 若通过浏览器会话访问，HTTP 状态码 200 或 204；若通过 curl + JWT，返回 401（预期行为）

---

## 场景 5：Tenant Services — 列出与切换

### 步骤

1. **列出租户服务**

```bash
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期**: 返回数组（可能为空）

2. **获取已启用服务**

```bash
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services/enabled \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 >= 0

3. **切换服务状态**（需要已有服务 ID）

```bash
SVC_ID=$(curl -s http://localhost:8080/api/v1/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id // empty')

if [ -n "$SVC_ID" ]; then
  curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"service_id\":\"$SVC_ID\",\"enabled\":true}" -w "%{http_code}"
fi
```

**预期**: 若有服务，HTTP 状态码 200 或 204；若无服务，跳过

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Identity Providers — CRUD 与模板查询 | ☐ | | | |
| 2 | Webhooks — CRUD 与测试触发 | ☐ | | | |
| 3 | ABAC Policies — 创建、发布与模拟 | ☐ | | | |
| 4 | Sessions — 查询与撤销 | ☐ | | | |
| 5 | Tenant Services — 列出与切换 | ☐ | | | |
