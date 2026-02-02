# RBAC - 权限分配测试

**模块**: RBAC 角色权限管理
**测试范围**: 权限分配、用户角色分配、有效权限查询
**场景数**: 5

---

## 数据库表结构参考

### role_permissions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| role_id | CHAR(36) | 角色 ID |
| permission_id | CHAR(36) | 权限 ID |

### user_tenant_roles 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_user_id | CHAR(36) | tenant_users 表 ID |
| role_id | CHAR(36) | 角色 ID |
| granted_at | TIMESTAMP | 授予时间 |
| granted_by | CHAR(36) | 授予者用户 ID |

---

## 场景 1：为角色分配权限

### 初始状态
- 存在角色 id=`{role_id}`
- 存在权限 id=`{permission_id}`
- 该权限尚未分配给该角色

### 目的
验证角色-权限分配功能

### 测试操作流程
1. 找到目标角色
2. 点击「管理权限」
3. 勾选要分配的权限
4. 保存

### 预期结果
- 显示分配成功
- 权限已勾选

### 预期数据状态
```sql
SELECT role_id, permission_id FROM role_permissions
WHERE role_id = '{role_id}' AND permission_id = '{permission_id}';
-- 预期: 存在记录
```

---

## 场景 2：从角色移除权限

### 初始状态
- 角色 `{role_id}` 已有权限 `{permission_id}`

### 目的
验证权限移除功能

### 测试操作流程
1. 打开角色的权限管理
2. 取消勾选目标权限
3. 保存

### 预期结果
- 显示更新成功
- 权限不再勾选

### 预期数据状态
```sql
SELECT COUNT(*) FROM role_permissions WHERE role_id = '{role_id}' AND permission_id = '{permission_id}';
-- 预期: 0
```

---

## 场景 3：为用户分配角色

### 初始状态
- 用户 `{user_id}` 已加入租户 `{tenant_id}`
- 存在角色 `{role_id}`
- 用户尚未拥有该角色

### 目的
验证用户-角色分配功能

### 测试操作流程
1. 进入用户管理
2. 选择「管理角色」
3. 选择租户和服务
4. 勾选角色
5. 保存

### 预期结果
- 显示分配成功
- 用户角色列表显示新角色

### 预期数据状态
```sql
SELECT utr.id, utr.role_id, utr.granted_at, utr.granted_by
FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}' AND utr.role_id = '{role_id}';
-- 预期: 存在记录
```

---

## 场景 4：移除用户的角色

### 初始状态
- 用户在租户中拥有角色 `{role_id}`

### 目的
验证用户角色移除功能

### 测试操作流程
1. 打开用户的角色管理
2. 取消勾选目标角色
3. 保存

### 预期结果
- 显示更新成功
- 角色从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}' AND utr.role_id = '{role_id}';
-- 预期: 0
```

---

## 场景 5：查询用户的有效权限（含继承）

### 初始状态
- 用户在租户中有角色 `Editor`
- `Editor` 继承自 `Viewer`
- `Viewer` 有权限：`content:read`
- `Editor` 有权限：`content:write`

### 目的
验证有效权限包含继承权限

### 测试操作流程
1. 调用 API 或 gRPC 获取用户权限

### 预期结果
- 返回权限包含：`content:read`, `content:write`

### 预期数据状态
```sql
-- 验证用户角色
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';

-- 验证有效权限（含继承）
WITH RECURSIVE role_tree AS (
    SELECT r.id, r.parent_role_id FROM user_tenant_roles utr
    JOIN roles r ON r.id = utr.role_id
    JOIN tenant_users tu ON tu.id = utr.tenant_user_id
    WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}'
    UNION ALL
    SELECT r.id, r.parent_role_id FROM roles r
    JOIN role_tree rt ON rt.parent_role_id = r.id
)
SELECT DISTINCT p.code FROM role_tree rt
JOIN role_permissions rp ON rp.role_id = rt.id
JOIN permissions p ON p.id = rp.permission_id;
-- 预期: content:read, content:write
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
| 1 | 为角色分配权限 | ☐ | | | |
| 2 | 从角色移除权限 | ☐ | | | |
| 3 | 为用户分配角色 | ☐ | | | |
| 4 | 移除用户角色 | ☐ | | | |
| 5 | 查询有效权限 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
