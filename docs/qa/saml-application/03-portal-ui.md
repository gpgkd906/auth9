# SAML Application（IdP 出站） - Portal 管理 UI 测试

**模块**: SAML Application
**测试范围**: Portal UI 入口可见性、创建表单、列表展示、启停切换、删除、Metadata URL 复制
**场景数**: 5
**优先级**: 高

---

## 背景说明

Phase 2 为 SAML Application 功能新增 Portal 管理 UI。管理员可通过 Tenant 详情页的 Quick Links 导航到 SAML Applications 页面，进行创建、查看、启停和删除操作。页面遵循与 Enterprise SSO Connectors 页面一致的单页模式（创建表单卡片 + 已注册列表卡片）。

Portal 路由：`/dashboard/tenants/:tenantId/saml-apps`

---

## 场景 1：Tenant 详情页入口可见性与导航

### 初始状态
- 已登录 Auth9 Portal
- 已存在至少一个租户

### 目的
验证 SAML Applications 入口在 Tenant 详情页 Quick Links 中可见，点击后正确导航到 SAML Applications 页面

### 测试操作流程

**Portal UI 操作**:
1. 导航至 `/dashboard/tenants`，点击任意租户进入详情页
2. 在右侧「Quick Links」卡片中查找「SAML Applications」按钮
3. 点击该按钮

### 预期结果
- Quick Links 卡片中显示「SAML Applications」按钮，带有 Link2Icon 图标
- 按钮位于「Enterprise SSO」按钮下方
- 点击后导航至 `/dashboard/tenants/{tenantId}/saml-apps`
- 页面标题显示 "{tenantName} 的 SAML 应用"（中文）或 "SAML Applications for {tenantName}"（英文）
- 页面顶部有返回按钮（ArrowLeftIcon），点击返回 Tenant 详情页

---

## 场景 2：创建 SAML Application — 完整表单提交

### 初始状态
- 已通过场景 1 导航至 SAML Applications 页面
- 该租户下尚无 entity_id 为 `https://sp.example.com` 的 SAML Application

#### 步骤 0: 验证 Token 类型
```bash
echo $TOKEN | cut -d. -f2 | base64 -d 2>/dev/null | jq '{token_type, tenant_id}'
# 预期: token_type = "access", tenant_id 非空
# 如果 token_type 不是 "access"，需先执行 Token Exchange 获取 Tenant Access Token
```

### 目的
验证通过 Portal 表单创建 SAML Application 成功，所有字段（含属性映射）正确提交

### 测试操作流程

**Portal UI 操作**:
1. 在「Register SAML Application」卡片中填写：
   - Application Name: `Test SP Application`
   - Entity ID (SP): `https://sp.example.com`
   - ACS URL: `https://sp.example.com/saml/acs`
   - SLO URL: `https://sp.example.com/saml/slo`
   - NameID Format: 选择 `Email`
   - Sign Assertions: 保持开启（默认）
   - Sign Responses: 保持开启（默认）
   - Encrypt Assertions: 保持关闭（默认）
2. 点击「Add Mapping」添加属性映射：
   - 第 1 行: Source = `email`, SAML Attribute = `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress`, Friendly Name = `email`
   - 第 2 行: Source = `display_name`, SAML Attribute = `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name`, Friendly Name = `displayName`
3. 再次点击「Add Mapping」，在新行中选择 Source = `tenant_roles`
   - 确认下拉框下方出现黄色提示文字（高级源提示）
   - 选择 Source = `tenant_permissions`，同样确认提示出现
   - 删除该行（点击 ✕ 按钮）
4. 点击「Register Application」按钮

**API 等效验证**:
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
      {"source": "email", "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress", "friendly_name": "email"},
      {"source": "display_name", "saml_attribute": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name", "friendly_name": "displayName"}
    ]
  }' | jq .
```

### 预期结果
- 页面显示绿色成功消息 "SAML 应用已注册" / "SAML application registered"
- 在「Registered SAML Applications」列表中出现新条目：
  - 显示名称 `Test SP Application`
  - 显示 Entity ID `https://sp.example.com`
  - 启用开关为 ON
  - 显示 IdP Metadata URL（可复制）
  - 显示 SSO URL（可复制，Auth9 SAML SSO 端点）
  - 显示配置摘要：NameID / Assertions 签名状态 / Mappings 数量

### 预期数据状态
```sql
SELECT id, name, entity_id, acs_url, enabled, sign_assertions, sign_responses,
       JSON_LENGTH(attribute_mappings) AS mapping_count
FROM saml_applications
WHERE tenant_id = '{tenant_id}' AND entity_id = 'https://sp.example.com';
-- 预期: 1 行，name='Test SP Application', enabled=1, sign_assertions=1,
-- sign_responses=1, mapping_count=2
```

---

## 场景 3：列表展示与 Metadata URL 复制

### 初始状态
- 场景 2 已成功创建至少 1 个 SAML Application

### 目的
验证列表中的 SAML Application 信息完整，Metadata URL 和 SSO URL 可一键复制

### 测试操作流程

**Portal UI 操作**:
1. 导航至 SAML Applications 页面（从 Tenant 详情页 Quick Links 进入）
2. 在列表中找到已创建的 SAML Application
3. 检查以下信息展示：
   - 应用名称和 Entity ID
   - 启用状态开关
   - IdP Metadata URL 行（带复制按钮）
   - SSO URL 行（带复制按钮）
   - 「Download IdP Certificate」下载链接（带下载图标）
   - 证书状态 badge（绿色/黄色/红色，显示剩余天数）
   - 配置摘要行（NameID 格式、签名状态、映射数量）
   - 「Setup Instructions」可折叠链接（带 chevron 图标）
4. 点击「Setup Instructions」展开
   - 确认显示 4 个配置指南区块：Generic SP Configuration、Salesforce、AWS IAM Identity Center、Google Workspace
   - 每个区块包含编号步骤列表
   - 再次点击折叠，内容隐藏
5. 点击 IdP Metadata URL 旁的复制按钮
6. 在浏览器中打开复制的 URL

### 预期结果
- 列表项显示所有信息字段（含证书下载链接和过期 badge）
- 复制按钮点击后显示 ✓ 反馈（约 2 秒后恢复）
- IdP Metadata URL 格式：`http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata`
- 浏览器中直接访问 Metadata URL 返回有效的 SAML IdP Metadata XML（无需登录，公开端点）
- XML 中 `<SingleSignOnService>` 的 `Location` 指向 Auth9 SAML SSO 端点

---

## 场景 4：启停切换 SAML Application

### 初始状态
- 已创建且处于 enabled 状态的 SAML Application

### 目的
验证通过 Portal Switch 切换 SAML Application 的启用/禁用状态

### 测试操作流程

**Portal UI 操作**:
1. 在列表中找到目标 SAML Application，确认 Switch 为开启状态
2. 点击 Switch 关闭该应用
3. 页面刷新或等待提交完成
4. 确认 Switch 状态已变为关闭
5. 再次点击 Switch 重新开启

**API 等效验证**:
```bash
# 禁用
curl -s -X PUT "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}' | jq '.data.enabled'
# 预期: false

# 重新启用
curl -s -X PUT "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}' | jq '.data.enabled'
# 预期: true
```

### 预期结果
- Switch 切换后页面显示成功消息
- Switch 视觉状态与实际数据一致
- 禁用的应用仍显示在列表中，但 Switch 为关闭状态

### 预期数据状态
```sql
SELECT id, name, enabled FROM saml_applications WHERE id = '{app_id}';
-- 禁用后: enabled=0
-- 重新启用后: enabled=1
```

---

## 场景 5：删除 SAML Application

### 初始状态
- 已创建至少 1 个 SAML Application
- 记录删除前的 `keycloak_client_id`

### 目的
验证通过 Portal 删除 SAML Application，同时清理 DB 和 SAML Client

### 测试操作流程

**Portal UI 操作**:
1. 在列表中找到要删除的 SAML Application
2. 点击红色「Delete」按钮
3. 等待操作完成

**API 等效验证**:
```bash
curl -s -X DELETE "http://localhost:8080/api/v1/tenants/{tenant_id}/saml-apps/{app_id}" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### 预期结果
- 页面显示成功消息 "SAML 应用已删除" / "SAML application deleted"
- 该应用从列表中消失
- 如果删除后无剩余应用，列表显示空状态文字

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM saml_applications WHERE id = '{app_id}';
-- 预期: cnt = 0

-- 验证 SAML Client 也已删除（通过数据库验证）:
-- SELECT COUNT(*) FROM saml_applications WHERE keycloak_client_id = '{keycloak_client_id}';
-- 预期: 404 Not Found
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人 | 备注 |
|---|------|------|----------|--------|------|
| 1 | Tenant 详情页入口可见性与导航 | ☐ | | | |
| 2 | 创建 SAML Application — 完整表单提交 | ☐ | | | |
| 3 | 列表展示与 Metadata URL 复制 | ☐ | | | |
| 4 | 启停切换 SAML Application | ☐ | | | |
| 5 | 删除 SAML Application | ☐ | | | |
