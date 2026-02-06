# 租户管理 - CRUD 操作测试

**模块**: 租户管理
**测试范围**: 创建、更新、删除基本操作
**场景数**: 5

---

## 数据库表结构参考

### tenants 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| name | VARCHAR(255) | 租户名称 |
| slug | VARCHAR(63) | URL 友好的唯一标识符 |
| logo_url | TEXT | Logo URL |
| settings | JSON | 租户设置 |
| status | VARCHAR(20) | 状态：active/inactive/suspended |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：创建租户

### 初始状态
- 用户已登录管理后台
- 数据库中无同名或同 slug 的租户

### 目的
验证创建租户功能的正确性

### 测试操作流程
1. 在管理后台点击「租户管理」菜单
2. 点击「创建租户」按钮
3. 填写表单：
   - 租户名称：`测试公司`
   - Slug：`test-company`
   - Logo URL：`https://example.com/logo.png`
4. 点击「创建」按钮

### 预期结果
- 显示创建成功提示
- 租户出现在列表中，状态为「Active」

### 预期数据状态
```sql
SELECT id, name, slug, logo_url, status FROM tenants WHERE slug = 'test-company';
-- 预期: 存在一条记录，status = 'active'

SELECT action, resource_type FROM audit_logs WHERE resource_type = 'tenant' ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'tenant.create'
```

---

## 场景 2：创建重复 Slug 的租户

### 初始状态
- 数据库中已存在 slug 为 `test-company` 的租户

### 目的
验证系统拒绝重复 slug

### 测试操作流程
1. 点击「创建租户」
2. 填写 Slug：`test-company`
3. 点击「创建」

### 预期结果
- 显示错误提示：「Slug 已存在」
- 租户未被创建

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE slug = 'test-company';
-- 预期: 1
```

---

## 场景 3：更新租户信息

### 初始状态
- 存在租户 id=`{tenant_id}`，name=`测试公司`

### 目的
验证租户信息更新功能

### 测试操作流程
1. 找到「测试公司」，点击「编辑」
2. 修改：
   - 名称：`测试公司（更新）`
   - Logo URL：`https://example.com/new-logo.png`
3. 点击「保存」

### 预期结果
- 显示更新成功
- 列表中显示新名称

### 预期数据状态
```sql
SELECT name, logo_url, updated_at FROM tenants WHERE id = '{tenant_id}';
-- 预期: name = '测试公司（更新）'，updated_at 为当前时间

SELECT action, old_value, new_value FROM audit_logs WHERE resource_id = '{tenant_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'tenant.update'，包含修改前后的值
```

---

## 场景 4：删除无关联数据的租户

### 初始状态
- 存在租户 id=`{tenant_id}`
- 该租户没有关联的用户、服务、Webhook、邀请

### 目的
验证租户删除功能

### 测试操作流程
1. 找到目标租户
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 租户从列表中消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE id = '{tenant_id}';
-- 预期: 0

SELECT action, resource_id FROM audit_logs WHERE resource_type = 'tenant' ORDER BY created_at DESC LIMIT 1;
-- 预期: action = 'tenant.delete'
```

---

## 场景 5：删除有关联数据的租户（级联删除）

### 初始状态
- 存在租户 id=`{tenant_id}`
- 该租户有以下关联数据：
  - 2 个用户关联（tenant_users）
  - 1 个服务（services）
  - 1 个 Webhook（webhooks）
  - 1 个邀请（invitations）

### 目的
验证删除时的级联删除

### 测试操作流程
1. 找到目标租户
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 所有关联数据被清除

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM tenant_users WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM services WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM webhooks WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM invitations WHERE tenant_id = '{tenant_id}';
-- 预期: 0
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建租户 | ☐ | | | |
| 2 | 创建重复 Slug 租户 | ☐ | | | |
| 3 | 更新租户信息 | ☐ | | | |
| 4 | 删除无关联数据租户 | ☐ | | | |
| 5 | 删除有关联数据租户 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
