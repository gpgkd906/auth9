# 集成测试 - Identity Engine 接口与 State 注入回归

**模块**: 集成测试
**测试范围**: `IdentityEngine` 抽象注入、Keycloak adapter 注入链、SessionService 回归、Identity Provider Service 回归
**场景数**: 3
**优先级**: 高

---

## 背景说明

本用例用于验证 Phase 1 FR1 完成后的关键回归点：

- `AppState` / `TestAppState` 已持有抽象身份后端，而不是仅依赖 `KeycloakClient`
- 默认 Keycloak backend 通过 `identity_engine/adapters/keycloak/` 注入，而不是直接把 `KeycloakClient` 当作 trait object
- `SessionService` 通过抽象会话能力访问身份后端
- `IdentityProviderService` 通过抽象联邦能力访问身份后端
- `KeycloakSyncService` 通过统一身份引擎抽象更新 realm 配置

该 FR 不改变外部 API 契约，因此 QA 重点是“现有行为未回退”。

---

## 场景 1：服务启动后健康检查正常

### 初始状态
- `auth9-core`、`auth9-keycloak`、`auth9-redis`、`auth9-tidb` 已启动

### 目的
验证 State 构造链在切换为 `IdentityEngine` 注入后仍可正常完成，服务可对外提供基础健康探针。

### 测试操作流程
1. 调用健康检查端点：
   ```bash
   curl -v http://localhost:8080/health
   ```

### 预期结果
- 返回 `200 OK`
- 响应 JSON 包含 `status = "healthy"`
- 不出现启动失败、依赖注入失败、服务注册缺失

---

## 场景 2：Session API 保持可访问

#### 步骤 0：生成带 `sid` 的 Identity Token

`/api/v1/users/me/sessions` 当前要求 Identity Token，且 token 中必须携带可解析的 `sid` claim。

```bash
IDENTITY_TOKEN=$(node - <<'NODE'
const jwt = require('jsonwebtoken');
const fs = require('fs');
const key = fs.readFileSync('.claude/skills/tools/jwt_private_clean.key', 'utf8');
const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: '47116b28-b60b-4b73-a9d0-baace9245cf0',
  email: 'admin@auth9.local',
  name: 'Platform Admin',
  token_type: 'identity',
  iss: 'http://localhost:8080',
  aud: 'auth9',
  sid: '11111111-1111-1111-1111-111111111111',
  iat: now,
  exp: now + 3600
};
process.stdout.write(jwt.sign(payload, key, { algorithm: 'RS256', keyid: 'auth9-current' }));
NODE
)
```

### 初始状态
- 服务已启动
- 已完成步骤 0

### 目的
验证 `SessionService` 通过抽象依赖注入后，现有 Session API 仍可正常进入业务层并返回响应。

### 测试操作流程
1. 调用会话列表接口：
   ```bash
   curl -v \
     -H "Authorization: Bearer $IDENTITY_TOKEN" \
     http://localhost:8080/api/v1/users/me/sessions
   ```

### 预期结果
- 返回 `200 OK`
- 响应格式保持为 `{"data":[...]}` 或 `{"data":[]}`
- 不返回 `500`、`panic`、`service not found`、`missing state` 一类错误

---

## 场景 3：Identity Provider API 保持可访问

#### 步骤 0：生成 Tenant Owner Access Token

```bash
TENANT_OWNER_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner)
```

### 初始状态
- 服务已启动
- 已完成步骤 0

### 目的
验证 `IdentityProviderService` 切换到 `FederationBroker` 抽象后，现有受保护端点仍可正常工作。

### 测试操作流程
1. 调用模板列表接口：
   ```bash
   curl -v \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     http://localhost:8080/api/v1/identity-providers/templates
   ```
2. 调用身份提供商列表接口：
   ```bash
   curl -v \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     http://localhost:8080/api/v1/identity-providers
   ```

### 预期结果
- 两个接口均返回 `200 OK`
- `/templates` 返回 provider 模板数组
- `/identity-providers` 返回当前 provider 列表，可为空数组
- 不返回 `500`、`panic`、`missing federation broker` 一类错误

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 服务启动后健康检查正常 | ☐ | | | |
| 2 | Session API 保持可访问 | ☐ | | | |
| 3 | Identity Provider API 保持可访问 | ☐ | | | |
