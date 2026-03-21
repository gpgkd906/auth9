# 授权安全 - 租户隔离测试

**模块**: 授权安全
**测试范围**: 多租户数据隔离
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-AUTHZ-01
**OWASP ASVS 5.0**: V8.1,V8.2,V4.2
**回归任务映射**: Backlog #2, #20


---

## 背景知识

Auth9 是多租户系统，核心隔离要求：
- 用户只能访问其所属租户的数据
- 租户管理员只能管理本租户资源
- 平台管理员可跨租户操作

关键数据表：
- `tenants` - 租户信息
- `tenant_users` - 用户-租户关联
- `services` - 租户下的服务
- `roles` / `permissions` - 租户下的 RBAC

> **⚠️ 关键测试要求：平台管理员绕过**
>
> `admin@auth9.local` 是默认平台管理员（由 `PLATFORM_ADMIN_EMAILS` 环境变量配置）。
> **平台管理员在策略引擎中拥有全局绕过权限**，可以跨租户访问所有数据、创建租户、访问系统设置等。
> 这是设计行为，不是安全漏洞。
>
> **测试租户隔离时，必须使用非平台管理员账号。** 使用 `admin@auth9.local` 测试会导致所有隔离检查被绕过，产生误报。

---

## 故障排除

| 现象 | 原因 | 解决方法 |
|------|------|----------|
| 所有跨租户请求返回 200 而非 403 | 使用了平台管理员账号（如 `admin@auth9.local`） | 创建非平台管理员的测试用户，用该用户的 token 测试 |
| Token 邮箱非管理员，但仍绕过隔离 | Token 的 `sub`（user_id）复用了管理员用户的 ID，DB 查询发现该 user_id 在 auth9-platform 租户中有 admin 角色 | **Token 的 user_id 也不能使用管理员用户的 ID**。使用随机 UUID 或非管理员用户的真实 ID |
| Token 无 tenant_id 却能访问租户数据 | Identity Token 被平台管理员绕过识别 | 确认测试账号不在 `PLATFORM_ADMIN_EMAILS` 列表中 |
| 租户管理员能执行平台操作 | 该用户同时是平台管理员 | 检查 `PLATFORM_ADMIN_EMAILS` 配置，确保测试用户不在其中 |
| **`gen-test-tokens.js tenant-owner` 生成的 token 可访问系统设置** | **`tenant-owner` 类型硬编码使用 `admin@auth9.local` 邮箱**（平台管理员），返回 200 是正确行为 | **测试非管理员场景时，使用 `tenant-access` 类型**（邮箱为 `regular-user@example.com`），而非 `tenant-owner` |

---

## 场景 1：跨租户数据访问 (IDOR)

### 前置条件
- **用户 A 必须是非平台管理员**（邮箱不在 `PLATFORM_ADMIN_EMAILS` 中，默认不能是 `admin@auth9.local`）
- 用户 A 属于租户 1
- 用户 A 不属于租户 2
- 租户 2 存在数据

### 攻击目标
验证是否可以访问其他租户的数据

### 攻击步骤
1. 以用户 A 身份登录
2. 尝试访问租户 2 的资源：
   - `GET /api/v1/tenants/{tenant_2_id}`
   - `GET /api/v1/tenants/{tenant_2_id}/users`
   - `GET /api/v1/services?tenant_id={tenant_2_id}`
3. 尝试枚举租户 ID

### 预期安全行为
- 返回 403 Forbidden
- 不泄露租户是否存在
- 审计日志记录访问尝试

### 验证方法
```bash
# 获取用户 A 的 Token (属于租户 1)
TOKEN_A="..."

# 尝试访问租户 2
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/tenants/{tenant_2_id}
# 预期: 403 {"error": "Access denied"}

# 尝试列出租户 2 的用户
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/tenants/{tenant_2_id}/users
# 预期: 403

# 检查审计日志
SELECT * FROM audit_logs
WHERE action = 'access_denied'
ORDER BY created_at DESC LIMIT 10;
```

### 修复建议
- 所有 API 检查租户归属
- 使用 JWT 中的 tenant_id 而非请求参数
- 实现租户作用域中间件
- 记录所有跨租户访问尝试

---

## 场景 2：批量操作租户泄露

### 前置条件
- **用户必须是非平台管理员**（邮箱不在 `PLATFORM_ADMIN_EMAILS` 中，默认不能是 `admin@auth9.local`）
- 用户拥有 demo 租户的 Tenant Access Token
- **⚠️ 严禁使用 Platform Admin Token 测试此场景** — Platform Admin 拥有全局绕过权限，会导致所有列表 API 返回全量数据，产生误报

### 攻击目标
验证列表 API 是否泄露其他租户数据

### 攻击步骤
1. 调用各种列表 API：
   - `GET /api/v1/users`
   - `GET /api/v1/services`
   - `GET /api/v1/roles`
2. 检查返回数据是否仅限当前租户
3. 尝试通过分页/过滤枚举其他租户数据

> **注意**: `GET /api/v1/audit-logs` 不在此场景测试范围内。审计日志 API 在策略引擎中要求 `PlatformAdmin` 权限（非平台管理员会收到 403），且审计日志按设计为全局可见（无 tenant_id 列），不适用租户隔离测试。

### 预期安全行为
- 列表 API 自动过滤为当前租户
- 不返回其他租户的任何数据
- 分页不暴露总数信息

### 验证方法
```bash
# ⚠️ 必须使用非 Platform Admin 的普通租户用户 Token
# 可通过 gen-test-tokens.js 生成指定用户的 tenant-access token:
TENANT_ID="<demo租户的ID>"
USER_ID="<非admin用户的ID>"
TOKEN=$(.claude/skills/tools/gen-test-tokens.js tenant-access --tenant-id "$TENANT_ID" --user-id "$USER_ID")

# 列出用户
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users
# 验证: 所有返回用户都属于当前租户

# 列出服务
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services
# 验证: 所有返回服务都属于当前租户

# SQL 验证
SELECT COUNT(*) FROM (
  -- API 返回的用户 ID 列表
) AS api_users
WHERE user_id NOT IN (
  SELECT user_id FROM tenant_users WHERE tenant_id = 'current_tenant'
);
# 预期: 0
```

### 常见误报原因

| 症状 | 原因 | 解决 |
|------|------|------|
| 所有列表返回全量数据 | 使用了 Platform Admin 用户的 Token | 换用非 Platform Admin 的普通租户用户，使用 gen-test-tokens.js 生成 |
| audit-logs 返回所有日志 | 审计日志是平台级资源，按设计对平台管理员全局可见 | 此 API 不属于租户隔离测试范围 |
| Token 邮箱非管理员但仍绕过隔离 | Token 的 user_id 复用了管理员用户的 ID | Token 的 user_id 也不能使用管理员用户的 ID |

### 修复建议
- 所有查询默认添加租户过滤
- Repository 层强制租户隔离
- 使用租户作用域的数据库连接
- 单元测试覆盖隔离逻辑

---

## 场景 3：关联资源跨租户访问

### 前置条件
- **用户 A 必须是非平台管理员**（邮箱不在 `PLATFORM_ADMIN_EMAILS` 中）
- 用户 A 属于租户 1
- 租户 2 下有服务、角色等资源

### 攻击目标
验证关联资源的跨租户访问

### 攻击步骤
1. 获取租户 2 的服务 ID
2. 尝试操作该服务：
   - `GET /api/v1/services/{tenant_2_service_id}`
   - `PUT /api/v1/services/{tenant_2_service_id}`
   - `GET /api/v1/services/{tenant_2_service_id}/roles`
3. 尝试将角色分配给租户 2 的用户

### 预期安全行为
- 所有资源访问检查租户归属
- 关联操作验证双方租户一致性
- 返回 403 或 404

### 验证方法
```bash
# ⚠️ 必须使用非平台管理员用户的 Token！
# 使用 gen-test-tokens.js 的 tenant-access 类型（email 为 regular-user@example.com）
# 严禁使用 tenant-owner 类型（email 为 admin@auth9.local，会触发平台管理员绕过）
TOKEN_A=$(node .claude/skills/tools/gen-test-tokens.js tenant-access --tenant-id "<租户1_ID>")

# 创建测试服务在另一个租户
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "INSERT INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status, created_at, updated_at) VALUES ('99999999-9999-9999-9999-999999999999', '<租户2_ID>', 'Cross-Tenant Test Service', 'http://localhost:9999', '[]', '[]', 'active', NOW(), NOW());"

# 访问其他租户的服务
curl -s -w "\nHTTP: %{http_code}" -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/services/99999999-9999-9999-9999-999999999999
# 预期: 403 Forbidden

# 尝试为其他租户用户分配角色
curl -X POST -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/rbac/assign \
  -d '{
    "user_id": "'$TENANT_2_USER_ID'",
    "tenant_id": "'$TENANT_1_ID'",
    "role_id": "'$ROLE_ID'"
  }'
# 预期: 400 "User not in tenant"

# 清理测试数据
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "DELETE FROM services WHERE id = '99999999-9999-9999-9999-999999999999';"
```

### 常见误报

| 症状 | 原因 | 解决 |
|------|------|------|
| 跨租户请求返回 200 而非 403 | 使用了 `tenant-owner` token 类型（email 为 `admin@auth9.local`，是平台管理员） | 使用 `tenant-access` token 类型（email 为 `regular-user@example.com`） |
| Token 的 user_id 是管理员 ID | DB 查询发现该 user_id 在 auth9-platform 租户有 admin 角色，触发平台管理员绕过 | Token 的 user_id 也不能使用管理员用户的 ID |

### 修复建议
- 资源访问先查询归属租户
- 关联操作验证所有实体租户一致
- 使用数据库约束或应用层检查
- 防止 ID 猜测 (使用 UUID)

---

## 场景 4：管理员权限边界测试

### 前置条件
- **租户 1 的管理员（非平台管理员）**（邮箱不在 `PLATFORM_ADMIN_EMAILS` 中）
- 平台管理员（如 `admin@auth9.local`，仅用于对比验证）

### 攻击目标
验证不同管理员的权限边界

### 攻击步骤
1. 租户管理员尝试：
   - 访问其他租户
   - 创建新租户
   - 访问平台级设置
2. 检查权限边界是否正确

### 预期安全行为
- 租户管理员仅能管理本租户
- 平台管理员才能创建/管理租户
- 系统设置仅平台管理员可访问
- **邮箱配置型平台管理员使用 TenantAccess Token（通过 token exchange 获得）不能创建/删除租户**
  - 租户创建/删除使用 `require_platform_admin_identity` 策略
  - 邮箱型管理员（email 在 `PLATFORM_ADMIN_EMAILS` 中）：仅接受 Identity Token
  - **DB 型管理员（auth9-platform 租户 admin 角色）：允许任何 Token 类型**（包括 TenantAccess Token），这是设计行为
  - 测试时需区分这两种管理员路径，避免误报

### 验证方法
```bash
# 租户管理员尝试创建租户（使用 TenantAccess Token）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name":"New Tenant","slug":"new-tenant"}'
# 预期: 403 "Platform admin required"

# 邮箱型平台管理员使用 TenantAccess Token 尝试创建租户
# （邮箱在 PLATFORM_ADMIN_EMAILS 中，但使用 TenantAccess Token 时此路径被拒绝）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST -H "Authorization: Bearer $PLATFORM_ADMIN_TENANT_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name":"New Tenant","slug":"new-tenant"}'
# 预期: 403 "Platform admin required"
# ⚠️ 注意: 如果该用户同时是 auth9-platform 租户的 admin（DB 型管理员），
# 则 DB 路径允许 TenantAccess Token 创建租户（返回 201）。这是设计行为，不是漏洞。
# 测试此场景时，$PLATFORM_ADMIN_TENANT_TOKEN 的 user_id 不能是 auth9-platform admin。

# 平台管理员使用 Identity Token 创建租户（正确方式）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST -H "Authorization: Bearer $PLATFORM_ADMIN_IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name":"New Tenant","slug":"new-tenant"}'
# 预期: 201 Created

# 租户管理员尝试访问系统设置
# ⚠️ 必须使用非平台管理员的租户管理员 Token
# DB 型管理员（auth9-platform 租户 admin 角色）可通过任何 Token 类型访问系统设置，返回 200（设计行为）
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/system/email
# 预期: 403（仅当用户不是 DB 型管理员时）
# ⚠️ 如果返回 200: 检查该用户是否在 auth9-platform 租户中有 admin 角色
```

### 修复建议
- 明确定义权限层级
- API 层检查调用者角色
- 敏感端点添加额外校验
- 完善权限矩阵文档

---

## 场景 5：租户级恶意 IP 黑名单隔离

### 前置条件
- 租户 A 和租户 B 都存在，且各自有独立用户
- 租户 A 管理员（非平台管理员）已在 `PUT /api/v1/tenants/{tenant_id}/security/malicious-ip-blacklist` 为租户 A 保存 `203.0.113.10`
- 租户 B 未保存该 IP

### 攻击目标
验证安全检测中的黑名单能力不会跨租户泄露或误封。

### 攻击步骤
1. 使用租户 A 用户触发来自 `203.0.113.10` 的 `LOGIN_ERROR` 事件
2. 使用租户 B 用户触发来自相同 IP 的 `LOGIN_ERROR` 事件
3. 查询 `security_alerts` 中最近的 `suspicious_ip` 记录
4. 再在平台级黑名单中加入 `203.0.113.10`，重复第 1 步

### 预期安全行为
- 只有租户 A 命中租户级黑名单时生成 `suspicious_ip`
- 租户 B 不因租户 A 的配置受到影响
- 当平台级与租户级同时存在时，平台级优先
- 告警 `details.blacklist_scope` 明确标识 `tenant` 或 `platform`

### 验证方法
```bash
SECRET="${WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

BODY='{"type":"LOGIN_ERROR","realmId":"auth9","clientId":"auth9-portal","userId":"{tenant_a_user_id}","ipAddress":"203.0.113.10","error":"invalid_user_credentials","time":'$(date +%s000)',"details":{"username":"tenant-a@example.com","email":"tenant-a@example.com"}}'
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | awk '{print $NF}')
curl -s -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$SIG" \
  -d "$BODY"

mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT tenant_id,
       JSON_EXTRACT(details, '$.blacklist_scope') AS blacklist_scope
FROM security_alerts
WHERE alert_type = 'suspicious_ip'
ORDER BY created_at DESC
LIMIT 5;"
```

### 修复建议
- 所有安全检测命中逻辑必须带 `tenant_id` 作用域
- 平台级规则与租户级规则显式分层，避免顺序漂移
- 告警详情保留 `blacklist_scope`，便于审计与排障

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 跨租户数据访问 (IDOR) | ☐ | | | |
| 2 | 批量操作租户泄露 | ☐ | | | |
| 3 | 关联资源跨租户访问 | ☐ | | | |
| 4 | 管理员权限边界测试 | ☐ | | | |
| 5 | 租户级恶意 IP 黑名单隔离 | ☐ | | | |

---

## 参考资料

- [OWASP IDOR Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Insecure_Direct_Object_Reference_Prevention_Cheat_Sheet.html)
- [CWE-639: Authorization Bypass Through User-Controlled Key](https://cwe.mitre.org/data/definitions/639.html)
- [Multi-tenancy Security Best Practices](https://docs.microsoft.com/en-us/azure/architecture/guide/multitenant/considerations/security)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTHZ-01  
**适用控制**: V8.1,V8.2,V4.2  
**关联任务**: Backlog #2, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-AUTHZ-01-C01 | 控制: V8.1 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-01-C02 | 控制: V8.2 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-01-C03 | 控制: V4.2 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
