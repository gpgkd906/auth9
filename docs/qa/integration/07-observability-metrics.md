# 集成测试 - 可观测性指标与请求追踪

**模块**: 集成测试
**测试范围**: Prometheus /metrics 端点、HTTP 指标采集、X-Request-ID 传播、业务指标记录
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 全栈可观测性功能通过以下环境变量启用：

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `OTEL_METRICS_ENABLED` | `false` | 启用 Prometheus /metrics 端点 |
| `OTEL_TRACING_ENABLED` | `false` | 启用 OpenTelemetry trace 导出 |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | (无) | OTLP 端点 |
| `LOG_FORMAT` | `pretty` | 日志格式: `json` / `pretty` |

端点：`GET /metrics` — 返回 Prometheus text exposition format

### 已定义的指标

| 指标名 | 类型 | 标签 |
|--------|------|------|
| `auth9_http_requests_total` | counter | method, path, status |
| `auth9_http_request_duration_seconds` | histogram | method, path |
| `auth9_http_requests_in_flight` | gauge | — |
| `auth9_grpc_requests_total` | counter | service, method, status |
| `auth9_grpc_request_duration_seconds` | histogram | service, method |
| `auth9_db_pool_connections_active` | gauge | — |
| `auth9_db_pool_connections_idle` | gauge | — |
| `auth9_redis_operations_total` | counter | operation |
| `auth9_redis_operation_duration_seconds` | histogram | operation |
| `auth9_auth_login_total` | counter | result |
| `auth9_auth_token_exchange_total` | counter | result |
| `auth9_auth_token_validation_total` | counter | result |
| `auth9_security_alerts_total` | counter | type, severity |
| `auth9_rate_limit_throttled_total` | counter | endpoint |
| `auth9_tenants_active_total` | gauge | — |
| `auth9_users_active_total` | gauge | — |
| `auth9_sessions_active_total` | gauge | — |

---

## 场景 1：/metrics 端点返回 Prometheus 格式数据

### 初始状态
- Auth9 服务运行中，`OTEL_METRICS_ENABLED=true`
- 使用可观测性 Compose 启动：`docker-compose -f docker-compose.yml -f docker-compose.observability.yml up -d`

### 目的
验证 /metrics 端点正常暴露 Prometheus 格式指标

### 测试操作流程
1. 调用 /metrics 端点（需携带 METRICS_TOKEN）：
   ```bash
   curl -s -H "Authorization: Bearer dev-metrics-token" http://localhost:8080/metrics
   ```
2. 检查输出是否包含 HELP 和 TYPE 行
3. 验证包含核心指标名称

### 预期结果
- 状态码 200
- Content-Type 为 text/plain
- 输出包含 `# HELP auth9_http_requests_total Total number of HTTP requests`
- 输出包含 `# TYPE auth9_http_requests_total counter`
- 输出包含所有 17 个指标的 HELP/TYPE 定义
- 需携带 `Authorization: Bearer <METRICS_TOKEN>` 头（由 `docker-compose.observability.yml` 中 `METRICS_TOKEN` 环境变量配置）

---

## 场景 2：HTTP 请求指标正确记录

### 初始状态
- Auth9 服务运行中，指标已启用
- 已执行若干 API 请求

### 目的
验证每次 HTTP 请求会正确增加 counter 和 histogram

### 测试操作流程
1. 记录当前指标基线：
   ```bash
   curl -s -H "Authorization: Bearer dev-metrics-token" http://localhost:8080/metrics | grep auth9_http_requests_total
   ```
2. 发送已知请求：
   ```bash
   curl -s http://localhost:8080/health
   curl -s http://localhost:8080/api/v1/tenants -H "Authorization: Bearer {TOKEN}"
   curl -s http://localhost:8080/api/v1/nonexistent
   ```
3. 再次读取指标：
   ```bash
   curl -s -H "Authorization: Bearer dev-metrics-token" http://localhost:8080/metrics | grep auth9_http_requests_total
   ```
4. 检查请求延迟 histogram：
   ```bash
   curl -s -H "Authorization: Bearer dev-metrics-token" http://localhost:8080/metrics | grep auth9_http_request_duration_seconds
   ```

### 预期结果
- `auth9_http_requests_total{method="GET",path="/health",status="200"}` 计数增加
- `auth9_http_requests_total{method="GET",path="/api/v1/tenants",status="200"}` 计数增加
- `auth9_http_requests_total{method="GET",path="/api/v1/nonexistent",status="404"}` 计数增加
- histogram bucket 中有 `auth9_http_request_duration_seconds_bucket` 数据
- histogram 的 `_sum` 和 `_count` 值大于 0

---

## 场景 3：X-Request-ID 传播

### 初始状态
- Auth9 服务运行中

### 目的
验证请求 ID 的提取和回传机制

### 测试操作流程
1. 不携带 X-Request-ID 发送请求，检查响应头：
   ```bash
   curl -v http://localhost:8080/health 2>&1 | grep -i x-request-id
   ```
2. 携带自定义 X-Request-ID 发送请求：
   ```bash
   curl -v -H "X-Request-ID: test-req-12345" http://localhost:8080/health 2>&1 | grep -i x-request-id
   ```
3. 验证 UUID 格式：
   ```bash
   REQUEST_ID=$(curl -sI http://localhost:8080/health | grep -i x-request-id | awk '{print $2}' | tr -d '\r')
   echo "$REQUEST_ID"
   ```

### 预期结果
- 步骤 1：响应头包含 `x-request-id`，值为自动生成的 UUID v4 格式（36 字符，含连字符）
- 步骤 2：响应头 `x-request-id: test-req-12345` 与请求中传入的值一致
- 步骤 3：自动生成的 ID 符合 UUID v4 格式（`xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`）

---

## 场景 4：UUID 路径段折叠防止高基数标签

### 初始状态
- Auth9 服务运行中，指标已启用
- 数据库中存在至少一个 tenant

### 目的
验证 path 标签中的 UUID 路径段被折叠为 `{id}`，防止 Prometheus 标签基数爆炸

### 测试操作流程
1. 获取一个 tenant ID：
   ```bash
   TENANT_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;")
   ```
2. 发送包含 UUID 路径的请求：
   ```bash
   curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID \
     -H "Authorization: Bearer {TOKEN}"
   ```
3. 检查指标中的 path 标签：
   ```bash
   curl -s -H "Authorization: Bearer dev-metrics-token" http://localhost:8080/metrics | grep 'auth9_http_requests_total.*tenants'
   ```

### 预期结果
- path 标签显示为 `/api/v1/tenants/{id}` 而非 `/api/v1/tenants/550e8400-e29b-...`
- 不同 UUID 的请求都归入同一个 path 标签 `/api/v1/tenants/{id}`
- 指标输出中不包含任何裸 UUID 路径段

---

## 场景 5：指标未启用时 /metrics 返回 404

### 初始状态
- Auth9 服务运行中，使用默认配置（`OTEL_METRICS_ENABLED` 未设置或为 `false`）
- 使用标准 Compose 启动：`docker-compose up -d`（不叠加 observability）

### 目的
验证指标功能在未启用时不暴露数据

### 测试操作流程
1. 调用 /metrics 端点：
   ```bash
   curl -v http://localhost:8080/metrics
   ```
2. 正常的 API 功能不受影响：
   ```bash
   curl -s http://localhost:8080/health
   ```

### 预期结果
- `/metrics` 返回状态码 404
- 响应体为 `Metrics not enabled`
- 其他 API 端点正常工作，不受指标开关影响
- 无性能退化（`metrics` crate 无 recorder 时所有操作为 no-op）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | /metrics 端点 Prometheus 格式 | ☐ | | | |
| 2 | HTTP 请求指标记录 | ☐ | | | |
| 3 | X-Request-ID 传播 | ☐ | | | |
| 4 | UUID 路径折叠 | ☐ | | | |
| 5 | 指标未启用返回 404 | ☐ | | | |
