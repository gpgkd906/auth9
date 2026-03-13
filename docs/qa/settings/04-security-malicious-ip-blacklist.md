# 系统设置 - 恶意 IP 黑名单测试

**模块**: 系统设置
**测试范围**: 平台级恶意 IP 黑名单配置、告警触发、输入校验
**场景数**: 5
**优先级**: 高

---

## 背景说明

本功能为平台级全局黑名单，用于拦截已知恶意来源 IP。管理员可在 Portal「设置」→「安全设置」中维护黑名单，系统在登录事件命中黑名单时生成 `suspicious_ip` 告警，且 `severity = critical`。

相关端点：

- `GET /api/v1/system/security/malicious-ip-blacklist`
- `PUT /api/v1/system/security/malicious-ip-blacklist`
- `POST /api/v1/keycloak/events`

---

## 数据库表结构参考

### malicious_ip_blacklist 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| ip_address | VARCHAR(45) | 黑名单 IP |
| reason | VARCHAR(255) | 黑名单原因 |
| created_by | CHAR(36) | 配置管理员 ID |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### security_alerts 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| alert_type | ENUM | `brute_force` / `slow_brute_force` / `new_device` / `impossible_travel` / `suspicious_ip` |
| severity | ENUM | `low` / `medium` / `high` / `critical` |
| details | JSON | 告警详情 |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：安全设置入口可见性与黑名单列表加载

### 初始状态
- 平台管理员已登录
- 当前环境可访问 Portal

### 目的
验证用户可以从可见导航进入黑名单管理页面，而不是依赖手输 URL。

### 测试操作流程
1. 在左侧导航点击「设置」
2. 确认设置页中存在「安全设置」入口并点击
3. 确认页面中存在「恶意 IP 黑名单」卡片
4. 观察黑名单输入框是否加载当前已保存 IP

### 预期结果
- 「设置」入口可见
- 「安全设置」入口可见且可点击
- 页面显示「恶意 IP 黑名单」区域
- 输入框中展示当前黑名单内容（每行一个 IP）

### 预期数据状态
```sql
SELECT COUNT(*) AS total FROM malicious_ip_blacklist;
-- 预期: 页面加载成功时，输入框中的 IP 行数与 total 一致
```

---

## 场景 2：管理员保存平台级恶意 IP 黑名单

### 初始状态
- 平台管理员已登录
- 当前黑名单可为空

### 目的
验证管理员可通过 UI 保存平台级恶意 IP 黑名单。

### 测试操作流程
1. 从左侧导航进入「设置」→「安全设置」
2. 在「恶意 IP 黑名单」输入框中填写：
   - `203.0.113.10`
   - `198.51.100.24`
3. 点击「保存黑名单」

### 预期结果
- 页面显示保存成功提示
- 刷新页面后仍能看到相同 IP 列表
- 黑名单按一行一个 IP 存储

### 预期数据状态
```sql
SELECT ip_address
FROM malicious_ip_blacklist
ORDER BY ip_address;
-- 预期: 返回 198.51.100.24 和 203.0.113.10
```

---

## 场景 3：非法 IP 输入被拒绝

### 初始状态
- 平台管理员已登录

### 目的
验证后端会拒绝非法 IP 输入，避免脏数据进入黑名单表。

### 测试操作流程
1. 进入「设置」→「安全设置」
2. 在「恶意 IP 黑名单」输入框中填写：
   - `203.0.113.10`
   - `not-an-ip`
3. 点击「保存黑名单」

### 预期结果
- 页面显示错误提示
- 保存失败
- 数据库中不出现 `not-an-ip`

### 预期数据状态
```sql
SELECT COUNT(*) AS invalid_count
FROM malicious_ip_blacklist
WHERE ip_address = 'not-an-ip';
-- 预期: invalid_count = 0
```

---

## 场景 4：黑名单 IP 触发 suspicious_ip critical 告警

### 初始状态
- 平台管理员已登录
- 黑名单中已存在 `203.0.113.10`

### 目的
验证命中平台级恶意 IP 黑名单时，系统生成 `suspicious_ip` 且严重度为 `critical`。

### 步骤 0: 验证环境状态

```bash
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
echo "$SECRET"
# 预期: 输出非空密钥；若为空，先补齐 auth9-core 的 webhook 签名配置
```

### 测试操作流程
1. 确认场景 2 已保存 `203.0.113.10`
2. 发送一条来自黑名单 IP 的登录失败事件：

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"550e8400-e29b-41d4-a716-446655440000","ipAddress":"203.0.113.10","error":"invalid_user_credentials","time":'$(date +%s000)',"details":{"username":"target@example.com","email":"target@example.com"}}'
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')
curl -s -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$SIG" \
  -d "$BODY"
```

3. 进入「安全告警」页面或直接查询数据库

### 预期结果
- 生成新的 `suspicious_ip` 告警
- 告警严重度为 `critical`
- `details.detection_reason = "ip_blacklist"`

### 预期数据状态
```sql
SELECT alert_type,
       severity,
       JSON_EXTRACT(details, '$.ip_address') AS ip_address,
       JSON_EXTRACT(details, '$.detection_reason') AS detection_reason
FROM security_alerts
WHERE alert_type = 'suspicious_ip'
ORDER BY created_at DESC
LIMIT 1;
-- 预期: severity = 'critical', ip_address = "203.0.113.10", detection_reason = "ip_blacklist"
```

---

## 场景 5：重复 IP 与空行保存时自动去重

### 初始状态
- 平台管理员已登录

### 目的
验证保存逻辑会忽略空行，并对重复 IP 自动去重。

### 测试操作流程
1. 进入「设置」→「安全设置」
2. 在输入框中填写：
   - `203.0.113.10`
   - 空行
   - `203.0.113.10`
3. 点击「保存黑名单」

### 预期结果
- 页面显示保存成功提示
- 黑名单中仅保留一条 `203.0.113.10`
- 空行不会写入数据库

### 预期数据状态
```sql
SELECT COUNT(*) AS duplicate_count
FROM malicious_ip_blacklist
WHERE ip_address = '203.0.113.10';
-- 预期: duplicate_count = 1
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
| 1 | 安全设置入口可见性与黑名单列表加载 | ☐ | | | |
| 2 | 管理员保存平台级恶意 IP 黑名单 | ☐ | | | |
| 3 | 非法 IP 输入被拒绝 | ☐ | | | |
| 4 | 黑名单 IP 触发 suspicious_ip critical 告警 | ☐ | | | |
| 5 | 重复 IP 与空行保存时自动去重 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
