# RBAC - 高级功能测试

**模块**: RBAC 角色权限管理
**测试范围**: 层次视图、循环检测、跨服务验证
**场景数**: 3

---

## 场景 1：角色层次视图

### 初始状态
- 服务下存在以下角色结构：
  ```
  Admin (根)
  ├── Editor
  │   └── Viewer
  └── Moderator
  ```

### 目的
验证角色层次视图正确显示

### 测试操作流程
1. 进入「角色与权限」页面
2. 切换到「层次结构」标签

### 预期结果
- 显示树形结构
- 正确显示父子关系
- 显示每个角色的权限数量

### 预期数据状态
```sql
SELECT r.name, p.name as parent_name FROM roles r
LEFT JOIN roles p ON p.id = r.parent_role_id
WHERE r.service_id = '{service_id}';

-- 预期
-- | name      | parent_name |
-- | Admin     | NULL        |
-- | Editor    | Admin       |
-- | Viewer    | Editor      |
-- | Moderator | Admin       |
```

---

## 场景 2：循环继承检测

### 初始状态
- 角色 A 继承自角色 B
- 尝试设置角色 B 继承自角色 A

### 目的
验证系统检测并阻止循环继承

### 测试操作流程
1. 编辑角色 B
2. 设置父角色为 A
3. 保存

### 预期结果
- 显示错误：「检测到循环继承」
- 修改被拒绝

### 预期数据状态
```sql
-- 角色 B 的 parent_role_id 保持不变
SELECT parent_role_id FROM roles WHERE name = 'B';
```

---

## 场景 3：跨服务权限分配验证

### 初始状态
- 服务 A 有权限 `perm-a`
- 服务 B 有角色 `role-b`

### 目的
验证不能跨服务分配权限

### 测试操作流程
1. 尝试为 role-b 分配 perm-a

### 预期结果
- UI 中只显示服务 B 的权限
- API 尝试会返回错误

### 预期数据状态
```sql
-- 不应存在跨服务的权限分配
SELECT rp.* FROM role_permissions rp
JOIN roles r ON r.id = rp.role_id
JOIN permissions p ON p.id = rp.permission_id
WHERE r.service_id != p.service_id;
-- 预期: 0 条记录
```

---

## 测试数据准备 SQL

```sql
-- 准备测试服务
INSERT INTO services (id, name, redirect_uris, logout_uris, status) VALUES
('11111111-1111-4111-8111-111111111111', 'Test Service', '[]', '[]', 'active');

-- 准备测试权限
INSERT INTO permissions (id, service_id, code, name) VALUES
('22222222-2222-4222-8222-222222222222', '11111111-1111-4111-8111-111111111111', 'content:read', '读取内容'),
('33333333-3333-4333-8333-333333333333', '11111111-1111-4111-8111-111111111111', 'content:write', '写入内容');

-- 准备测试角色
INSERT INTO roles (id, service_id, name, parent_role_id) VALUES
('44444444-4444-4444-8444-444444444444', '11111111-1111-4111-8111-111111111111', 'Viewer', NULL),
('55555555-5555-4555-8555-555555555555', '11111111-1111-4111-8111-111111111111', 'Editor', '44444444-4444-4444-8444-444444444444');

-- 清理
DELETE FROM role_permissions WHERE role_id IN ('44444444-4444-4444-8444-444444444444', '55555555-5555-4555-8555-555555555555');
DELETE FROM user_tenant_roles WHERE role_id IN ('44444444-4444-4444-8444-444444444444', '55555555-5555-4555-8555-555555555555');
DELETE FROM roles WHERE service_id = '11111111-1111-4111-8111-111111111111';
DELETE FROM permissions WHERE service_id = '11111111-1111-4111-8111-111111111111';
DELETE FROM services WHERE id = '11111111-1111-4111-8111-111111111111';
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
| 1 | 角色层次视图 | ☐ | | | |
| 2 | 循环继承检测 | ☐ | | | |
| 3 | 跨服务权限分配 | ☐ | | | |
| 4 | 认证状态检查 | ☐ | | | |
