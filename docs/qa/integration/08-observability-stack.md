# 集成测试 - 可观测性基础设施栈

**模块**: 集成测试
**测试范围**: Docker Compose 可观测性栈、Grafana 仪表盘、Prometheus 采集、业务与安全指标验证
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 可观测性栈通过叠加 Docker Compose 文件启动：

```bash
docker-compose -f docker-compose.yml -f docker-compose.observability.yml up -d
```

### 服务与端口

| 服务 | 镜像 | 端口 | 用途 |
|------|------|------|------|
| prometheus | prom/prometheus:v2.51.0 | 9090 | 指标采集 |
| grafana | grafana/grafana:11.0.0 | 3001 | 仪表盘 |
| loki | grafana/loki:3.0.0 | 3100 | 日志聚合 |
| tempo | grafana/tempo:2.4.1 | 4317/3200 | 分布式追踪 |
| promtail | grafana/promtail:3.0.0 | — | Docker 日志采集 |

### 告警规则

| 告警名 | 触发条件 | 级别 |
|--------|---------|------|
| HighErrorRate | 5xx 比例 > 5% 持续 5m | critical |
| HighLoginFailureRate | 登录失败率 > 30% 持续 5m | warning |
| SecurityAlertsSpike | 安全告警速率 > 1/min 持续 2m | critical |
| HighP99Latency | P99 延迟 > 2s 持续 5m | warning |
| DatabasePoolExhaustion | 活跃连接占比 > 90% 持续 5m | critical |

---

## 场景 1：可观测性栈完整启动

### 初始状态
- Docker Compose 基础服务已启动
- 未叠加可观测性 Compose

### 目的
验证可观测性栈所有服务能正常启动并保持健康

### 测试操作流程
1. 启动完整栈：
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.observability.yml up -d
   ```
2. 等待服务就绪（约 30 秒），检查容器状态：
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.observability.yml ps
   ```
3. 验证各服务可访问：
   ```bash
   curl -s http://localhost:9090/-/ready       # Prometheus
   curl -s http://localhost:3001/api/health     # Grafana
   curl -s http://localhost:3100/ready          # Loki
   curl -s http://localhost:3200/ready          # Tempo
   ```

### 预期结果
- 所有容器状态为 Up（`auth9-prometheus`, `auth9-grafana`, `auth9-loki`, `auth9-tempo`, `auth9-promtail`）
- Prometheus: 返回正常响应
- Grafana: 返回 `{"commit":"...","database":"ok",...}`
- Loki: 返回 `ready`
- Tempo: 返回 `ready`
- auth9-core 服务环境变量自动注入 `OTEL_METRICS_ENABLED=true`

---

## 场景 2：Prometheus 成功采集 auth9-core 指标

### 初始状态
- 可观测性栈已启动
- auth9-core 指标端点已启用

### 目的
验证 Prometheus 能自动发现并采集 auth9-core 的 /metrics 端点

### 测试操作流程
1. 检查 Prometheus targets 状态：
   ```bash
   curl -s http://localhost:9090/api/v1/targets | python3 -m json.tool
   ```
2. 生成一些请求数据：
   ```bash
   for i in $(seq 1 10); do curl -s http://localhost:8080/health > /dev/null; done
   ```
3. 等待 15 秒（一个 scrape interval），查询指标：
   ```bash
   curl -s 'http://localhost:9090/api/v1/query?query=auth9_http_requests_total' | python3 -m json.tool
   ```
4. 检查告警规则是否加载：
   ```bash
   curl -s http://localhost:9090/api/v1/rules | python3 -m json.tool
   ```

### 预期结果
- targets 中 `auth9-core` job 状态为 `up`
- scrape endpoint 为 `auth9-core:8080/metrics`
- 查询返回 `auth9_http_requests_total` 有值
- rules API 返回 5 条告警规则（HighErrorRate, HighLoginFailureRate, SecurityAlertsSpike, HighP99Latency, DatabasePoolExhaustion）

---

## 场景 3：Grafana 仪表盘自动加载

### 初始状态
- 可观测性栈已启动
- Grafana 匿名访问已启用（开发模式）

### 目的
验证 4 个预配置仪表盘自动加载，数据源正确连接

### 测试操作流程
1. 打开 Grafana：在浏览器访问 `http://localhost:3001`
2. 检查数据源是否配置：
   ```bash
   curl -s http://localhost:3001/api/datasources | python3 -m json.tool
   ```
3. 检查仪表盘列表：
   ```bash
   curl -s http://localhost:3001/api/search?type=dash-db | python3 -m json.tool
   ```
4. 验证每个仪表盘可访问：
   ```bash
   curl -s http://localhost:3001/api/dashboards/uid/auth9-overview | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['dashboard']['title'])"
   curl -s http://localhost:3001/api/dashboards/uid/auth9-auth | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['dashboard']['title'])"
   curl -s http://localhost:3001/api/dashboards/uid/auth9-security | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['dashboard']['title'])"
   curl -s http://localhost:3001/api/dashboards/uid/auth9-infra | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['dashboard']['title'])"
   ```

### 预期结果
- 3 个数据源已配置：Prometheus（默认）、Loki、Tempo
- 4 个仪表盘自动出现在 Auth9 文件夹中：
  - Auth9 - System Overview（uid: `auth9-overview`，6 个面板）
  - Auth9 - Authentication（uid: `auth9-auth`，4 个面板）
  - Auth9 - Security（uid: `auth9-security`，4 个面板）
  - Auth9 - Infrastructure（uid: `auth9-infra`，4 个面板）
- 无需手动导入仪表盘

---

## 场景 4：业务指标与数据库连接池指标验证

### 初始状态
- 可观测性栈已启动
- auth9-core 后台指标任务已运行（DB 池指标 15s 间隔，业务指标 60s 间隔）

### 目的
验证后台任务定期上报数据库连接池和业务计数指标

### 测试操作流程
1. 等待至少 60 秒让后台任务执行
2. 检查数据库连接池指标：
   ```bash
   curl -s http://localhost:8080/metrics | grep auth9_db_pool
   ```
3. 检查业务计数指标：
   ```bash
   curl -s http://localhost:8080/metrics | grep auth9_tenants_active_total
   curl -s http://localhost:8080/metrics | grep auth9_users_active_total
   curl -s http://localhost:8080/metrics | grep auth9_sessions_active_total
   ```
4. 与数据库实际值对比：
   ```bash
   mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT COUNT(*) FROM tenants WHERE status = 'active';"
   mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT COUNT(DISTINCT user_id) FROM tenant_users;"
   mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT COUNT(*) FROM sessions WHERE revoked_at IS NULL;"
   ```

### 预期结果
- `auth9_db_pool_connections_active` 和 `auth9_db_pool_connections_idle` 有值（> 0）
- `auth9_tenants_active_total` 值与 `SELECT COUNT(*) FROM tenants WHERE status = 'active'` 一致
- `auth9_users_active_total` 值与 `SELECT COUNT(DISTINCT user_id) FROM tenant_users` 一致
- `auth9_sessions_active_total` 值与有效会话数一致
- 指标值会随时间更新（非固定值）

---

## 场景 5：Redis 与限流指标验证

### 初始状态
- 可观测性栈已启动
- Rate limiting 已配置（如 forgot-password: 5 req/min）

### 目的
验证 Redis 操作指标和限流指标正确记录

### 测试操作流程
1. 触发 Redis 操作（通过正常 API 调用）：
   ```bash
   TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
   curl -s http://localhost:8080/api/v1/tenants -H "Authorization: Bearer $TOKEN"
   ```
2. 检查 Redis 指标：
   ```bash
   curl -s http://localhost:8080/metrics | grep auth9_redis_operations_total
   curl -s http://localhost:8080/metrics | grep auth9_redis_operation_duration_seconds
   ```
3. 触发限流（发送超过限制的请求）：
   ```bash
   for i in $(seq 1 10); do
     curl -s -o /dev/null -w "%{http_code}\n" -X POST \
       http://localhost:8080/api/v1/auth/forgot-password \
       -H "Content-Type: application/json" \
       -d '{"email":"test@example.com"}'
   done
   ```
4. 检查限流指标：
   ```bash
   curl -s http://localhost:8080/metrics | grep auth9_rate_limit_throttled_total
   ```

### 预期结果
- `auth9_redis_operations_total` 按 operation 标签（get, set, delete）分别计数
- `auth9_redis_operation_duration_seconds` 有 histogram bucket 数据
- 超过限流阈值后 `auth9_rate_limit_throttled_total` 计数增加
- 限流指标包含 `endpoint` 标签标识被限流的端点

---

## 通用场景：JSON 日志格式验证

### 测试操作流程
1. 使用可观测性 Compose 启动（`LOG_FORMAT=json` 自动注入）
2. 查看 auth9-core 日志：
   ```bash
   docker logs auth9-core --tail 10
   ```
3. 验证日志为 JSON 格式

### 预期结果
- 每行日志为有效 JSON 对象
- 包含 `timestamp`、`level`、`target`、`message` 等字段
- 不再是 pretty 格式的彩色文本

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 可观测性栈完整启动 | ☐ | | | |
| 2 | Prometheus 采集 auth9-core | ☐ | | | |
| 3 | Grafana 仪表盘自动加载 | ☐ | | | |
| 4 | 业务与 DB 连接池指标 | ☐ | | | |
| 5 | Redis 与限流指标 | ☐ | | | |
| — | JSON 日志格式 | ☐ | | | |
