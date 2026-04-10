# 审计日志测试

**模块**: 审计日志
**测试范围**: 查看、筛选、分页审计记录
**场景数**: 5

---

## 数据库表结构参考

### audit_logs 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGINT | 自增主键 |
| actor_id | CHAR(36) | 操作者用户 ID（关联 users.id） |
| action | VARCHAR(100) | 操作类型，采用 `{domain}.{operation}` 命名空间格式 |
| resource_type | VARCHAR(50) | 资源类型 |
| resource_id | CHAR(36) | 资源 ID |
| old_value | JSON | 修改前的值 |
| new_value | JSON | 修改后的值 |
| ip_address | VARCHAR(45) | 操作者 IP |
| created_at | TIMESTAMP | 操作时间 |

> **注意**: 表中没有 `actor_email` / `actor_display_name` / `user_agent` 字段。如需获取操作者邮箱或显示名，需 JOIN `users` 表。

### action 类型

**注意**: action 值采用 `{domain}.{operation}` 命名空间格式，而非简单动词。

| 值 | 说明 |
|----|------|
| tenant.create | 创建租户 |
| tenant.update | 更新租户 |
| tenant.delete | 删除租户 |
| user.create | 创建用户 |
| user.update | 更新用户 |
| user.delete | 删除用户 |
| user.add_to_tenant | 用户加入租户 |
| user.remove_from_tenant | 用户移出租户 |
| user.update_role_in_tenant | 更新用户租户角色 |
| user.mfa.enable | 启用 MFA |
| user.mfa.disable | 禁用 MFA |
| role.create | 创建角色 |
| role.update | 更新角色 |
| role.delete | 删除角色 |
| role.assign_permission | 角色分配权限 |
| role.remove_permission | 角色移除权限 |
| rbac.assign_roles | 分配角色 |
| rbac.unassign_role | 取消角色分配 |
| permission.create | 创建权限 |
| permission.delete | 删除权限 |
| service.create | 创建服务 |
| service.update | 更新服务 |
| service.delete | 删除服务 |
| hosted_login.password | 密码登录 |
| hosted_login.logout | 登出 |
| hosted_login.mfa_challenge | MFA 验证 |
| token_exchange.rest.succeeded | Token 交换成功 |
| invitation.created | 创建邀请 |
| invitation.accepted | 接受邀请 |

### resource_type 类型
| 值 | 说明 |
|----|------|
| tenant | 租户 |
| user | 用户 |
| service | 服务 |
| role | 角色 |
| permission | 权限 |
| webhook | Webhook |
| invitation | 邀请 |
| identity_provider | 身份提供商 |
| system_settings | 系统设置 |

### 排错指南

| 现象 | 原因 | 解决方案 |
|------|------|----------|
| 登录超时 (30s+) / "fetch failed" | auth9-core 内存压力导致 V8 heap limit 错误 | 运行 `./scripts/reset-docker.sh` 重启环境，确认 `docker logs auth9-core` 无 OOM 错误后重试 |
| 审计日志 action 为 `hosted_login.password` 而非 `create` | action 采用 `{domain}.{operation}` 命名空间格式 | 参考上方 action 类型表，使用完整的命名空间值进行筛选和断言 |
| 刚重置后审计日志条目很少 | 环境重置清空数据，只有重置后的操作（登录、Token交换）会产生审计日志 | 先执行若干管理操作（创建租户/用户/角色等），再验证审计日志 |

---

## 场景 1：审计日志入口可见性与列表查看

### 初始状态
- 管理员已登录
- 系统有审计日志记录

### 目的
验证审计日志列表正确显示

### 测试操作流程
1. 进入「审计日志」页面

### 预期结果
- 显示审计日志表格，包含列：
  - Action（操作类型）
  - Resource（资源类型:资源ID）
  - Actor（操作者邮箱或名称）
  - Time（操作时间）
- 日志按时间倒序排列
- 显示总日志数和分页信息

### 预期数据状态
```sql
SELECT action, resource_type, resource_id, actor_email, created_at
FROM audit_logs
ORDER BY created_at DESC
LIMIT 50;
```

---

## 场景 2：验证操作生成审计日志

### 初始状态
- 管理员已登录
- 准备执行某个操作（如创建租户）

### 目的
验证管理操作自动生成审计日志

### 测试操作流程
1. 创建一个新租户：名称 `Audit Test Tenant`
2. 进入「审计日志」页面
3. 查找最新的日志记录

### 预期结果
- 出现新的审计日志记录
- action = `tenant.create`（注意：使用 `{domain}.{operation}` 命名空间格式，不是简写 `create`）
- resource_type = `tenant`
- resource_id = 新创建的租户 ID
- actor_id = 当前管理员的 user.id（如需邮箱请 JOIN users 表）
- new_value 包含创建的数据

### 预期数据状态
```sql
SELECT al.action, al.resource_type, al.resource_id, u.email AS actor_email, al.new_value, al.created_at
FROM audit_logs al
LEFT JOIN users u ON u.id = al.actor_id
WHERE al.resource_type = 'tenant' AND al.action = 'tenant.create'
ORDER BY al.created_at DESC
LIMIT 1;
-- 预期: 存在刚创建的租户的日志
```

---

## 场景 3：验证更新操作记录新旧值

### 初始状态
- 管理员已登录
- 存在可编辑的资源

### 目的
验证更新操作记录修改前后的值

### 测试操作流程
1. 找到一个现有租户
2. 更新租户名称：`Original Name` → `Updated Name`
3. 进入「审计日志」页面
4. 查找该更新操作的日志

### 预期结果
- 出现 action = `update` 的日志
- old_value 包含原名称
- new_value 包含新名称
- 可以对比修改前后的差异

### 预期数据状态
```sql
SELECT action, resource_id, old_value, new_value
FROM audit_logs
WHERE resource_type = 'tenant' AND action = 'update'
ORDER BY created_at DESC
LIMIT 1;
-- 预期: old_value 和 new_value 显示变更内容
```

---

## 场景 4：分页浏览审计日志

### 初始状态
- 管理员已登录
- 系统有超过 50 条审计日志

### 目的
验证审计日志分页功能

### 测试操作流程
1. 进入「审计日志」页面
2. 确认显示分页信息
3. 点击下一页
4. 点击上一页

### 预期结果
- 显示「Page X of Y」
- 数据正确分页加载
- 分页导航正常工作

### 预期数据状态
```sql
SELECT COUNT(*) as total FROM audit_logs;
-- 预期: total > 50

SELECT * FROM audit_logs
ORDER BY created_at DESC
LIMIT 50 OFFSET 50;
-- 预期: 第二页的数据
```

---

## 场景 5：无审计日志时的显示

### 初始状态
- 管理员已登录
- 系统无审计日志（新安装或已清空）

### 目的
验证无日志时的空状态显示

### 测试操作流程
1. 进入「审计日志」页面

### 预期结果
- 显示「No audit logs found」提示
- 页面正常渲染，无报错
- 分页信息显示 0 条记录

### 预期数据状态
```sql
SELECT COUNT(*) FROM audit_logs;
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
| 1 | 查看审计日志列表 | ☐ | | | |
| 2 | 验证操作生成审计日志 | ☐ | | | |
| 3 | 验证更新操作记录新旧值 | ☐ | | | |
| 4 | 分页浏览审计日志 | ☐ | | | |
| 5 | 无审计日志时的显示 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
