# 身份提供商 - LDAP/Active Directory 企业 SSO 连接器管理

**模块**: 身份提供商
**测试范围**: LDAP 连接器创建、配置校验、连接测试、删除级联、LDAP 用户搜索
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 新增 LDAP/Active Directory 作为第三种企业 SSO 连接器类型（与 SAML、OIDC 并列），复用统一管理 API。

端点：
- `POST /api/v1/tenants/{tenant_id}/sso/connectors` — 创建 LDAP 连接器（provider_type=ldap）
- `GET /api/v1/tenants/{tenant_id}/sso/connectors` — 列出所有连接器（含 LDAP）
- `PUT /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}` — 更新连接器
- `DELETE /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}` — 删除（级联清理 ldap_group_role_mappings）
- `POST /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/test` — 测试 LDAP 连接
- `POST /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/ldap-search-users` — 搜索 LDAP 用户

LDAP 必填配置字段（存于 config JSON）：`serverUrl`, `bindDn`, `bindPassword`, `baseDn`

## 数据库表结构参考

```sql
-- 使用现有统一表，provider_type='ldap'
-- enterprise_sso_connectors: id, tenant_id, alias, provider_type, config(JSON), enabled, ...
-- enterprise_sso_domains: id, connector_id, domain, ...
-- ldap_group_role_mappings: id, tenant_id, connector_id, ldap_group_dn, role_id, ...
```

---

## 步骤 0（Gate Check）

```bash
# 1. 获取管理员 Token
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 2. 获取租户 ID
TENANT_ID=$(curl -sf http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')
echo "Tenant: $TENANT_ID"
```

---

## 场景 1：创建 LDAP 连接器成功

### 初始状态
- 已存在租户 `{tenant_id}`
- 当前租户尚未使用 alias `ldap-corp`

### 目的
验证 LDAP 连接器创建、config 校验、域名绑定落库

### 测试操作流程

**方式一：Portal UI**
1. 进入「Tenants → {tenant} → Enterprise SSO」
2. 将「Provider Type」切换为 `LDAP / Active Directory`
3. 确认表单显示 LDAP 专属字段：Server URL、Bind DN、Bind Password、Base DN、属性映射等
4. 填写以下信息：
   - Alias: `ldap-corp`
   - Display Name: `Corporate LDAP`
   - Server URL: `ldaps://ldap.corp.example.com:636`
   - Bind DN: `cn=auth9,ou=services,dc=corp,dc=example,dc=com`
   - Bind Password: `test-bind-password` <!-- pragma: allowlist secret -->
   - Base DN: `ou=users,dc=corp,dc=example,dc=com`
   - Domains: `corp.example.com`
5. 点击「Create Connector」

**方式二：API**
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "ldap-corp",
    "display_name": "Corporate LDAP",
    "provider_type": "ldap",
    "enabled": true,
    "priority": 100,
    "domains": ["corp.example.com"],
    "config": {
      "serverUrl": "ldaps://ldap.corp.example.com:636",
      "bindDn": "cn=auth9,ou=services,dc=corp,dc=example,dc=com",
      "bindPassword": "test-bind-password", <!-- pragma: allowlist secret -->
      "baseDn": "ou=users,dc=corp,dc=example,dc=com",
      "userSearchFilter": "(uid={username})",
      "attrUsername": "uid",
      "attrEmail": "mail"
    }
  }'
```

### 预期结果
- HTTP 200，返回 `data.provider_type` 为 `ldap`
- Portal 连接器列表出现新记录，显示 `LDAP`
- 在列表中可见「Group Mappings」按钮（仅 LDAP 类型显示）

### 预期数据状态
```sql
SELECT id, alias, provider_type, enabled, JSON_EXTRACT(config, '$.serverUrl') AS server_url
FROM enterprise_sso_connectors
WHERE tenant_id = '{tenant_id}' AND alias = 'ldap-corp';
-- 预期: 1 行，provider_type='ldap', server_url='ldaps://ldap.corp.example.com:636'

SELECT domain FROM enterprise_sso_domains
WHERE connector_id = '{connector_id}';
-- 预期: 1 行，domain='corp.example.com'
```

---

## 场景 2：缺少必填 LDAP 配置字段时创建失败

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证 LDAP 连接器的 config 校验拒绝缺少必填字段的请求

### 测试操作流程
```bash
# 缺少 bindDn 和 baseDn
curl -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "ldap-incomplete",
    "provider_type": "ldap",
    "domains": ["incomplete.example.com"],
    "config": {
      "serverUrl": "ldaps://ldap.example.com:636"
    }
  }'
```

### 预期结果
- HTTP 422 或 400
- 错误消息包含 `bindDn`, `bindPassword`, `baseDn` 缺失提示

### 预期数据状态
```sql
SELECT * FROM enterprise_sso_connectors
WHERE tenant_id = '{tenant_id}' AND alias = 'ldap-incomplete';
-- 预期: 0 行（未创建）
```

---

## 场景 3：测试 LDAP 连接（Test Connection）

### 初始状态
- 已存在 LDAP 连接器 `{connector_id}`

### 目的
验证「Test」操作对 LDAP 连接器执行 bind + base DN 搜索测试

### 测试操作流程

**方式一：Portal UI**
1. 在连接器卡片点击「Test」按钮

**方式二：API**
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/test" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200
- 返回 `data.ok` 为 `true` 或 `false`
- `data.message` 包含具体的连接/bind 结果描述
- 若配置的 LDAP 服务器不可达，`ok=false` 且 `message` 包含 "Connection failed" 类似信息

---

## 场景 4：Active Directory 配置默认值

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证 `isActiveDirectory=true` 时，默认值自动切换为 AD 标准（sAMAccountName、memberOf）

### 测试操作流程
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "ad-corp",
    "display_name": "Corporate AD",
    "provider_type": "ldap",
    "enabled": true,
    "domains": ["acme.example.com"],
    "config": {
      "serverUrl": "ldaps://ad.acme.example.com:636",
      "bindDn": "cn=auth9,ou=services,dc=acme,dc=example,dc=com",
      "bindPassword": "test-ad-password", <!-- pragma: allowlist secret -->
      "baseDn": "ou=users,dc=acme,dc=example,dc=com",
      "isActiveDirectory": "true",
      "adDomain": "acme.example.com"
    }
  }'
```

### 预期结果
- HTTP 200，连接器创建成功
- 查询 config 确认 `isActiveDirectory` 已保存
- 内部默认使用 `sAMAccountName` 作为用户名属性（未显式指定 `attrUsername` 时）

### 预期数据状态
```sql
SELECT JSON_EXTRACT(config, '$.isActiveDirectory') AS is_ad,
       JSON_EXTRACT(config, '$.adDomain') AS ad_domain
FROM enterprise_sso_connectors
WHERE tenant_id = '{tenant_id}' AND alias = 'ad-corp';
-- 预期: is_ad='true', ad_domain='acme.example.com'
```

---

## 场景 5：删除 LDAP 连接器级联清理 group_role_mappings

### 初始状态
- 已存在 LDAP 连接器 `{connector_id}`
- 该连接器已配置至少一条 LDAP 组角色映射

### 目的
验证删除 LDAP 连接器时级联清理 `ldap_group_role_mappings`、`enterprise_sso_domains`

### 测试操作流程
1. 先创建一条组角色映射：
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/ldap-group-mappings" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "ldap_group_dn": "cn=admins,ou=groups,dc=corp,dc=example,dc=com",
    "ldap_group_display_name": "Administrators",
    "role_id": "{role_id}"
  }'
```
2. 确认映射已创建
3. 删除连接器：
```bash
curl -X DELETE "http://localhost:8080/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 连接器删除成功
- 连接器列表中不再显示该记录

### 预期数据状态
```sql
SELECT * FROM enterprise_sso_connectors WHERE id = '{connector_id}';
-- 预期: 0 行

SELECT * FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 0 行

SELECT * FROM ldap_group_role_mappings WHERE connector_id = '{connector_id}';
-- 预期: 0 行（级联删除）
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建 LDAP 连接器成功 | ☐ | | | |
| 2 | 缺少必填字段创建失败 | ☐ | | | |
| 3 | 测试 LDAP 连接 | ☐ | | | |
| 4 | Active Directory 配置默认值 | ☐ | | | |
| 5 | 删除连接器级联清理 | ☐ | | | |
