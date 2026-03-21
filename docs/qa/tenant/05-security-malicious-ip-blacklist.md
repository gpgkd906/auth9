# 租户管理 - 租户级恶意 IP 黑名单测试

**模块**: 租户管理
**测试范围**: 租户详情页安全设置、租户级恶意 IP 黑名单配置、租户隔离与优先级
**场景数**: 5
**优先级**: 高

---

## 背景说明

本功能在现有平台级恶意 IP 黑名单之外，新增租户级恶意 IP 黑名单隔离能力。管理员可在 Portal 的租户详情页「Security Settings」中维护当前租户的黑名单；登录事件命中时，仅影响该租户，且平台级黑名单优先于租户级黑名单。

相关端点：

- `GET /api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist`
- `PUT /api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist`
- `POST /api/v1/keycloak/events`

---

## 数据库表结构参考

### tenant_malicious_ip_blacklist 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 ID |
| ip_address | VARCHAR(45) | 黑名单 IP |
| reason | VARCHAR(255) | 黑名单原因 |
| created_by | CHAR(36) | 配置管理员 ID |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### malicious_ip_blacklist 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| ip_address | VARCHAR(45) | 平台级黑名单 IP |
| reason | VARCHAR(255) | 平台级黑名单原因 |

### security_alerts 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| alert_type | ENUM | `suspicious_ip` 等 |
| severity | ENUM | `low` / `medium` / `high` / `critical` |
| details | JSON | 告警详情，包含 `detection_reason`、`blacklist_scope` |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：租户详情页安全设置入口可见性与黑名单列表加载

### 初始状态
- 租户管理员已登录 Portal
- 系统中已存在租户 `{tenant_id}`，且该租户已保存至少 1 条租户级黑名单 IP

### 目的
验证用户可以从可见 UI 入口进入租户级黑名单功能，而不是依赖手输 URL。

### 测试操作流程
1. 在左侧导航点击「Tenants」
2. 在租户列表中点击目标租户名称进入详情页
3. 确认页面中存在「Security Settings」卡片
4. 确认卡片内存在「Tenant Malicious IP Blacklist」区域
5. 观察输入框是否加载当前租户已保存的 IP 列表

### 预期结果
- 用户可以从「Tenants」列表进入租户详情页
- 租户详情页可见「Security Settings」卡片
- 卡片内存在租户级黑名单输入框和「Save blacklist」按钮
- 输入框仅展示当前租户的黑名单 IP（每行一个）

### 预期数据状态
```sql
SELECT COUNT(*) AS total
FROM tenant_malicious_ip_blacklist
WHERE tenant_id = '{tenant_id}';
-- 预期: 输入框中的 IP 行数与 total 一致
```

---

## 场景 2：租户管理员保存租户级恶意 IP 黑名单

### 初始状态
- 租户管理员已登录 Portal
- 已从「Tenants」列表进入目标租户详情页

### 目的
验证租户管理员可在租户上下文中保存仅对当前租户生效的黑名单。

### 测试操作流程
1. 从「Tenants」列表进入目标租户详情页
2. 在「Security Settings」卡片中的「Tenant Malicious IP Blacklist」输入框填写：
   - `203.0.113.10`
   - `198.51.100.24`
3. 点击「Save blacklist」
4. 刷新页面并重新进入该租户详情页

### 预期结果
- 页面显示保存成功提示
- 刷新后仍能看到相同 IP 列表
- 不会影响其他租户详情页中的黑名单内容

### 预期数据状态
```sql
SELECT tenant_id, ip_address
FROM tenant_malicious_ip_blacklist
WHERE tenant_id = '{tenant_id}'
ORDER BY ip_address;
-- 预期: 返回 198.51.100.24 和 203.0.113.10
```

---

## 场景 3：非法 IP 输入被拒绝

### 初始状态
- 租户管理员已登录 Portal
- 已从「Tenants」列表进入目标租户详情页

### 目的
验证后端拒绝非法 IP，避免脏数据写入租户级黑名单。

### 测试操作流程
1. 进入目标租户详情页「Security Settings」
2. 在「Tenant Malicious IP Blacklist」输入框填写：
   - `203.0.113.10`
   - `not-an-ip`
3. 点击「Save blacklist」

### 预期结果
- 页面显示错误提示
- 保存失败
- 数据库中不出现 `not-an-ip`

### 预期数据状态
```sql
SELECT COUNT(*) AS invalid_count
FROM tenant_malicious_ip_blacklist
WHERE tenant_id = '{tenant_id}' AND ip_address = 'not-an-ip';
-- 预期: invalid_count = 0
```

---

## 场景 4：租户级黑名单仅影响当前租户

### 初始状态
- 租户 A = `{tenant_a_id}`，租户 B = `{tenant_b_id}`
- 用户 A 仅属于租户 A，用户 B 仅属于租户 B
- 租户 A 已保存租户级黑名单 IP `203.0.113.10`
- 租户 B 未保存该 IP

### 目的
验证同一 IP 仅在当前租户命中，不影响其他租户。

### 步骤 0: 验证环境状态

```bash
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
echo "$SECRET"
# 预期: 输出非空；若为空，先补齐 auth9-core 的 webhook 签名配置
```

```sql
-- userId 必须使用 identity_subject，而不是 auth9 users.id
SELECT u.id AS auth9_user_id,
       u.identity_subject,
       u.email,
       COUNT(tu.tenant_id) AS tenant_count
FROM users u
LEFT JOIN tenant_users tu ON tu.user_id = u.id
WHERE u.email = 'tenant-a@example.com'
GROUP BY u.id, u.identity_subject, u.email;
-- 预期:
-- 1. identity_subject 非空，后续 webhook BODY 使用该值
-- 2. tenant_count = 1；若 > 1，本场景不成立，需要先准备单租户测试用户
```

### 测试操作流程
1. 向租户 A 用户发送一条登录失败事件
2. 向租户 B 用户发送同一 IP 的登录失败事件
3. 查询最近的 `suspicious_ip` 告警

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"{tenant_a_user_id}","ipAddress":"203.0.113.10","error":"invalid_user_credentials","time":'$(date +%s000)',"details":{"username":"tenant-a@example.com","email":"tenant-a@example.com"}}'
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')
curl -s -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$SIG" \
  -d "$BODY"
```

### 预期结果
- 租户 A 产生 `suspicious_ip` 告警
- 租户 B 不因租户 A 的黑名单配置而产生同类告警
- 告警 `details.blacklist_scope = "tenant"`

### 预期数据状态
```sql
SELECT tenant_id,
       alert_type,
       severity,
       JSON_EXTRACT(details, '$.blacklist_scope') AS blacklist_scope
FROM security_alerts
WHERE alert_type = 'suspicious_ip'
  AND JSON_EXTRACT(details, '$.ip_address') = '"203.0.113.10"'
ORDER BY created_at DESC
LIMIT 5;
-- 预期: 租户 A 存在 blacklist_scope = "tenant" 的记录；租户 B 不存在同类记录
```

### 常见误报排查

| 现象 | 原因 | 解决 |
|------|------|------|
| webhook 返回 204，但没有生成 `suspicious_ip` 告警 | BODY 里的 `userId` 填了 auth9 `users.id`，不是 `identity_subject` | 先执行「步骤 0」查询 `identity_subject`，并在 webhook 请求中使用它 |
| 登录事件写入了错误租户或 `tenant_id` 为空 | 测试用户属于多个 tenant，本场景“仅属于租户 A/B”的前提不成立 | 重新准备单租户用户，再验证租户级黑名单 |
| 租户黑名单已保存，但命中的是平台级规则或没有任何规则 | 平台级黑名单中也存在同一 IP，或测试前未清理旧规则 | 先确认平台级 `malicious_ip_blacklist` 不包含该 IP，再发事件 |

---

## 场景 5：平台级黑名单优先于租户级黑名单

### 初始状态
- 平台级 `malicious_ip_blacklist` 已存在 `203.0.113.10`
- 当前租户 `{tenant_id}` 的 `tenant_malicious_ip_blacklist` 也存在 `203.0.113.10`
- 当前租户用户 `{tenant_user_id}` 存在且仅属于该租户

### 目的
验证平台级与租户级并存时，平台级命中优先。

### 步骤 0: 验证环境状态

```bash
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
echo "$SECRET"
# 预期: 输出非空；若为空，先补齐 auth9-core 的 webhook 签名配置
```

### 测试操作流程
1. 确认平台级和租户级黑名单均已包含 `203.0.113.10`
2. 发送该租户用户的登录失败事件
3. 查询最新一条 `suspicious_ip` 告警

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"{tenant_user_id}","ipAddress":"203.0.113.10","error":"invalid_user_credentials","time":'$(date +%s000)',"details":{"username":"priority@example.com","email":"priority@example.com"}}'
SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')
curl -s -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$SIG" \
  -d "$BODY"
```

### 预期结果
- 生成新的 `suspicious_ip` 告警
- 告警严重度为 `critical`
- `details.blacklist_scope = "platform"`，不会记录为 `tenant`

### 预期数据状态
```sql
SELECT alert_type,
       severity,
       JSON_EXTRACT(details, '$.blacklist_scope') AS blacklist_scope,
       JSON_EXTRACT(details, '$.detection_reason') AS detection_reason
FROM security_alerts
WHERE alert_type = 'suspicious_ip'
ORDER BY created_at DESC
LIMIT 1;
-- 预期: severity = 'critical', blacklist_scope = "platform", detection_reason = "ip_blacklist"
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页。

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 `auth9_session` cookie
   - 在当前会话点击「Sign out」退出登录
2. 从租户列表重新进入目标租户详情页

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可从「Tenants」列表重新进入目标租户详情页

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 租户详情页安全设置入口可见性与黑名单列表加载 | ☐ | | | |
| 2 | 租户管理员保存租户级恶意 IP 黑名单 | ☐ | | | |
| 3 | 非法 IP 输入被拒绝 | ☐ | | | |
| 4 | 租户级黑名单仅影响当前租户 | ☐ | | | |
| 5 | 平台级黑名单优先于租户级黑名单 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
