# 会话与安全 - Token 黑名单故障安全

**模块**: 会话与安全
**测试范围**: Token 黑名单 Fail-Closed 策略（Redis 不可用时拒绝请求返回 503）
**场景数**: 4
**优先级**: 高

---

## 背景说明

`require_auth` 中间件在验证 JWT 后，会检查 Redis 中的 Token 黑名单（用于登出/强制下线后的 token 撤销）。

**改进前（Fail-Open）**：Redis 不可用时记录 warn 日志但放行请求，已撤销的 token 在 Redis 故障期间仍然有效。

**改进后（Fail-Closed）**：Redis 不可用时先进行 1 次快速重试；若仍失败，返回 `503 Service Unavailable`（而非 401，避免客户端清除合法 token）。

中间件代码位置：`auth9-core/src/middleware/require_auth.rs`

---

## 场景 1：Redis 正常 — 已撤销 Token 被拒绝（401）

### 初始状态
- 用户已登录，持有有效 Identity Token（含 `sid` session ID）
- 用户已执行登出操作，session ID 已加入 Redis 黑名单

### 目的
验证 Redis 正常时，已撤销的 Token 被正确拦截

### 测试操作流程
1. 使用有效凭证登录，获取 Identity Token
2. 调用登出接口撤销该 session
3. 使用已撤销的 Token 访问受保护端点：
   ```bash
   curl -s -w "\n%{http_code}" \
     -H "Authorization: Bearer {revoked_token}" \
     http://localhost:8080/api/v1/tenants
   ```

### 预期结果
- HTTP 状态码：`401 Unauthorized`
- 响应体包含：
  ```json
  {
    "error": "Token has been revoked",
    "code": "UNAUTHORIZED"
  }
  ```

### 预期数据状态
```sql
-- 确认 session 在 Redis 黑名单中
-- Redis CLI:
-- GET session:blacklist:{session_id}
-- 预期: 存在且未过期
```

---

## 场景 2：Redis 完全不可用 — 返回 503（Fail-Closed）

### 初始状态
- 用户持有有效 Identity Token（含 `sid`）
- Redis 服务被手动停止

### 目的
验证 Redis 不可用时，中间件执行 Fail-Closed 策略，返回 503 而非放行

### 测试操作流程
1. 使用有效凭证登录，获取 Identity Token
2. 停止 Redis 服务：
   ```bash
   docker stop auth9-redis
   ```
3. 使用有效 Token 访问受保护端点：
   ```bash
   curl -s -w "\n%{http_code}" \
     -H "Authorization: Bearer {valid_token}" \
     http://localhost:8080/api/v1/tenants
   ```
4. 测试完毕后恢复 Redis：
   ```bash
   docker start auth9-redis
   ```

### 预期结果
- HTTP 状态码：`503 Service Unavailable`
- 响应体包含：
  ```json
  {
    "error": "Authentication service temporarily unavailable",
    "code": "SERVICE_UNAVAILABLE"
  }
  ```
- 后端日志中出现 `error` 级别日志：`Token blacklist check failed after retry, rejecting request (fail-closed)`
- **不是 401**（客户端不应丢弃 Token）
- **不是 200**（不应放行未验证黑名单的请求）

---

## 场景 3：Redis 短暂抖动后恢复 — 重试成功放行

### 初始状态
- 用户持有有效 Identity Token（含 `sid`，未被撤销）
- Redis 服务短暂不可用后立即恢复

### 目的
验证中间件的 1 次快速重试机制能容忍短暂网络抖动

### 测试操作流程
1. 使用有效凭证登录，获取 Identity Token
2. 在并发负载下短暂重启 Redis（模拟抖动）：
   ```bash
   docker restart auth9-redis
   ```
3. 在 Redis 恢复后立即发送请求：
   ```bash
   curl -s -w "\n%{http_code}" \
     -H "Authorization: Bearer {valid_token}" \
     http://localhost:8080/api/v1/tenants
   ```

### 预期结果
- 如果重试时 Redis 已恢复：HTTP 状态码 `200 OK`，请求正常通过
- 如果重试时 Redis 仍未恢复：HTTP 状态码 `503 Service Unavailable`
- 无论哪种情况，**绝不返回 200 同时跳过黑名单检查**

---

## 场景 4：无 Cache 配置时 — 跳过黑名单检查（向后兼容）

### 初始状态
- auth9-core 启动时未配置 Redis（`cache` 为 `None`）
- 用户持有有效 Identity Token

### 目的
验证未配置 Cache 时中间件不执行黑名单检查，请求正常通过（保持向后兼容）

### 测试操作流程
1. 在不配置 Redis 的情况下启动 auth9-core（移除 `REDIS_URL` 环境变量）
2. 使用有效 Token 访问受保护端点：
   ```bash
   curl -s -w "\n%{http_code}" \
     -H "Authorization: Bearer {valid_token}" \
     http://localhost:8080/api/v1/tenants
   ```

### 预期结果
- HTTP 状态码：`200 OK`
- 请求正常通过（未配置 cache 时不执行黑名单检查）
- 无 error/warn 日志产生

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Redis 正常 — 已撤销 Token 被拒绝（401） | ☐ | | | |
| 2 | Redis 完全不可用 — 返回 503（Fail-Closed） | ☐ | | | |
| 3 | Redis 短暂抖动后恢复 — 重试成功放行 | ☐ | | | |
| 4 | 无 Cache 配置时 — 跳过黑名单检查 | ☐ | | | |
