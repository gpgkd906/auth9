# 租户管理 - 列表与设置测试

**模块**: 租户管理
**测试范围**: 分页、搜索、详情、设置、验证
**场景数**: 5

---

## 场景 1：租户列表分页

### 初始状态
- 数据库中存在 25 个租户

### 目的
验证租户列表分页功能

### 测试操作流程
1. 打开租户管理页面
2. 观察第一页显示数量
3. 点击「下一页」

### 预期结果
- 第一页显示 20 条（默认）
- 第二页显示剩余 5 条
- 分页控件显示正确

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE status != 'deleted';
-- 预期: 25
```

---

## 场景 2：查看租户详情

### 初始状态
- 存在租户 id=`{tenant_id}`

### 目的
验证租户详情页正确显示

### 测试操作流程
1. 在列表中点击目标租户名称

### 预期结果
- 显示租户基本信息
- 显示关联的用户数量
- 显示关联的服务数量
- 显示创建和更新时间

### 预期数据状态
```sql
SELECT t.*,
       (SELECT COUNT(*) FROM tenant_users WHERE tenant_id = t.id) as user_count,
       (SELECT COUNT(*) FROM services WHERE tenant_id = t.id) as service_count
FROM tenants t WHERE t.id = '{tenant_id}';
```

---

## 场景 3：搜索租户

### 初始状态
- 存在以下租户：
  - `Acme Corporation` (slug: acme)
  - `Beta Company` (slug: beta)
  - `Acme Labs` (slug: acme-labs)

### 目的
验证租户搜索功能

### 测试操作流程
1. 在搜索框输入 `acme`
2. 按回车

### 预期结果
- 显示 2 条结果：`Acme Corporation` 和 `Acme Labs`

### 预期数据状态
```sql
SELECT name, slug FROM tenants WHERE name LIKE '%acme%' OR slug LIKE '%acme%';
-- 预期: 2 条记录
```

---

## 场景 4：租户设置更新

### 初始状态
- 存在租户 id=`{tenant_id}`，settings=`{"require_mfa": false}`

### 目的
验证租户设置的更新功能

### 测试操作流程
1. 进入租户详情页 `/dashboard/tenants/{tenant_id}`
2. 在「Security Settings」区域找到「Require MFA」开关
3. 开启开关（自动保存）

### 预期结果
- 显示设置保存成功
- 该租户下用户需要启用 MFA

### 预期数据状态
```sql
SELECT settings FROM tenants WHERE id = '{tenant_id}';
-- 预期: {"require_mfa": true, ...}
```

---

## 场景 5：Slug 格式验证

### 初始状态
- 用户尝试创建租户

### 目的
验证 Slug 格式验证

### 测试操作流程
测试以下非法 Slug：
1. 包含大写：`TestCompany`
2. 包含特殊字符：`test@company`
3. 以连字符开头：`-test-company`
4. 超过 63 字符

### 预期结果
- 每种情况都显示格式错误提示
- 租户不应被创建

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE slug IN ('TestCompany', 'test@company', '-test-company');
-- 预期: 0
```

---

## 测试数据准备 SQL

```sql
-- 准备测试租户
INSERT INTO tenants (id, name, slug, settings, status) VALUES
('11111111-1111-1111-1111-111111111111', 'Acme Corporation', 'acme', '{"require_mfa": false}', 'active'),
('22222222-2222-2222-2222-222222222222', 'Beta Company', 'beta', '{"require_mfa": false}', 'active'),
('33333333-3333-3333-3333-333333333333', 'Acme Labs', 'acme-labs', '{"require_mfa": true}', 'active');

-- 清理
DELETE FROM tenants WHERE slug IN ('acme', 'beta', 'acme-labs', 'test-company');
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
| 1 | 租户列表分页 | ☐ | | | |
| 2 | 查看租户详情 | ☐ | | | |
| 3 | 搜索租户 | ☐ | | | |
| 4 | 租户设置更新 | ☐ | | | |
| 5 | Slug 格式验证 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
