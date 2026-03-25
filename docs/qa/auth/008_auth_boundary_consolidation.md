# Auth Boundary Consolidation（认证边界收敛）

**模块**: auth (Token Validation / Access Control)
**测试范围**: AuthUser audience 校验（R1）、Redis 故障 fail-closed 策略（R2）、token_type 区分访问控制（R3）
**场景数**: 5
**优先级**: 高
**前置文档**: [session/09-dynamic-audience-validation.md](../session/09-dynamic-audience-validation.md)

---

## 背景说明

本功能将 AuthUser extractor 的认证边界从"宽松放行"收敛为"严格校验"，涉及三项变更：

**R1 — Audience 动态验证**:
- AuthUser extractor 对 tenant_access token 通过 Redis 缓存验证 audience 是否已注册
- audience 未注册 → 401 Unauthorized
- Redis 缓存查询失败 → 503 Service Unavailable

**R2 — Fail-closed 策略**:
- 移除原有 no-cache fallback 的默认放行逻辑
- Redis 不可用时，audience 校验直接失败，返回 503
- 确保基础设施故障不会导致未授权访问

**R3 — token_type 字段用于访问控制**:
- AuthUser 已携带 `token_type` 字段（Identity / TenantAccess / ServiceClient）
- Handler 可据此实施细粒度访问控制
- Identity token 仅限白名单路径（已有行为不变）
- Service client token 不受影响（已有行为不变）

**受影响行为**:
- 所有受保护端点的 tenant_access token 校验逻辑
- Redis 故障时的降级策略（由 fail-open 改为 fail-closed）

---

## 场景 1：tenant_access token audience 已注册 — 正常放行（R1）

### 步骤 0（Gate Check）
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

# 获取管理员 Token（Identity Token）
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $TOKEN | head -c 20
# 预期: 输出 JWT 前缀

# 获取测试租户 ID
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')
echo $TENANT_ID
# 预期: 非空 UUID

# 获取租户下已注册 service 的 client_id（用于 token exchange）
CLIENT_ID=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].clients[0].client_id')
echo $CLIENT_ID
# 预期: 非空字符串
```

### 初始状态
- Auth9 Core 运行中，Redis 正常
- 租户下存在至少一个 Service（audience 已在 Redis 中注册）

### 目的
验证使用合法 audience 的 tenant_access token 访问受保护端点时正常放行

### 测试操作流程

#### API 测试
1. 通过 token exchange 获取 tenant_access token（audience 为已注册的 service identifier）：
```bash
TENANT_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/token-exchange \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"tenant_id\": \"$TENANT_ID\",
    \"client_id\": \"$CLIENT_ID\"
  }" | jq -r '.access_token')
echo $TENANT_TOKEN | head -c 20
# 预期: 输出 JWT 前缀
```

2. 使用 tenant_access token 访问受保护端点：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TENANT_TOKEN"
```

### 预期结果
- 步骤 1：获得有效的 tenant_access token
- 步骤 2：HTTP **200**，正常返回用户信息

---

## 场景 2：tenant_access token audience 未注册 — 401 拒绝（R1）

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

### 初始状态
- Auth9 Core 运行中，Redis 正常
- 拥有一个 tenant_access token，其 audience 指向已被删除或从未注册的 service

### 目的
验证 audience 未在 Redis 缓存中注册的 tenant_access token 被拒绝访问，返回 401

### 测试操作流程

#### API 测试
1. 构造一个 audience 无效的请求（使用过期或伪造的 tenant_access token，audience 不在已注册列表中）：
```bash
# 方法 A：先获取有效 tenant_access token，然后删除对应 service，再用该 token 请求
# 创建临时 service
TEMP_SERVICE=$(curl -s -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Temp Audience Test Service",
    "identifier": "https://api.example.com/temp-aud-test"
  }')
TEMP_SERVICE_ID=$(echo $TEMP_SERVICE | jq -r '.id // .data.id')
TEMP_CLIENT_ID=$(echo $TEMP_SERVICE | jq -r '.clients[0].client_id // .data.clients[0].client_id')

# 获取该 service 的 tenant_access token
STALE_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/token-exchange \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"tenant_id\": \"$TENANT_ID\",
    \"client_id\": \"$TEMP_CLIENT_ID\"
  }" | jq -r '.access_token')

# 删除该 service（audience 从 Redis 中移除）
curl -s -X DELETE http://localhost:8080/api/v1/tenants/$TENANT_ID/services/$TEMP_SERVICE_ID \
  -H "Authorization: Bearer $TOKEN"
```

2. 使用失效 audience 的 token 访问受保护端点：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $STALE_TOKEN"
```

### 预期结果
- 步骤 2：HTTP **401** Unauthorized，错误消息指示 audience 无效或未注册

---

## 场景 3：Redis 不可用时 audience 校验 fail-closed — 503（R2）

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

CLIENT_ID=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].clients[0].client_id')

# 获取 tenant_access token（在 Redis 正常时获取）
TENANT_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/token-exchange \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"tenant_id\": \"$TENANT_ID\",
    \"client_id\": \"$CLIENT_ID\"
  }" | jq -r '.access_token')
```

### 初始状态
- Auth9 Core 运行中
- 已持有有效的 tenant_access token
- Redis 即将被关停

### 目的
验证 Redis 不可用时，tenant_access token 的 audience 校验执行 fail-closed 策略，返回 503 而非放行

### 测试操作流程

#### API 测试
1. 停止 Redis：
```bash
docker stop auth9-redis
# 或: docker-compose stop redis
```

2. 使用 tenant_access token 访问受保护端点：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TENANT_TOKEN"
```

3. 恢复 Redis：
```bash
docker start auth9-redis
# 或: docker-compose start redis
```

4. 再次访问同一端点，确认恢复：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TENANT_TOKEN"
```

### 预期结果
- 步骤 2：HTTP **503** Service Unavailable（fail-closed，拒绝放行）
- 步骤 4：HTTP **200**，Redis 恢复后正常放行

### 清理
```bash
# 确保 Redis 已恢复
docker start auth9-redis 2>/dev/null; true
```

---

## 场景 4：Identity token 仅限白名单路径 — 非白名单路径 403（R3）

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')
```

### 初始状态
- Auth9 Core 运行中，Redis 正常
- 拥有 Identity token（非 tenant_access token）

### 目的
验证 Identity token（`token_type = Identity`）仅能访问白名单路径，访问需要 tenant_access token 的端点时被拒绝

### 测试操作流程

#### API 测试
1. 使用 Identity token 访问白名单路径（如 `/api/v1/users/me/tenants`）：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/users/me/tenants \
  -H "Authorization: Bearer $TOKEN"
```

2. 使用 Identity token 访问需要 tenant_access 权限的端点：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/tenants/$TENANT_ID/users \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 步骤 1：HTTP **200**，正常返回租户列表（白名单路径允许 Identity token）
- 步骤 2：HTTP **403** Forbidden，Identity token 不允许访问需要 tenant_access 权限的端点

---

## 场景 5：Service client token 不受 audience 校验影响（R3）

### 步骤 0（Gate Check）
```bash
curl -sf http://localhost:8080/health | jq .
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

# 获取 service 的 client credentials
SERVICE_INFO=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" | jq '.data[0].clients[0]')
SC_CLIENT_ID=$(echo $SERVICE_INFO | jq -r '.client_id')
SC_CLIENT_SECRET=$(echo $SERVICE_INFO | jq -r '.client_secret')
SERVICE_IDENTIFIER=$(curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].identifier')
```

### 初始状态
- Auth9 Core 运行中，Redis 正常
- 拥有 Service 的 client_id 和 client_secret

### 目的
验证通过 client_credentials 流获取的 Service client token（`token_type = ServiceClient`）不受 audience 校验逻辑影响，正常访问 API

### 测试操作流程

#### API 测试
1. 通过 client_credentials 获取 service client token：
```bash
SC_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/client-credentials \
  -H "Content-Type: application/json" \
  -d "{
    \"client_id\": \"$SC_CLIENT_ID\",
    \"client_secret\": \"$SC_CLIENT_SECRET\",
    \"audience\": \"$SERVICE_IDENTIFIER\"
  }" | jq -r '.access_token')
echo $SC_TOKEN | head -c 20
# 预期: 输出 JWT 前缀
```

2. 使用 service client token 访问受保护端点：
```bash
curl -s -w "\n%{http_code}" http://localhost:8080/api/v1/tenants/$TENANT_ID/users \
  -H "Authorization: Bearer $SC_TOKEN"
```

### 预期结果
- 步骤 1：获得有效的 service client token
- 步骤 2：HTTP **200**，Service client token 正常访问，不受 audience 动态校验逻辑影响
