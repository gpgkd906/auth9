# 会话与安全 - 边界测试

**模块**: 会话与安全
**测试范围**: 边界情况、性能
**场景数**: 5

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

## 场景 1：撤销当前会话

### 初始状态
- 用户尝试撤销当前正在使用的会话

### 目的
验证系统正确处理此边界情况

### 测试操作流程
1. 在会话列表中尝试撤销当前会话

### 预期结果
- 选项1：显示警告或禁止操作
- 选项2：操作成功后用户被强制登出

---

## 场景 2：并发会话限制

### 初始状态
- 系统配置了最大并发会话数（如 5 个）
- 用户已有 5 个活跃会话

### 目的
验证并发会话限制

### 测试操作流程
1. 尝试从第 6 个设备登录

### 预期结果
- 选项1：拒绝登录，提示超出会话限制
- 选项2：自动撤销最早的会话

---

## 场景 3：社交登录事件记录

### 初始状态
- 用户通过 Google/GitHub 等社交账号登录

### 目的
验证社交登录事件被正确记录

### 测试操作流程
1. 点击「使用 Google 登录」
2. 完成 Google 授权
3. 登录成功

### 预期结果
- 登录成功
- 社交登录事件被记录

### 预期数据状态
```sql
SELECT event_type FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'social'
```

---

## 场景 4：可疑 IP 告警（密码喷洒检测）

### 初始状态
- seed.sql 数据已加载
- 系统中存在多个用户账号

### 目的
验证可疑 IP 检测（基于密码喷洒行为模式：同一 IP 在短时间内尝试登录 5+ 个不同账户）

**注意**：系统**不支持 IP 黑名单功能**，可疑 IP 告警仅通过行为模式检测触发。

### 测试操作流程
1. 从同一 IP 模拟对 5 个以上不同账户的登录失败事件：

```bash
SECRET="dev-webhook-secret"
for i in $(seq 1 6); do
  BODY="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"spray-user-$i\",\"error\":\"invalid_user_credentials\",\"ipAddress\":\"10.99.99.99\",\"details\":{\"username\":\"spray-target-$i@example.com\"}}"
  SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)
  curl -s -X POST "http://localhost:8080/api/v1/keycloak/events" \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=$SIG" -d "$BODY"
  sleep 0.5
done
```

2. 查询安全告警表确认告警生成

### 预期结果
- 同一 IP 对 5+ 不同账户登录失败后触发 `suspicious_ip` 告警
- 告警严重度为 `critical`

### 预期数据状态
```sql
SELECT alert_type, severity, details FROM security_alerts
WHERE alert_type = 'suspicious_ip' ORDER BY created_at DESC LIMIT 1;
-- 预期: severity = 'critical', details 包含 detection_reason = 'password_spray'
```

### 故障排查

| 症状 | 原因 | 解决 |
|------|------|------|
| 无告警生成 | 不同账户数未达阈值（默认 5） | 确保发送 5+ 个不同 userId 的事件 |
| 请求返回 401 | 缺少 HMAC 签名或密钥不匹配 | 检查 `x-keycloak-signature` 头和 `SECRET` 值 |
| 事件入库但无告警 | 事件间隔超出检测窗口（默认 10 分钟） | 确保所有事件在 10 分钟内发送 |

---

## 场景 5：大量登录事件性能

### 初始状态
- 系统有大量登录事件记录（10万+）

### 目的
验证登录事件查询性能

### 测试操作流程
1. 打开登录事件列表
2. 进行各种过滤和搜索

### 预期结果
- 页面响应时间 < 3秒
- 分页功能正常

---

## 测试数据准备 SQL

```sql
-- 准备测试用户
INSERT INTO users (id, keycloak_id, email, display_name) VALUES
('user-sess-1111-1111-111111111111', 'kc-sess-1', 'session-test@example.com', 'Session Test');

-- 准备测试会话
INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES
('sess-1111-1111-1111-111111111111', 'user-sess-1111-1111-111111111111', 'desktop', '192.168.1.1', 'Beijing', NOW()),
('sess-2222-2222-2222-222222222222', 'user-sess-1111-1111-111111111111', 'mobile', '192.168.1.2', 'Shanghai', DATE_SUB(NOW(), INTERVAL 1 HOUR));

-- 准备测试安全告警
INSERT INTO security_alerts (id, user_id, alert_type, severity, details, created_at) VALUES
('alert-1111-1111-1111-111111111111', 'user-sess-1111-1111-111111111111', 'brute_force', 'high',
 '{"attempts": 10}', NOW());

-- 清理
DELETE FROM security_alerts WHERE id LIKE 'alert-%';
DELETE FROM sessions WHERE id LIKE 'sess-%';
DELETE FROM users WHERE id LIKE 'user-sess-%';
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
| 1 | 撤销当前会话 | ☐ | | | |
| 2 | 并发会话限制 | ☐ | | | |
| 3 | 社交登录事件 | ☐ | | | |
| 4 | 可疑 IP 告警 | ☐ | | | |
| 5 | 大量事件性能 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
