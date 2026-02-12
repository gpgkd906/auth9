# Action 安全测试

**模块**: Action 安全性
**测试范围**: 沙箱隔离、注入防护、租户隔离、资源限制、权限控制
**场景数**: 12

---

## 安全测试原则

1. **零信任原则**: 假设所有 Action 脚本都是潜在恶意的
2. **深度防御**: 多层安全机制（沙箱、超时、资源限制、权限检查）
3. **最小权限**: 脚本只能访问必要的上下文信息
4. **审计追踪**: 所有执行和失败都完整记录

---

## 场景 1：V8 沙箱隔离 - 文件系统访问阻止

### 攻击场景
恶意脚本尝试读取服务器文件系统

### 测试 Action 脚本
```typescript
// 尝试访问 Deno API
try {
  const content = Deno.readTextFile("/etc/passwd");
  context.claims = context.claims || {};
  context.claims.leaked_data = content;
} catch (e) {
  context.claims = context.claims || {};
  context.claims.blocked = true;
  context.claims.error = String(e);
}
context;
```

### 执行方式
1. 创建上述 Action（trigger: post-login）
2. 尝试登录
3. 检查日志和 Token

### 预期结果
- ✅ **Action 执行失败** 或脚本捕获 `ReferenceError: Deno is not defined`
- ✅ Token 中 **不包含** `leaked_data` 字段
- ✅ Token 包含 `blocked: true` 和 `error` 字段（证明 Deno 被删除）
- ✅ 执行日志记录错误

### 验证方法
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
# 解码 JWT Token
echo $TOKEN | cut -d. -f2 | base64 -d | jq
# 预期: 不包含 /etc/passwd 内容
```

### 预期数据状态
```sql
SELECT success, error_message FROM action_executions
WHERE action_id = '{action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: success = false 或 error_message 包含 "Deno is not defined"
```

---

## 场景 2：V8 沙箱隔离 - Node.js API 阻止

### 攻击场景
恶意脚本尝试使用 Node.js require() 访问文件系统

### 测试 Action 脚本
```typescript
// 尝试使用 Node.js API
try {
  const fs = require("fs");
  const content = fs.readFileSync("/etc/passwd", "utf-8");
  context.claims = context.claims || {};
  context.claims.leaked_data = content;
} catch (e) {
  context.claims = context.claims || {};
  context.claims.blocked = true;
}
context;
```

### 预期结果
- ✅ 脚本捕获 `ReferenceError: require is not defined`
- ✅ Token 包含 `blocked: true`
- ✅ Token **不包含** `leaked_data`

---

## 场景 3：V8 沙箱隔离 - 进程访问阻止

### 攻击场景
恶意脚本尝试访问 process 对象获取环境变量

### 测试 Action 脚本
```typescript
// 尝试访问 process.env
try {
  context.claims = context.claims || {};
  context.claims.env = process.env;
  context.claims.jwt_secret = process.env.JWT_SECRET;
} catch (e) {
  context.claims = context.claims || {};
  context.claims.blocked = true;
}
context;
```

### 预期结果
- ✅ 脚本捕获 `ReferenceError: process is not defined`
- ✅ Token **不包含** JWT_SECRET 或其他敏感环境变量

---

## 场景 4：资源耗尽攻击 - 无限循环

### 攻击场景
恶意脚本运行无限循环消耗 CPU

### 测试 Action 脚本
```typescript
// 无限循环
while (true) {
  const x = 1 + 1;
}
context;
```

### 执行方式
1. 创建上述 Action，设置 `timeout_ms = 1000`
2. 尝试登录
3. 记录开始和结束时间

### 预期结果
- ✅ Action 在 **约 1 秒**后被强制终止
- ✅ 执行日志记录 `timeout` 或 `execution exceeded` 错误
- ✅ 登录流程中断（严格模式）
- ✅ 用户 **无法** 获取 Token

### 性能验证
```bash
# 测试超时控制精度
START=$(date +%s)
# 触发登录
END=$(date +%s)
DURATION=$((END - START))
# 预期: DURATION ≈ 1-2 秒（1 秒超时 + 网络开销）
```

### 预期数据状态
```sql
SELECT duration_ms, error_message FROM action_executions
WHERE action_id = '{action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期:
-- - duration_ms ≈ 1000
-- - error_message 包含 "timeout" 或 "exceeded"
```

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

## 场景 6：租户隔离 - 跨租户数据访问

### 攻击场景
租户 A 的 Action 尝试访问租户 B 的数据

### 准备数据
```sql
-- 创建两个租户
INSERT INTO tenants (id, slug, name) VALUES
  ('tenant-a-id', 'tenant-a', 'Tenant A'),
  ('tenant-b-id', 'tenant-b', 'Tenant B');

-- 租户 A 创建 Action
-- 租户 B 创建用户
```

### 测试 Action 脚本（租户 A）
```typescript
// 尝试猜测租户 B 的用户 ID
const guessed_user_id = "tenant-b-user-id";

// 即使猜对 ID，ActionContext 也不应包含其他租户数据
context.claims = context.claims || {};
context.claims.attacked_user = guessed_user_id;

// 尝试修改 tenant_id（应该失败）
context.tenant.id = "tenant-b-id";

context;
```

### 预期结果
- ✅ ActionContext 只包含 **租户 A** 的数据
- ✅ 即使脚本修改 `context.tenant.id`，实际租户上下文 **不变**
- ✅ 生成的 Token 仍然绑定到租户 A
- ✅ 脚本 **无法** 调用跨租户查询（因为没有提供 Host Functions）

### 验证方法
```bash
# 解码 Token
echo $TOKEN | cut -d. -f2 | base64 -d | jq '.tenant_id'
# 预期: "tenant-a-id"（未被篡改）
```

### 预期数据状态
```sql
-- 验证执行日志的租户 ID
SELECT tenant_id FROM action_executions
WHERE action_id = '{tenant_a_action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: tenant_id = 'tenant-a-id'

-- 验证租户 B 的数据未被访问
SELECT COUNT(*) FROM action_executions
WHERE action_id IN (SELECT id FROM actions WHERE tenant_id = 'tenant-b-id')
  AND executed_at > NOW() - INTERVAL 1 MINUTE;
-- 预期: COUNT = 0（租户 B 的 Actions 未被触发）
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

## 场景 9：命令注入防护

### 攻击场景
假设未来 Action 支持调用外部命令（当前不支持，但测试防御性设计）

### 测试 Action 脚本
```typescript
// 尝试执行命令（应该失败）
try {
  // 假设有一个 exec() host function（实际不应该有）
  const result = exec("rm -rf /");
  context.claims = context.claims || {};
  context.claims.result = result;
} catch (e) {
  context.claims = context.claims || {};
  context.claims.blocked = true;
}
context;
```

### 预期结果
- ✅ `exec` 未定义，抛出 `ReferenceError`
- ✅ **设计原则**: 永远不提供 shell 命令执行的 Host Functions

---

## 场景 10：权限提升攻击

### 攻击场景
普通用户尝试通过 Action 脚本提升自己的权限

### 测试 Action 脚本
```typescript
// 尝试将自己提升为管理员
context.claims = context.claims || {};
context.claims.roles = ["admin", "superuser"];
context.claims.permissions = ["*"];
context.user.is_admin = true;  // 尝试修改用户属性
context;
```

### 预期结果
- ✅ Claims 中可以写入任意值
- ✅ **关键**: auth9-core 在后续授权检查时 **必须**:
  - 从数据库重新加载用户角色/权限（不信任 Token claims）
  - 或使用签名的 claims（如 JWT 中的标准 claims）
  - Token claims 仅用于应用层逻辑，**不用于** 核心权限判断

### 验证方法
1. 以普通用户登录
2. 执行上述 Action
3. 尝试访问管理员功能（如删除租户）
4. **预期**: 403 Forbidden（权限检查失败）

### 代码审查重点
```rust
// 检查 auth9-core 的授权中间件
// 确保不直接使用 Token claims 进行权限判断
// ✅ 从数据库查询: rbac_service.get_user_permissions(user_id)
// ❌ 直接读取: token.claims.get("permissions")
```

---

## 场景 11：Token 伪造攻击

### 攻击场景
攻击者尝试伪造包含恶意 claims 的 Token

### 攻击步骤
1. 获取一个有效的 Identity Token
2. 修改 JWT payload（添加 `admin: true`）
3. 使用错误的密钥重新签名
4. 尝试使用伪造的 Token 访问 API

### 预期结果
- ✅ JWT 验证失败（签名不匹配）
- ✅ API 返回 401 Unauthorized
- ✅ 日志记录 `invalid signature` 错误

### 测试方法
```bash
# 1. 获取有效 Token
VALID_TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 2. 手动构造伪造 Token（使用 jwt.io 或 Python jwt 库）
FORGED_TOKEN="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiYWRtaW4iOnRydWV9.FAKE_SIGNATURE"

# 3. 尝试访问 API
curl -X GET http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $FORGED_TOKEN"

# 预期: HTTP 401
```

---

## 场景 12：Action 脚本注入攻击

### 攻击场景
攻击者尝试在创建 Action 时注入恶意代码到其他 Actions

### 测试 Action 脚本
```typescript
// 尝试注入代码到全局作用域
globalThis.maliciousFunction = function() {
  // 恶意逻辑
};

// 尝试污染 context 原型
Object.prototype.hacked = true;

context;
```

### 预期结果
- ✅ 每个 Action 在 **独立的 V8 isolate** 中执行
- ✅ 全局对象修改 **不影响** 其他 Actions
- ✅ 下一个 Action 执行时，`globalThis.maliciousFunction` **不存在**
- ✅ `Object.prototype.hacked` **不存在**

### 验证方法
1. 创建上述恶意 Action（执行顺序 = 0）
2. 创建一个正常 Action（执行顺序 = 10）：
   ```typescript
   // 检查是否被污染
   context.claims = context.claims || {};
   context.claims.is_hacked = typeof globalThis.maliciousFunction !== "undefined";
   context.claims.prototype_hacked = Object.prototype.hasOwnProperty("hacked");
   context;
   ```
3. 登录并解码 Token
4. **预期**: `is_hacked: false`, `prototype_hacked: false`

---

## 权限控制测试

### 1. 未授权用户创建 Action

**测试方法**:
```bash
# 使用普通用户 Token（无 action:write 权限）
USER_TOKEN=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = 'user@example.com';")
# 生成 user token（需要实现）

curl -X POST http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Unauthorized Action",
    "trigger_id": "post-login",
    "script": "context;"
  }'
```

**预期**: HTTP 403 Forbidden

### 2. 跨租户 Action 访问

**测试方法**:
```bash
# 租户 A 的管理员尝试访问租户 B 的 Action
curl -X GET http://localhost:8080/api/v1/tenants/{tenant_b_id}/actions/{action_id} \
  -H "Authorization: Bearer $TENANT_A_ADMIN_TOKEN"
```

**预期**: HTTP 403 Forbidden 或 404 Not Found

### 3. 删除他人的 Action

**测试方法**:
```bash
# 普通用户尝试删除管理员的 Action
curl -X DELETE http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{admin_action_id} \
  -H "Authorization: Bearer $USER_TOKEN"
```

**预期**: HTTP 403 Forbidden

---

## 速率限制测试（Rate Limiting）

### 1. Action 创建速率限制

**测试方法**:
```bash
# 短时间内创建大量 Actions
for i in {1..50}; do
  curl -X POST http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"Action $i\", \"trigger_id\": \"post-login\", \"script\": \"context;\"}"
done
```

**预期**:
- 前 N 个请求成功（如前 20 个）
- 后续请求返回 HTTP 429 Too Many Requests
- 响应包含 `Retry-After` 头

### 2. Action 执行频率限制（如果实现）

**测试方法**:
- 短时间内多次触发同一 Action（如连续登录 100 次）
- **预期**: 如果有执行频率限制，超出后应该降级或排队

---

## 日志审计

### 1. 敏感操作日志记录

**验证内容**:
```sql
-- 验证所有 Action 执行都有日志
SELECT COUNT(*) FROM action_executions
WHERE action_id = '{action_id}';
-- 预期: 与实际执行次数一致

-- 验证失败日志记录错误信息
SELECT error_message FROM action_executions
WHERE action_id = '{action_id}' AND success = false;
-- 预期: error_message 不为空
```

### 2. 敏感信息脱敏

**验证内容**:
```sql
-- 检查日志中不应包含明文密码或密钥
SELECT error_message FROM action_executions
WHERE error_message LIKE '%password%'
   OR error_message LIKE '%secret%'
   OR error_message LIKE '%JWT_SECRET%';
-- 预期: 无结果或已脱敏
```

---

## 回归测试检查清单

### 沙箱隔离
- [ ] Deno API 被禁用（无文件系统访问）
- [ ] Node.js require() 被禁用
- [ ] process 对象被禁用
- [ ] globalThis 污染被隔离（isolate 间独立）

### 资源限制
- [ ] 无限循环被超时控制（1-3 秒）
- [ ] 大内存分配被堆限制（100MB）
- [ ] CPU 密集型脚本被超时控制

### 租户隔离
- [ ] Action 只能访问所属租户数据
- [ ] ActionContext 不包含其他租户信息
- [ ] 跨租户 Action 访问被阻止

### 注入防护
- [ ] SQL 注入：后端使用参数化查询
- [ ] XSS：前端转义或使用 React 自动转义
- [ ] 命令注入：不提供 shell 执行 Host Functions

### 权限控制
- [ ] 未授权用户无法创建/修改 Actions
- [ ] 跨租户访问被拒绝
- [ ] Token 签名验证生效

### 审计日志
- [ ] 所有执行都有日志记录
- [ ] 失败日志包含错误信息
- [ ] 敏感信息已脱敏

---

## 安全建议

### 1. 定期安全审计
- 每季度进行渗透测试
- 代码审查关注权限检查和输入验证

### 2. 最小权限原则
- Action 脚本只能访问必要的上下文
- 不提供数据库查询、文件系统、网络请求等 Host Functions（除非业务必需）

### 3. 监控告警
- 监控异常高的 Action 失败率
- 监控超时频率
- 监控资源使用异常

### 4. 安全更新
- 及时更新 Deno Core 版本
- 关注 V8 安全公告

### 5. 输入验证
- Action 脚本创建时进行 TypeScript 编译验证
- 限制脚本大小（如 100KB）
- 限制 Action 数量（如每租户最多 50 个）
