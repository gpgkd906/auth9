# 会话与安全 - 动态 Audience 验证

**模块**: 会话与安全
**测试范围**: Tenant Access Token 的 audience (aud) 动态验证：Redis SET 种子、Service 创建/删除后 audience 自动更新、跨 service_id exchange 可用性
**场景数**: 5
**优先级**: 高

---

## 背景说明

Tenant Access Token 的 `aud` 字段等于签发时使用的 `service_id`（即 `clients.client_id`）。middleware 验证 token 时需要确认 `aud` 对应一个合法的已注册 client。

**当前架构**:
- 启动时从 `clients` 表加载所有 `client_id` 到 Redis SET `auth9:valid_audiences`
- middleware 用 `SISMEMBER` O(1) 查询验证 audience
- Service 创建时 `SADD`，删除时 `SREM`
- 环境变量 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 仅作为额外种子（向后兼容），不再是唯一来源

**被替代的旧架构**:
- ~~静态环境变量 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 白名单~~
- ~~`verify_tenant_access_token_strict` 硬编码 audience 列表~~
- ~~动态创建的 Service 无法通过 middleware 验证~~

---

## 步骤 0: Gate Check

```bash
# 确认 auth9-core 正常运行（audience 验证与 backend 无关）
curl -sf http://localhost:8080/health | jq .

# 确认 Redis 中 audience set 已种子化
docker exec auth9-redis redis-cli SMEMBERS auth9:valid_audiences
# 预期: 至少包含 "auth9-portal"（启动时从 clients 表加载）
```

---

## 场景 1：启动时 audience set 从 DB 正确加载

### 初始状态
- auth9-core 刚启动

### 目的
验证启动时 `clients` 表中所有 `client_id` 被加载到 Redis SET

### 测试操作流程

```bash
# 查询 DB 中的 client_id
mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT client_id FROM clients ORDER BY client_id;"

# 查询 Redis SET
docker exec auth9-redis redis-cli SMEMBERS auth9:valid_audiences
```

### 预期结果
- Redis SET 包含 DB 中所有 `client_id`
- 如果 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 环境变量非空，其值也在 SET 中
- 启动日志包含 `Audience validation set loaded into Redis` 及 count

---

## 场景 2：不同 service_id 签发的 tenant token 均可通过 middleware

### 初始状态
- 用户属于 `{tenant_id}`
- 已知至少两个不同的 client_id（如 `auth9-portal`、`auth9-demo`）

### 目的
验证 middleware 不再硬编码 audience 白名单，所有已注册 client 签发的 token 均有效

### 测试操作流程

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# Exchange with auth9-portal
PORTAL_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"{tenant_id}","service_id":"auth9-portal"}' | jq -r '.access_token')

# Exchange with auth9-demo
DEMO_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"{tenant_id}","service_id":"auth9-demo"}' | jq -r '.access_token')

# Both should access protected endpoint
curl -sf http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $PORTAL_TOKEN" | jq '.data | length'

curl -sf http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $DEMO_TOKEN" | jq '.data | length'
```

### 预期结果
- 两个 token 的 `aud` 分别为 `auth9-portal` 和 `auth9-demo`
- 两个 token 均返回 `200 OK`
- 不应出现 `401 Unauthorized`

### 预期数据状态

```sql
-- 两个 client_id 都在 Redis SET 中
-- docker exec auth9-redis redis-cli SISMEMBER auth9:valid_audiences "auth9-portal"
-- → 1
-- docker exec auth9-redis redis-cli SISMEMBER auth9:valid_audiences "auth9-demo"
-- → 1
```

---

## 场景 3：动态创建 Service 后新 client_id 立即可用

### 初始状态
- 用户为 admin，属于 `{tenant_id}`
- `clients` 表中不存在 `dynamic-test-client`

### 目的
验证创建 Service 后 `SADD` 自动触发，新 client_id 的 tenant token 可立即通过 middleware

### 测试操作流程

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
TENANT_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"{tenant_id}","service_id":"auth9-portal"}' | jq -r '.access_token')

# 1. Create new service
# 注意: 服务创建端点是 POST /api/v1/services（不是 /api/v1/tenants/{tenant_id}/services，后者是 toggle_service）
curl -sf -X POST "http://localhost:8080/api/v1/services" \
  -H "Authorization: Bearer $TENANT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Dynamic Test Service","tenant_id":"{tenant_id}","redirect_uris":["https://test.example.com/callback"],"logout_uris":[]}'

# 2. Get new client_id
NEW_CLIENT_ID=$(curl -sf "http://localhost:8080/api/v1/services" \
  -H "Authorization: Bearer $TENANT_TOKEN" | jq -r '.data[-1].clients[0].client_id')

# 3. Verify it's in Redis SET
docker exec auth9-redis redis-cli SISMEMBER auth9:valid_audiences "$NEW_CLIENT_ID"

# 4. Exchange token with new client (requires both tenant_id and service_id)
NEW_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"tenant_id\":\"{tenant_id}\",\"service_id\":\"$NEW_CLIENT_ID\"}" | jq -r '.access_token')

# 5. Use token to access protected endpoint
curl -sf http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $NEW_TOKEN" | jq '.data | length'
```

### 预期结果
- 步骤 3: SISMEMBER 返回 `1`（已在 SET 中）
- 步骤 5: 返回 `200 OK`（无需重启 auth9-core）

---

## 场景 4：删除 Service 后旧 token 失效

### 初始状态
- 上一场景创建的 Service 仍存在
- 持有该 Service 签发的 `{new_token}`

### 目的
验证删除 Service 后 `SREM` 自动触发，旧 token 不再通过 middleware

### 测试操作流程

```bash
# 1. 确认旧 token 仍有效
curl -sf http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer {new_token}" | jq '.data | length'

# 2. 删除 Service
curl -sf -X DELETE "http://localhost:8080/api/v1/tenants/{tenant_id}/services/{service_id}" \
  -H "Authorization: Bearer $TENANT_TOKEN"

# 3. 确认 client_id 已从 Redis SET 移除
docker exec auth9-redis redis-cli SISMEMBER auth9:valid_audiences "{new_client_id}"

# 4. 用旧 token 访问 protected endpoint
curl -s http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer {new_token}" | jq .
```

### 预期结果
- 步骤 1: `200 OK`
- 步骤 3: SISMEMBER 返回 `0`（已移除）
- 步骤 4: `401 Unauthorized`

---

## 场景 5：不存在的 audience 被拒绝

### 初始状态
- Redis SET 中不包含 `fake-nonexistent-client`

### 目的
验证伪造 audience 的 token 无法通过 middleware

### 测试操作流程

手动签发一个 `aud=fake-nonexistent-client` 的 token（或通过测试工具），然后用它访问 protected endpoint。

在无法手动签发时，可通过反面验证：

```bash
# 确认 fake client 不在 SET 中
docker exec auth9-redis redis-cli SISMEMBER auth9:valid_audiences "fake-nonexistent-client"
# 预期: 0
```

### 预期结果
- 伪造 audience 的 token 返回 `401 Unauthorized`
- Redis SISMEMBER 对未注册 client_id 返回 `0`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 启动时 audience set 从 DB 正确加载 | ☐ | | | |
| 2 | 不同 service_id 签发的 tenant token 均可通过 | ☐ | | | **核心回归点：旧版本只允许白名单中的 aud** |
| 3 | 动态创建 Service 后新 client_id 立即可用 | ☐ | | | |
| 4 | 删除 Service 后旧 token 失效 | ☐ | | | |
| 5 | 不存在的 audience 被拒绝 | ☐ | | | |
