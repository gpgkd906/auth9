# SAML Application（IdP 出站） - Metadata 与输入校验测试

**模块**: SAML Application
**测试范围**: IdP Metadata XML 获取、属性映射校验、输入验证与跨租户隔离
**场景数**: 5
**优先级**: 高

---

## 背景说明

SAML Application 提供 IdP Metadata XML 端点（公开，无需认证），外部 SP 可通过此端点获取 Auth9 的 SAML IdP 配置信息。此外需验证属性映射、输入校验和租户隔离的正确性。

公开端点：
- `GET /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata` — IdP Metadata XML（无需 Authorization）

---

## 场景 1：获取 IdP Metadata XML（公开端点）

### 初始状态
- 已创建 SAML Application `{app_id}`（enabled）
- Keycloak 正常运行

### 目的
验证 Metadata 端点无需认证即可访问，返回有效 XML，URL 使用公开域名（`KC_HOSTNAME`）

### 测试操作流程

**API 操作**（注意：无 Authorization header）:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata"
```

### 预期结果
- HTTP 200，Content-Type 为 `application/xml`
- 返回有效的 SAML IdP Metadata XML
- XML 中包含：
  - `<EntityDescriptor>` 根元素
  - `<IDPSSODescriptor>` 元素
  - `<SingleSignOnService>` 的 `Location` 属性指向 `http://localhost:8081/realms/auth9/protocol/saml`（Docker 环境）
  - `<KeyDescriptor use="signing">` 包含 X509 签名证书
- **关键校验**：URL 中不包含 Keycloak 内部地址（如 `http://keycloak:8080`）

---

## 场景 2：创建时输入校验 — 必填字段与 URL 格式

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证创建 SAML Application 时的输入校验规则

### 测试操作流程

**2a. 缺少必填字段 name**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "entity_id": "https://sp.example.com",
    "acs_url": "https://sp.example.com/acs"
  }' | jq .
# 预期: 422 Validation Error
```

**2b. 无效的 ACS URL**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Bad URL SP",
    "entity_id": "https://sp.example.com/bad",
    "acs_url": "not-a-valid-url"
  }' | jq .
# 预期: 422 Validation Error，提示 URL 格式无效
```

**2c. 空 entity_id**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Empty Entity",
    "entity_id": "",
    "acs_url": "https://sp.example.com/acs"
  }' | jq .
# 预期: 422 Validation Error
```

### 预期结果
- 所有请求返回 HTTP 422
- 无数据写入 DB

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM saml_applications
WHERE tenant_id = '{tenant_id}' AND name IN ('Bad URL SP', 'Empty Entity');
-- 预期: cnt = 0
```

---

## 场景 3：属性映射 — 无效 source 被拒绝

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证 attribute_mappings 中使用不支持的 source 字段时被拒绝

### 测试操作流程

**API 操作**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Bad Mapping SP",
    "entity_id": "https://bad-mapping.example.com",
    "acs_url": "https://bad-mapping.example.com/acs",
    "attribute_mappings": [
      {
        "source": "invalid_field_name",
        "saml_attribute": "urn:oid:something",
        "friendly_name": "bad"
      }
    ]
  }' | jq .
```

### 预期结果
- HTTP 422 Validation Error
- 错误信息包含 `"invalid_field_name"` 或 `"Invalid attribute mapping source"`
- 合法的 source 值为：`email`, `display_name`, `first_name`, `last_name`, `user_id`, `tenant_roles`, `tenant_permissions`

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM saml_applications
WHERE entity_id = 'https://bad-mapping.example.com';
-- 预期: cnt = 0
```

---

## 场景 4：跨租户隔离 — 无法访问其他租户的 SAML Application

### 初始状态
- 租户 A `{tenant_a_id}` 下已创建 SAML Application `{app_id}`
- 持有租户 B `{tenant_b_id}` 的 Tenant Access Token（或直接用租户 B 的 ID 访问）

### 目的
验证租户间数据隔离，租户 B 无法获取/更新/删除租户 A 的 SAML Application

### 测试操作流程

**API 操作 — 用租户 B 的路径获取租户 A 的 app**:
```bash
# 使用租户 B 的 tenant_id 尝试获取属于租户 A 的 app_id
curl -s "http://localhost:8080/api/v1/tenants/{tenant_b_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN_B" | jq .
# 预期: 404 Not Found
```

**API 操作 — 用租户 A 的路径但租户 B 的 Token**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_a_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN_B" | jq .
# 预期: 403 Forbidden（policy enforcement）
```

### 预期结果
- 跨租户路径返回 404（数据不可见）
- 越权 Token 返回 403（policy 拒绝）

---

## 场景 5：默认值验证 — 最小化创建

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证仅提供必填字段（name、entity_id、acs_url）时，默认值被正确应用

### 测试操作流程

**API 操作**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Minimal SP",
    "entity_id": "https://minimal.example.com",
    "acs_url": "https://minimal.example.com/acs"
  }' | jq .
```

### 预期结果
- HTTP 200，创建成功
- 返回数据中默认值：
  - `sign_assertions`: `true`
  - `sign_responses`: `true`
  - `encrypt_assertions`: `false`
  - `name_id_format`: 包含 `emailAddress`
  - `enabled`: `true`
  - `attribute_mappings`: `[]`（空数组）

### 预期数据状态
```sql
SELECT name, sign_assertions, sign_responses, encrypt_assertions, enabled,
       name_id_format, attribute_mappings
FROM saml_applications WHERE entity_id = 'https://minimal.example.com';
-- 预期: sign_assertions=1, sign_responses=1, encrypt_assertions=0, enabled=1
-- name_id_format='urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress'
-- attribute_mappings='[]'
```

---

## 通用场景：认证状态检查

### 目的
验证 CRUD 端点（非 metadata）要求认证，metadata 端点不要求

### 测试操作流程
```bash
# CRUD 端点无 Token → 401
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" | jq .status
# 预期: 401

# Metadata 公开端点无 Token → 200（或 404 如果 app_id 无效）
curl -s -o /dev/null -w "%{http_code}" \
  "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata"
# 预期: 200（如果 app 存在且 Keycloak 可达）
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人 | 备注 |
|---|------|------|----------|--------|------|
| 1 | 获取 IdP Metadata XML（公开端点） | ☐ | | | |
| 2 | 创建时输入校验 — 必填字段与 URL 格式 | ☐ | | | |
| 3 | 属性映射 — 无效 source 被拒绝 | ☐ | | | |
| 4 | 跨租户隔离 | ☐ | | | |
| 5 | 默认值验证 — 最小化创建 | ☐ | | | |
| G | 认证状态检查 | ☐ | | | |
