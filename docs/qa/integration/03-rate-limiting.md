# 集成测试 - 限流（Rate Limiting）测试

**模块**: 集成测试
**测试范围**: API 限流机制、滑动窗口算法、429 响应
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 实现了基于 Redis 的滑动窗口限流中间件，支持三种限流维度：

- **按用户限流**：携带 Identity Token 时，按 `user_id` 维度限流
- **按租户+客户端限流**：携带 Tenant Access Token 时，按 `tenant_id` + `client_id` 维度限流
- **按 IP 地址限流**：未认证请求使用客户端 IP
- **按端点配置**：不同 API 端点可配置不同的限流规则
- **租户倍率**：特定租户可配置限流倍率（如 Premium 租户 2x）

> **注意**: 限流维度取决于认证状态，而非 `x-tenant-id` 头。已认证请求按用户/租户维度限流，未认证请求按 IP 限流。

### 限流响应
- 状态码：`429 Too Many Requests`
- 响应头：`Retry-After: <seconds>`
- 响应头：`X-RateLimit-Remaining: <count>`、`X-RateLimit-Reset: <timestamp>`
- 响应体：
  ```json
  {
    "error": "Rate limit exceeded",
    "code": "RATE_LIMITED",
    "retry_after": 30
  }
  ```

### 默认配置
- 默认：100 请求 / 60 秒
- 端点覆盖示例：`POST:/api/v1/auth/token` = 10 请求 / 60 秒

---

## 场景 1：正常请求包含限流响应头

### 初始状态
- 限流功能已启用
- Redis 连接正常

### 目的
验证正常请求的响应中包含限流相关头信息

### 测试操作流程
1. 发送一个正常请求：
   ```bash
   curl -v http://localhost:8080/api/v1/tenants
   ```
2. 检查响应头

### 预期结果
- 状态码 200（正常响应）
- 响应头包含 `X-RateLimit-Remaining`（如 99）
- 响应头包含 `X-RateLimit-Reset`（Unix 时间戳）

---

## 场景 2：超过限流阈值返回 429

### 初始状态
- 限流配置：某端点 10 请求 / 60 秒
- 尚未有任何请求

### 目的
验证超过限流阈值后请求被拒绝

### 测试操作流程
1. 使用脚本连续发送 11 个请求到限流端点：
   ```bash
   for i in $(seq 1 11); do
     curl -s -o /dev/null -w "%{http_code}\n" \
       -X POST http://localhost:8080/api/v1/auth/token \
       -H "Content-Type: application/json" \
       -d '{"grant_type": "client_credentials", "client_id": "test", "client_secret": "test"}'
   done
   ```
2. 记录每个请求的状态码

### 预期结果
- 前 10 个请求：正常返回（200 或 401 取决于凭证是否正确）
- 第 11 个请求：状态码 429
- 429 响应包含 `Retry-After` 头
- 429 响应体包含 `"code": "RATE_LIMITED"`

---

## 场景 3：限流窗口过后恢复

### 初始状态
- 某 IP 已达到限流上限
- 限流窗口为 60 秒

### 目的
验证限流窗口过后请求恢复正常

### 测试操作流程
1. 触发限流（收到 429 响应）
2. 等待 `Retry-After` 指定的秒数（或等待 60 秒）
3. 再次发送请求

### 预期结果
- 等待后请求恢复正常（不再返回 429）
- `X-RateLimit-Remaining` 重置

---

## 场景 4：不同 IP 的限流独立（未认证请求）

### 初始状态
- 限流功能已启用
- 准备两个不同的来源 IP
- **必须使用未认证请求**（如 `/health` 端点），已认证请求按 user_id/tenant_id 限流而非 IP

### 目的
验证未认证请求中，不同 IP 地址的限流计数器独立

### 测试操作流程
1. 使用 IP-A（通过 `X-Forwarded-For` 模拟）发送**未认证**请求直到触发限流：
   ```bash
   curl -H "X-Forwarded-For: 10.0.0.1" http://localhost:8080/health
   ```
2. 使用 IP-B 发送请求：
   ```bash
   curl -H "X-Forwarded-For: 10.0.0.2" http://localhost:8080/health
   ```

### 预期结果
- IP-A 触发限流后，IP-B 的请求仍然正常
- 两个 IP 各自有独立的 `X-RateLimit-Remaining` 计数

### 预期数据状态
```bash
# Redis 中应有两个独立的限流 key
redis-cli KEYS "auth9:ratelimit:ip:10.0.0.1:*"
# 预期: 存在

redis-cli KEYS "auth9:ratelimit:ip:10.0.0.2:*"
# 预期: 存在
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| 已认证请求的 Redis key 为 `auth9:ratelimit:user:{user_id}:...` 而非 IP | 已认证请求按 user_id 维度限流（设计如此） | 使用未认证端点（如 `/health`）测试 IP 独立限流 |
| 不同 IP 的已认证请求共享限流计数 | 同一 user_id 的请求共享限流计数（设计如此） | 预期行为：同一用户不同 IP 共享限流 |

---

## 场景 5：Redis 不可用时的降级行为

### 初始状态
- 限流功能已启用
- Redis 连接中断

### 目的
验证 Redis 故障时限流中间件的降级行为（Fail Open）

### 测试操作流程
1. 停止 Redis 服务：
   ```bash
   docker stop auth9-redis
   ```
2. 发送 API 请求：
   ```bash
   curl -v http://localhost:8080/api/v1/tenants
   ```
3. 检查响应

### 预期结果
- 请求正常通过（Fail Open 策略）
- 不返回 429（降级放行）
- 响应中可能不包含 `X-RateLimit-Remaining` 头
- 应用日志中记录 Redis 连接错误

### 恢复操作
```bash
docker start auth9-redis
```

---

## 测试工具推荐

### 使用 k6 进行限流压测
```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  vus: 1,
  iterations: 15,
};

export default function() {
  let res = http.get('http://localhost:8080/api/v1/tenants', {
    headers: {
      'Authorization': 'Bearer YOUR_TOKEN',
    },
  });

  check(res, {
    'status is 200 or 429': (r) => r.status === 200 || r.status === 429,
    'has rate limit header': (r) => r.headers['X-Ratelimit-Remaining'] !== undefined || r.status === 429,
  });

  // No sleep - we want to hit rate limit
}
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 正常请求包含限流头 | ☐ | | | |
| 2 | 超过阈值返回 429 | ☐ | | | |
| 3 | 窗口过后恢复 | ☐ | | | |
| 4 | 不同 IP 独立限流 | ☐ | | | |
| 5 | Redis 不可用降级 | ☐ | | | |
