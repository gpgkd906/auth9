# 集成测试 - 中性身份字段迁移回归

**模块**: 集成测试
**测试范围**: `users.identity_subject`、`sessions.provider_session_id`、`enterprise_sso_connectors.provider_alias` 的 schema 回填、主路径读写与兼容性
**场景数**: 4
**优先级**: 高

---

## 背景说明

Phase 1 FR3 将用户、会话、企业 SSO connector 的核心模型从 `keycloak_*` 命名迁移到中性字段：

- `users.identity_subject`
- `sessions.provider_session_id`
- `enterprise_sso_connectors.provider_alias`

本阶段仍保留旧列，目标是验证：

1. migration 已新增新列并完成回填
2. 新创建的数据优先写入中性字段，同时兼容旧列
3. 运行中的 API 主路径返回中性字段
4. 企业 SSO 连接器创建/查询/测试/删除不回归

---

## 场景 1：migration 后三张表存在中性字段并具备索引

### 初始状态
- `auth9-init` 已完成
- `auth9-core` 已启动

### 目的
验证 schema 已完成新增字段与索引创建。

### 测试操作流程
1. 查询列定义：
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SHOW COLUMNS FROM users LIKE 'identity_subject';
SHOW COLUMNS FROM sessions LIKE 'provider_session_id';
SHOW COLUMNS FROM enterprise_sso_connectors LIKE 'provider_alias';
"
```
2. 查询索引：
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SHOW INDEX FROM users WHERE Key_name IN ('idx_users_identity_subject_unique','idx_users_identity_subject');
SHOW INDEX FROM sessions WHERE Key_name='idx_sessions_provider_session';
SHOW INDEX FROM enterprise_sso_connectors WHERE Key_name='idx_enterprise_sso_provider_alias';
"
```

### 预期结果
- 三张表均存在新列
- `users.identity_subject`、`enterprise_sso_connectors.provider_alias` 为唯一索引
- `sessions.provider_session_id` 为普通查询索引

---

## 场景 2：创建用户后 API 与数据库优先返回 `identity_subject`

### 初始状态
- 已获取平台管理员 Identity Token：
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

### 目的
验证用户创建路径已把中性字段作为主读写语义。

### 测试操作流程
1. 创建用户：
```bash
EMAIL="qa-neutral-$(date +%H%M%S)@example.com"
curl -X POST 'http://localhost:8080/api/v1/users' \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"display_name\":\"Neutral Model QA\"}"
```
2. 查询数据库：
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT email, identity_subject, keycloak_id
FROM users
WHERE email = '$EMAIL';
"
```

### 预期结果
- HTTP 状态码 `201`
- 返回体 `data.identity_subject` 非空，不再暴露 `data.keycloak_id`
- 数据库中 `identity_subject` 非空，且在当前 migration period 下与 `keycloak_id` 保持一致

---

## 场景 3：Session API 在新 schema 下保持可访问

### 步骤 0：生成带 `sid` 的 Identity Token

```bash
ADMIN_USER_ID=$(mysql -N -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "SELECT id FROM users WHERE email='admin@auth9.local' LIMIT 1;")

SESSION_TOKEN=$(ADMIN_USER_ID="$ADMIN_USER_ID" node - <<'NODE'
const jwt = require('jsonwebtoken');
const fs = require('fs');
const key = fs.readFileSync('.claude/skills/tools/jwt_private_clean.key', 'utf8');
const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: process.env.ADMIN_USER_ID,
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

### 目的
验证会话读取主路径在 schema 迁移后未回退。

### 测试操作流程
1. 调用会话列表接口：
```bash
curl 'http://localhost:8080/api/v1/users/me/sessions' \
  -H "Authorization: Bearer $SESSION_TOKEN"
```

### 预期结果
- HTTP 状态码 `200`
- 返回体仍为 `{"data":[...]}` 结构
- 不出现 `500`、`unknown column provider_session_id`、`unknown field keycloak_session_id` 一类错误

---

## 场景 4：企业 SSO connector 主路径使用 `provider_alias`

### 步骤 0：生成 Tenant Owner Token

```bash
TENANT_ID=$(mysql -N -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "SELECT id FROM tenants WHERE slug='demo' LIMIT 1;")
TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner --tenant-id "$TENANT_ID")
```

### 目的
验证 connector 创建、查询、测试、删除主路径已切到 `provider_alias`，且数据库兼容写入旧列。

### 测试操作流程
1. 创建连接器：
```bash
ALIAS="neutral-$(date +%H%M%S)"
DOMAIN="$ALIAS.example.com"

curl -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{
    \"alias\":\"$ALIAS\",
    \"provider_type\":\"saml\",
    \"enabled\":true,
    \"priority\":100,
    \"domains\":[\"$DOMAIN\"],
    \"config\":{
      \"entityId\":\"https://$DOMAIN/entity\",
      \"ssoUrl\":\"https://$DOMAIN/sso\",
      \"certificate\":\"-----BEGIN CERTIFICATE-----TEST-----END CERTIFICATE-----\"
    }
  }"
```
2. 调用列表与 test 接口：
```bash
curl "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors" \
  -H "Authorization: Bearer $TOKEN"

curl -X POST "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/{connector_id}/test" \
  -H "Authorization: Bearer $TOKEN"
```
3. 查询数据库：
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT alias, provider_alias, keycloak_alias
FROM enterprise_sso_connectors
WHERE id = '{connector_id}';
"
```
4. 删除连接器：
```bash
curl -X DELETE "http://localhost:8080/api/v1/tenants/$TENANT_ID/sso/connectors/{connector_id}" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 创建响应返回 `data.provider_alias`
- 列表接口返回的 connector 结构包含 `provider_alias`
- test 接口返回 `ok = true`
- 数据库中 `provider_alias` 非空，且 migration period 下与 `keycloak_alias` 一致
- 删除接口返回成功，连接器被清理

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | migration 后三张表存在中性字段并具备索引 | ☐ | | | |
| 2 | 创建用户后 API 与数据库优先返回 `identity_subject` | ☐ | | | |
| 3 | Session API 在新 schema 下保持可访问 | ☐ | | | |
| 4 | 企业 SSO connector 主路径使用 `provider_alias` | ☐ | | | |
