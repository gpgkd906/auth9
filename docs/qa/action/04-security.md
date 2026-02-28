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
- ✅ 脚本的 try-catch 捕获 `TypeError: Deno.readTextFile is not a function`（Deno 对象存在但 readTextFile 不是已注册的 op）
- ✅ Token 中 **不包含** `leaked_data` 字段
- ✅ Token 包含 `blocked: true` 和 `error` 字段（证明文件系统 API 不可用）
- ✅ **Action 执行成功**（`success=1`），因为 try-catch 优雅地处理了错误

> **注意**: `success=1` 是正确行为。脚本的 try-catch 捕获了错误并设置了 `blocked: true`，
> 然后正常返回 context。验证重点是 **Token 中的 claims**，而非 action_executions 表的 success 字段。
> 只有未被 try-catch 捕获的异常才会导致 `success=0`。

### 验证方法
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
# 解码 JWT Token
echo $TOKEN | cut -d. -f2 | base64 -d | jq
# 预期: 包含 blocked=true, 不包含 leaked_data
```

### 预期数据状态
```sql
SELECT success, error_message FROM action_executions
WHERE action_id = '{action_id}'
ORDER BY executed_at DESC LIMIT 1;
-- 预期: success = 1, error_message = NULL
-- （脚本通过 try-catch 处理了错误，没有未捕获异常）
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
- ✅ 脚本的 try-catch 捕获 `ReferenceError: require is not defined`
- ✅ Token 包含 `blocked: true`
- ✅ Token **不包含** `leaked_data`
- ✅ **Action 执行成功**（`success=1`），因为 try-catch 优雅地处理了错误

> **注意**: 与场景 1 相同，`success=1` 表示脚本正常完成，验证重点是 Token claims。

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
- ✅ 脚本的 try-catch 捕获 `ReferenceError: process is not defined`
- ✅ Token **不包含** JWT_SECRET 或其他敏感环境变量
- ✅ Token 包含 `blocked: true`
- ✅ **Action 执行成功**（`success=1`），因为 try-catch 优雅地处理了错误

> **注意**: 与场景 1 相同，`success=1` 表示脚本正常完成，验证重点是 Token claims。

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
- ✅ Action 在 **约 1 秒**后被强制终止（`success=0`, `error_message` 包含 "timeout"）
- ✅ 执行日志记录 `timeout` 或 `execution exceeded` 错误
- ✅ 若 `strict_mode=1`：登录流程中断，用户 **无法** 获取 Token
- ✅ 若 `strict_mode=0`（默认）：登录仍然成功，但 Action 执行记录为失败

> **注意**: 必须在创建 Action 时设置 `strict_mode=1` 才能阻断登录流程。
> 默认 `strict_mode=0` 时，Action 失败不影响登录，这是设计行为。

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

## 故障排除（安全测试常见误判）

| 症状 | 原因 | 正确判断 |
|------|------|----------|
| `success=1` 但认为 API 未被阻止 | 脚本使用 try-catch 处理了错误 | 检查 Token claims 中是否有 `blocked: true`，而非 success 字段 |
| `error_message=NULL` | 脚本完成运行没有未捕获异常 | 正常行为，try-catch 捕获的错误不记录在 error_message |
| `duration_ms=0` | 脚本执行极快（try-catch 跳转） | 正常行为，错误在执行初期就被捕获 |
| `strict_mode=0` 时登录未阻断 | Action 默认非严格模式 | 需创建 Action 时显式设置 `strict_mode=1` |
| `Deno` 对象存在但 API 不可用 | deno_core 提供 Deno 对象，但仅注册 3 个安全 op | V8 沙箱正常工作，只有 fetch/setTimeout/console 可用 |

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | V8 沙箱隔离 - 文件系统访问阻止 | ☐ | | | |
| 2 | V8 沙箱隔离 - Node.js API 阻止 | ☐ | | | |
| 3 | V8 沙箱隔离 - 进程访问阻止 | ☐ | | | |
| 4 | 资源耗尽攻击 - 无限循环 | ☐ | | | |
