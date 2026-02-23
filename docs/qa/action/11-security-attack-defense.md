# Action 安全测试（攻击与防护）

**模块**: Action 安全性
**测试范围**: 命令注入、权限提升、Token 伪造、脚本注入
**场景数**: 4

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
- ✅ Claims 中可以写入任意值（Action 脚本的设计用途）
- ✅ **关键安全保障 — Token Exchange 切断 claims 传播链**:
  - Action 修改的 claims **仅存在于 Identity Token** 中（post-login 阶段生成）
  - Token Exchange（`POST /api/v1/auth/tenant-token` 或 gRPC `ExchangeToken`）生成 TenantAccess Token 时，**从数据库重新加载** roles/permissions，**不传播** Identity Token 中的自定义 claims
  - 因此，即使 Action 脚本注入 `roles: ["admin"]`，TenantAccess Token 中的权限仍然来自数据库
- ✅ API handler 使用 `enforce()` 基于 TenantAccess Token 中的 DB-sourced roles/permissions 进行授权，这是安全的

### 验证方法
1. 以普通用户登录（触发 post-login Action，Identity Token 包含注入的 claims）
2. 进行 Token Exchange 获取 TenantAccess Token
3. 使用 TenantAccess Token 尝试访问管理员功能（如删除租户）
4. **预期**: 403 Forbidden（TenantAccess Token 的 roles/permissions 来自数据库，不受 Action claims 影响）

### 代码审查重点
```rust
// Token Exchange 安全链:
// 1. Identity Token 包含 Action 修改的 custom claims（仅用于应用层逻辑）
// 2. Token Exchange 从 DB 获取 roles/permissions → 生成 TenantAccess Token
// 3. API handler enforce() 检查 TenantAccess Token 中的 DB-sourced 权限
//
// ✅ grpc/token_exchange.rs: find_user_roles_in_tenant_for_service() 从 DB 查询
// ✅ api/auth.rs: create_tenant_access_token_with_session() 不传播 custom claims
// ✅ enforce() 检查的 roles/permissions 来自 TenantAccess Token（DB-sourced）
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

curl -X POST http://localhost:8080/api/v1/services/{service_id}/actions \
  -H "Authorization: Bearer $USER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Unauthorized Action",
    "trigger_id": "post-login",
    "script": "context;"
  }'
```

**预期**: HTTP 403 Forbidden

### 2. 跨 Service Action 访问

**测试方法**:
```bash
# Service A 的管理员尝试访问 Service B 的 Action
curl -X GET http://localhost:8080/api/v1/services/{service_b_id}/actions/{action_id} \
  -H "Authorization: Bearer $SERVICE_A_ADMIN_TOKEN"
```

**预期**: HTTP 403 Forbidden 或 404 Not Found

### 3. 删除他人的 Action

**测试方法**:
```bash
# 普通用户尝试删除管理员的 Action
curl -X DELETE http://localhost:8080/api/v1/services/{service_id}/actions/{admin_action_id} \
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
  curl -X POST http://localhost:8080/api/v1/services/{service_id}/actions \
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


---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 命令注入防护 | ☐ | | | |
| 2 | 权限提升攻击 | ☐ | | | |
| 3 | Token 伪造攻击 | ☐ | | | |
| 4 | Action 脚本注入攻击 | ☐ | | | |
