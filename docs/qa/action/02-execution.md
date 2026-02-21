# Action 执行测试

**模块**: Action 执行引擎
**测试范围**: Action 脚本执行、触发器集成、上下文修改
**场景数**: 8

---

## 前置条件

### 测试用户准备
```sql
-- 确保存在测试用户
SELECT id, email, display_name FROM users WHERE email = 'test@example.com';
-- 如不存在，通过注册流程创建
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

## 场景 1：Post-Login 触发器执行

### 初始状态
- 存在已启用的 post-login Action
- 用户未登录

### 目的
验证 post-login Action 在用户登录时自动执行

### 测试操作流程
1. 访问 Auth9 Portal 登录页：`http://localhost:3000/login`
2. 使用测试账号登录：`test@example.com` / `Test123!`
3. 登录成功后，捕获 Identity Token

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

### 目的
验证 Action 失败时阻止认证流程（严格模式）

### 测试操作流程
1. 确保 Error Action 已启用
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

## 场景 5：Action 超时控制

### 初始状态
创建一个会超时的 Action（timeout_ms=1000，且 `strict_mode=true`）：
```typescript
// 故意延迟
const start = Date.now();
while (Date.now() - start < 2000) {
  // 阻塞 2 秒
}
context;
```

### 目的
验证 Action 超时保护机制

### 测试操作流程
1. 创建上述 Action，设置 `timeout_ms = 1000` 且 `strict_mode = true`
2. 尝试登录
3. 观察是否在 1 秒后超时中断

### 预期结果
- 登录失败或超时错误
- 执行日志记录超时错误
- 用户体验：等待约 1 秒后返回错误

> 说明：仅当 `strict_mode=true` 时，Action 超时/报错才会中断认证流程。  
> 若 `strict_mode=false`，超时会被记录，但登录流程继续。

### 预期数据状态
```sql
SELECT success, error_message, duration_ms FROM action_executions
WHERE action_id = '{timeout_action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期:
-- - success = false
-- - error_message 包含 "timeout" 或 "exceeded"
-- - duration_ms ≈ 1000
```

---

## 场景 6：禁用 Action 不执行

### 初始状态
- 存在 Action，但 enabled = false

### 目的
验证禁用的 Action 不会执行

### 测试操作流程
1. 禁用某个 post-login Action
2. 登录
3. 验证该 Action 未执行

### 预期结果
- 登录成功
- Token 不包含该 Action 添加的 claims
- 执行日志中没有该 Action 的新记录

### 预期数据状态
```sql
-- 验证最近无新的执行记录
SELECT executed_at FROM action_executions
WHERE action_id = '{disabled_action_id}'
  AND executed_at > NOW() - INTERVAL 1 MINUTE;
-- 预期: 无记录
```

---

## 场景 7：Action 上下文信息验证

### 初始状态
创建一个打印上下文的 Action：
```typescript
// 验证上下文结构
if (!context.user || !context.tenant || !context.request) {
  throw new Error("Context incomplete");
}

if (!context.user.email || !context.user.id) {
  throw new Error("User info missing");
}

// 添加确认 claim
context.claims = context.claims || {};
context.claims.context_validated = true;
context;
```

### 目的
验证传递给 Action 的上下文包含完整信息

### 测试操作流程
1. 创建上述 Action
2. 登录
3. 验证成功（说明上下文完整）

### 预期结果
- 登录成功
- Token 包含 `context_validated: true`
- 未抛出 "Context incomplete" 错误

### 预期数据状态
```sql
SELECT success FROM action_executions
WHERE action_id = '{context_action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: success = true
```

---

## 场景 8：Service 隔离

### 初始状态
- Service A 有 Action A
- Service B 有 Action B

### 目的
验证 Action 执行的 Service 隔离

### 测试操作流程
1. 以 Service A 用户身份登录
2. 验证仅执行 Service A 的 Actions
3. 以 Service B 用户身份登录
4. 验证仅执行 Service B 的 Actions

### 预期结果
- Service A 用户 Token 仅包含 Action A 的 claims
- Service B 用户 Token 仅包含 Action B 的 claims
- 执行日志中 service_id 正确匹配

### 预期数据状态
```sql
-- 验证 Service A 的执行记录
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_a_id}'
  AND action_id IN (SELECT id FROM actions WHERE service_id = '{service_a_id}');
-- 预期: 仅包含 Service A 的 Actions

-- 验证不会误执行其他 Service 的 Actions
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_a_id}'
  AND action_id IN (SELECT id FROM actions WHERE service_id = '{service_b_id}');
-- 预期: COUNT = 0
```

---

## 性能测试

### 1. 单个 Action 执行延迟
**目标**: < 20ms (P90)

**测试方法**:
```sql
SELECT AVG(duration_ms), MAX(duration_ms), MIN(duration_ms)
FROM action_executions
WHERE success = true
  AND executed_at > NOW() - INTERVAL 1 HOUR;
```

**预期**: AVG < 15ms, MAX < 50ms

### 2. 多个 Actions 累积延迟
**场景**: 5 个 Actions 同时启用

**测试方法**: 登录并测量总耗时

**预期**: 总延迟 < 100ms

### 3. 高并发执行
**测试方法**: 使用 `hey` 工具模拟 20 并发登录
```bash
# 需要使用实际的登录流程，这里简化为 token exchange
hey -n 100 -c 20 \
  -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/auth/login
```

**预期**:
- 所有 Actions 正确执行
- 无死锁或资源竞争
- P95 延迟 < 200ms

---

## 错误处理测试

### 1. 脚本编译错误
**Action 脚本**:
```typescript
// 语法错误
context.claims = ;
```

**预期**: 创建时或执行时捕获错误

### 2. 运行时类型错误
**Action 脚本**:
```typescript
context.claims.nonExistent.property = "value";
```

**预期**: 执行失败，记录错误日志

### 3. 内存溢出保护
**Action 脚本**:
```typescript
const arr = [];
for (let i = 0; i < 100000000; i++) {
  arr.push(new Array(1000));
}
context;
```

**预期**: V8 堆限制触发，Action 失败，不影响系统

### 4. 无限循环保护
**Action 脚本**:
```typescript
while (true) {
  const x = 1 + 1;
}
```

**预期**: 超时机制触发，在 timeout_ms 后终止

---

## 回归测试检查清单

- [ ] 所有 6 种触发器类型都能正常触发（目前仅实现 post-login）
- [ ] Action 失败时认证流程正确中断
- [ ] 多个 Actions 按顺序执行
- [ ] 禁用的 Actions 不执行
- [ ] 超时控制生效
- [ ] Service 隔离正确
- [ ] 执行日志完整记录
- [ ] 统计数据准确更新
- [ ] 性能指标达标
- [ ] 错误处理健壮
