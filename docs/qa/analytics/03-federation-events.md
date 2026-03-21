# 联邦审计与安全事件 QA

> **模块**: analytics / identity-provider
> **关联 FR**: Phase4 FR5 — 联邦审计与安全事件
> **前置条件**: 系统已启动且 API 可访问，至少配置一个 Social Provider (Google/GitHub) 和一个 Enterprise SSO Connector (OIDC/SAML)
>
> **重要**: 联邦事件 (`federation_success`, `federation_failed`, `identity_linked`, `identity_unlinked`) 只有在 **实际执行对应操作** 后才会产生。仅查询数据库不构成测试——必须先完成真实的社交登录/SAML SSO/身份绑定流程。种子数据中的 `success`/`social`/`failed_password` 事件是基础登录事件，不包含联邦事件。

---

## 步骤 0: Gate Check

```bash
# 获取管理员令牌
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 验证 API 可访问
curl -sf http://localhost:8080/health && echo "OK"

# 验证 login_events 表已包含新列
docker exec auth9-tidb mysql -uroot -P4000 -e \
  "DESCRIBE auth9.login_events;" 2>/dev/null | grep -E "provider_alias|provider_type"
```

**预期**: `provider_alias` 和 `provider_type` 列存在于 `login_events` 表中。

---

## 场景 1: Social 登录成功产出 federation_success 事件

**目标**: 验证通过社交登录 (如 Google) 成功登录后，Auth9 在 `login_events` 表中写入 `federation_success` 事件，并包含 `provider_alias` 和 `provider_type`。

**步骤**:

1. 通过浏览器执行完整的社交登录流程（Google OAuth）
2. 登录成功后，查询数据库

**预期数据状态**:

```sql
SELECT id, user_id, event_type, provider_alias, provider_type, failure_reason, created_at
FROM auth9.login_events
WHERE event_type = 'federation_success'
ORDER BY created_at DESC
LIMIT 5;
```

| 字段 | 预期值 |
|------|--------|
| event_type | `federation_success` |
| provider_alias | 对应的 provider alias (如 `google`) |
| provider_type | `google` / `github` / `oidc` |
| failure_reason | `NULL` |

---

## 场景 2: SAML 断言验证失败产出 federation_failed 事件

**目标**: 验证当 Enterprise SAML IdP 返回无效断言时，Auth9 记录 `federation_failed` 事件，并包含具体失败原因。

**步骤**:

1. 配置一个 SAML Connector 使用错误的 `entityId`
2. 触发 Enterprise SSO 登录
3. IdP 返回的 Issuer 与配置不匹配，验证失败

**预期数据状态**:

```sql
SELECT id, event_type, provider_alias, provider_type, failure_reason, created_at
FROM auth9.login_events
WHERE event_type = 'federation_failed'
ORDER BY created_at DESC
LIMIT 5;
```

| 字段 | 预期值 |
|------|--------|
| event_type | `federation_failed` |
| provider_type | `saml` |
| failure_reason | `invalid_issuer` / `invalid_audience` / `invalid_assertion` / `assertion_expired` |

---

## 场景 3: 身份绑定产出 identity_linked 事件

**目标**: 验证用户手动绑定社交身份后，Auth9 记录 `identity_linked` 事件。

**步骤**:

1. 用户已登录，访问身份管理页面
2. 点击绑定社交账号 (如 GitHub)
3. 完成 OAuth 流程

**预期数据状态**:

```sql
SELECT id, user_id, event_type, provider_alias, provider_type, created_at
FROM auth9.login_events
WHERE event_type = 'identity_linked'
ORDER BY created_at DESC
LIMIT 5;
```

| 字段 | 预期值 |
|------|--------|
| event_type | `identity_linked` |
| user_id | 当前用户 UUID |
| provider_alias | 绑定的 provider alias |
| provider_type | `google` / `github` / `oidc` / `saml` |

---

## 场景 4: 身份解绑产出 identity_unlinked 事件和审计日志

**目标**: 验证用户解绑社交身份后，Auth9 同时记录 `identity_unlinked` 事件和审计日志。

**步骤**:

1. 获取用户的已绑定身份列表

```bash
curl -s http://localhost:8080/api/v1/users/me/linked-identities \
  -H "Authorization: Bearer $TOKEN" | jq
```

2. 解绑身份

```bash
IDENTITY_ID="<从步骤1获取的身份ID>"
curl -s -X DELETE "http://localhost:8080/api/v1/users/me/linked-identities/$IDENTITY_ID" \
  -H "Authorization: Bearer $TOKEN" | jq
```

**预期响应**: `{"message": "Identity unlinked successfully."}`

**预期数据状态**:

```sql
-- 验证 identity_unlinked 事件
SELECT id, user_id, event_type, provider_alias, provider_type
FROM auth9.login_events
WHERE event_type = 'identity_unlinked'
ORDER BY created_at DESC
LIMIT 1;

-- 验证审计日志
SELECT id, action, resource_type, resource_id
FROM auth9.audit_logs
WHERE action = 'identity.unlinked'
ORDER BY created_at DESC
LIMIT 1;
```

| 表 | 字段 | 预期值 |
|----|------|--------|
| login_events | event_type | `identity_unlinked` |
| audit_logs | action | `identity.unlinked` |
| audit_logs | resource_type | `linked_identity` |

---

## 场景 5: 联邦事件统计包含 federation_success / federation_failed

**目标**: 验证 Analytics API 的统计数据正确包含联邦事件。

**步骤**:

```bash
# 获取过去 7 天的登录统计
# 注意: 正确的端点是 /api/v1/analytics/login-stats（不是 /stats）
curl -s "http://localhost:8080/api/v1/analytics/login-stats?period=weekly" \
  -H "Authorization: Bearer $TOKEN" | jq
```

**预期**: `successful_logins` 计数包含 `federation_success` 事件，`failed_logins` 计数包含 `federation_failed` 事件。`by_event_type` 字典中包含 `federation_success` 和/或 `federation_failed` 键（如果有相应事件）。
