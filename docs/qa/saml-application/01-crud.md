# SAML Application（IdP 出站） - CRUD 管理测试

**模块**: SAML Application
**测试范围**: SAML Application 创建、列表、获取、更新、删除（API 层）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 作为 SAML Identity Provider，可向外部 Service Provider 签发 SAML Assertion。管理员通过 API 注册外部 SP 信息（Entity ID、ACS URL、属性映射等），Auth9 在内置 OIDC 引擎中创建对应 SAML Client，并提供 IdP Metadata XML 供 SP 配置。

## 入口可见性说明

本文件聚焦 API CRUD，不单独覆盖 Portal UI 入口可见性。Portal 侧入口可见性与导航进入路径统一在 [03-portal-ui.md](./03-portal-ui.md) 验证。

端点：
- `GET    /api/v1/tenants/{tenant_id}/saml-apps` — 列出所有 SAML Application
- `POST   /api/v1/tenants/{tenant_id}/saml-apps` — 创建 SAML Application
- `GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}` — 获取单个
- `PUT    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}` — 更新
- `DELETE /api/v1/tenants/{tenant_id}/saml-apps/{app_id}` — 删除
- `GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata` — 获取 IdP Metadata XML（公开）
- `GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate` — 下载 IdP 签名证书 PEM（公开）
- `GET    /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate-info` — 证书过期信息（受保护）

> **Phase 1（API）+ Phase 2（Portal UI）+ Phase 3（证书/加密/SLO）** 已完成。API 测试通过 curl 执行，Portal UI 测试参见 [03-portal-ui.md](./03-portal-ui.md)，证书与加密测试参见 [04-certificate-encryption.md](./04-certificate-encryption.md)。

---

## 数据库表结构参考

```sql
-- saml_applications
CREATE TABLE saml_applications (
    id CHAR(36) NOT NULL PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    entity_id VARCHAR(512) NOT NULL,
    acs_url VARCHAR(1024) NOT NULL,
    slo_url VARCHAR(1024) NULL,
    name_id_format VARCHAR(128) NOT NULL DEFAULT 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
    sign_assertions BOOLEAN NOT NULL DEFAULT TRUE,
    sign_responses BOOLEAN NOT NULL DEFAULT TRUE,
    encrypt_assertions BOOLEAN NOT NULL DEFAULT FALSE,
    sp_certificate TEXT NULL,
    attribute_mappings JSON NOT NULL DEFAULT '[]',
    keycloak_client_id VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_saml_app_tenant_entity (tenant_id, entity_id),
    INDEX idx_saml_app_tenant (tenant_id),
    UNIQUE INDEX idx_saml_app_kc_client (keycloak_client_id)
);
```

---

## 场景 1：创建 SAML Application 成功

### 初始状态
- 已存在租户 `{tenant_id}`
- 该租户下尚无 entity_id 为 `https://sp.example.com` 的 SAML Application
- 持有有效的 Tenant Access Token

#### 步骤 0: 验证 Token 类型
```bash
echo $TOKEN | cut -d. -f2 | base64 -d 2>/dev/null | jq '{token_type, tenant_id}'
# 预期: token_type = "access", tenant_id 非空
# 如果 token_type 不是 "access"，需先执行 Token Exchange 获取 Tenant Access Token
```

### 目的
验证 SAML Application 创建成功，数据正确写入 DB，Auth9 内置 OIDC 引擎中同步创建 SAML Client

### 测试操作流程

**API 操作**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test SP Application",
    "entity_id": "https://sp.example.com",
    "acs_url": "https://sp.example.com/saml/acs",
    "slo_url": "https://sp.example.com/saml/slo",
    "name_id_format": "email",
    "sign_assertions": true,
    "sign_responses": true,
    "encrypt_assertions": false,
    "attribute_mappings": [
      {
        "source": "email",
        "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
        "friendly_name": "email"
      },
      {
        "source": "display_name",
        "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
        "friendly_name": "displayName"
      }
    ]
  }' | jq .
```

### 预期结果
- HTTP 200，返回 `data` 包含：
  - `id`：UUID 格式
  - `name`：`"Test SP Application"`
  - `entity_id`：`"https://sp.example.com"`
  - `acs_url`：`"https://sp.example.com/saml/acs"`
  - `slo_url`：`"https://sp.example.com/saml/slo"`
  - `name_id_format`：包含 `emailAddress`
  - `sign_assertions`：`true`
  - `enabled`：`true`
  - `attribute_mappings`：2 条映射
  - `sso_url`：Auth9 SAML SSO 端点 URL

### 预期数据状态
```sql
SELECT id, tenant_id, name, entity_id, acs_url, enabled, keycloak_client_id
FROM saml_applications
WHERE tenant_id = '{tenant_id}' AND entity_id = 'https://sp.example.com';
-- 预期: 返回 1 行，name='Test SP Application', enabled=1, keycloak_client_id 非空
```

---

## 场景 2：创建重复 Entity ID 被拒绝

### 初始状态
- 场景 1 的 SAML Application 已存在（entity_id = `https://sp.example.com`）

### 目的
验证同一租户下 entity_id 唯一性约束

### 测试操作流程

**API 操作**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Duplicate SP",
    "entity_id": "https://sp.example.com",
    "acs_url": "https://sp.example.com/saml/acs2"
  }' | jq .
```

### 预期结果
- HTTP 409 Conflict
- 错误信息包含 `"already exists"`

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt
FROM saml_applications
WHERE tenant_id = '{tenant_id}' AND entity_id = 'https://sp.example.com';
-- 预期: cnt = 1（仍为 1，未新增）
```

---

## 场景 3：列表与获取单个 SAML Application

### 初始状态
- 已创建至少 1 个 SAML Application

### 目的
验证列表和单条获取接口返回正确数据及 `sso_url` 字段

### 测试操作流程

**API 操作 — 列表**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**API 操作 — 获取单个**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### 预期结果
- **列表**：HTTP 200，返回数组，包含已创建的 SAML Application
- **单个**：HTTP 200，返回完整数据（含 `sso_url`、`attribute_mappings`）
- 每条记录包含 `sso_url` 字段，指向 Auth9 SAML SSO 端点

> **注意**: `sso_url` 的值依赖 `AUTH9_CORE_PUBLIC_URL` 环境变量。在默认 Docker 开发环境中，该变量可能未设置，导致 `sso_url` 为空字符串或使用 localhost 地址。这是预期行为，不是 bug。生产环境中应配置 `AUTH9_CORE_PUBLIC_URL` 为实际的公开访问 URL。

---

## 场景 4：更新 SAML Application

### 初始状态
- 已创建 SAML Application `{app_id}`

### 目的
验证部分字段更新成功（name、acs_url、enabled），SAML Client 同步更新

### 测试操作流程

**API 操作**:
```bash
curl -s -X PUT "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated SP Application",
    "acs_url": "https://sp.example.com/saml/acs-v2",
    "enabled": false
  }' | jq .
```

### 预期结果
- HTTP 200
- 返回数据中 `name` = `"Updated SP Application"`
- `acs_url` = `"https://sp.example.com/saml/acs-v2"`
- `enabled` = `false`
- 其他未更新字段保持不变（如 `entity_id`、`sign_assertions`）

### 预期数据状态
```sql
SELECT name, acs_url, enabled, updated_at
FROM saml_applications WHERE id = '{app_id}';
-- 预期: name='Updated SP Application', acs_url='https://sp.example.com/saml/acs-v2', enabled=0
-- updated_at 应大于 created_at
```

---

## 场景 5：删除 SAML Application

### 初始状态
- 已创建 SAML Application `{app_id}`，关联 Keycloak Client `{keycloak_client_id}`

### 目的
验证删除操作同时清理 DB 记录和 SAML Client

### 测试操作流程

**API 操作**:
```bash
curl -s -X DELETE "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### 预期结果
- HTTP 200，消息包含 `"deleted successfully"`

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM saml_applications WHERE id = '{app_id}';
-- 预期: cnt = 0

-- 验证 SAML Client 也已删除（通过数据库验证）:
SELECT COUNT(*) FROM saml_applications WHERE keycloak_client_id = '{keycloak_client_id}';
-- 预期: 0
```

---

## 通用场景：认证状态检查

### 目的
验证未认证或 Token 无效时接口返回 401

### 测试操作流程
```bash
# 无 Token
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" | jq .
# 预期: 401 Unauthorized

# 无效 Token
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer invalid_token" | jq .
# 预期: 401 Unauthorized
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人 | 备注 |
|---|------|------|----------|--------|------|
| 1 | 创建 SAML Application 成功 | ☐ | | | |
| 2 | 创建重复 Entity ID 被拒绝 | ☐ | | | |
| 3 | 列表与获取单个 SAML Application | ☐ | | | |
| 4 | 更新 SAML Application | ☐ | | | |
| 5 | 删除 SAML Application | ☐ | | | |
| G | 认证状态检查 | ☐ | | | |
