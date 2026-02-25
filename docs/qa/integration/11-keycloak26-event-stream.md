# 集成测试 - Keycloak 26 事件桥接回归

**模块**: 集成测试
**测试范围**: Keycloak 26 升级、登录事件 Webhook 接入（ext-event-http SPI）
**场景数**: 4
**优先级**: 高

---

## 背景说明

Auth9 登录事件链路使用 Webhook 模式：

1. Keycloak 26 负责认证与事件产出（事件监听器使用 `jboss-logging` + `ext-event-http`）
2. `ext-event-http` SPI 插件（p2-inc/keycloak-events v0.51）将事件实时推送到 auth9-core 的 `POST /api/v1/keycloak/events`
3. auth9-core 接收事件后写入 `login_events`、触发 `security_alerts`

---

## 场景 1：Keycloak 26 基础健康与参数兼容性

### 初始状态
- 本地环境已升级为 `quay.io/keycloak/keycloak:26.3.3`

### 目的
验证 Keycloak 26 可正常启动，且未使用旧参数。

### 测试操作流程
1. 启动服务：
   ```bash
   docker-compose up -d keycloak
   ```
2. 查看容器日志：
   ```bash
   docker logs auth9-keycloak 2>&1 | tail -n 100
   ```
3. 检查健康端点：
   ```bash
   curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/health/ready
   ```

### 预期结果
- Keycloak 正常启动，无未知参数报错
- `/health/ready` 返回 `200`
- 日志中不出现 `legacy-logout-redirect-uri` 参数错误

---

## 场景 2：Webhook 事件推送与处理成功

### 初始状态
- Keycloak ext-event-http SPI 已加载（`keycloak-events-*.jar` 在 providers 中）
- seeder 已配置 `ext-event-http` 监听器（`KEYCLOAK_WEBHOOK_SECRET` 已设置）

### 目的
验证 Keycloak 通过 ext-event-http 推送的事件可被 auth9-core 正确处理并写入登录事件表。

### 测试操作流程
1. 通过 Portal 执行一次失败登录（输入错误密码）
2. 等待 2-3 秒后查询数据库

### 预期结果
- `login_events` 新增 `event_type='failed_password'` 记录

### 预期数据状态
```sql
SELECT event_type, email, ip_address
FROM login_events
WHERE email = 'john@example.com'
ORDER BY created_at DESC
LIMIT 1;
-- 预期: event_type='success', ip_address='192.168.1.100'
```

---

## 场景 3：重复事件去重（基于 event id）

### 初始状态
- Stream 消费链路正常

### 目的
验证同一事件 ID 重复写入时不会重复入库。

### 测试操作流程
1. 连续写入两条相同 `id` 的事件：
   ```bash
   EVENT='{"id":"evt-dedup-001","type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"198.51.100.10","error":"invalid_user_credentials","time":'"$(($(date +%s)*1000))"',"details":{"username":"target","email":"target@example.com"}}'
   redis-cli XADD auth9:keycloak:events * payload "$EVENT"
   redis-cli XADD auth9:keycloak:events * payload "$EVENT"
   ```
2. 查询最近 5 分钟相同邮箱失败事件数量。

### 预期结果
- 仅记录 1 条有效失败事件（重复事件被跳过）

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt
FROM login_events
WHERE email = 'target@example.com'
  AND event_type = 'failed_password'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);
-- 预期: cnt = 1
```

---

## 场景 4：过期事件拒绝（时间窗防重放）

### 初始状态
- Stream 消费链路正常

### 目的
验证超过时间窗（5 分钟）的事件被拒绝，不写入数据库。

### 测试操作流程
1. 写入一条旧时间戳事件：
   ```bash
   redis-cli XADD auth9:keycloak:events * payload '{"id":"evt-old-001","type":"LOGIN","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"203.0.113.11","time":1600000000000,"details":{"username":"old","email":"old@example.com"}}'
   ```
2. 查询 `old@example.com` 最近记录。

### 预期结果
- 事件被拒绝，不新增登录记录

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt
FROM login_events
WHERE email = 'old@example.com'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 1 HOUR);
-- 预期: cnt = 0
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Keycloak 26 基础健康与参数兼容性 | ☐ | | | |
| 2 | Webhook 事件推送与处理成功 | ☐ | | | |
| 3 | 重复事件去重（基于 event id） | ☐ | | | |
| 4 | 过期事件拒绝（时间窗防重放） | ☐ | | | |
