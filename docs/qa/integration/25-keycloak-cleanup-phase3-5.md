# Keycloak 遗留清理 Phase 3-5 验证

**模块**: integration
**关联 FR**: `docs/feature_request/cleanup_keycloak_remnants.md` (Phase 3 + Phase 4 + Phase 5)
**前置条件**: auth9-core 和 auth9-portal 已构建部署

---

## 场景 1: 新路径接收 Identity Event

**目的**: 验证 POST `/api/v1/identity/events`（canonical 路径）能正确接收 Identity Event 并返回 200。

**类型**: API 验证

### 步骤 0: Gate Check

```bash
# 确认 API 健康
curl -sf http://localhost:8080/health && echo "OK"
```

### 步骤

1. 构造 HMAC 签名请求体

```bash
# 设置 webhook secret（使用新的 IDENTITY_WEBHOOK_SECRET）
SECRET="${IDENTITY_WEBHOOK_SECRET:-your-webhook-secret}"

# 构造 event payload
PAYLOAD='{"type":"user.login","userId":"test-user-001","tenantId":"test-tenant-001","timestamp":"2026-03-21T00:00:00Z"}'

# 计算 HMAC-SHA256 签名
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $2}')
```

2. 发送到新路径

```bash
curl -v -X POST http://localhost:8080/api/v1/identity/events \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Signature: sha256=$SIGNATURE" \
  -d "$PAYLOAD"
```

### 预期结果

- HTTP 状态码: `200`
- 响应体包含确认消息或空 body
- 服务日志中记录 identity event 处理

### 代码验证

```bash
# 确认路由注册
grep -n "identity/events" auth9-core/src/server/mod.rs
# 预期: 包含 POST /api/v1/identity/events 路由

# 确认文件已重命名
ls auth9-core/src/domains/identity/api/identity_event.rs
# 预期: 文件存在

ls auth9-core/src/domains/identity/api/keycloak_event.rs 2>&1
# 预期: No such file or directory

# 确认结构体已重命名
grep -n "struct IdentityEvent" auth9-core/src/domains/identity/api/identity_event.rs
# 预期: 至少 1 个匹配

grep -rn "struct KeycloakEvent" auth9-core/src/
# 预期: 无匹配
```

---

## 场景 2: 旧路径兼容性

**目的**: 验证 POST `/api/v1/keycloak/events`（deprecated alias）仍然能正常工作，确保向后兼容。

**类型**: API 验证

### 步骤 0: Gate Check

```bash
curl -sf http://localhost:8080/health && echo "OK"
```

### 步骤

1. 使用与场景 1 相同的 payload 和签名

```bash
SECRET="${IDENTITY_WEBHOOK_SECRET:-your-webhook-secret}"
PAYLOAD='{"type":"user.login","userId":"test-user-001","tenantId":"test-tenant-001","timestamp":"2026-03-21T00:00:00Z"}'
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $2}')
```

2. 发送到旧路径

```bash
curl -v -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Signature: sha256=$SIGNATURE" \
  -d "$PAYLOAD"
```

### 预期结果

- HTTP 状态码: `200`
- 行为与新路径 `/api/v1/identity/events` 完全一致
- 两个路径共用同一个 handler 函数

### 代码验证

```bash
# 确认两条路由指向同一 handler
grep -n "keycloak/events\|identity/events" auth9-core/src/server/mod.rs
# 预期: 两条路由均注册，handler 函数相同
```

---

## 场景 3: IDENTITY_WEBHOOK_SECRET 优先级

**目的**: 验证配置读取逻辑优先使用 `IDENTITY_WEBHOOK_SECRET`，回退到 `KEYCLOAK_WEBHOOK_SECRET`。

**类型**: 配置验证

### 步骤

1. 验证优先级逻辑（代码级）

```bash
# 确认配置读取逻辑
grep -rn "IDENTITY_WEBHOOK_SECRET\|KEYCLOAK_WEBHOOK_SECRET" auth9-core/src/config/
# 预期: IDENTITY_WEBHOOK_SECRET 为首选项，KEYCLOAK_WEBHOOK_SECRET 为 fallback
```

2. 验证 `.env.example` 更新

```bash
# 确认 .env.example 使用新变量名
grep -n "IDENTITY_WEBHOOK_SECRET" auth9-core/.env.example
# 预期: 至少 1 个匹配

grep -n "IDENTITY_BACKEND" auth9-core/.env.example
# 预期: IDENTITY_BACKEND=auth9_oidc（默认值）
```

3. 验证 K8s 部署配置

```bash
# 确认 K8s secret 模板已更新
grep -rn "IDENTITY_WEBHOOK_SECRET" k8s/ deploy/ || echo "检查部署模板目录"
# 预期: 新变量名出现在 secret 或 configmap 定义中

grep -rn "KEYCLOAK_WEBHOOK_SECRET" k8s/ deploy/ || echo "旧变量已移除或标记为 deprecated"
```

4. 运行时验证（仅设置旧变量，确认 fallback 生效）

```bash
# 场景 A: 仅设置 IDENTITY_WEBHOOK_SECRET
IDENTITY_WEBHOOK_SECRET="new-test-value" cargo run -- --check-config 2>&1 | grep -i "webhook"  # pragma: allowlist secret
# 预期: 配置加载成功，使用 new-test-value

# 场景 B: 仅设置 KEYCLOAK_WEBHOOK_SECRET（回退）
unset IDENTITY_WEBHOOK_SECRET
KEYCLOAK_WEBHOOK_SECRET="old-test-value" cargo run -- --check-config 2>&1 | grep -i "webhook"  # pragma: allowlist secret
# 预期: 配置加载成功，回退使用 old-test-value
```

### 预期结果

- 当 `IDENTITY_WEBHOOK_SECRET` 和 `KEYCLOAK_WEBHOOK_SECRET` 同时设置时，使用前者
- 仅设置 `KEYCLOAK_WEBHOOK_SECRET` 时，回退使用该值
- `.env.example` 默认使用 `IDENTITY_WEBHOOK_SECRET`
- `.env.example` 中 `IDENTITY_BACKEND` 默认值为 `auth9_oidc`

---

## 场景 4: Enterprise SSO 连接器返回 provider_alias

**目的**: 验证 GET 租户 SSO 连接器接口返回 `provider_alias` 字段（而非旧的 `keycloak_alias`）。

**类型**: API 验证

### 步骤 0: Gate Check

```bash
curl -sf http://localhost:8080/health && echo "OK"
```

### 步骤

1. 获取管理员 Token

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

2. 查询 SSO 连接器

```bash
# 假设已有租户和 SSO 配置
TENANT_ID="your-test-tenant-id"

curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/sso-connectors \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

3. 验证字段名称

```bash
# 确认响应中使用 provider_alias
curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/sso-connectors \
  -H "Authorization: Bearer $TOKEN" | jq '.[0] | keys' | grep -E "provider_alias|keycloak_alias"
# 预期: 包含 provider_alias，不包含 keycloak_alias
```

### 代码验证

```bash
# Portal 类型定义已更新
grep -rn "provider_alias" auth9-portal/app/
# 预期: SSO 连接器相关类型使用 provider_alias

grep -rn "keycloak_alias" auth9-portal/app/
# 预期: 无匹配（已完全替换）

# 后端 API 响应结构已更新
grep -rn "provider_alias" auth9-core/src/domains/
# 预期: enterprise SSO 相关模型使用 provider_alias

grep -rn "keycloak_alias" auth9-core/src/domains/
# 预期: 无匹配
```

### 预期结果

- API 响应 JSON 中使用 `provider_alias` 字段
- Portal 前端代码中所有 `keycloak_alias` 引用已替换为 `provider_alias`
- 后端 model 和序列化逻辑中无 `keycloak_alias` 残留

---

## 场景 5: BackendIdentityProvider 序列化兼容

**目的**: 验证 `BackendIdentityProvider`（原 `KeycloakIdentityProvider`）的 JSON 序列化/反序列化使用 camelCase，保持向后兼容。

**类型**: 单元测试验证

### 步骤

1. 运行相关单元测试

```bash
cd auth9-core

# 运行 identity provider 相关测试
cargo test backend_identity_provider -- --nocapture
# 预期: 所有测试通过

# 运行完整的 identity 模块测试
cargo test identity -- --nocapture 2>&1 | grep "test result"
# 预期: test result: ok
```

2. 代码级验证

```bash
# 确认结构体重命名
grep -rn "struct BackendIdentityProvider" auth9-core/src/
# 预期: 至少 1 个匹配

grep -rn "struct KeycloakIdentityProvider" auth9-core/src/
# 预期: 无匹配

# 确认 serde 使用 camelCase
grep -A 2 "struct BackendIdentityProvider" auth9-core/src/ -rn
# 预期: #[serde(rename_all = "camelCase")] 注解存在
```

3. 序列化兼容性验证

```bash
# 确认 JSON 字段名为 camelCase 格式
grep -rn "camelCase" auth9-core/src/domains/ | grep -i "identity.*provider\|backend.*provider"
# 预期: BackendIdentityProvider 使用 camelCase 序列化
```

### 预期结果

- `KeycloakIdentityProvider` 已重命名为 `BackendIdentityProvider`
- `#[serde(rename_all = "camelCase")]` 注解保留，确保 JSON 序列化兼容
- 所有引用旧结构体名的代码已更新
- 单元测试全部通过
