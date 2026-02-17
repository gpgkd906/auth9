# 租户管理 - B2B 组织自助创建

**模块**: 租户管理
**测试范围**: 已认证用户自助创建组织（`POST /api/v1/organizations`）、域名验证、Pending 状态、`/users/me/tenants` 接口
**场景数**: 5
**优先级**: 高

---

## 背景说明

B2B 场景下，新用户首次 OAuth 登录后不再自动加入 demo 租户，而是通过自助流程创建组织。

### 端点

**创建组织**：`POST /api/v1/organizations`（需 Identity Token）

请求体：
```json
{
  "name": "Acme Corp",
  "slug": "acme-corp",
  "domain": "acme.com",
  "logo_url": "https://example.com/logo.png"
}
```

响应（成功，域名匹配）：
```json
{
  "id": "{tenant_id}",
  "name": "Acme Corp",
  "slug": "acme-corp",
  "domain": "acme.com",
  "status": "active"
}
```

响应（成功，域名不匹配）：
```json
{
  "id": "{tenant_id}",
  "name": "Acme Corp",
  "slug": "acme-corp",
  "domain": "acme.com",
  "status": "pending"
}
```

**获取我的租户**：`GET /api/v1/users/me/tenants`（需 Identity Token）

响应：
```json
[
  {
    "id": "{tenant_user_id}",
    "tenant_id": "{tenant_id}",
    "user_id": "{user_id}",
    "role_in_tenant": "owner",
    "joined_at": "2026-02-18T...",
    "tenant": { "id": "...", "name": "...", "slug": "...", "status": "active" }
  }
]
```

---

## 数据库表结构参考

### tenants 表（新增字段）
| 字段 | 类型 | 说明 |
|------|------|------|
| domain | VARCHAR(255) | 组织邮箱域名（新增） |
| status | VARCHAR(20) | 新增 `pending` 状态 |

---

## 场景 1：域名匹配 — 创建组织自动 Active

### 初始状态
- 用户已通过 OAuth 登录，邮箱为 `user@acme.com`
- 持有有效的 Identity Token
- 不存在 slug 为 `acme-corp` 的租户

### 目的
验证创建者邮箱域名与组织域名匹配时，租户状态为 `active`，创建者成为 `owner`

### 测试操作流程
1. 调用创建组织 API：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Acme Corp", "slug": "acme-corp", "domain": "acme.com"}'
   ```
2. 记录返回的 `{tenant_id}`
3. 查询数据库验证

### 预期结果
- HTTP 201，返回租户信息
- `status` 为 `active`
- `domain` 为 `acme.com`

### 预期数据状态
```sql
SELECT id, name, slug, domain, status FROM tenants WHERE slug = 'acme-corp';
-- 预期: status = 'active', domain = 'acme.com'

SELECT tu.role_in_tenant, tu.user_id
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE t.slug = 'acme-corp';
-- 预期: 1 条记录，role_in_tenant = 'owner'
```

---

## 场景 2：域名不匹配 — 创建组织为 Pending 状态

### 初始状态
- 用户已通过 OAuth 登录，邮箱为 `user@gmail.com`
- 持有有效的 Identity Token
- 不存在 slug 为 `acme-pending` 的租户

### 目的
验证创建者邮箱域名与组织域名不匹配时，租户状态为 `pending`

### 测试操作流程
1. 调用创建组织 API：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Acme Pending", "slug": "acme-pending", "domain": "acme.com"}'
   ```
2. 查询数据库验证

### 预期结果
- HTTP 201，返回租户信息
- `status` 为 `pending`
- 创建者仍被添加为 `owner`

### 预期数据状态
```sql
SELECT id, name, slug, domain, status FROM tenants WHERE slug = 'acme-pending';
-- 预期: status = 'pending', domain = 'acme.com'

SELECT tu.role_in_tenant FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE t.slug = 'acme-pending';
-- 预期: 1 条记录，role_in_tenant = 'owner'
```

---

## 场景 3：Slug 重复 — 创建被拒绝

### 初始状态
- 已存在 slug 为 `acme-corp` 的租户

### 目的
验证 slug 唯一性校验

### 测试操作流程
1. 调用创建组织 API 使用已存在的 slug：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Another Acme", "slug": "acme-corp", "domain": "another.com"}'
   ```

### 预期结果
- HTTP 409 Conflict
- 返回错误消息包含 slug 已存在的提示
- 数据库中不创建新租户

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE slug = 'acme-corp';
-- 预期: 1（不变）
```

---

## 场景 4：域名格式验证

### 初始状态
- 用户已登录，持有 Identity Token

### 目的
验证组织域名格式校验正确拒绝非法域名

### 测试操作流程
1. 使用空域名创建：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Test", "slug": "test-empty-domain", "domain": ""}'
   ```
2. 使用非法格式域名创建：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Test", "slug": "test-bad-domain", "domain": "not a domain!"}'
   ```
3. 使用带协议的域名创建：
   ```bash
   curl -X POST http://localhost:8080/api/v1/organizations \
     -H "Authorization: Bearer {identity_token}" \
     -H "Content-Type: application/json" \
     -d '{"name": "Test", "slug": "test-proto-domain", "domain": "https://acme.com"}'
   ```

### 预期结果
- 所有请求返回 HTTP 422 或 400
- 错误消息提示域名格式无效
- 数据库中不创建新租户

---

## 场景 5：GET /api/v1/users/me/tenants — 获取当前用户的租户列表

### 初始状态
- 用户已通过 OAuth 登录，是 2 个租户的成员（1 个 owner + 1 个 member）
- 持有有效的 Identity Token

### 目的
验证 `/users/me/tenants` 返回当前用户的所有租户成员关系

### 测试操作流程
1. 调用 API：
   ```bash
   curl http://localhost:8080/api/v1/users/me/tenants \
     -H "Authorization: Bearer {identity_token}"
   ```
2. 检查返回数据

### 预期结果
- HTTP 200
- 返回数组包含 2 个 `TenantUserWithTenant` 对象
- 每个对象包含 `tenant_id`、`role_in_tenant`、`tenant` 嵌套对象（含 `name`、`slug`、`status`）
- 不返回其他用户的租户

### 预期数据状态
```sql
SELECT tu.tenant_id, tu.role_in_tenant, t.name, t.slug, t.status
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}';
-- 预期: 2 条记录，与 API 返回一致
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 域名匹配 — 自动 Active | ☐ | | | |
| 2 | 域名不匹配 — Pending 状态 | ☐ | | | |
| 3 | Slug 重复 — 创建被拒绝 | ☐ | | | |
| 4 | 域名格式验证 | ☐ | | | |
| 5 | GET /users/me/tenants 接口 | ☐ | | | |
