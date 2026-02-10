# 授权安全 - System/Admin 配置接口授权测试

**模块**: 授权安全  
**测试范围**: `/api/v1/system/*` 与策略类接口的授权校验（防越权配置修改）  
**场景数**: 8  
**风险等级**: 🔴 极高  

---

## 背景知识

Auth9 当前有三类主要 JWT（见 `auth9-core/src/jwt/mod.rs`）:
- **Identity Token**: `aud=auth9`，登录后签发，通常不带租户上下文。
- **Tenant Access Token**: `aud=<service_client_id>`，Token Exchange 后签发，携带 `tenant_id/roles/permissions`。
- **Service Client Token**: `aud=auth9-service`，client_credentials 用于服务到服务。

本测试文档关注 “已认证 (Authentication) 但未授权 (Authorization)” 的越权风险:  
接口如果只验证 token 真伪，却不验证调用者是否具备平台管理员/租户管理员权限，将导致高敏感配置可被任意登录主体篡改。

---

## 测试准备

### 账号/Token 准备

至少准备如下 4 类 token（任一获取方式可行，Portal 登录或脚本均可）:

1. `PLATFORM_ADMIN_ID_TOKEN`: 平台管理员 Identity Token（email 在 `PLATFORM_ADMIN_EMAILS` 中）
2. `TENANT_OWNER_ACCESS_TOKEN`: 目标租户 owner 的 TenantAccess Token（带 `tenant_id`）
3. `TENANT_MEMBER_ACCESS_TOKEN`: 目标租户普通成员 TenantAccess Token（无 admin 权限）
4. `SERVICE_CLIENT_TOKEN`: 某个 service 的 ServiceClient Token（如实现了 client_credentials）

### 目标租户与校验手段

准备:
- `TENANT_ID`: 目标租户 UUID
- 数据库只读校验方式（任选一种）:
  - 直接查询 TiDB 对应表
  - 或通过 GET 接口读取配置对比

---

## 场景 1：普通租户成员越权更新 system 邮件配置

### 攻击目标
验证非平台管理员是否能更新系统邮件 provider 配置（影响全局邮件投递与安全通知）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `PUT /api/v1/system/email`

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"

curl -sS -i -X PUT "http://localhost:8080/api/v1/system/email" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"type":"smtp","host":"attacker.example","port":25,"username":"x","password":"y","use_tls":false,"from_email":"noreply@example.com","from_name":"Auth9"}}'
```

### 预期安全行为
- 返回 `403 Forbidden`
- 数据库/system_settings 不发生变更
- 记录未授权尝试（建议写入 audit log 或 security alert）

---

## 场景 2：普通租户成员越权发送 system 测试邮件

### 攻击目标
验证非平台管理员是否能触发测试邮件发送（可能用于骚扰或探测邮件系统）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `POST /api/v1/system/email/send-test`

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"

curl -sS -i -X POST "http://localhost:8080/api/v1/system/email/send-test" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"to_email":"victim@example.com"}'
```

### 预期安全行为
- 返回 `403 Forbidden`
- 不应向外部发送任何邮件

---

## 场景 3：普通租户成员越权更新 system 品牌配置

### 攻击目标
验证非平台管理员是否能篡改品牌配置（影响 Portal/Keycloak 主题展示与用户信任链）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `PUT /api/v1/system/branding`

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"

curl -sS -i -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"primary_color":"#000000","secondary_color":"#ffffff","background_color":"#ffffff","text_color":"#000000","company_name":"Hacked"}}'
```

### 预期安全行为
- 返回 `403 Forbidden`
- branding 配置不发生变更

---

## 场景 4：普通租户成员越权更新 system 邮件模板

### 攻击目标
验证非平台管理员是否能修改用于邀请/重置密码/安全告警等模板的内容（可用于钓鱼与账号接管辅助）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `PUT /api/v1/system/email-templates/:type`（例如 `invitation`）

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"

curl -sS -i -X PUT "http://localhost:8080/api/v1/system/email-templates/invitation" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"subject":"PWN","html_body":"<p>pwn</p>","text_body":"pwn"}'
```

### 预期安全行为
- 返回 `403 Forbidden`
- 模板内容不变更

---

## 场景 5：普通租户成员越权重置 system 邮件模板

### 攻击目标
验证非平台管理员是否能重置模板（造成业务中断或规避合规模板）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `DELETE /api/v1/system/email-templates/:type`

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"

curl -sS -i -X DELETE "http://localhost:8080/api/v1/system/email-templates/invitation" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期安全行为
- 返回 `403 Forbidden`
- 模板不变化

---

## 场景 6：普通租户成员越权更新租户密码策略

### 攻击目标
验证非 owner/admin 是否能修改租户密码策略（降低安全基线或造成锁号风险）。

### 攻击步骤
使用 `TENANT_MEMBER_ACCESS_TOKEN` 调用:
- `PUT /api/v1/tenants/:id/password-policy`

### 验证方法
```bash
TOKEN="$TENANT_MEMBER_ACCESS_TOKEN"
TENANT_ID="$TENANT_ID"

curl -sS -i -X PUT "http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"min_length":4,"require_uppercase":false,"require_lowercase":false,"require_number":false,"require_symbol":false,"max_age_days":0}'
```

### 预期安全行为
- 返回 `403 Forbidden`
- 密码策略不变更

---

## 场景 7：Service Client Token 越权修改 system 配置

### 攻击目标
验证 `aud=auth9-service` 的服务 token 不能修改任何 system 配置（否则属于高危权限边界破坏）。

### 攻击步骤
使用 `SERVICE_CLIENT_TOKEN` 重放场景 1/3/4 任意请求。

### 预期安全行为
- 返回 `403 Forbidden`

---

## 场景 8：授权成功路径（正向用例）

### 目标
确保正确的主体仍能完成管理操作，避免修复后产生误伤。

### 步骤与预期
1. `PLATFORM_ADMIN_ID_TOKEN` 调用 system 配置更新:
   - 预期: `200/201`，配置成功更新
2. `TENANT_OWNER_ACCESS_TOKEN` 调用租户密码策略更新:
   - 预期: `200/201`，策略成功更新
3. 以上操作应写入 audit log:
   - 预期: `audit_logs` 中出现 `system.email.update`、`system.branding.update`、`tenant.password_policy.update`（事件名可根据实现调整）

---

## 自动化测试建议（工程落地）

建议将上述场景固化为后端集成测试，避免回归:
- 位置建议: `auth9-core/tests/api/http/system_config_authz_http_test.rs`
- 核心断言:
  - 非授权主体: HTTP 403
  - 授权主体: 2xx
  - 配置未变更/已变更的数据库断言（如使用 mock repository，则断言 repo 方法未被调用）

实现要点:
- 测试中构造不同类型 JWT（平台管理员 email 与普通 email），并作为 `Authorization: Bearer` 请求头。
- 对 system handler 统一要求 `AuthUser`（或在 middleware 中注入解析结果）后，才能在编译期降低“忘记做授权”的概率。

---

## 修复建议（与测试联动）

1. `/api/v1/system/*` 必须引入平台管理员授权校验（`PLATFORM_ADMIN_EMAILS` 或后续升级模型）。  
2. 租户策略类接口（例如 password policy）必须做 tenant owner/admin 校验。  
3. 为 system/策略变更补齐 audit log 与告警（未授权尝试建议写 security alert）。  
