# RBAC - 权限管理测试

**模块**: RBAC 角色权限管理
**测试范围**: 权限 CRUD 和验证
**场景数**: 4

---

## 数据库表结构参考

### permissions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属服务 ID |
| code | VARCHAR(100) | 权限代码（如 user:read） |
| name | VARCHAR(255) | 权限名称 |
| description | TEXT | 描述 |

---

## 场景 1：创建权限

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务下无 code=`user:read` 的权限

### 目的
验证权限创建功能

### 测试操作流程
1. 进入「角色与权限」页面
2. 切换到「权限」标签
3. 点击「创建权限」
4. 填写：
   - 服务：选择目标服务
   - 权限代码：`user:read`
   - 名称：`读取用户`
   - 描述：`允许查看用户列表和详情`
5. 点击「创建」

### 预期结果
- 显示创建成功
- 权限出现在列表中

### 预期数据状态
```sql
SELECT id, service_id, code, name, description FROM permissions
WHERE code = 'user:read' AND service_id = '{service_id}';
-- 预期: 存在记录
```

---

## 场景 2：创建重复 code 的权限

### 初始状态
- 服务 `{service_id}` 下已存在 code=`user:read` 的权限

### 目的
验证权限 code 唯一性约束

### 测试操作流程
1. 尝试创建同样 code 的权限

### 预期结果
- 显示错误：「权限代码已存在」

### 预期数据状态
```sql
SELECT COUNT(*) FROM permissions WHERE service_id = '{service_id}' AND code = 'user:read';
-- 预期: 1
```

---

## 场景 3：删除权限

### 初始状态
- 存在权限 id=`{permission_id}`
- 该权限已分配给某个角色

### 目的
验证权限删除和级联处理

### 测试操作流程
1. 找到目标权限
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 权限从列表消失
- 角色-权限关联被清除

### 预期数据状态
```sql
SELECT COUNT(*) FROM permissions WHERE id = '{permission_id}';
-- 预期: 0

SELECT COUNT(*) FROM role_permissions WHERE permission_id = '{permission_id}';
-- 预期: 0
```

---

## 场景 4：权限代码格式验证

### 初始状态
- 用户尝试创建权限

### 目的
验证权限代码格式

### 测试操作流程
测试以下代码：
1. 标准格式：`report:export` ✓
2. 带命名空间：`admin:user:delete` ✓
3. 非法字符：`user@read` ✗
4. 空格：`user read` ✗

### 预期结果
- 非法格式被拒绝

### 预期数据状态
```sql
-- 无非法权限被创建
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
| 1 | 创建权限 | ☐ | | | |
| 2 | 创建重复 code 权限 | ☐ | | | |
| 3 | 删除权限 | ☐ | | | |
| 4 | 权限代码格式验证 | ☐ | | | |
| 5 | 认证状态检查 | ☐ | | | |
