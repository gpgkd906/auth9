# 集成测试 - Keycloak 事件接收器兼容测试（Webhook）

**模块**: 集成测试
**测试范围**: Keycloak 事件兼容入口（Webhook）接收、签名验证、事件映射、安全检测联动
**场景数**: 5
**优先级**: 中

---

## 背景说明

Auth9 当前默认事件主链路为 Redis Stream 消费（见 `integration/11-keycloak26-event-stream.md`）。
本文档用于验证兼容入口 `POST /api/v1/keycloak/events` 在回归/应急场景下仍可正确处理事件。

### 事件类型映射

| Keycloak 事件 | Auth9 LoginEventType | 说明 |
|--------------|---------------------|------|
| `LOGIN` / `CODE_TO_TOKEN` | `Success` | 登录成功 |
| `LOGIN_ERROR` (invalid_user_credentials) | `FailedPassword` | 密码错误 |
| `LOGIN_ERROR` (invalid_totp) | `FailedMfa` | MFA 验证失败 |
| `LOGIN_ERROR` (user_disabled) | `Locked` | 账户被锁定 |
| `IDENTITY_PROVIDER_LOGIN` | `Social` | 社交登录 |
| `USER_DISABLED_BY_TEMPORARY_LOCKOUT` | `Locked` | 暴力破解锁定 |
| `LOGOUT` / `REGISTER` / `REFRESH_TOKEN` | 忽略 | 非登录事件 |

### 签名验证（仅在 `KEYCLOAK_EVENT_SOURCE=webhook` 且配置 secret 时生效）
使用 HMAC-SHA256 签名，头部：`X-Keycloak-Signature: sha256=<hex>`

---

## 场景 1：接收登录成功事件

### 初始状态
- Auth9 服务运行中
- Keycloak webhook_secret 已配置

### 目的
验证成功接收并处理 Keycloak 登录事件

### 测试操作流程
1. 模拟 Keycloak 发送登录成功事件：
   ```bash
   SECRET="your-webhook-secret"
   BODY='{"type":"LOGIN","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"192.168.1.100","time":1704067200000,"details":{"username":"john","email":"john@example.com","authMethod":"password"}}'
   SIGNATURE="sha256=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')"

   curl -X POST http://localhost:8080/api/v1/keycloak/events \
     -H "Content-Type: application/json" \
     -H "X-Keycloak-Signature: $SIGNATURE" \
     -d "$BODY"
   ```
2. 检查 login_events 表

### 预期结果
- 状态码 204 No Content
- `login_events` 表新增一条 `event_type=success` 的记录
- 记录包含正确的 `ip_address`、`email`

### 预期数据状态
```sql
SELECT event_type, email, ip_address FROM login_events
ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type='success', email='john@example.com', ip_address='192.168.1.100'
```

---

## 场景 2：接收登录失败事件并触发安全检测

### 初始状态
- Auth9 服务运行中
- 安全检测功能已启用

### 目的
验证登录失败事件被正确处理并触发安全分析

### 测试操作流程
1. 连续发送 10 个登录失败事件（模拟暴力破解）：
   ```json
   {
     "type": "LOGIN_ERROR",
     "realmId": "auth9",
     "userId": "550e8400-e29b-41d4-a716-446655440000",
     "ipAddress": "192.168.1.200",
     "error": "invalid_user_credentials",
     "time": 1704067200000,
     "details": {
       "username": "target-user",
       "email": "target@example.com"
     }
   }
   ```
2. 检查 login_events 和 security_alerts

### 预期结果
- 每个事件返回 204
- `login_events` 表新增 10 条 `event_type=failed_password` 记录
- `security_alerts` 表可能新增暴力破解告警

### 预期数据状态
```sql
SELECT COUNT(*) FROM login_events
WHERE email = 'target@example.com' AND event_type = 'failed_password'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);
-- 预期: 10

SELECT alert_type, severity FROM security_alerts
WHERE user_id = '550e8400-e29b-41d4-a716-446655440000'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 可能存在 brute_force 告警
```

---

## 场景 3：签名验证失败拒绝事件

### 初始状态
- Auth9 配置了 webhook_secret

### 目的
验证无效签名的事件被拒绝

### 测试操作流程
1. 发送带错误签名的事件：
   ```bash
   curl -X POST http://localhost:8080/api/v1/keycloak/events \
     -H "Content-Type: application/json" \
     -H "X-Keycloak-Signature: sha256=0000000000000000000000000000000000000000000000000000000000000000" \
     -d '{"type":"LOGIN","time":0}'
   ```
2. 发送不带签名头的事件：
   ```bash
   curl -X POST http://localhost:8080/api/v1/keycloak/events \
     -H "Content-Type: application/json" \
     -d '{"type":"LOGIN","time":0}'
   ```

### 预期结果
- 步骤 1：状态码 401，错误信息「Invalid webhook signature」
- 步骤 2：状态码 401，错误信息「Missing webhook signature」
- `login_events` 表无新增记录

---

## 场景 4：非登录事件被忽略

### 初始状态
- Auth9 服务运行中

### 目的
验证非登录事件（如 LOGOUT、REGISTER）被正确忽略

### 测试操作流程
1. 发送 LOGOUT 事件（使用正确签名）：
   ```json
   {"type": "LOGOUT", "realmId": "auth9", "time": 0}
   ```
2. 发送 REGISTER 事件：
   ```json
   {"type": "REGISTER", "realmId": "auth9", "time": 0}
   ```
3. 发送 Admin 事件（无 type 字段）：
   ```json
   {"operationType": "CREATE", "resourceType": "USER", "realmId": "auth9", "time": 0}
   ```

### 预期结果
- 所有请求返回 204 No Content（确认收到但不处理）
- `login_events` 表无新增记录

---

## 场景 5：社交登录事件处理

### 初始状态
- Auth9 服务运行中
- 已配置 Google 身份提供商

### 目的
验证社交登录事件被正确分类

### 测试操作流程
1. 发送社交登录成功事件（使用正确签名）：
   ```json
   {
     "type": "IDENTITY_PROVIDER_LOGIN",
     "realmId": "auth9",
     "userId": "550e8400-e29b-41d4-a716-446655440000",
     "ipAddress": "10.0.0.1",
     "time": 1704067200000,
     "details": {
       "username": "google-user",
       "email": "user@gmail.com",
       "identityProvider": "google"
     }
   }
   ```
2. 检查 login_events

### 预期结果
- 状态码 204
- `login_events` 新增 `event_type=social` 记录

### 预期数据状态
```sql
SELECT event_type, email FROM login_events
WHERE email = 'user@gmail.com'
ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type='social'
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 登录成功事件 | ☐ | | | |
| 2 | 登录失败 + 安全检测 | ☐ | | | |
| 3 | 签名验证失败 | ☐ | | | |
| 4 | 非登录事件忽略 | ☐ | | | |
| 5 | 社交登录事件 | ☐ | | | |
