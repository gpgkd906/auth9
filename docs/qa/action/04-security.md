# Action 安全测试

**模块**: Action 安全性
**测试范围**: 沙箱隔离、注入防护、Service 隔离、资源限制、权限控制
**场景数**: 4

---

## 安全测试原则

1. **零信任原则**: 假设所有 Action 脚本都是潜在恶意的
2. **深度防御**: 多层安全机制（沙箱、超时、资源限制、权限检查）
3. **最小权限**: 脚本只能访问必要的上下文信息
4. **审计追踪**: 所有执行和失败都完整记录

## 前置条件

> **重要**: Post-login Action 仅在 `authorization_code` grant 流程中触发（即通过浏览器 Portal 登录）。
> `client_credentials` grant (M2M token) 不会触发 post-login Action，这是设计如此。
>
> 因此，所有安全测试 Action 必须创建在 **Portal 所使用的服务**（如 `Auth9 Admin Portal`）上，
> 然后通过浏览器登录来触发执行。不能通过 M2M API 调用来测试 post-login Action。

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| Action 创建成功但不触发 | Action 创建在非 Portal 服务上 | 将 Action 关联到 Portal 使用的服务 |
| M2M token 不含 custom claims | client_credentials 不触发 Action | 使用浏览器 Portal 登录触发 |
| Action 在其他服务上未触发 | 只有 authorization_code grant 触发 | 仅通过 Portal 登录测试 |

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


---

## 说明

场景 5-12 已拆分为：
- `docs/qa/action/10-security-boundary.md`（场景 5-8）
- `docs/qa/action/11-security-attack-defense.md`（场景 9-12）

---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | V8 沙箱隔离 - 文件系统访问阻止 | ☐ | | | |
| 2 | V8 沙箱隔离 - Node.js API 阻止 | ☐ | | | |
| 3 | V8 沙箱隔离 - 进程访问阻止 | ☐ | | | |
| 4 | 资源耗尽攻击 - 无限循环 | ☐ | | | |
