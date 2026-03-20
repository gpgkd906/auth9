# SDK - 可观测性与系统配置 API 子客户端

**模块**: SDK
**测试范围**: Auth9Client 的 auditLogs / analytics / securityAlerts / system / emailTemplates / branding 子客户端
**场景数**: 5
**优先级**: 中

---

## 背景说明

### 子客户端架构

`@auth9/core` 的 `Auth9Client` Phase 4 新增 6 个子客户端，覆盖可观测性与系统配置：

| 子客户端 | 方法数 | API 前缀 |
|---------|--------|---------|
| `client.auditLogs` | 1 | `/api/v1/audit-logs` |
| `client.analytics` | 3 | `/api/v1/analytics/login-stats`, `/api/v1/analytics/login-events`, `/api/v1/analytics/daily-trend` |
| `client.securityAlerts` | 2 | `/api/v1/security/alerts` |
| `client.system` | 6 | `/api/v1/system/email`, `/api/v1/system/email/test`, `/api/v1/system/email/send-test`, `/api/v1/system/security/malicious-ip-blacklist` |
| `client.emailTemplates` | 6 | `/api/v1/system/email-templates` |
| `client.branding` | 6 | `/api/v1/system/branding`, `/api/v1/public/branding`, `/api/v1/services/{id}/branding` |

### 前置条件

- auth9-core 运行中 (`http://localhost:8080/health`)
- 已获取有效的 Admin Token（平台管理员）
- `npm run build` 在 `sdk/packages/core` 通过

---

## 步骤 0：Gate Check — 获取 Admin Token 并验证 Build

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $TOKEN | head -c 20
```

**预期**: 输出 JWT token 前 20 个字符（非空）

```bash
cd sdk/packages/core && npm run build && npm run test
```

**预期**: Build 成功，216 个测试全部通过

---

## 场景 1：AuditLogs + Analytics 子客户端 — 日志与统计查询

### 步骤

1. **查询审计日志（无过滤）**

```bash
curl -s http://localhost:8080/api/v1/audit-logs \
  -H "Authorization: Bearer $TOKEN" | jq '.pagination'
```

**预期**: 返回分页结构 `{ page, perPage, total, totalPages }`，`data` 为数组

2. **查询审计日志（带过滤）**

```bash
curl -s "http://localhost:8080/api/v1/audit-logs?resource_type=tenant&per_page=5" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数组长度 ≤ 5

3. **获取登录统计**

```bash
curl -s http://localhost:8080/api/v1/analytics/login-stats \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期**: 返回 `{ totalLogins, successfulLogins, failedLogins, uniqueUsers, ... }` 结构（snake_case）

4. **获取每日趋势**

```bash
curl -s "http://localhost:8080/api/v1/analytics/daily-trend?days=7" \
  -H "Authorization: Bearer $TOKEN" | jq '.data[0]'
```

**预期**: 返回 `{ date, total, successful, failed }` 结构

### 验收检查清单

- [ ] 审计日志分页正常返回
- [ ] 过滤参数生效
- [ ] 登录统计字段完整
- [ ] 每日趋势数据结构正确

---

## 场景 2：SecurityAlerts 子客户端 — 安全告警查询与解决

### 步骤

1. **查询安全告警列表**

```bash
curl -s http://localhost:8080/api/v1/security/alerts \
  -H "Authorization: Bearer $TOKEN" | jq '.pagination'
```

**预期**: 返回分页结构，`data` 数组中每条记录包含 `id`, `alertType`, `severity`, `createdAt`

2. **按严重度过滤告警**

```bash
curl -s "http://localhost:8080/api/v1/security/alerts?severity=high" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数量 ≥ 0，所有记录 severity 为 "high"

3. **解决一条告警（如果存在）**

```bash
ALERT_ID=$(curl -s "http://localhost:8080/api/v1/security/alerts?per_page=1" \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id // empty')

if [ -n "$ALERT_ID" ]; then
  curl -s -X POST "http://localhost:8080/api/v1/security/alerts/$ALERT_ID/resolve" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" | jq '.data.resolved_at'
fi
```

**预期**: 如果有告警，`resolved_at` 非空

### 验收检查清单

- [ ] 告警列表分页返回正确
- [ ] severity 过滤生效
- [ ] resolve 操作返回更新后的告警记录

---

## 场景 3：System 子客户端 — 邮件设置与 IP 黑名单

### 步骤

1. **获取邮件设置**

```bash
curl -s http://localhost:8080/api/v1/system/email \
  -H "Authorization: Bearer $TOKEN" | jq '.data.value.type'
```

**预期**: 返回邮件配置类型（`"smtp"`, `"ses"`, `"oracle"`, 或 `"none"`）

> **注意**: 邮件配置在 `.data.value` 下，不是 `.data.config`。响应结构为 `{ data: { category, setting_key, value: { type, host, ... }, description, updated_at } }`。

2. **测试邮件连接**

```bash
curl -s -X POST http://localhost:8080/api/v1/system/email/test \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" | jq '.success'
```

**预期**: 返回 `true` 或 `false`（取决于邮件服务配置）

3. **获取系统级恶意 IP 黑名单**

```bash
curl -s http://localhost:8080/api/v1/system/security/malicious-ip-blacklist \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回数组（可能为空）

### 验收检查清单

- [ ] 邮件设置返回正确的 value 结构（`.data.value.type`）
- [ ] 邮件测试端点响应正常
- [ ] IP 黑名单端点返回正确结构

---

## 场景 4：EmailTemplates 子客户端 — 模板管理与预览

### 步骤

1. **列出所有邮件模板**

```bash
curl -s http://localhost:8080/api/v1/system/email-templates \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 返回模板数量 ≥ 1（至少包含内置模板如 invitation, password_reset 等）

2. **获取单个模板**

```bash
curl -s http://localhost:8080/api/v1/system/email-templates/invitation \
  -H "Authorization: Bearer $TOKEN" | jq '.data.metadata.template_type'
```

**预期**: 返回 `"invitation"`

3. **预览模板渲染**

```bash
curl -s -X POST http://localhost:8080/api/v1/system/email-templates/invitation/preview \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"subject":"Test {{invite_link}}","html_body":"<h1>Hello</h1>","text_body":"Hello"}' | jq '.data.subject'
```

**预期**: 返回渲染后的 subject 字符串

4. **重置模板（如已自定义）**

```bash
curl -s -X DELETE http://localhost:8080/api/v1/system/email-templates/invitation \
  -H "Authorization: Bearer $TOKEN" | jq '.data.is_customized'
```

**预期**: 返回 `false`（模板已重置为默认）

### 验收检查清单

- [ ] 模板列表包含预期的内置模板
- [ ] 单个模板获取返回完整结构（metadata + content）
- [ ] 预览渲染正常返回
- [ ] 重置操作正确执行

---

## 场景 5：Branding 子客户端 — 品牌配置管理

### 步骤

1. **获取系统品牌配置**

```bash
curl -s http://localhost:8080/api/v1/system/branding \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

**预期**: 返回 `{ primaryColor, secondaryColor, backgroundColor, textColor, allowRegistration, emailOtpEnabled, ... }`

2. **获取公开品牌配置（无需认证）**

```bash
curl -s http://localhost:8080/api/v1/public/branding | jq '.data.primary_color'
```

**预期**: 返回颜色值字符串（如 `"#1a73e8"`）

3. **获取 Service 级品牌配置**

```bash
SERVICE_ID=$(curl -s http://localhost:8080/api/v1/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id // empty')

if [ -n "$SERVICE_ID" ]; then
  curl -s "http://localhost:8080/api/v1/services/$SERVICE_ID/branding" \
    -H "Authorization: Bearer $TOKEN" | jq '.data'
fi
```

**预期**: 返回品牌配置或 404（如果 Service 未设置自定义品牌）

### 验收检查清单

- [ ] 系统品牌配置返回完整结构
- [ ] 公开品牌端点无需认证即可访问
- [ ] Service 级品牌端点正确响应

---

## SDK 单元测试验证

```bash
cd sdk/packages/core && npm run test -- --reporter=verbose 2>&1 | grep -E "(audit|analytics|security-alerts|system|email-templates|branding)"
```

**预期**: 6 个新测试文件全部通过

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | AuditLogs + Analytics 子客户端 — 日志与统计查询 | ☐ | | | |
| 2 | SecurityAlerts 子客户端 — 安全告警查询与解决 | ☐ | | | |
| 3 | System 子客户端 — 邮件设置与 IP 黑名单 | ☐ | | | |
| 4 | EmailTemplates 子客户端 — 模板管理与预览 | ☐ | | | |
| 5 | Branding 子客户端 — 品牌配置管理 | ☐ | | | |
