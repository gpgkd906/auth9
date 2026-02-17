# 身份提供商 - 租户级企业 SSO 连接器管理测试

**模块**: 身份提供商
**测试范围**: 租户级企业连接器创建、更新、删除、测试与域名唯一约束
**场景数**: 5
**优先级**: 高

---

## 背景说明

新增租户级企业 SSO 管理能力，管理员可在租户维度配置 SAML/OIDC 连接器。

端点：
- `GET /api/v1/tenants/{tenant_id}/sso/connectors`
- `POST /api/v1/tenants/{tenant_id}/sso/connectors`
- `PUT /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}`
- `DELETE /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}`
- `POST /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/test`

---

## 场景 1：创建 SAML 连接器成功

### 初始状态
- 已存在租户 `{tenant_id}`
- 当前租户尚未使用 alias `{connector_alias}`

### 目的
验证 SAML 连接器创建与域名绑定成功落库

### 测试操作流程
1. 在租户详情页进入「Enterprise SSO」
2. 点击「Create Connector」并填写：
   - Alias：`{connector_alias}`
   - Provider Type：`saml`
   - Domains：`{corp_domain}`
   - Entity ID：`{entity_id}`
   - SSO URL：`{sso_url}`
   - Certificate：`{certificate}`
3. 点击「Create Connector」提交
4. 可选：在 `http://localhost:3002/dashboard` 的「Enterprise SSO QA Panel」执行同等操作（Create SAML Connector）

### 预期结果
- 页面提示创建成功
- 连接器列表出现新记录
- 展示 provider type 为 `SAML`，域名包含 `{corp_domain}`

### 预期数据状态
```sql
SELECT id, tenant_id, alias, provider_type, enabled, keycloak_alias
FROM enterprise_sso_connectors
WHERE tenant_id = '{tenant_id}' AND alias = '{connector_alias}';
-- 预期: 返回 1 行，provider_type='saml'

SELECT domain, connector_id
FROM enterprise_sso_domains
WHERE domain = '{corp_domain}';
-- 预期: 返回 1 行，connector_id 对应上方连接器
```

---

## 场景 2：创建 OIDC 连接器成功

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证 OIDC 连接器字段校验与创建

### 测试操作流程
1. 调用 demo 代理创建接口（推荐）：
```bash
curl -X POST 'http://localhost:3002/demo/enterprise/connectors' \
  -H 'Content-Type: application/json' \
  -d '{
    "tenantId":"{tenant_id}",
    "alias":"{oidc_alias}",
    "provider_type":"oidc",
    "enabled":true,
    "priority":100,
    "domains":["{oidc_domain}"],
    "config":{
      "clientId":"{client_id}",
      "clientSecret":"{client_secret}",
      "authorizationUrl":"{authorization_url}",
      "tokenUrl":"{token_url}"
    }
  }'
```
2. 或直连 core 接口：
```bash
curl -X POST 'http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors' \
  -H 'Authorization: Bearer {tenant_access_token}' \
  -H 'Content-Type: application/json' \
  -d '{
    "alias":"{oidc_alias}",
    "provider_type":"oidc",
    "enabled":true,
    "priority":100,
    "domains":["{oidc_domain}"],
    "config":{
      "clientId":"{client_id}",
      "clientSecret":"{client_secret}",
      "authorizationUrl":"{authorization_url}",
      "tokenUrl":"{token_url}"
    }
  }'
```
3. 查询连接器列表确认返回

### 预期结果
- HTTP 状态码 `200`
- 返回的 `data.alias` 为 `{oidc_alias}`
- 可在列表中看到 OIDC 连接器

### 预期数据状态
```sql
SELECT alias, provider_type, JSON_EXTRACT(config, '$.authorizationUrl') AS authorization_url
FROM enterprise_sso_connectors
WHERE tenant_id = '{tenant_id}' AND alias = '{oidc_alias}';
-- 预期: provider_type='oidc' 且 authorization_url 非空
```

---

## 场景 3：域名冲突时创建失败

### 初始状态
- 域名 `{corp_domain}` 已被租户 A 的连接器占用

### 目的
验证 `enterprise_sso_domains.domain` 全局唯一约束

### 测试操作流程
1. 在租户 B 再次创建连接器（可通过 `POST /demo/enterprise/connectors`），domains 包含 `{corp_domain}`
2. 观察接口响应

### 预期结果
- HTTP 状态码 `409`
- 错误提示为域名/连接器重复冲突

### 预期数据状态
```sql
SELECT tenant_id, connector_id, domain
FROM enterprise_sso_domains
WHERE domain = '{corp_domain}';
-- 预期: 仅保留原有 1 条绑定，不新增第二条
```

---

## 场景 4：更新连接器启用状态与域名成功

### 初始状态
- 已存在连接器 `{connector_id}`，enabled=true，domains 包含 `{old_domain}`

### 目的
验证连接器更新可同步到数据库并替换域名列表

### 测试操作流程
1. 调用更新接口（可通过 `PUT /demo/enterprise/connectors/{connector_id}`），将 `enabled=false`，domains 修改为 `[{new_domain}]`
2. 刷新列表页

### 预期结果
- 接口返回更新成功
- 页面状态开关显示为禁用
- 连接器域名从 `{old_domain}` 变更为 `{new_domain}`

### 预期数据状态
```sql
SELECT enabled FROM enterprise_sso_connectors WHERE id = '{connector_id}';
-- 预期: enabled = 0

SELECT domain FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 仅 1 行且 domain = '{new_domain}'
```

---

## 场景 5：测试连接与删除连接器

### 初始状态
- 已存在连接器 `{connector_id}`

### 目的
验证「Test」与「Delete」操作可正常执行并清理数据

### 测试操作流程
1. 在连接器卡片点击「Test」或调用 `POST /demo/enterprise/connectors/{connector_id}/test`
2. 记录返回消息（成功或失败原因）
3. 点击「Delete」或调用 `DELETE /demo/enterprise/connectors/{connector_id}?tenantId={tenant_id}` 删除同一连接器
4. 重新请求连接器列表

### 预期结果
- 「Test」返回结构化结果：`ok` + `message`
- 删除后列表中不再显示该连接器

### 预期数据状态
```sql
SELECT * FROM enterprise_sso_connectors WHERE id = '{connector_id}';
-- 预期: 0 行

SELECT * FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 0 行
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建 SAML 连接器成功 | ☐ | | | |
| 2 | 创建 OIDC 连接器成功 | ☐ | | | |
| 3 | 域名冲突时创建失败 | ☐ | | | |
| 4 | 更新连接器启用状态与域名成功 | ☐ | | | |
| 5 | 测试连接与删除连接器 | ☐ | | | |
