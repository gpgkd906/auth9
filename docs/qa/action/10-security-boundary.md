# Action 安全测试（边界与隔离）

**模块**: Action 安全性
**测试范围**: 资源限制、Service 隔离、注入防护
**场景数**: 4

---
## 场景 5：资源耗尽攻击 - 大内存分配

### 攻击场景
恶意脚本尝试分配大量内存导致 OOM

### 测试 Action 脚本
```typescript
// 尝试分配大量内存
const arr = [];
for (let i = 0; i < 100000000; i++) {
  arr.push(new Array(1000).fill("x"));
}
context.claims = context.claims || {};
context.claims.allocated = arr.length;
context;
```

### 预期结果
- ✅ V8 堆限制触发（100MB）
- ✅ Action 执行失败，记录 `out of memory` 或 `heap limit` 错误
- ✅ **不影响** auth9-core 主进程（isolate 隔离）
- ✅ 用户无法登录

### 预期数据状态
```sql
SELECT error_message FROM action_executions
WHERE action_id = '{action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: error_message 包含 "memory" 或 "heap"
```

---

## 场景 6：Service 隔离 - 跨 Service 数据访问

### 攻击场景
Service A 的 Action 尝试访问 Service B 的数据

### 准备数据
```sql
-- 创建两个 Service
INSERT INTO services (id, slug, name) VALUES
  ('service-a-id', 'service-a', 'Service A'),
  ('service-b-id', 'service-b', 'Service B');

-- Service A 创建 Action
-- Service B 创建用户
```

### 测试 Action 脚本（Service A）
```typescript
// 尝试猜测 Service B 的用户 ID
const guessed_user_id = "service-b-user-id";

// 即使猜对 ID，ActionContext 也不应包含其他 Service 数据
context.claims = context.claims || {};
context.claims.attacked_user = guessed_user_id;

// 尝试修改 service_id（应该失败）
context.tenant.id = "service-b-id";

context;
```

### 预期结果
- ✅ ActionContext 只包含 **Service A** 的数据
- ✅ 即使脚本修改 `context.tenant.id`，实际 Service 上下文 **不变**
- ✅ 生成的 Token 仍然绑定到 Service A
- ✅ 脚本 **无法** 调用跨 Service 查询（因为没有提供 Host Functions）

### 验证方法
```bash
# 解码 Token
echo $TOKEN | cut -d. -f2 | base64 -d | jq '.service_id'
# 预期: "service-a-id"（未被篡改）
```

### 预期数据状态
```sql
-- 验证执行日志的 Service ID
SELECT service_id FROM action_executions
WHERE action_id = '{service_a_action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: service_id = 'service-a-id'

-- 验证 Service B 的数据未被访问
SELECT COUNT(*) FROM action_executions
WHERE action_id IN (SELECT id FROM actions WHERE service_id = 'service-b-id')
  AND executed_at > NOW() - INTERVAL 1 MINUTE;
-- 预期: COUNT = 0（Service B 的 Actions 未被触发）
```

---

## 场景 7：SQL 注入防护

### 攻击场景
恶意脚本尝试通过用户输入注入 SQL（即使没有 DB Host Functions，也测试输入处理）

### 测试 Action 脚本
```typescript
// 尝试在 claims 中注入 SQL
const malicious_email = "'; DROP TABLE users; --";
context.claims = context.claims || {};
context.claims.email = malicious_email;
context.claims.search = "admin' OR '1'='1";
context;
```

### 预期结果
- ✅ Claims 中可以包含任意字符串（因为是 JSON）
- ✅ **关键**: auth9-core 在后续使用这些 claims 时 **必须** 使用参数化查询
- ✅ 数据库 **不执行** SQL 注入

### 验证方法（核心代码审查）
```rust
// 检查 auth9-core 中所有使用 claims 的代码
// 确保使用 sqlx 的参数绑定，而非字符串拼接
// 例如:
// ✅ sqlx::query!("SELECT * FROM users WHERE email = ?", email)
// ❌ format!("SELECT * FROM users WHERE email = '{}'", email)
```

**注意**: 此场景主要是代码审查，而非运行时测试。

---

## 场景 8：XSS 防护（Claims 注入）

### 攻击场景
恶意脚本在 claims 中注入 JavaScript 代码，期望在前端执行

### 测试 Action 脚本
```typescript
// 注入 XSS payload
context.claims = context.claims || {};
context.claims.display_name = "<script>alert('XSS')</script>";
context.claims.bio = "<img src=x onerror=alert('XSS')>";
context;
```

### 预期结果
- ✅ Claims 成功写入 Token（JSON 字符串）
- ✅ **关键**: auth9-portal 在显示时 **必须** 转义或使用 React 的自动转义
- ✅ 浏览器 **不执行** JavaScript

### 验证方法（前端测试）
1. 登录并获取包含 XSS payload 的 Token
2. 在 auth9-portal 中查看用户资料
3. 检查 DevTools Console：**不应该** 出现 alert 弹窗
4. 检查页面 DOM：`<script>` 标签应该被转义为 `&lt;script&gt;`

**React 默认转义**: React 的 `{variable}` 自动转义 HTML，但需验证所有 `dangerouslySetInnerHTML` 使用。

---


---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 资源耗尽攻击 - 大内存分配 | ☐ | | | |
| 2 | Service 隔离 - 跨 Service 数据访问 | ☐ | | | |
| 3 | SQL 注入防护 | ☐ | | | |
| 4 | XSS 防护（Claims 注入） | ☐ | | | |
