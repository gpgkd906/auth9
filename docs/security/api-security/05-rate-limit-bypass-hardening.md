# API 安全 - 限流绕过与 DoS 放大专项测试

**模块**: API 安全
**测试范围**: Trusted Header 绕过、路径高基数、Redis 故障降级滥用
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-API-05
**OWASP ASVS 5.0**: V4.4,V13.3,V16.2
**回归任务映射**: Backlog #3, #19, #20


---

## 去重说明

与 `api-security/03-rate-limiting.md` 的区别：
- `03-rate-limiting` 覆盖基础限流功能与通用 DoS 防护。
- 本文档只覆盖“可绕过/可放大”的缺陷复现路径：`x-tenant-id` 伪造、动态路径 key 膨胀、Redis 故障下滥用窗口。

---

## 场景 1：通过轮换 x-tenant-id 绕过单源限流

### 实现说明

> **当前实现**: auth9-core 的限流中间件 **不使用** `x-tenant-id` header 作为限流键。
> 限流键的提取逻辑（`rate_limit.rs:extract_key_from_verified_token`）：
> 1. 优先从 **已验证的 JWT** 中提取 `user_id` 或 `tenant_id + client_id`
> 2. 无有效 JWT 时回退到 **客户端 IP**（`x-forwarded-for` / `x-real-ip`）
>
> 因此，轮换 `x-tenant-id` header **不会**影响限流键——所有无认证请求共享同一 IP 桶。

### 前置条件
- 同一来源 IP
- 了解默认 IP 限流阈值（默认: 100 请求 / 60 秒，通过 `RATE_LIMIT_DEFAULT_REQUESTS` 配置）

### 攻击目标
验证限流是否错误信任 `x-tenant-id`，导致同一攻击者通过 header 轮换绕过。

### 攻击步骤
1. 在 60 秒内发送 **超过 IP 限流阈值**的请求，每次使用不同 `x-tenant-id`。
2. 记录状态码分布。
3. 确认 429 出现，证明 `x-tenant-id` 不影响限流。

### 预期安全行为
- `x-tenant-id` header 应被限流层完全忽略
- 限流主键应基于可信身份（认证主体/IP），而非客户端任意 header
- 当请求数超过 IP 阈值时应触发 429

### 验证方法
```bash
# 注意: 默认 IP 限流是 100 请求/60 秒，需发送 >100 请求才能验证 429
# 如果仅发送 40 请求，不会触发 429（因为 40 < 100），这是正常行为而非绕过
for i in $(seq 1 120); do
  code=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "x-tenant-id: fake-$i" \
    "http://localhost:8080/api/v1/tenants")
  echo "$i -> $code"
done
# 预期: 前 ~100 个返回 200/401, 之后返回 429
# 关键验证: x-tenant-id 轮换不影响限流（429 仍会出现）
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 40 个请求全部返回 200 | 40 < 100（默认 IP 限流阈值），未超限 | 发送 >100 请求，或降低 `RATE_LIMIT_DEFAULT_REQUESTS` |
| 所有请求返回 401 | 未携带有效 Bearer token，需认证的端点 | 使用无需认证的端点，或携带有效 token |

### 修复建议
- 当前实现已正确：忽略未认证上下文中的 `x-tenant-id`
- 仅信任服务端从 JWT 解析出的 tenant context

---

## 场景 2：伪造 x-tenant-id 污染其他租户限流桶

### 实现说明

> **当前实现**: 限流键中的 `tenant_id` **仅从已验证 JWT 的 claims 提取**，不从 `x-tenant-id` header 读取。
> - 未认证请求 → 使用 `RateLimitKey::Ip` (按 IP 限流)
> - 持有 Identity Token → 使用 `RateLimitKey::User` (按 user_id 限流)
> - 持有 Tenant Access Token → 使用 `RateLimitKey::TenantClient` (按 JWT 中的 tenant_id + client_id 限流)
>
> 因此，攻击者伪造 `x-tenant-id` header **不会**污染目标租户的限流桶。

### 前置条件
- 已知目标租户标识：`VICTIM_TENANT_ID`
- 攻击者和受害者使用**不同的 JWT token**（不同 user_id/client_id）

### 攻击目标
验证攻击者能否通过伪造 header 消耗其他租户限流额度。

### 攻击步骤
1. 攻击者请求携带 `x-tenant-id: VICTIM_TENANT_ID` 并快速发送。
2. 目标租户正常用户发起同端点请求。
3. 比较目标租户是否异常提前命中 429。

### 预期安全行为
- 攻击者请求不应影响目标租户限流余额
- 不应出现跨租户限流干扰
- `x-tenant-id` header 不影响限流键

### 验证方法
```bash
# 攻击者侧（使用攻击者自己的 token 或无 token）
# 注意: 无论 x-tenant-id 设置为什么值，限流键都基于 IP（无 token 时）或 JWT claims（有 token 时）
for i in $(seq 1 20); do
  curl -s -o /dev/null \
    -H "x-tenant-id: $VICTIM_TENANT_ID" \
    "http://localhost:8080/api/v1/tenants"
done

# 目标租户合法请求（使用受害者自己的 token）
curl -i -H "Authorization: Bearer $VICTIM_TOKEN" \
  "http://localhost:8080/api/v1/tenants"
# 预期: 200 OK（受害者的限流桶不受攻击者影响）
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 攻击者和受害者都返回 429 | 使用了同一 IP 且都无 token（共享 IP 桶） | 确保受害者使用 Bearer token（限流键变为 user_id） |
| 攻击者首个请求即返回 429 | 之前的测试已耗尽 IP 限流额度 | 等待 60 秒窗口过期，或重启 Redis 清除状态 |

### 修复建议
- 当前实现已正确：tenant 维度限流绑定 JWT 中的 tenant claim
- `x-tenant-id` header 不参与限流决策

---

## 场景 3：动态路径高基数导致 Redis key 膨胀

### 前置条件
- Redis 可访问
- 可构造大量不同路径参数

### 攻击目标
验证 endpoint key 是否使用原始 path，导致每个 ID 形成独立限流键。

### 攻击步骤
1. 构造 200 个不同资源 ID 请求同一路由模板。
2. 统计 Redis 中新增限流 key 数量。

### 预期安全行为
- 同模板路由应聚合（例如 `GET:/api/v1/users/{id}`）
- key 数不应随参数值线性增长

### 验证方法
```bash
for i in $(seq 1 200); do
  curl -s -o /dev/null \
    "http://localhost:8080/api/v1/users/00000000-0000-0000-0000-$(printf '%012d' $i)"
done

redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" | wc -l
```

### 修复建议
- 使用路由模板或归一化 endpoint 作为限流维度。

---

## 场景 4：Redis 故障窗口下 fail-open 被用于请求洪泛

### 实现说明

> **当前实现**: auth9-core 已具备 `InMemoryRateLimiter` 内存兜底限流（`rate_limit.rs:116-157`）。
> 当 Redis 不可用时，限流中间件会自动切换到内存滑动窗口计数器，使用与 Redis 相同的阈值配置。
>
> **注意**: Redis 连接超时期间，部分请求可能因等待 Redis 响应而超时（返回 5xx），
> 这些超时错误**不是**限流放行——它们是连接失败。仅统计 200 响应来评估是否存在洪泛。

### 前置条件
- 可控制测试环境 Redis 启停
- 目标端点为高成本接口（如列表查询、写操作）
- 了解默认限流阈值（默认: 100 请求 / 60 秒）

### 攻击目标
验证 Redis 故障时是否无条件放行，造成 DoS 放大窗口。

### 攻击步骤
1. 停止 Redis。
2. 对目标端点持续高频请求（**超过阈值数量**）。
3. 统计 200 响应数量，确认内存兜底是否生效。

### 预期安全行为
- 内存兜底限流应在 Redis 故障期间自动激活
- 200 响应数量不应超过限流阈值（默认 100）
- 日志应显示 `"Redis unavailable for rate limiting, using in-memory fallback"`
- 超过阈值后应返回 429

### 验证方法
```bash
docker stop auth9-redis

# 发送超过限流阈值的请求以验证兜底生效
# 注意: 部分请求可能因 Redis 连接超时返回 5xx（这不是限流放行）
# 关键: 统计 200 响应数量，确认不超过阈值
for i in $(seq 1 150); do
  curl -s -o /dev/null -w "%{http_code}\n" \
    "http://localhost:8080/api/v1/tenants?limit=100"
done | sort | uniq -c
# 预期: 200 数量 ≤ 100, 429 出现在超过阈值后, 可能有部分 5xx（连接超时）

docker start auth9-redis
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 大量 5xx 错误 | Redis 连接超时传播到其他组件 | 正常现象；仅关注 200 和 429 的比例 |
| 30 个请求全部 200 且无 429 | 30 < 100（默认阈值），未超限 | 发送 >100 请求，或降低 `RATE_LIMIT_DEFAULT_REQUESTS` |
| 无 `in-memory fallback` 日志 | 限流未启用或日志级别过高 | 设置 `RUST_LOG=info` 或 `RUST_LOG=auth9_core::middleware::rate_limit=warn` |

### 修复建议
- 当前实现已有内存兜底，但可考虑进一步优化：
  - 关键端点使用更严格的降级阈值
  - Redis 错误按端点分级处理（高风险端点默认拒绝）

---

## 场景 5：恢复后限流状态一致性与指标告警

### 前置条件
- 已执行场景 4（Redis 故障后恢复）
- 可访问指标端点 `/metrics`
- **`METRICS_TOKEN` 环境变量已配置**（未设置时 `/metrics` 返回 404，这是安全设计而非 bug）

### 攻击目标
验证 Redis 恢复后限流状态可恢复且有可观测告警。

### 攻击步骤
1. Redis 恢复后继续发送请求，观察限流是否重新生效。
2. 查询指标中限流异常与拦截计数。

### 预期安全行为
- 限流恢复生效（达到阈值后返回 429）
- 指标可反映 Redis 异常和限流拦截事件，便于告警

### 验证方法
```bash
# 步骤 1：验证限流恢复
for i in $(seq 1 30); do
  curl -s -o /dev/null -w "%{http_code}\n" \
    "http://localhost:8080/api/v1/tenants"
done

# 步骤 2：查询指标（需携带 METRICS_TOKEN）
# 注意: /metrics 端点需要 Bearer token 认证，未配置 METRICS_TOKEN 时返回 404
METRICS_TOKEN="${METRICS_TOKEN:-your-metrics-token}"
curl -s -H "Authorization: Bearer $METRICS_TOKEN" \
  "http://localhost:8080/metrics" | rg "rate_limit|redis"
# 预期指标:
# - auth9_rate_limit_throttled_total (被限流拒绝的请求计数)
# - auth9_rate_limit_unavailable_total (Redis 不可用时的降级计数)
# - auth9_http_requests_total (含 status=429 的限流响应)
```

### 常见失败排查

| 症状 | 原因 | 修复方法 |
|------|------|---------|
| `/metrics` 返回 404 | `METRICS_TOKEN` 未配置 | 设置 `METRICS_TOKEN=<token>` 环境变量后重启 auth9-core |
| 无 `rate_limit` 相关指标 | 限流未触发过 | 先触发限流（超过阈值），再查询指标 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 通过轮换 x-tenant-id 绕过单源限流 | ☐ | | | |
| 2 | 伪造 x-tenant-id 污染其他租户限流桶 | ☐ | | | |
| 3 | 动态路径高基数导致 Redis key 膨胀 | ☐ | | | |
| 4 | Redis 故障窗口下 fail-open 被用于请求洪泛 | ☐ | | | |
| 5 | 恢复后限流状态一致性与指标告警 | ☐ | | | |

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-05  
**适用控制**: V4.4,V13.3,V16.2  
**关联任务**: Backlog #3, #19, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-API-05-C01 | 控制: V4.4 | 任务: #3, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-05-C02 | 控制: V13.3 | 任务: #3, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-05-C03 | 控制: V16.2 | 任务: #3, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
