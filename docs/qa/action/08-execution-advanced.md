# Action 执行测试（进阶）

**模块**: Action 执行引擎
**测试范围**: 超时控制、禁用行为、上下文验证、Service 隔离
**场景数**: 4

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


---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Action 超时控制 | ☐ | | | |
| 2 | 禁用 Action 不执行 | ☐ | | | |
| 3 | Action 上下文信息验证 | ☐ | | | |
| 4 | Service 隔离 | ☐ | | | |
