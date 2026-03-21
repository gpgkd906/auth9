# 认证流程 - 企业 SAML Broker 原生登录测试

**模块**: 认证流程
**测试范围**: Auth9 原生企业 SAML 连接器 broker 流程（SP metadata 生成、AuthnRequest 发起、SAML Response 校验、connector CRUD）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 原生 broker 直接处理企业 SAML SP-initiated 登录流程：authorize → 构建 AuthnRequest → 重定向到 IdP → IdP POST SAMLResponse 到 ACS → 校验签名/issuer/audience/时间窗口 → 用户解析 → 登录完成。（注：Keycloak 已退役，所有企业 SAML broker 流程由 Auth9 内置引擎处理）

端点：
- `GET /api/v1/enterprise-sso/authorize/{alias}` — 发起企业 SSO 登录（SAML 分支构建 AuthnRequest）
- `POST /api/v1/enterprise-sso/saml/acs` — ACS 回调处理 SAML Response
- `GET /api/v1/enterprise-sso/saml/metadata/{alias}` — SP metadata XML 生成
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
| config | JSON | 协议配置（含 entityId, singleSignOnServiceUrl, signingCertificate, nameIDPolicyFormat） |

### `linked_identities` 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 主键 |
| user_id | CHAR(36) | Auth9 用户 ID |
| provider_type | VARCHAR(50) | `saml` |
| provider_alias | VARCHAR(100) | 连接器 alias |
| external_user_id | VARCHAR(255) | IdP NameID |
| external_email | VARCHAR(255) | IdP 邮箱属性 |

---

## 场景 1：创建 SAML 连接器仅保存到 Auth9 数据库

### 步骤 0（Gate Check）
- 已获取管理员 JWT token
- 已存在租户 `{tenant_id}`

### 目的
验证创建 SAML 类型连接器时仅保存到 Auth9 数据库

### 测试操作流程
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "corp-saml-test",
    "display_name": "Corp SAML Test",
    "provider_type": "saml",
    "enabled": true,
    "priority": 100,
    "domains": ["corp.example.com"],
    "config": {
      "entityId": "https://idp.example.com/saml/metadata",
      "singleSignOnServiceUrl": "https://idp.example.com/saml/sso",
      "signingCertificate": "MIIDpDCCAoygAwIBAgIGAXoEZ3WBMA0GCSqGSIb3DQEBCwUAMIGSMQswCQYDVQQG"
    }
  }'
```

### 预期结果
- HTTP 200，返回创建的连接器
- `provider_type` 为 `saml`

### 预期数据状态
```sql
SELECT id, alias, provider_type, enabled, provider_alias
FROM enterprise_sso_connectors
WHERE alias = 'corp-saml-test' AND tenant_id = '{tenant_id}';
-- 预期: 1 行，provider_type = 'saml', enabled = 1

SELECT domain FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: 1 行，domain = 'corp.example.com'
```

---

## 场景 2：SP Metadata 端点返回有效 XML

### 步骤 0（Gate Check）
- 场景 1 的 SAML 连接器已创建

### 目的
验证 SP metadata 端点返回符合 SAML 2.0 规范的 XML

### 测试操作流程
```bash
curl -s "http://localhost:8080/api/v1/enterprise-sso/saml/metadata/corp-saml-test"
```

### 预期结果
- HTTP 200
- Content-Type: `application/xml`
- 返回 XML 包含：
  - `<md:EntityDescriptor>` 根元素
  - `entityID` 属性（Auth9 实例 URL）
  - `<md:SPSSODescriptor>` 元素
  - `<md:AssertionConsumerService>` 元素，Location 包含 `/api/v1/enterprise-sso/saml/acs`，Binding 为 `urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST`
  - `<md:NameIDFormat>` 包含 `emailAddress`

---

## 场景 3：SAML 连接器 test 端点验证证书和 SSO URL

### 步骤 0（Gate Check）
- 场景 1 的连接器已创建

### 目的
验证 test 端点对 SAML 连接器执行原生证书格式校验和 SSO URL 可达性检查

### 测试操作流程
```bash
curl -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}/test" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- HTTP 200
- 因场景 1 使用的是截断证书，预期 `ok: false`，`message` 包含 `certificate` 或证书校验错误
- 如使用有效证书且 `https://idp.example.com` 不可达：`ok: false`，`message` 包含 `unreachable`

---

## 场景 4：Discovery 对 SAML 连接器返回 Auth9 broker URL

### 步骤 0（Gate Check）
- 已创建 SAML 连接器，绑定域名 `corp.example.com`

### 目的
验证 discovery 对 SAML 连接器返回 Auth9 原生 broker 地址

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
- `data.authorize_url` **不包含** 外部认证服务 URL

---

## 场景 5：删除 SAML 连接器

### 步骤 0（Gate Check）
- 场景 1 的连接器已创建

### 目的
验证删除 SAML 连接器时正确清理 Auth9 数据库

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

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | SAML 连接器创建仅保存到 Auth9 数据库 | ☐ | | | API |
| 2 | SP Metadata 端点返回有效 XML | ☐ | | | API |
| 3 | SAML 连接器 test 端点验证证书 | ☐ | | | API |
| 4 | Discovery 对 SAML 返回 Auth9 broker URL | ☐ | | | API |
| 5 | SAML 连接器删除 | ☐ | | | API |
