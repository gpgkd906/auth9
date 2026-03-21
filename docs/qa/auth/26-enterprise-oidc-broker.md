# 认证流程 - 企业 OIDC Broker 原生登录测试

**模块**: 认证流程
**测试范围**: Auth9 原生企业 OIDC 连接器 broker 流程（authorize、callback、用户创建、租户关联、claim mapping）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 原生 broker 直接处理企业 OIDC authorization code 流程：authorize → 外部 IdP → callback → token exchange → userinfo → claim mapping → 用户解析 → 登录完成。（注：Keycloak 已退役，所有企业 OIDC broker 流程由 Auth9 内置引擎处理）

端点：
- `GET /api/v1/enterprise-sso/authorize/{alias}` — 发起企业 OIDC 登录
- `GET /api/v1/enterprise-sso/callback` — 处理 IdP 回调
- `POST /api/v1/tenants/{tenant_id}/sso/connectors` — CRUD 管理
- `POST /api/v1/tenants/{tenant_id}/sso/connectors/{id}/test` — 连通性测试

---

## 数据库表结构参考

### `enterprise_sso_connectors` 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 连接器主键 |
| tenant_id | CHAR(36) | 租户 ID |
| alias | VARCHAR(100) | 连接器别名 |
| provider_type | VARCHAR(20) | `saml` / `oidc` |
| enabled | BOOLEAN | 是否启用 |
| config | JSON | 协议配置（含 clientId, clientSecret, authorizationUrl, tokenUrl, userInfoUrl, scopes, claimSub, claimEmail, claimName） |

### `linked_identities` 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 主键 |
| user_id | CHAR(36) | Auth9 用户 ID |
| provider_type | VARCHAR(50) | `oidc` |
| provider_alias | VARCHAR(100) | 连接器 alias |
| external_user_id | VARCHAR(255) | 外部 IdP 用户标识 |
| external_email | VARCHAR(255) | 外部 IdP 邮箱 |

---

## 场景 1：创建 OIDC 连接器仅保存到 Auth9 数据库

### 步骤 0（Gate Check）
- 已获取管理员 JWT token
- 已存在租户 `{tenant_id}`

### 目的
验证创建 OIDC 类型连接器时仅保存到 Auth9 数据库

### 测试操作流程
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "corp-oidc-test",
    "display_name": "Corp OIDC Test",
    "provider_type": "oidc",
    "enabled": true,
    "priority": 100,
    "domains": ["corp.example.com"],
    "config": {
      "clientId": "test-client-id",
      "clientSecret": "test-placeholder-value",
      "authorizationUrl": "https://idp.example.com/authorize",
      "tokenUrl": "https://idp.example.com/token",
      "userInfoUrl": "https://idp.example.com/userinfo"
    }
  }'
```

### 预期结果
- HTTP 200，返回创建的连接器
- `provider_type` 为 `oidc`

### 预期数据状态
```sql
SELECT id, alias, provider_type, enabled, provider_alias
FROM enterprise_sso_connectors
WHERE alias = 'corp-oidc-test' AND tenant_id = '{tenant_id}';
-- 预期: 1 行，provider_type = 'oidc', enabled = 1

SELECT domain FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 1 行，domain = 'corp.example.com'
```

---

## 场景 2：OIDC 连接器 userInfoUrl 为必填字段

### 步骤 0（Gate Check）
- 已获取管理员 JWT token

### 目的
验证创建 OIDC 连接器时 `userInfoUrl` 为必填字段

### 测试操作流程
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "missing-userinfo",
    "provider_type": "oidc",
    "domains": ["missing.example.com"],
    "config": {
      "clientId": "test",
      "clientSecret": "test-value",
      "authorizationUrl": "https://idp.example.com/auth",
      "tokenUrl": "https://idp.example.com/token"
    }
  }'
```

### 预期结果
- HTTP 422 或 400
- 错误信息包含 `userInfoUrl`

### 预期数据状态
```sql
SELECT COUNT(*) FROM enterprise_sso_connectors WHERE alias = 'missing-userinfo';
-- 预期: 0（未创建）
```

---

## 场景 3：OIDC 连接器测试端点验证外部 IdP 可达性

### 步骤 0（Gate Check）
- 场景 1 的连接器已创建

### 目的
验证 test 端点对 OIDC 连接器检查实际 IdP 端点可达性

### 测试操作流程
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/test" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200
- 对于不可达的外部 IdP（`https://idp.example.com` 不存在）：`ok: false`，`message` 包含 `unreachable` 或连接错误信息
- 对于可达的 IdP：`ok: true`，`message` 包含 `OIDC authorization endpoint is reachable`

---

## 场景 4：删除 OIDC 连接器

### 步骤 0（Gate Check）
- 场景 1 的连接器已创建

### 目的
验证删除 OIDC 连接器时正确清理 Auth9 数据库

### 测试操作流程
```bash
curl -X DELETE "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200，消息为 `Connector deleted successfully.`

### 预期数据状态
```sql
SELECT COUNT(*) FROM enterprise_sso_connectors WHERE id = '{connector_id}';
-- 预期: 0

SELECT COUNT(*) FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 0
```

---

## 场景 5：Discovery 对 OIDC 连接器返回 Auth9 broker URL

### 步骤 0（Gate Check）
- 已创建 OIDC 连接器，绑定域名 `corp.example.com`

### 目的
验证 discovery 对 OIDC 连接器返回 Auth9 原生 broker 地址

### 测试操作流程
```bash
curl -X POST 'http://localhost:8080/api/v1/enterprise-sso/discovery?response_type=code&client_id=auth9-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback&scope=openid%20email%20profile&state=test-state&nonce=test-nonce' \
  -H 'Content-Type: application/json' \
  -d '{"email":"user@corp.example.com"}'
```

### 预期结果
- HTTP 200
- `data.authorize_url` 包含 `/api/v1/enterprise-sso/authorize/` 和 `login_challenge=`
- `data.authorize_url` 包含 `login_hint=user%40corp.example.com`
- `data.authorize_url` **不包含** `kc_idp_hint`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | OIDC 连接器创建仅保存到 Auth9 数据库 | ☐ | | | API |
| 2 | userInfoUrl 为 OIDC 必填字段 | ☐ | | | API |
| 3 | OIDC 连接器 test 端点验证 IdP 可达性 | ☐ | | | API |
| 4 | OIDC 连接器删除 | ☐ | | | API |
| 5 | Discovery 对 OIDC 返回 Auth9 broker URL | ☐ | | | API |
