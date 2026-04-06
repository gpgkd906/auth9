# Portal SSO Connector Domain 编辑功能

**类型**: 前端功能增强
**严重程度**: Medium
**影响范围**: auth9-portal (Frontend)
**前置依赖**: 无（后端 API 已就绪）
**被依赖**: 无
**来源 Ticket**: `docs/ticket/identity-provider_03-sso-connectors_scenario4_260406_201500.md`

---

## 背景

Enterprise SSO connector 的后端更新接口 `PUT /api/v1/tenants/{tenant_id}/sso/connectors/{connector_id}` 已完整实现，支持更新 `enabled`、`display_name`、`priority`、`config`、`domains` 等字段。前端 API 客户端 `tenantSsoApi.update()` 也已封装该接口，且 `UpdateTenantSsoConnectorInput` 类型已定义 `domains?: string[]` 字段。

然而，Portal UI 中连接器卡片仅提供：
- **启用/禁用开关**（`intent=toggle`，仅发送 `{ enabled }`)
- **Test 按钮**
- **Delete 按钮**
- **SP Metadata 链接**（SAML）/ **LDAP Group Mappings 链接**（LDAP）

**域名显示为只读文本**（`connector.domains.join(", ")`），无任何编辑入口。用户无法通过 UI 修改已创建连接器的域名，必须删除并重新创建连接器。

QA 场景 4（`docs/qa/identity-provider/03-tenant-enterprise-sso-connectors.md`）要求域名可通过 UI 更新，但当前实现无法满足。

---

## 期望行为

### R1: 连接器卡片增加"编辑域名"交互

在每个连接器卡片中，域名文本区域应可点击进入编辑模式，或提供一个 Edit 按钮触发编辑。推荐方式：

- 域名文本旁增加编辑图标（pencil icon）
- 点击后域名区域变为可编辑的 `<Input>` 字段（逗号分隔格式，与创建表单一致）
- 提供 Save / Cancel 按钮

**涉及文件**:
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.sso.tsx` — 连接器卡片 UI 组件

### R2: 新增 `intent=update_domains` action 处理

在页面 `action` 函数中新增 intent 分支，接收 `connector_id` 和 `domains` 字段，调用 `tenantSsoApi.update()` 传入 `{ domains }`:

```typescript
if (intent === "update_domains") {
  const connectorId = String(formData.get("connector_id") || "");
  const domains = String(formData.get("domains") || "")
    .split(",")
    .map((v) => v.trim())
    .filter(Boolean);
  await tenantSsoApi.update(tenantId, connectorId, { domains }, accessToken || undefined);
  return { success: true, message: translate(locale, "tenants.sso.connectorUpdated") };
}
```

**涉及文件**:
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.sso.tsx` — action 函数

### R3: 域名输入校验

编辑域名时需前端校验：
- 至少填写 1 个域名（非空）
- 每个域名为合法域名格式（包含至少一个 `.`）
- 域名去重（自动去除重复项）

**涉及文件**:
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.sso.tsx` — 表单校验逻辑

### R4: i18n 支持

新增必要的 i18n key（如编辑按钮提示文本、保存/取消按钮等），覆盖 en-US、zh-CN、ja 三种语言。

**涉及文件**:
- `auth9-portal/app/i18n/locales/en-US.ts`
- `auth9-portal/app/i18n/locales/zh-CN.ts`
- `auth9-portal/app/i18n/locales/ja.ts`

### R5: 编辑操作后页面状态刷新

域名更新成功后，页面应自动刷新连接器列表（React Router 的 action 返回后 loader 自动 revalidate），确保更新后的域名立即显示。

**涉及文件**:
- `auth9-portal/app/routes/dashboard.tenants.$tenantId.sso.tsx` — 已有 revalidation 机制，无需额外处理

---

## 验证方法

### 代码验证

```bash
# 确认 action 中新增了 update_domains intent
grep -n "update_domains" auth9-portal/app/routes/dashboard.tenants.\$tenantId.sso.tsx

# 确认 i18n key 覆盖
grep -n "editDomains\|saveDomains" auth9-portal/app/i18n/locales/en-US.ts
```

### 手动验证（对应 QA 场景 4）

1. 导航到 Portal → Tenants → {Tenant} → Enterprise SSO
2. 找到已有的 SSO 连接器卡片
3. 点击域名旁的编辑图标
4. 将域名从 `corp.example.com` 修改为 `new-corp.example.com`
5. 点击 Save
6. 确认页面显示更新成功消息，域名已变更
7. 数据库验证：

```sql
SELECT domain FROM enterprise_sso_domains WHERE connector_id = '{connector_id}';
-- 预期: domain = 'new-corp.example.com'
```

### E2E 测试验证

```bash
cd auth9-portal && npm run test:e2e:full
```

在全栈 E2E 测试中验证域名编辑流程端到端正常。

---

## 参考

- 后端 API 实现: `auth9-core/src/domains/tenant_access/api/tenant_sso.rs`
- 前端 API 客户端: `auth9-portal/app/services/api/enterprise-sso.ts`（`tenantSsoApi.update`、`UpdateTenantSsoConnectorInput`）
- Portal SSO 页面: `auth9-portal/app/routes/dashboard.tenants.$tenantId.sso.tsx`
- QA 文档: `docs/qa/identity-provider/03-tenant-enterprise-sso-connectors.md`（场景 4）
- Ticket: `docs/ticket/identity-provider_03-sso-connectors_scenario4_260406_201500.md`
