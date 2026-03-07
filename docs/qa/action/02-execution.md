# Action 执行测试

**模块**: Action 执行引擎
**测试范围**: Action 脚本执行、触发器集成、上下文修改
**场景数**: 4

---

## 前置条件

### 步骤 0: 验证用户租户成员身份

Post-login Actions 仅在用户有租户成员身份时执行。执行测试前必须验证：

```sql
-- 检查测试用户是否属于至少一个租户（必须 > 0）
SELECT COUNT(*) AS tenant_count
FROM tenant_users tu
WHERE tu.user_id = (SELECT id FROM users WHERE email = 'test@example.com');
-- 预期: > 0
-- 如果为 0，需先将用户添加到租户: POST /api/v1/tenants/{id}/users
```

> 如果 Service 没有 `tenant_id` 且用户没有任何租户成员身份，post-login Actions 将被静默跳过（不会报错，也不会有执行记录）。这是系统设计行为，不是 bug。

### 测试用户准备
```sql
-- 确保存在测试用户
SELECT id, email, display_name FROM users WHERE email = 'test@example.com';
-- 如不存在，通过注册流程创建

-- **重要**: 确保用户已加入至少一个租户
SELECT tu.id, tu.tenant_id, t.name
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = (SELECT id FROM users WHERE email = 'test@example.com');
-- 如果为空，需要先将用户添加到租户
```

### 测试 Action 准备
使用 API 或 Portal 创建以下 Actions：

1. **Simple Claims Action** (post-login)
```typescript
context.claims = context.claims || {};
context.claims.test = "success";
context;
```

2. **Conditional Action** (post-login)
```typescript
if (context.user.email.endsWith("@example.com")) {
  context.claims = context.claims || {};
  context.claims.domain_verified = true;
}
context;
```

3. **Error Action** (post-login)
```typescript
throw new Error("Test error");
```

---

## 场景 1：执行入口可见性与 Post-Login 触发器执行

### 初始状态
- 存在已启用的 post-login Action
- 用户未登录

### 目的
验证 post-login Action 在用户登录时自动执行

### 测试操作流程
1. 访问 Auth9 Portal 登录页：`http://localhost:3000/login`
2. 使用测试账号登录：`test@example.com` / `Test123!`
3. 登录成功后，捕获 Identity Token

#### 步骤 0b: 确认使用正确的登录流程

必须通过 Auth9 Portal 登录流程触发 post-login Action：

```
正确路径: Portal /login → Auth9 /api/v1/auth/authorize → Keycloak → callback → Action 执行
错误路径: 直接调用 Keycloak grant_type=password（会绕过 Action 执行链路）
```

> Keycloak 直连获取的 token 不会触发 Auth9 的 post-login Action，使用此方式测试会产生误报。

### 验证方式
```bash
# 解码 JWT Token（从浏览器 DevTools Application > Local Storage 获取）
TOKEN="<从浏览器获取的 token>"
echo $TOKEN | cut -d. -f2 | base64 -d | jq

# 或使用 jwt.io 在线解码
```

### 预期结果
- 登录成功
- Identity Token 的 claims 中包含 Action 添加的自定义字段：
  ```json
  {
    "sub": "...",
    "email": "test@example.com",
    "test": "success",  // ← Action 添加的 claim
    "exp": ...
  }
  ```

### 预期数据状态
```sql
-- 验证执行日志
SELECT action_id, success, duration_ms, error_message, executed_at
FROM action_executions
WHERE trigger_id = 'post-login'
  AND service_id = '{service_id}'
ORDER BY executed_at DESC
LIMIT 1;
-- 预期:
-- - success = true
-- - duration_ms < 500
-- - error_message IS NULL

-- 验证 Action 统计更新
SELECT execution_count, error_count, last_executed_at
FROM actions
WHERE id = '{action_id}';
-- 预期:
-- - execution_count 增加 1
-- - error_count 保持不变
-- - last_executed_at 更新为刚才的时间
```

---

## 场景 2：条件性 Claims 修改

### 初始状态
- 存在 Conditional Action (见上方准备)
- 用户邮箱为 `test@example.com`

### 目的
验证 Action 可以基于条件修改 claims

### 测试操作流程
1. 使用 `test@example.com` 登录
2. 验证 Token 包含 `domain_verified: true`
3. 使用非 `@example.com` 邮箱登录
4. 验证 Token **不包含** `domain_verified` 字段

### 预期结果
- `@example.com` 用户：Token 包含 `domain_verified: true`
- 其他域用户：Token 不包含该字段
- 两次登录都成功（条件不满足时不抛错）

---

## 场景 3：Action 执行失败（严格模式）

### 初始状态
- 存在 Error Action（会抛出错误）
- Action 处于启用状态
- **Action 的 `strict_mode` 必须设置为 `true`**（默认为 `false`，不会阻止登录）

### 目的
验证 Action 失败时阻止认证流程（严格模式）

### 测试操作流程
1. 确保 Error Action 已启用，**且 `strict_mode` 已开启**（通过 Portal 编辑 Action 勾选 Strict Mode，或通过 API 设置 `"strict_mode": true`）
2. 尝试登录
3. 观察登录流程是否中断

### 预期结果（Portal UI）
- 登录失败
- 显示错误消息（可能是通用错误，不暴露脚本细节）
- 用户未能获取 Token

### 预期结果（Keycloak）
- 重定向回登录页，带有错误参数
- 或显示 Keycloak 错误页面

### 预期数据状态
```sql
-- 验证执行日志记录失败
SELECT success, error_message FROM action_executions
WHERE action_id = '{error_action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期:
-- - success = false
-- - error_message = 'Test error'

-- 验证 Action 错误计数增加
SELECT error_count, last_error FROM actions
WHERE id = '{error_action_id}';
-- 预期:
-- - error_count 增加 1
-- - last_error = 'Test error'

-- 验证没有创建 session（登录失败）
SELECT COUNT(*) FROM sessions
WHERE user_id = '{test_user_id}'
  AND created_at > NOW() - INTERVAL 1 MINUTE;
-- 预期: COUNT = 0（登录被阻止）
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| Action 失败但登录成功 | Error Action 的 `strict_mode` 为 `false`（默认值） | 编辑 Action，将 `strict_mode` 设为 `true` |
| Action 未执行 | Error Action 未启用或未绑定到正确的 service/trigger | 检查 Action 的 `enabled` 状态和 `trigger_id` 为 `post-login` |
| 登录正常但无执行日志 | Action 绑定的 service_id 与当前登录的 service 不匹配 | 确认 Action 属于登录使用的 Service |

---

## 场景 4：多个 Actions 顺序执行

### 初始状态
创建 3 个 post-login Actions，execution_order 分别为 0, 10, 20：

**Action A (order=0)**:
```typescript
context.claims = context.claims || {};
context.claims.order_a = "first";
context;
```

**Action B (order=10)**:
```typescript
context.claims = context.claims || {};
context.claims.order_b = "second";
context;
```

**Action C (order=20)**:
```typescript
context.claims = context.claims || {};
context.claims.order_c = "third";
context;
```

### 目的
验证多个 Actions 按 execution_order 顺序执行

### 测试操作流程
1. 登录
2. 检查 Token claims

### 预期结果
- Token 包含所有三个 claims：
  ```json
  {
    "order_a": "first",
    "order_b": "second",
    "order_c": "third"
  }
  ```
- 执行日志按顺序记录 3 条：Action A → B → C

### 预期数据状态
```sql
-- 验证执行顺序
SELECT action_id, executed_at FROM action_executions
WHERE trigger_id = 'post-login'
  AND service_id = '{service_id}'
  AND executed_at > NOW() - INTERVAL 1 MINUTE
ORDER BY executed_at ASC;
-- 预期: 3 条记录，按 A → B → C 顺序
```

---


---

## 说明

场景 5-8（超时/禁用/上下文/Service 隔离）已拆分到 `docs/qa/action/08-execution-advanced.md`。

---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Post-Login 触发器执行 | ☐ | | | |
| 2 | 条件性 Claims 修改 | ☐ | | | |
| 3 | Action 执行失败（严格模式） | ☐ | | | |
| 4 | 多个 Actions 顺序执行 | ☐ | | | |
