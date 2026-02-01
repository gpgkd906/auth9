# 用户管理 - 验证与边界测试

**模块**: 用户管理
**测试范围**: 边界情况、输入验证
**场景数**: 3

---

## 场景 1：用户重复加入同一租户

### 初始状态
- 用户 `{user_id}` 已是租户 `{tenant_id}` 的成员

### 目的
验证系统阻止重复加入

### 测试操作流程
1. 尝试将用户再次添加到同一租户

### 预期结果
- 显示错误：「用户已是该租户成员」

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}' AND tenant_id = '{tenant_id}';
-- 预期: 1（仍然只有一条）
```

---

## 场景 2：修改用户在租户中的角色

### 初始状态
- 用户 `{user_id}` 在租户 `{tenant_id}` 中的角色为 `member`

### 目的
验证租户角色修改功能

### 测试操作流程
1. 打开用户的租户管理界面
2. 找到目标租户
3. 修改角色为 `admin`
4. 保存

### 预期结果
- 显示更新成功
- 角色显示为 `admin`

### 预期数据状态
```sql
SELECT role_in_tenant FROM tenant_users WHERE user_id = '{user_id}' AND tenant_id = '{tenant_id}';
-- 预期: admin
```

---

## 场景 3：邮箱格式验证

### 初始状态
- 用户尝试创建用户

### 目的
验证邮箱格式验证

### 测试操作流程
测试以下非法邮箱：
1. 无 @ 符号：`invalidemail`
2. 无域名：`test@`
3. 无用户名：`@example.com`
4. 含特殊字符：`test<script>@example.com`

### 预期结果
- 每种情况都显示邮箱格式错误
- 用户不应被创建

### 预期数据状态
```sql
-- 无新记录被创建
```

---

## 测试数据准备 SQL

```sql
-- 准备测试租户
INSERT INTO tenants (id, name, slug, settings, status) VALUES
('aaaa1111-1111-1111-1111-111111111111', 'Test Tenant A', 'test-a', '{}', 'active'),
('aaaa2222-2222-2222-2222-222222222222', 'Test Tenant B', 'test-b', '{}', 'active');

-- 准备测试用户
INSERT INTO users (id, keycloak_id, email, display_name, mfa_enabled) VALUES
('bbbb1111-1111-1111-1111-111111111111', 'kc-user-1', 'existing@example.com', '已存在用户', false);

-- 清理
DELETE FROM tenant_users WHERE user_id LIKE 'bbbb%';
DELETE FROM users WHERE id LIKE 'bbbb%';
DELETE FROM tenants WHERE id LIKE 'aaaa%';
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 重复加入租户 | ☐ | | | |
| 2 | 修改租户角色 | ☐ | | | |
| 3 | 邮箱格式验证 | ☐ | | | |
