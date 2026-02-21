# 用户管理 - 密码策略测试

**模块**: 用户管理
**测试范围**: 密码策略配置、强制执行、账户锁定
**场景数**: 5
**优先级**: 高

---

## 背景

Auth9 支持租户级别的密码策略配置（`tenants.password_policy` JSON 字段），包括：
- 最小长度要求
- 字符类型要求（大写、小写、数字、特殊字符）
- 密码年龄限制
- 密码历史检查（防止重用）
- 账户锁定策略

---

## 场景 1：最小长度和字符类型要求

### 初始状态
- 租户已配置密码策略：
  ```json
  {
    "min_length": 12,
    "require_uppercase": true,
    "require_lowercase": true,
    "require_number": true,
    "require_symbol": true
  }
  ```

### 目的
验证密码创建时强制执行策略

### 测试操作流程
1. 尝试创建用户，使用弱密码 `password`
2. 尝试使用 `Password1`（缺少特殊字符）
3. 尝试使用 `Password!`（缺少数字）
4. 尝试使用 `Pass1!`（长度不足 12）
5. 使用符合要求的密码 `MySecurePass123!`

### 预期结果
- 步骤 1-4 都失败，返回 400 错误，错误信息具体说明缺少什么
- 步骤 5 成功创建用户

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE email = 'policy-test@example.com';
-- 预期: 1（仅步骤 5 成功）

-- 检查 Keycloak 用户密码是否同步
```

---

## 场景 2：密码历史检查（防止重用）

> **架构说明**: Auth9 采用 Headless Keycloak 架构，密码历史检查由 Keycloak 的
> `passwordHistory(N)` 策略执行。Auth9 负责将 `history_count` 同步到 Keycloak realm
> 密码策略字符串中。Auth9 侧不存储密码历史（无 `password_history` 表），所有密码
> 存储和历史比对由 Keycloak 管理。

### 初始状态
- 租户密码策略配置：
  ```json
  {
    "history_count": 5
  }
  ```
- 用户已存在，通过 Keycloak 有密码修改历史

### 目的
验证 Auth9 将 `history_count` 同步到 Keycloak 的 `passwordHistory` 策略

### 测试操作流程
1. 通过 Auth9 API 设置密码策略（`PUT /api/v1/tenants/{id}/password-policy`）
2. 验证 Keycloak realm 密码策略字符串包含 `passwordHistory(5)`
3. 用户通过 Keycloak OIDC 登录后尝试修改密码为历史密码

### 验证方式
```bash
# 验证 Keycloak 策略同步
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -d "grant_type=password&client_id=admin-cli&username=admin&password=admin" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

curl -s "http://localhost:8081/admin/realms/auth9" \
  -H "Authorization: Bearer $KC_TOKEN" \
  | python3 -c "import sys,json; print(json.load(sys.stdin).get('passwordPolicy',''))"
# 预期包含: passwordHistory(5)
```

### 预期结果
- 步骤 2: Keycloak 策略字符串包含 `passwordHistory(5)`
- 步骤 3: Keycloak 拒绝重用密码（由 Keycloak 直接返回错误）

### 预期数据状态
```sql
-- Auth9 侧验证密码策略配置
SELECT JSON_EXTRACT(password_policy, '$.history_count') AS history_count
FROM tenants WHERE slug = 'test-tenant';
-- 预期: 5
```

---

## 场景 3：密码年龄限制（强制定期修改）

> **架构说明**: Auth9 采用 Headless Keycloak 架构，密码年龄限制由 Keycloak 的
> `forceExpiredPasswordChange(N)` 策略执行。Auth9 负责策略配置同步。
> 登录通过 Keycloak OIDC 流程，密码过期时 Keycloak 会显示 "UPDATE_PASSWORD" 页面。

### 初始状态
- 租户密码策略：
  ```json
  {
    "max_age_days": 90
  }
  ```
- 用户密码设置于 100 天前

### 目的
验证 Auth9 将 `max_age_days` 同步到 Keycloak 的 `forceExpiredPasswordChange` 策略

### 测试操作流程
1. 通过 Auth9 API 设置密码策略（`PUT /api/v1/tenants/{id}/password-policy`）
2. 验证 Keycloak realm 密码策略包含 `forceExpiredPasswordChange(90)`
3. 用户通过 Auth9 登录入口触发 OIDC 流程尝试登录（底层由 Keycloak 执行）
4. Keycloak 应显示 "UPDATE_PASSWORD" required action 页面
5. 用户修改密码后成功登录

### 验证方式
```bash
# 验证 Keycloak 策略同步
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -d "grant_type=password&client_id=admin-cli&username=admin&password=admin" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

curl -s "http://localhost:8081/admin/realms/auth9" \
  -H "Authorization: Bearer $KC_TOKEN" \
  | python3 -c "import sys,json; print(json.load(sys.stdin).get('passwordPolicy',''))"
# 预期包含: forceExpiredPasswordChange(90)
```

### 预期结果
- 步骤 2: Keycloak 策略字符串包含 `forceExpiredPasswordChange(90)`
- 步骤 4: 密码过期用户看到 Keycloak 密码修改页面（非 auth9 错误）
- 步骤 5: 修改后成功登录

### 预期数据状态
```sql
SELECT password_changed_at FROM users WHERE id = '{user_id}';
-- 预期: 当前时间（最新修改时间）

-- 检查审计日志
SELECT action, details FROM audit_logs
WHERE resource_type = 'user'
  AND resource_id = '{user_id}'
  AND action = 'password_change'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 最新一条密码修改记录
```

---

## 场景 4：账户锁定策略（暴力破解防护）

> **架构说明**: Auth9 采用 Headless Keycloak 架构，账户锁定由 Keycloak 的 Brute Force
> Protection 执行。Auth9 负责将 `lockout_threshold` 和 `lockout_duration_mins` 同步到
> Keycloak realm 设置。登录事件默认通过 Redis Stream 异步传递到 auth9-core 消费链路，
> 再触发安全检测（兼容 webhook 入口用于回归）。

### 初始状态
- 租户密码策略：
  ```json
  {
    "lockout_threshold": 5,
    "lockout_duration_mins": 30
  }
  ```
- 用户账户正常，无锁定

### 目的
验证 Auth9 将锁定策略同步到 Keycloak，Keycloak 正确执行暴力破解防护

### 测试操作流程
1. 通过 Auth9 API 设置密码策略（`PUT /api/v1/tenants/{id}/password-policy`）
2. 验证 Keycloak realm 的 brute force 配置正确
3. 通过 Auth9 登录入口触发 OIDC 流程，使用错误密码尝试 5 次
4. 第 6 次使用正确密码尝试登录
5. 检查 Keycloak 登录事件是否传递到 auth9（Stream 主链路）

### 验证方式
```bash
# 验证 Keycloak brute force 配置
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -d "grant_type=password&client_id=admin-cli&username=admin&password=admin" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

curl -s "http://localhost:8081/admin/realms/auth9" \
  -H "Authorization: Bearer $KC_TOKEN" \
  | python3 -c "
import sys,json
r = json.load(sys.stdin)
print('bruteForceProtected:', r.get('bruteForceProtected'))
print('failureFactor:', r.get('failureFactor'))
print('maxFailureWaitSeconds:', r.get('maxFailureWaitSeconds'))
print('waitIncrementSeconds:', r.get('waitIncrementSeconds'))
"
# 预期: bruteForceProtected=True, failureFactor=5, waitIncrementSeconds=1800
```

### 预期结果
- 步骤 2: Keycloak realm `bruteForceProtected=true`, `failureFactor=5`, `maxFailureWaitSeconds=1800`
- 步骤 4: Keycloak 返回 `user_disabled` 错误（账户被 Keycloak 暂时锁定）
- 步骤 5: Keycloak 登录事件触发 auth9 `login_events` 和 `security_alerts` 记录

### 预期数据状态
```sql
-- 检查安全告警（由 Keycloak 登录事件触发）
SELECT alert_type, severity FROM security_alerts
WHERE user_id = '{user_id}'
  AND alert_type = 'brute_force'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 条记录，severity = 'high'

-- 检查登录事件（由 Keycloak 登录事件触发）
SELECT event_type, COUNT(*) FROM login_events
WHERE user_id = '{user_id}'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 1 HOUR)
GROUP BY event_type;
-- 预期: failed_password: 5+
```

---

## 场景 5：管理员绕过密码策略（特殊场景）

> **架构说明**: Auth9 管理员通过 `PUT /api/v1/users/{id}/password` 设置密码，
> 内部调用 Keycloak 用户更新 API 设置 credentials。`temporary: true` 会在
> Keycloak 用户上添加 `UPDATE_PASSWORD` required action，用户通过 Keycloak
> OIDC 登录时会被强制修改密码。

### 初始状态
- 租户密码策略要求 12 位，包含大小写数字特殊字符
- 平台管理员需要为用户设置临时密码

### 目的
验证管理员可以设置临时弱密码，Keycloak 强制用户首次登录修改

### 测试操作流程
1. 管理员通过 Auth9 API 为新用户设置临时密码 `Temp123!`（`temporary: true`）
2. 验证 Keycloak 用户的 `requiredActions` 包含 `UPDATE_PASSWORD`
3. 用户通过 Auth9 登录入口触发 OIDC 流程，使用临时密码登录
4. Keycloak 强制显示密码修改页面
5. 用户修改为符合策略的密码 `MyNewPassword456!`

### 验证方式
```bash
# 设置临时密码后验证 Keycloak 用户状态
KC_TOKEN=$(curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
  -d "grant_type=password&client_id=admin-cli&username=admin&password=admin" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")

curl -s "http://localhost:8081/admin/realms/auth9/users/{keycloak_user_id}" \
  -H "Authorization: Bearer $KC_TOKEN" \
  | python3 -c "import sys,json; print('requiredActions:', json.load(sys.stdin).get('requiredActions',[]))"
# 预期: requiredActions: ['UPDATE_PASSWORD']
```

### 预期结果
- 步骤 1: Auth9 API 返回成功（管理员可以绕过策略）
- 步骤 2: Keycloak 用户 `requiredActions` 包含 `UPDATE_PASSWORD`
- 步骤 3-4: Keycloak OIDC 登录接受临时密码但强制跳转到密码修改页面
- 步骤 5: 修改成功，`requiredActions` 清空

### 预期数据状态
```sql
-- 检查审计日志
SELECT action, actor_id FROM audit_logs
WHERE resource_type = 'user'
  AND resource_id = '{user_id}'
  AND action = 'user.password.admin_set'
ORDER BY created_at DESC LIMIT 1;
-- 预期: actor_id 为管理员 ID

SELECT action FROM audit_logs
WHERE resource_type = 'user'
  AND resource_id = '{user_id}'
  AND action = 'password_change'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 用户自己修改密码的记录
```

---

## 测试数据准备

### 设置租户密码策略
```sql
UPDATE tenants SET password_policy = JSON_OBJECT(
  'min_length', 12,
  'require_uppercase', true,
  'require_lowercase', true,
  'require_number', true,
  'require_symbol', true,
  'history_count', 5,
  'max_age_days', 90,
  'lockout_threshold', 5,
  'lockout_duration_mins', 30
)
WHERE slug = 'test-tenant';
```

### 密码历史（Keycloak 管理）
```
密码历史由 Keycloak 的 passwordHistory(N) 策略管理。
Auth9 不存储密码历史（无 password_history 表）。
验证方式：通过 Keycloak Admin API 查看 realm 密码策略字符串。
```

---

## 注意事项

1. **Headless Keycloak 架构**：Auth9 不直接处理用户名/密码登录，所有认证通过 Keycloak OIDC 流程。`/api/v1/auth/token` 仅支持 `authorization_code`、`client_credentials`、`refresh_token` grant types，不支持 `password` grant
2. **密码策略同步**：Auth9 配置的策略通过 `KeycloakSyncService` 同步到 Keycloak realm，包括密码复杂度、年龄限制、历史记录和暴力破解防护
3. **密码设置方式**：使用 Keycloak 用户更新 API（GET-merge-PUT）设置密码，绕过 Keycloak 23.x `reset-password` 端点的已知 bug
4. **登录事件来源**：登录事件默认通过 Redis Stream 异步传递到 auth9-core，而非在 auth9 token 端点直接记录
5. **向后兼容**：现有用户的密码可能不符合新策略，但不强制立即修改
6. **审计合规**：所有密码修改操作必须记录到 `audit_logs`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 配置密码复杂度策略并验证弱密码拒绝 | ☐ | | | |
| 2 | 密码历史策略（禁止复用最近 N 次密码） | ☐ | | | |
| 3 | 密码过期策略（max_age_days） | ☐ | | | |
| 4 | 暴力破解防护（失败次数锁定） | ☐ | | | |
| 5 | 管理员绕过密码策略（特殊场景） | ☐ | | | |
