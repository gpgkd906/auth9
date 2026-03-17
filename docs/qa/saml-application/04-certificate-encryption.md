# SAML Application（IdP 出站） - 证书端点、加密校验与 SLO 测试

**模块**: SAML Application
**测试范围**: IdP 签名证书下载、证书过期信息、Assertion 加密校验、SLO POST Binding、Portal 证书 UI
**场景数**: 5
**优先级**: 高

---

## 背景说明

Phase 3 新增以下能力：
- **证书下载端点**（公开）：`GET /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate` — 返回 IdP 签名证书 PEM
- **证书信息端点**（受保护）：`GET /api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate-info` — 返回证书过期时间与告警
- **Assertion 加密校验**：`encrypt_assertions=true` 时必须提供 `sp_certificate`
- **SLO POST Binding**：`slo_url` 同时注册 Redirect 和 POST 两种绑定

## 入口可见性说明

本文件仅补充证书与加密能力回归；Portal UI 入口可见性仍以 [03-portal-ui.md](./03-portal-ui.md) 为准，QA 应从已验证入口导航进入对应详情页后再执行证书相关检查。

---

## 场景 1：下载 IdP 签名证书（公开端点）

### 初始状态
- 已创建 SAML Application `{app_id}`
- Keycloak 正常运行

### 目的
验证证书端点无需认证即可访问，返回有效 PEM 格式证书

### 测试操作流程

**API 操作**（无 Authorization header）:
```bash
curl -s -D - "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate"
```

### 预期结果
- HTTP 200
- `Content-Type` 包含 `application/x-pem-file`
- `Content-Disposition` 包含 `attachment; filename="idp-signing.crt"`
- 响应体以 `-----BEGIN CERTIFICATE-----` 开头，以 `-----END CERTIFICATE-----` 结尾
- 证书内容与 Metadata XML 中 `<ds:X509Certificate>` 一致（可通过对比 base64 内容验证）

**验证证书格式有效**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate" \
  | openssl x509 -noout -text 2>&1 | head -5
# 预期: 显示 Certificate: Data: Version: 等信息，无错误
```

---

## 场景 2：获取证书过期信息（受保护端点）

### 初始状态
- 已创建 SAML Application `{app_id}`
- 持有有效的 Tenant Access Token

#### 步骤 0: 验证 Token 类型
```bash
echo $TOKEN | cut -d. -f2 | base64 -d 2>/dev/null | jq '{token_type, tenant_id}'
# 预期: token_type = "access", tenant_id 非空
# 如果 token_type 不是 "access"，需先执行 Token Exchange 获取 Tenant Access Token
```

### 目的
验证证书信息端点返回过期时间、剩余天数和告警标志

### 测试操作流程

**API 操作**:
```bash
curl -s "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate-info" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**无 Token 访问**:
```bash
curl -s -o /dev/null -w "%{http_code}" \
  "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate-info"
# 预期: 401（受保护端点）
```

### 预期结果
- HTTP 200，返回 `data` 包含：
  - `certificate_pem`：有效 PEM 格式证书
  - `expires_at`：ISO 8601 日期时间（如 `"2027-01-01T00:00:00Z"`）
  - `days_until_expiry`：整数，> 0（除非证书已过期）
  - `expires_soon`：布尔值（剩余天数 < 30 时为 `true`）
- 无 Token 时返回 401

---

## 场景 3：Assertion 加密 — 缺少 SP 证书被拒绝

### 初始状态
- 已存在租户 `{tenant_id}`

### 目的
验证 `encrypt_assertions=true` 时必须提供 `sp_certificate`，否则返回校验错误

### 测试操作流程

**3a. 创建时缺少 SP 证书**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Encrypt No Cert SP",
    "entity_id": "https://encrypt-nocert.example.com",
    "acs_url": "https://encrypt-nocert.example.com/acs",
    "encrypt_assertions": true
  }' | jq .
# 预期: 422 Validation Error，消息包含 "sp_certificate"
```

**3b. 更新时开启加密但无 SP 证书**:
```bash
# 先创建一个不加密的应用
APP_ID=$(curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Plain SP",
    "entity_id": "https://plain-sp.example.com",
    "acs_url": "https://plain-sp.example.com/acs"
  }' | jq -r '.data.id')

# 尝试开启加密但不提供证书
curl -s -X PUT "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/$APP_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"encrypt_assertions": true}' | jq .
# 预期: 422 Validation Error，消息包含 "sp_certificate"
```

**3c. 创建时加密且提供 SP 证书（成功）**:
```bash
curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Encrypted SP",
    "entity_id": "https://encrypted.example.com",
    "acs_url": "https://encrypted.example.com/acs",
    "encrypt_assertions": true,
    "sp_certificate": "MIICpDCCAYwCCQDexample..."
  }' | jq .
# 预期: 200，encrypt_assertions=true
```

### 预期结果
- 3a: HTTP 422，错误信息包含 `"sp_certificate"`
- 3b: HTTP 422，错误信息包含 `"sp_certificate"`
- 3c: HTTP 200，创建成功，`encrypt_assertions` = `true`

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM saml_applications
WHERE entity_id = 'https://encrypt-nocert.example.com';
-- 预期: cnt = 0（创建被拒绝）

SELECT encrypt_assertions, sp_certificate IS NOT NULL AS has_cert
FROM saml_applications WHERE entity_id = 'https://encrypted.example.com';
-- 预期: encrypt_assertions=1, has_cert=1
```

---

## 场景 4：SLO POST Binding 验证

### 初始状态
- Keycloak 正常运行
- 持有管理员级别的 Keycloak Admin Token（用于验证 Keycloak Client 属性）

### 目的
验证配置 `slo_url` 时，Keycloak SAML Client 同时注册 Redirect 和 POST 两种 SLO 绑定

### 测试操作流程

**创建带 SLO URL 的 SAML Application**:
```bash
APP_RESULT=$(curl -s -X POST "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "SLO Test SP",
    "entity_id": "https://slo-test.example.com",
    "acs_url": "https://slo-test.example.com/acs",
    "slo_url": "https://slo-test.example.com/slo"
  }')
KC_CLIENT_ID=$(echo $APP_RESULT | jq -r '.data.keycloak_client_id')
echo "Keycloak Client UUID: $KC_CLIENT_ID"
```

**验证 Keycloak Client 属性**:
```bash
# 获取 Keycloak Admin Token
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -d "grant_type=client_credentials&client_id=admin-cli&client_secret={admin_secret}" \
  | jq -r '.access_token')

# 查看 SAML Client 属性
curl -s "http://localhost:8081/admin/realms/auth9/clients/$KC_CLIENT_ID" \
  -H "Authorization: Bearer $KC_TOKEN" \
  | jq '.attributes | {
    saml_single_logout_service_url_redirect,
    saml_single_logout_service_url_post
  }'
```

### 预期结果
- SAML Application 创建成功，`slo_url` = `"https://slo-test.example.com/slo"`
- Keycloak Client 属性中同时包含：
  - `saml_single_logout_service_url_redirect` = `"https://slo-test.example.com/slo"`
  - `saml_single_logout_service_url_post` = `"https://slo-test.example.com/slo"`

---

## 场景 5：Portal UI — 证书下载与过期告警展示

### 初始状态
- 已登录 Auth9 Portal
- 已创建至少一个 SAML Application

### 目的
验证 Portal 列表中显示证书下载链接和过期状态 badge

### 测试操作流程

**Portal UI 操作**:
1. 从 Tenant 详情页 Quick Links 导航至 SAML Applications 页面
2. 在已注册列表中找到 SAML Application 条目
3. 检查条目中是否显示：
   - 「Download IdP Certificate」/「下载 IdP 证书」下载链接（带下载图标）
   - 证书状态 badge（绿色/黄色/红色）
4. 点击证书下载链接
5. 检查下载的文件

**验证加密表单提示**:
1. 在创建表单中开启「Encrypt Assertions」开关
2. 检查 SP Certificate 字段

### 预期结果
- 列表中每个 SAML Application 显示：
  - 「Download IdP Certificate」链接，点击后下载 `idp-signing.crt` 文件
  - 证书状态 badge：
    - 绿色 "Valid (N days)" — 剩余 > 30 天
    - 黄色 "Expires in N days" — 剩余 < 30 天
    - 红色 "Certificate expired" — 已过期
- 下载的证书文件为有效 PEM 格式
- 开启加密后：
  - SP Certificate 字段标记为必填（红色 `*`）
  - 字段边框变为黄色高亮
  - 显示提示文字 "SP certificate is required when encryption is enabled."

---

## 通用场景：证书端点公开/受保护权限验证

### 目的
验证 `/certificate` 为公开端点，`/certificate-info` 为受保护端点

### 测试操作流程
```bash
# /certificate 无 Token → 200
curl -s -o /dev/null -w "%{http_code}" \
  "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate"
# 预期: 200

# /certificate-info 无 Token → 401
curl -s -o /dev/null -w "%{http_code}" \
  "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/certificate-info"
# 预期: 401
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人 | 备注 |
|---|------|------|----------|--------|------|
| 1 | 下载 IdP 签名证书（公开端点） | ☐ | | | |
| 2 | 获取证书过期信息（受保护端点） | ☐ | | | |
| 3 | Assertion 加密 — 缺少 SP 证书被拒绝 | ☐ | | | |
| 4 | SLO POST Binding 验证 | ☐ | | | |
| 5 | Portal UI — 证书下载与过期告警展示 | ☐ | | | |
| G | 证书端点公开/受保护权限验证 | ☐ | | | |
