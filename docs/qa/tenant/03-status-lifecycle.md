# 租户管理 - 状态生命周期测试

**模块**: 租户管理
**测试范围**: 租户状态流转（Active/Inactive/Suspended）及其业务影响
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 租户有三种状态：

| 状态 | 说明 |
|------|------|
| `active` | 正常运行，所有功能可用（默认） |
| `inactive` | 已停用，业务功能应受限 |
| `suspended` | 已暂停，可能因违规或欠费 |

**重要说明**：
- 状态变更**必须通过 API 进行**，不要直接修改数据库。直接修改数据库不会触发缓存清除和审计日志。
- 通过 API 变更状态时，Redis 缓存会**自动清除**，审计日志会**自动记录**。
- Portal UI 租户详情页提供状态下拉选择器，可直接编辑状态。
- 非 active 状态的租户会**阻止写操作**（创建 Webhook、发送邀请、添加用户）和 **Token Exchange**。

状态通过 `PUT /api/v1/tenants/{id}` 更新：
```json
{
  "status": "suspended"
}
```

---

## 场景 1：将租户状态设为 Inactive

### 初始状态
- 存在租户 id=`{tenant_id}`，status=`active`
- 该租户有关联用户和服务

### 目的
验证租户状态可以正确切换为 inactive

### 测试操作流程
1. 进入租户详情页 `/dashboard/tenants/{tenant_id}`
2. 修改状态为 `Inactive`（或调用 API）：
   ```bash
   PUT /api/v1/tenants/{tenant_id}
   { "status": "inactive" }
   ```
3. 刷新页面确认状态显示

### 预期结果
- 状态更新成功
- 租户详情页显示状态为 `Inactive`
- 审计日志记录状态变更

### 预期数据状态
```sql
SELECT status FROM tenants WHERE id = '{tenant_id}';
-- 预期: inactive

SELECT action, old_value, new_value FROM audit_logs
WHERE resource_type = 'tenant' AND resource_id = '{tenant_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'tenant.update'，包含 status 变更
```

---

## 场景 2：将租户状态设为 Suspended

### 初始状态
- 存在租户 id=`{tenant_id}`，status=`active`

### 目的
验证租户可以被暂停

### 测试操作流程
1. 调用 API 暂停租户：
   ```bash
   PUT /api/v1/tenants/{tenant_id}
   { "status": "suspended" }
   ```
2. 查看租户列表，确认状态标识

### 预期结果
- 状态更新成功
- 租户列表中显示 `Suspended` 状态标识

### 预期数据状态
```sql
SELECT status FROM tenants WHERE id = '{tenant_id}';
-- 预期: suspended
```

---

## 场景 3：恢复 Suspended 租户为 Active

### 初始状态
- 存在租户 id=`{tenant_id}`，status=`suspended`

### 目的
验证租户可以从暂停状态恢复

### 测试操作流程
1. 调用 API 恢复租户：
   ```bash
   PUT /api/v1/tenants/{tenant_id}
   { "status": "active" }
   ```
2. 验证租户下的用户能否正常操作

### 预期结果
- 状态更新为 `active`
- 租户功能恢复正常

### 预期数据状态
```sql
SELECT status FROM tenants WHERE id = '{tenant_id}';
-- 预期: active

-- 审计日志应记录两次状态变更
SELECT action, old_value, new_value FROM audit_logs
WHERE resource_type = 'tenant' AND resource_id = '{tenant_id}'
  AND action = 'tenant.update'
ORDER BY created_at DESC LIMIT 2;
-- 预期: 2 条记录，分别为 suspended→active 和 active→suspended
```

---

## 场景 4：Inactive 租户的 Token Exchange 行为

### 初始状态
- 租户 id=`{tenant_id}`，status=`inactive`
- 用户已登录，持有 Identity Token
- 用户是该租户的成员

### 目的
验证非 active 状态的租户在 Token Exchange 时的行为

### 测试操作流程
1. 调用 gRPC Token Exchange 请求该租户的 Access Token：
   ```protobuf
   ExchangeTokenRequest {
     identity_token: "<Identity Token>"
     tenant_id: "{tenant_id}"
   }
   ```
2. 检查响应

### 预期结果
- 返回错误「Tenant is not active (status: 'inactive')」，HTTP 403 / gRPC PermissionDenied
- 拒绝发放 Token

### 预期数据状态
```sql
SELECT status FROM tenants WHERE id = '{tenant_id}';
-- 预期: inactive
```

---

## 场景 5：租户状态对管理操作的影响

### 初始状态
- 租户 id=`{tenant_id}`，status=`suspended`

### 目的
验证暂停状态的租户能否继续进行管理操作

### 测试操作流程
1. 尝试在该租户下创建用户：
   ```bash
   POST /api/v1/users
   { "email": "new@example.com", ... }
   ```
   然后添加到该租户
2. 尝试在该租户下创建邀请：
   ```bash
   POST /api/v1/tenants/{tenant_id}/invitations
   { "email": "invite@example.com", ... }
   ```
3. 尝试在该租户下创建 Webhook：
   ```bash
   POST /api/v1/tenants/{tenant_id}/webhooks
   { "url": "https://example.com/hook", ... }
   ```

### 预期结果
- 所有写操作应被阻止，返回 HTTP 403 错误
- 错误信息包含「Tenant is not active (status: 'suspended'). Write operations are not allowed on non-active tenants.」

---

## 测试数据准备 SQL

```sql
-- 创建测试租户（状态为 active）
INSERT INTO tenants (id, name, slug, status, settings, created_at, updated_at)
VALUES (
  'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
  'Status Test Tenant',
  'status-test',
  'active',
  '{}',
  NOW(),
  NOW()
);

-- 添加测试用户关联
INSERT INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)
VALUES (
  UUID(),
  'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
  '{user_id}',
  'admin',
  NOW()
);

-- 清理
DELETE FROM tenants WHERE slug = 'status-test';
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 设为 Inactive | ☐ | | | |
| 2 | 设为 Suspended | ☐ | | | |
| 3 | 恢复为 Active | ☐ | | | |
| 4 | Inactive 租户 Token Exchange | ☐ | | | |
| 5 | Suspended 租户管理操作 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
