# 会话与安全 - 安全告警测试

**模块**: 会话与安全
**测试范围**: 安全告警检测、管理
**场景数**: 5

---

## 测试前置数据（必需）

在执行本文件场景前，依次执行：

### Step 1: 创建数据库种子数据
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 < docs/qa/session/seed.sql
```

### Step 2: 在 Keycloak 中创建目标用户
```bash
# 获取 Keycloak Admin Token
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials&client_id=admin-cli&client_secret=admin" \
  | jq -r '.access_token')

# 创建 target@example.com 用户（忽略已存在错误）
curl -s -o /dev/null -w "%{http_code}" -X POST "http://localhost:8081/admin/realms/auth9/users" \
  -H "Authorization: Bearer $KC_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"target","email":"target@example.com","firstName":"Target","lastName":"User","enabled":true,"emailVerified":true,"credentials":[{"type":"password","value":"Target123!","temporary":false}]}'

# 获取 Keycloak 分配的用户 ID 并更新数据库
KC_USER_ID=$(curl -s "http://localhost:8081/admin/realms/auth9/users?email=target@example.com" \
  -H "Authorization: Bearer $KC_TOKEN" | jq -r '.[0].id')

mysql -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "UPDATE users SET keycloak_id='$KC_USER_ID' WHERE id='50587266-c621-42d7-9d3d-8fc8e0ed00ef';"

echo "Target user Keycloak ID: $KC_USER_ID"
```

说明：
- `seed.sql` 会创建目标用户的会话数据和历史登录记录（用于安全告警检测）
- 管理员：`admin@auth9.local`
- 目标用户：`target@example.com`（密码：`Target123!`）
- Step 2 确保用户同时存在于 Keycloak 和数据库中，并同步 keycloak_id
- seed.sql 预置的登录记录使 new_device 和 impossible_travel 检测能在本地环境触发

---

## 数据库表结构参考

### security_alerts 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 相关用户 ID |
| tenant_id | CHAR(36) | 租户 ID |
| alert_type | ENUM | brute_force/new_device/impossible_travel/suspicious_ip |
| severity | ENUM | low/medium/high/critical |
| details | JSON | 告警详情 |
| resolved_at | TIMESTAMP | 解决时间 |
| resolved_by | CHAR(36) | 解决人 ID |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：暴力破解告警

### 初始状态
- 同一用户短时间内多次登录失败

### 目的
验证暴力破解检测和告警

### 测试操作流程
1. 对同一账户连续尝试错误密码 10+ 次

### 预期结果
- 触发暴力破解告警
- 告警出现在列表中

### 预期数据状态
```sql
SELECT alert_type, severity, details, created_at FROM security_alerts
WHERE user_id = '{user_id}' AND alert_type = 'brute_force' ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在记录，severity = 'high'
```

---

## 场景 2：新设备登录告警

### 初始状态
- 用户已有至少一条成功登录记录（seed.sql 已预置）
- 用户从未在当前设备/浏览器登录过

### 目的
验证新设备登录检测

### 前置条件说明
新设备检测要求用户已有登录历史（`!known_fingerprints.is_empty()`），首次注册用户不会触发此告警。
seed.sql 已预置一条来自不同设备（IP: 203.0.113.10）的历史登录记录。

### 测试操作流程
1. 确认 seed.sql 已执行（包含历史登录记录）
2. 从新设备/新浏览器登录 target@example.com
3. 登录成功

### 预期结果
- 登录成功
- 触发新设备登录告警

### 预期数据状态
```sql
SELECT alert_type, severity, details FROM security_alerts
WHERE user_id = '{user_id}' AND alert_type = 'new_device' ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在记录，severity = 'medium'
```

---

## 场景 3：异地登录告警

### 初始状态
- 用户最近 10 分钟内有一条来自 IP:203.0.113.10 的成功登录（seed.sql 已预置）
- 用户即将从不同 IP/位置登录

### 目的
验证异地登录（Impossible Travel）检测

### 前置条件说明
异地登录检测要求：(1) 上一次登录有 location 数据，(2) 当前登录有不同的 location，(3) 两次登录间隔 < 1 小时。
seed.sql 预置了一条 10 分钟前来自公网 IP (203.0.113.10) 的登录记录（location = "IP:203.0.113.10"）。
本地 Docker 环境中 Keycloak 事件的 IP 为私网地址（如 192.168.65.1），映射为 "Local Network"，
与预置的 "IP:203.0.113.10" 不同，因此会触发 impossible_travel 告警。

### 测试操作流程
1. 确认 seed.sql 已执行（包含 10 分钟前的登录记录）
2. 在本地浏览器中登录 target@example.com

### 预期结果
- 触发 impossible_travel 告警

### 预期数据状态
```sql
SELECT alert_type, severity, details FROM security_alerts
WHERE user_id = '{user_id}' AND alert_type = 'impossible_travel' ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在记录，severity = 'high'
```

---

## 场景 4：解决安全告警

### 初始状态
- 存在未解决的安全告警 id=`{alert_id}`

### 目的
验证告警解决功能

### 测试操作流程
1. 找到目标告警
2. 点击「解决」
3. 添加备注

### 预期结果
- 告警状态变为已解决
- 记录解决人和时间

### 预期数据状态
```sql
SELECT resolved_at, resolved_by FROM security_alerts WHERE id = '{alert_id}';
-- 预期: resolved_at 有值，resolved_by = 当前管理员 ID
```

---

## 场景 5：安全告警过滤

### 初始状态
- 存在多种类型和状态的安全告警

### 目的
验证告警列表过滤功能

### 测试操作流程
1. 打开安全告警页面
2. 测试过滤：
   - 按状态：未解决/已解决/全部
   - 按严重程度：Critical/High/Medium/Low
   - 按类型：暴力破解/新设备/异地登录/可疑IP

### 预期结果
- 每个过滤条件正确显示对应告警

### 预期数据状态
```sql
SELECT COUNT(*) FROM security_alerts WHERE resolved_at IS NULL AND severity IN ('high', 'critical');
-- 用于验证未解决高危告警数量
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
| 1 | 暴力破解告警 | ☐ | | | |
| 2 | 新设备登录告警 | ☐ | | | |
| 3 | 异地登录告警 | ☐ | | | |
| 4 | 解决安全告警 | ☐ | | | |
| 5 | 安全告警过滤 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
