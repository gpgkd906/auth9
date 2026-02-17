# 集成测试 - 安全加固第二轮（事务性级联删除 & Webhook Secret 强制校验）

**模块**: 集成测试
**测试范围**: P0-2 用户/租户删除事务原子性 + P0-3 生产环境 Keycloak Webhook Secret 强制校验
**场景数**: 5
**优先级**: 高

---

## 背景说明

### 事务性级联删除（P0-2）

用户删除和租户删除涉及多张关联表的级联清理。改进前各步骤独立执行，任一步失败会导致数据不一致。改进后使用数据库事务包装所有级联删除操作，失败时自动回滚。

- 用户删除：`UserService::delete()` — 清理 `user_tenant_roles`、`sessions`、`password_reset_tokens`、`linked_identities`、`passkeys`、`login_events`、`security_alerts`、`tenant_users`，最后删除 `users` 记录
- 租户删除：`TenantService::delete()` — 清理 `clients`、`role_permissions`、`user_tenant_roles`、`roles`、`permissions`、`webhooks`、`invitations`、`tenant_users`、`login_events`、`security_alerts`、`actions`、`services`，最后删除 `tenants` 记录

外部系统操作（Keycloak 用户删除、Webhook 通知）在事务 commit 之后执行，确保数据库一致性优先。

### Webhook Secret 强制校验（P0-3）

`POST /api/v1/keycloak/events` 端点接收 Keycloak 事件。未配置 `KEYCLOAK_WEBHOOK_SECRET` 时，签名验证被跳过，任何人可发送伪造事件。

改进后：
- **生产环境**：启动时校验 `KEYCLOAK_WEBHOOK_SECRET` 是否已配置，未配置则拒绝启动
- **非生产环境**：打印 warn 日志，允许启动

---

## 场景 1：用户删除 — 级联操作原子性验证

### 初始状态
- 租户 `{tenant_id}` 下存在用户 `{user_id}`
- 该用户拥有角色分配（`user_tenant_roles`）、登录事件（`login_events`）、会话（`sessions`）等关联数据

### 目的
验证用户删除操作在事务中完成，所有关联数据同时清除，无孤儿记录

### 测试操作流程
1. 确认用户关联数据存在：
   ```sql
   SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
   SELECT COUNT(*) FROM user_tenant_roles WHERE tenant_user_id IN (SELECT id FROM tenant_users WHERE user_id = '{user_id}');
   SELECT COUNT(*) FROM login_events WHERE user_id = '{user_id}';
   SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}';
   ```
2. 通过 Portal「用户管理」页面删除用户，或调用 API：
   ```bash
   curl -s -w "\n%{http_code}" -X DELETE \
     -H "Authorization: Bearer {admin_token}" \
     http://localhost:8080/api/v1/tenants/{tenant_id}/users/{user_id}
   ```
3. 验证所有关联数据已清除

### 预期结果
- HTTP 状态码：`204 No Content`
- 用户记录从 `users` 表删除

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM user_tenant_roles WHERE tenant_user_id IN (SELECT id FROM tenant_users WHERE user_id = '{user_id}');
-- 预期: 0

SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM login_events WHERE user_id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM security_alerts WHERE user_id = '{user_id}';
-- 预期: 0
```

---

## 场景 2：租户删除 — 级联操作原子性验证

### 初始状态
- 租户 `{tenant_id}` 拥有服务、角色、权限、用户关联、Webhook 等完整数据
- 至少 1 个服务下有客户端（`clients`）

### 目的
验证租户删除操作在事务中完成，所有关联数据（包括深层嵌套如 services → clients → roles → permissions）同时清除

### 测试操作流程
1. 确认租户关联数据存在：
   ```sql
   SELECT COUNT(*) FROM services WHERE tenant_id = '{tenant_id}';
   SELECT COUNT(*) FROM clients WHERE service_id IN (SELECT id FROM services WHERE tenant_id = '{tenant_id}');
   SELECT COUNT(*) FROM roles WHERE service_id IN (SELECT id FROM services WHERE tenant_id = '{tenant_id}');
   SELECT COUNT(*) FROM tenant_users WHERE tenant_id = '{tenant_id}';
   SELECT COUNT(*) FROM webhooks WHERE tenant_id = '{tenant_id}';
   ```
2. 通过 Portal「租户管理」页面删除租户，或调用 API：
   ```bash
   curl -s -w "\n%{http_code}" -X DELETE \
     -H "Authorization: Bearer {admin_token}" \
     http://localhost:8080/api/v1/tenants/{tenant_id}
   ```
3. 验证所有关联数据已清除

### 预期结果
- HTTP 状态码：`204 No Content`
- 租户记录及所有关联数据从数据库删除

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenants WHERE id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM services WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM clients WHERE service_id IN (SELECT id FROM services WHERE tenant_id = '{tenant_id}');
-- 预期: 0

SELECT COUNT(*) FROM roles WHERE service_id IN (SELECT id FROM services WHERE tenant_id = '{tenant_id}');
-- 预期: 0

SELECT COUNT(*) FROM permissions WHERE service_id IN (SELECT id FROM services WHERE tenant_id = '{tenant_id}');
-- 预期: 0

SELECT COUNT(*) FROM tenant_users WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM webhooks WHERE tenant_id = '{tenant_id}';
-- 预期: 0

SELECT COUNT(*) FROM invitations WHERE tenant_id = '{tenant_id}';
-- 预期: 0
```

---

## 场景 3：删除后外部系统同步验证

### 初始状态
- 租户 `{tenant_id}` 下存在用户 `{user_id}`，该用户在 Keycloak 中有对应账户
- 租户配置了至少 1 个 Webhook

### 目的
验证删除操作在数据库事务 commit 后正确执行 Keycloak 用户删除和 Webhook 通知

### 测试操作流程
1. 记录用户在 Keycloak 中的 ID（从 `users.keycloak_id` 获取）
2. 删除用户：
   ```bash
   curl -s -w "\n%{http_code}" -X DELETE \
     -H "Authorization: Bearer {admin_token}" \
     http://localhost:8080/api/v1/tenants/{tenant_id}/users/{user_id}
   ```
3. 检查 Keycloak 中用户是否已删除：
   ```bash
   # Keycloak Admin API
   curl -s -w "\n%{http_code}" \
     -H "Authorization: Bearer {keycloak_admin_token}" \
     http://localhost:8081/admin/realms/auth9/users/{keycloak_user_id}
   ```
4. 检查 Webhook 投递记录（如配置了 Webhook）

### 预期结果
- 用户从数据库和 Keycloak 中均已删除
- Keycloak 查询返回 `404 Not Found`
- Webhook 端点收到 `user.deleted` 事件

---

## 场景 4：生产环境未配置 Webhook Secret — 启动失败

### 初始状态
- 环境变量 `AUTH9_ENV=production`（或等效生产标识）
- 未设置 `KEYCLOAK_WEBHOOK_SECRET` 环境变量

### 目的
验证生产环境下未配置 Webhook Secret 时，auth9-core 拒绝启动

### 测试操作流程
1. 设置生产环境标识并移除 Webhook Secret：
   ```bash
   export AUTH9_ENV=production
   unset KEYCLOAK_WEBHOOK_SECRET
   ```
2. 尝试启动 auth9-core：
   ```bash
   cd auth9-core && cargo run
   ```

### 预期结果
- 进程以非零退出码终止
- 错误信息包含：`Keycloak webhook secret is not configured (KEYCLOAK_WEBHOOK_SECRET) in production`
- 明确提示：`without it, anyone can send spoofed events to POST /api/v1/keycloak/events`

---

## 场景 5：非生产环境未配置 Webhook Secret — 警告但正常启动

### 初始状态
- 环境变量 `AUTH9_ENV=development`（或未设置，默认非生产）
- 未设置 `KEYCLOAK_WEBHOOK_SECRET` 环境变量

### 目的
验证非生产环境下未配置 Webhook Secret 时，auth9-core 正常启动但输出警告

### 测试操作流程
1. 确保非生产环境：
   ```bash
   export AUTH9_ENV=development
   unset KEYCLOAK_WEBHOOK_SECRET
   ```
2. 启动 auth9-core：
   ```bash
   cd auth9-core && cargo run
   ```
3. 检查启动日志

### 预期结果
- 进程正常启动，监听端口
- 日志中出现 `warn` 级别消息：`Keycloak webhook secret is not configured (KEYCLOAK_WEBHOOK_SECRET); webhook signature verification is disabled`
- 服务正常响应请求

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 用户删除 — 级联操作原子性验证 | ☐ | | | |
| 2 | 租户删除 — 级联操作原子性验证 | ☐ | | | |
| 3 | 删除后外部系统同步验证 | ☐ | | | |
| 4 | 生产环境未配置 Webhook Secret — 启动失败 | ☐ | | | |
| 5 | 非生产环境未配置 Webhook Secret — 警告但正常启动 | ☐ | | | |
