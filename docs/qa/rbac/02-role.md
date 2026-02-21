# RBAC - 角色管理测试

**模块**: RBAC 角色权限管理
**测试范围**: 角色 CRUD 和继承
**场景数**: 5

---

## 数据库表结构参考

### roles 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属服务 ID |
| name | VARCHAR(100) | 角色名称 |
| description | TEXT | 描述 |
| parent_role_id | CHAR(36) | 父角色 ID（继承） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：创建角色

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务下无同名角色

### 目的
验证角色创建功能

### 测试操作流程
1. 进入「角色与权限」页面
2. 切换到「角色」标签
3. 点击「创建角色」
4. 填写：
   - 服务：选择目标服务
   - 角色名称：`Editor`
   - 描述：`可以编辑内容`
   - 父角色：无
5. 点击「创建」

### 预期结果
- 显示创建成功
- 角色出现在列表中

### 预期数据状态
```sql
SELECT id, service_id, name, description, parent_role_id FROM roles
WHERE name = 'Editor' AND service_id = '{service_id}';
-- 预期: 存在记录，parent_role_id = NULL
```

---

## 场景 2：创建带继承的角色

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务下存在角色 `Viewer` (id=`{viewer_role_id}`)

### 目的
验证角色继承功能

### 测试操作流程
1. 创建新角色：
   - 名称：`Editor`
   - 父角色：选择 `Viewer`
2. 点击「创建」

### 预期结果
- 角色创建成功
- 层次视图显示继承关系

### 预期数据状态
```sql
SELECT name, parent_role_id FROM roles WHERE name = 'Editor' AND service_id = '{service_id}';
-- 预期: parent_role_id = '{viewer_role_id}'
```

---

## 场景 3：更新角色

### 初始状态
- 存在角色 id=`{role_id}`，name=`Editor`

### 目的
验证角色更新功能

### 测试操作流程
1. 找到目标角色
2. 点击「编辑」
3. 修改：
   - 名称：`Content Editor`
   - 描述：`可以编辑和发布内容`
4. 保存

### 预期结果
- 显示更新成功
- 列表显示新名称

### 预期数据状态
```sql
SELECT name, description, updated_at FROM roles WHERE id = '{role_id}';
-- 预期: name = 'Content Editor'
```

---

## 场景 4：删除角色

### 初始状态
- 存在角色 id=`{role_id}`
- 该角色有权限关联
- 该角色已分配给用户

### 目的
验证角色删除的级联处理

### 测试操作流程
1. 找到目标角色
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 角色从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM roles WHERE id = '{role_id}';
-- 预期: 0

SELECT COUNT(*) FROM role_permissions WHERE role_id = '{role_id}';
-- 预期: 0

SELECT COUNT(*) FROM user_tenant_roles WHERE role_id = '{role_id}';
-- 预期: 0
```

---

## 场景 5：删除有子角色的角色

### 初始状态
- 存在父角色 `Admin` (id=`{admin_role_id}`)
- 存在子角色 `Super Admin`，parent_role_id = `{admin_role_id}`

### 目的
验证删除父角色时的处理

### 测试操作流程
1. 尝试删除 `Admin` 角色

### 预期结果
- 选项1：显示错误「该角色有子角色，无法删除」
- 选项2：删除成功，子角色 parent_role_id 置为 NULL

### 预期数据状态
```sql
-- 根据实现方式验证
SELECT parent_role_id FROM roles WHERE name = 'Super Admin';
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户未登录（无有效 session cookie）

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 打开浏览器的隐私/无痕模式（确保无遗留 cookie）
2. 直接访问本页面对应的 URL（如 `/dashboard/roles`）

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

### 注意事项
> **Session 为持久化 Cookie（maxAge: 8 小时）**：关闭浏览器不会清除 session。
> 要测试未认证状态，请使用以下方法之一：
> 1. 使用浏览器隐私/无痕模式（推荐）
> 2. 手动点击「Sign out」退出登录
> 3. 手动清除 `auth9_session` cookie
> 4. 等待 session 过期（8 小时）
>
> | 症状 | 原因 | 解决方法 |
> |------|------|----------|
> | 关闭浏览器后重新打开仍可访问 Dashboard | 持久化 Cookie 未过期 | 使用无痕模式或手动清除 Cookie |
> | 清除 Cookie 后页面未跳转 | 浏览器缓存 | 强制刷新（Ctrl+Shift+R） |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建角色 | ☐ | | | |
| 2 | 创建带继承的角色 | ☐ | | | |
| 3 | 更新角色 | ☐ | | | |
| 4 | 删除角色 | ☐ | | | |
| 5 | 删除有子角色的角色 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
