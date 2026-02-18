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

### 前置条件
- 同一来源 IP
- 限流阈值示例：10 请求 / 60 秒

### 攻击目标
验证限流是否错误信任 `x-tenant-id`，导致同一攻击者通过 header 轮换绕过。

### 攻击步骤
1. 在 60 秒内发送 40 次请求，每次使用不同 `x-tenant-id`。
2. 记录状态码分布。

### 预期安全行为
- 应触发 429，不能持续 200/401 放行
- 限流主键应基于可信身份（认证主体/IP），而非客户端任意 header

### 验证方法
```bash
for i in $(seq 1 40); do
  code=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "x-tenant-id: fake-$i" \
    "http://localhost:8080/api/v1/tenants")
  echo "$i -> $code"
done
```

### 修复建议
- 忽略未认证上下文中的 `x-tenant-id`。
- 仅信任服务端解析出的 tenant context。

---

## 场景 2：伪造 x-tenant-id 污染其他租户限流桶

### 前置条件
- 已知目标租户标识：`VICTIM_TENANT_ID`

### 攻击目标
验证攻击者能否通过伪造 header 消耗他租户限流额度。

### 攻击步骤
1. 攻击者请求携带 `x-tenant-id: VICTIM_TENANT_ID` 并快速发送。
2. 目标租户正常用户发起同端点请求。
3. 比较目标租户是否异常提前命中 429。

### 预期安全行为
- 攻击者请求不应影响目标租户限流余额
- 不应出现跨租户限流干扰

### 验证方法
```bash
# 攻击者侧
for i in $(seq 1 20); do
  curl -s -o /dev/null \
    -H "x-tenant-id: $VICTIM_TENANT_ID" \
    "http://localhost:8080/api/v1/tenants"
done

# 目标租户合法请求
curl -i -H "Authorization: Bearer $VICTIM_TOKEN" \
  "http://localhost:8080/api/v1/tenants"
```

### 修复建议
- tenant 维度限流必须绑定已认证 token 中的 tenant claim。

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

### 前置条件
- 可控制测试环境 Redis 启停
- 目标端点为高成本接口（如列表查询、写操作）

### 攻击目标
验证 Redis 故障时是否无条件放行，造成 DoS 放大窗口。

### 攻击步骤
1. 停止 Redis。
2. 对目标端点持续高频请求。
3. 观察是否仍持续放行且无 429。

### 预期安全行为
- 对敏感/高成本端点应有降级保护（例如 fail-closed 或本地兜底限流）
- 不应在 Redis 故障期间完全失去流量控制

### 验证方法
```bash
docker stop auth9-redis

for i in $(seq 1 100); do
  curl -s -o /dev/null -w "%{http_code}\n" \
    "http://localhost:8080/api/v1/tenants?limit=100"
done

docker start auth9-redis
```

### 修复建议
- 关键端点引入本地令牌桶兜底。
- Redis 错误按端点分级处理（高风险端点默认拒绝）。

---

## 场景 5：恢复后限流状态一致性与指标告警

### 前置条件
- 已执行场景 4（Redis 故障后恢复）
- 可访问指标端点 `/metrics`

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
for i in $(seq 1 30); do
  curl -s -o /dev/null -w "%{http_code}\n" \
    "http://localhost:8080/api/v1/tenants"
done

curl -s "http://localhost:8080/metrics" | rg "rate_limit|redis"
```

### 修复建议
- 增加 `auth9_rate_limit_error_total` 等指标并接入告警规则。

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
