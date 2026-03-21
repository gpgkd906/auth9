# 会话与安全 - 登录事件测试

**模块**: 会话与安全
**测试范围**: 登录事件记录、分析
**场景数**: 5

---

## 架构说明

Auth9 的登录事件产生和记录流程：

1. **用户名/密码与 MFA 验证由 Auth9 内置 OIDC 引擎处理** → 引擎产生事件
2. **事件通过 Webhook 推送** → 事件兼容入口将事件实时推送到 auth9-core 的 `POST /api/v1/keycloak/events` 端点
3. **Auth9 Core 记录和分析** → Auth9 接收事件后写入 `login_events` 表，并触发安全检测（如暴力破解告警）

**关键点**：本文档测试的是事件接收和记录链路，通过直接调用 Webhook API 模拟事件，不通过浏览器登录流程。

### Webhook 事件模拟方法

所有场景使用以下模式模拟事件推送：

```bash
SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
BODY='<event JSON>'
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "x-keycloak-signature: sha256=$SIG" \
  -d "$BODY"
```

---

## 测试前置数据（必需）

在执行本文件场景前，先执行：

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 < docs/qa/session/seed.sql
```

说明：
- `seed.sql` 会创建管理员与目标用户的会话数据
- 管理员：`admin@auth9.local`
- 目标用户：`target@example.com`

---

## 数据库表结构参考

### login_events 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGINT | 自增主键 |
| user_id | CHAR(36) | 用户 ID |
| email | VARCHAR(320) | 用户邮箱 |
| event_type | ENUM | success/failed_password/failed_mfa/locked/social |
| ip_address | VARCHAR(45) | IP 地址 |
| device_type | VARCHAR(50) | 设备类型 |
| failure_reason | VARCHAR(255) | 失败原因 |
| created_at | TIMESTAMP | 事件时间 |

---

## 场景 1：登录成功事件记录

### 初始状态
- Auth9 服务运行中
- Webhook secret 已配置

### 目的
验证成功登录事件被正确记录

### 测试操作流程
1. 通过 Webhook API 模拟一次登录成功事件：
   ```bash
   SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
   BODY='{"type":"LOGIN","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"192.168.1.100","time":'"$(($(date +%s)*1000))"',"details":{"username":"testuser","email":"testuser@example.com","authMethod":"password"}}'
   SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

   curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
     -H "Content-Type: application/json" \
     -H "x-keycloak-signature: sha256=$SIG" \
     -d "$BODY"
   ```
2. 查询数据库

### 预期结果
- Webhook 返回 HTTP 204
- 事件被记录

### 预期数据状态
```sql
SELECT event_type, ip_address, created_at FROM login_events
WHERE email = 'testuser@example.com' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'success', ip_address = '192.168.1.100'
```

---

## 场景 2：登录失败事件记录

### 初始状态
- Auth9 服务运行中
- Webhook secret 已配置

### 目的
验证失败登录事件被正确记录

### 测试操作流程
1. 通过 Webhook API 模拟一次登录失败事件：
   ```bash
   SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
   BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"192.168.1.200","error":"invalid_user_credentials","time":'"$(($(date +%s)*1000))"',"details":{"username":"user","email":"user@example.com"}}'
   SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

   curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
     -H "Content-Type: application/json" \
     -H "x-keycloak-signature: sha256=$SIG" \
     -d "$BODY"
   ```
2. 查询数据库

### 预期结果
- Webhook 返回 HTTP 204
- 失败事件被记录

### 预期数据状态
```sql
SELECT event_type, failure_reason FROM login_events
WHERE email = 'user@example.com' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_password'
```

### 故障排查

| 症状 | 原因 | 解决 |
|------|------|------|
| Webhook 返回 401 | 签名不匹配 | 确认 `SECRET` 与 auth9-core 配置的 Webhook Secret（环境变量 `KEYCLOAK_WEBHOOK_SECRET`，历史遗留名）一致 |
| 事件未记录 | auth9-core 未运行 | 检查 `docker ps` 确认 auth9-core 容器正常 |
| Webhook 返回 204 但无记录 | 事件类型未映射 | 确认 `type` 字段为有效类型（LOGIN、LOGIN_ERROR 等） |

---

## 场景 3：MFA 失败事件记录

### 初始状态
- Auth9 服务运行中
- Webhook secret 已配置

### 目的
验证 MFA 失败事件被记录

### 测试操作流程
1. 通过 Webhook API 模拟一次 MFA 验证失败事件：
   ```bash
   SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
   BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"192.168.1.150","error":"invalid_totp","time":'"$(($(date +%s)*1000))"',"details":{"username":"mfa-user","email":"mfa-user@example.com","credentialType":"otp"}}'
   SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

   curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
     -H "Content-Type: application/json" \
     -H "x-keycloak-signature: sha256=$SIG" \
     -d "$BODY"
   ```
2. 查询数据库

### 预期结果
- Webhook 返回 HTTP 204
- MFA 失败事件被记录

### 预期数据状态
```sql
SELECT event_type, failure_reason FROM login_events
WHERE email = 'mfa-user@example.com' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_mfa'
```

---

## 场景 4：账户锁定事件

### 初始状态
- Auth9 服务运行中
- Webhook secret 已配置

### 目的
验证账户锁定事件被正确记录

### 测试操作流程
1. 通过 Webhook API 连续发送 5 个登录失败事件，模拟暴力破解：
   ```bash
   SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
   USER_ID="550e8400-e29b-41d4-a716-446655440000"

   for i in $(seq 1 5); do
     BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"'"$USER_ID"'","ipAddress":"10.0.0.50","error":"invalid_user_credentials","time":'"$(($(date +%s)*1000 + i))"',"details":{"username":"locked-user","email":"locked-user@example.com"}}'
     SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)
     curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
       -H "Content-Type: application/json" \
       -H "x-keycloak-signature: sha256=$SIG" \
       -d "$BODY"
     echo ""
   done
   ```
2. 发送一个账户锁定事件：
   ```bash
   BODY='{"type":"USER_DISABLED_BY_TEMPORARY_LOCKOUT","realmId":"auth9","clientId":"auth9-portal","userId":"'"$USER_ID"'","ipAddress":"10.0.0.50","time":'"$(($(date +%s)*1000 + 6))"',"details":{"username":"locked-user","email":"locked-user@example.com"}}'
   SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)
   curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
     -H "Content-Type: application/json" \
     -H "x-keycloak-signature: sha256=$SIG" \
     -d "$BODY"
   ```
3. 查询数据库

### 预期结果
- 所有 Webhook 返回 HTTP 204
- 5 条失败事件 + 1 条锁定事件被记录

### 预期数据状态
```sql
SELECT event_type, created_at FROM login_events
WHERE email = 'locked-user@example.com' ORDER BY created_at DESC LIMIT 6;
-- 预期: 最新一条为 'locked'，其余 5 条为 'failed_password'
```

---

## 场景 5：登录分析统计

### 初始状态
- 系统有一定数量的登录事件数据

### 目的
验证登录分析功能

### 测试操作流程
1. 进入「分析」页面
2. 选择时间范围：7天/14天/30天/90天

### 预期结果
- 显示总登录次数
- 显示成功/失败比例
- 显示按设备类型分布

### 预期数据状态
```sql
SELECT
    COUNT(*) as total,
    SUM(CASE WHEN event_type = 'success' THEN 1 ELSE 0 END) as success,
    COUNT(DISTINCT user_id) as unique_users
FROM login_events WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY);
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 登录成功事件 | ☐ | | | |
| 2 | 登录失败事件 | ☐ | | | |
| 3 | MFA 失败事件 | ☐ | | | |
| 4 | 账户锁定事件 | ☐ | | | |
| 5 | 登录分析统计 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
