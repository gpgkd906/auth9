# 用户管理 - CRUD 操作测试

**模块**: 用户管理
**测试范围**: 用户创建、更新基本操作
**场景数**: 5

---

## 数据库表结构参考

### users 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| keycloak_id | VARCHAR(255) | Keycloak 用户 ID |
| email | VARCHAR(255) | 邮箱（唯一） |
| display_name | VARCHAR(255) | 显示名称 |
| mfa_enabled | BOOLEAN | 是否启用 MFA |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：创建用户

### 初始状态
- 管理员已登录
- 数据库中无同邮箱的用户

### 目的
验证用户创建功能，确保同步到 Keycloak

### 测试操作流程
1. 进入「用户管理」页面
2. 点击「创建用户」
3. 填写：
   - 邮箱：`newuser@example.com`
   - 显示名称：`新用户`
   - 密码：`SecurePass123!`
4. 点击「创建」

### 预期结果
- 显示创建成功
- 用户出现在列表中
- MFA 状态默认关闭

### 预期数据状态
```sql
SELECT id, keycloak_id, email, display_name, mfa_enabled FROM users WHERE email = 'newuser@example.com';
-- 预期: 存在记录，keycloak_id 非空

-- Keycloak 验证：用户 newuser@example.com 存在
```

---

## 场景 2：创建重复邮箱的用户

### 初始状态
- 已存在邮箱为 `existing@example.com` 的用户

### 目的
验证系统拒绝重复邮箱

### 测试操作流程
1. 点击「创建用户」
2. 填写邮箱：`existing@example.com`
3. 点击「创建」

### 预期结果
- 显示错误：「邮箱已存在」

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE email = 'existing@example.com';
-- 预期: 1
```

---

## 场景 3：更新用户信息

### 初始状态
- 存在用户 id=`{user_id}`，display_name=`旧名称`

### 目的
验证用户信息更新功能

### 测试操作流程
1. 找到目标用户
2. 点击「编辑」
3. 修改显示名称为：`新名称`
4. 保存

### 预期结果
- 显示更新成功
- 列表显示新名称

### 预期数据状态
```sql
SELECT display_name, updated_at FROM users WHERE id = '{user_id}';
-- 预期: display_name = '新名称'

-- Keycloak 验证：用户名称已同步更新
```

---

## 场景 4：添加用户到租户

### 初始状态
- 存在用户 id=`{user_id}`
- 存在租户 id=`{tenant_id}`
- 用户尚未加入该租户

### 目的
验证添加用户到租户功能

### 测试操作流程
1. 找到目标用户
2. 点击「管理租户」
3. 点击「添加到租户」
4. 选择租户，角色：`admin`
5. 确认

### 预期结果
- 显示添加成功
- 租户列表中显示新租户

### 预期数据状态
```sql
SELECT id, tenant_id, user_id, role_in_tenant FROM tenant_users
WHERE user_id = '{user_id}' AND tenant_id = '{tenant_id}';
-- 预期: 存在记录，role_in_tenant = 'admin'
```

---

## 场景 5：从租户移除用户

### 初始状态
- 用户 `{user_id}` 已加入租户 `{tenant_id}`
- 用户在该租户有分配的角色

### 目的
验证从租户移除用户和级联删除

### 测试操作流程
1. 打开用户的租户管理界面
2. 找到要移除的租户
3. 点击「移除」
4. 确认

### 预期结果
- 显示移除成功
- 该租户从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}' AND tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM user_tenant_roles WHERE tenant_user_id = '{tenant_user_id}';
-- 预期: 0（级联删除）
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建用户 | ☐ | | | |
| 2 | 创建重复邮箱用户 | ☐ | | | |
| 3 | 更新用户信息 | ☐ | | | |
| 4 | 添加用户到租户 | ☐ | | | |
| 5 | 从租户移除用户 | ☐ | | | |
