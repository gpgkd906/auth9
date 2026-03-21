# SDK - 认证流程与凭证管理 API 子客户端

**模块**: SDK
**测试范围**: Auth9Client 的 password / passkeys / emailOtp / auth / organizations 子客户端
**场景数**: 5
**优先级**: 高

---

## 背景说明

### 子客户端架构

`@auth9/core` 的 `Auth9Client` Phase 3 新增 5 个子客户端，覆盖认证流程与凭证管理：

| 子客户端 | 方法数 | API 前缀 |
|---------|--------|---------|
| `client.password` | 6 | `/api/v1/auth/forgot-password`, `/api/v1/auth/reset-password`, `/api/v1/users/me/password`, `/api/v1/users/{id}/password`, `/api/v1/tenants/{id}/password-policy` |
| `client.passkeys` | 6 | `/api/v1/users/me/passkeys`, `/api/v1/auth/webauthn/authenticate` |
| `client.emailOtp` | 2 | `/api/v1/auth/email-otp` |
| `client.auth` | 5 | `/api/v1/auth/authorize`, `/api/v1/auth/logout`, `/api/v1/auth/tenant-token`, `/api/v1/auth/userinfo`, `/api/v1/enterprise-sso/discovery` |
| `client.organizations` | 2 | `/api/v1/organizations`, `/api/v1/users/me/tenants` |

### 前置条件

- auth9-core 运行中 (`http://localhost:8080/health`)
- 已获取有效的 **Tenant Access Token**（用于密码策略等管理端点）和 **Identity Token**（用于 passkeys、userinfo 等 `/me` 端点）
- `npm run build` 在 `sdk/packages/core` 通过

> **⚠️ 重要: Token 类型说明**
> `gen-admin-token.sh` 生成的是 **Identity Token**（`token_type: "identity"`），只能用于 tenant-token exchange、userinfo、passkeys 等 `/me` 端点。
> 密码策略管理（`GET/PUT /api/v1/tenants/{id}/password-policy`）、管理员设置用户密码（`PUT /api/v1/users/{id}/password`）等管理端点需要 **Tenant Access Token**。
> 必须先用 Identity Token 换取 Tenant Access Token（见步骤 0）。

---

## 步骤 0：获取 Token

```bash
# 1. 获取 Identity Token（用于 passkeys / userinfo / organizations 等 /me 端点）
IDENTITY_TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $IDENTITY_TOKEN | head -c 20

# 2. 获取 tenant_id
TENANT_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;" 2>/dev/null)
echo "Tenant: $TENANT_ID"

# 3. 用 Identity Token 换取 Tenant Access Token（用于密码策略、用户管理等管理端点）
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"tenant_id\":\"$TENANT_ID\",\"service_id\":\"auth9-portal\"}" | jq -r '.access_token')
echo $TOKEN | head -c 20
```

**预期**: `$IDENTITY_TOKEN` 为 Identity Token，`$TOKEN` 为 Tenant Access Token（均非空）。
- 场景 1（密码策略）和场景 5（organizations 创建）使用 `$TOKEN`（Tenant Access Token）
- 场景 2（passkeys）使用 `$IDENTITY_TOKEN`
- 场景 4（userinfo）使用 `$IDENTITY_TOKEN`

---

## 场景 1：Password 子客户端 — 密码重置与策略管理

### 步骤

1. **忘记密码请求**

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}' | jq .
```

**预期**: 返回成功（无论邮箱是否存在，防止枚举）

2. **获取密码策略**

```bash
TENANT_ID=$(curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `data` 包含 `min_length`、`require_uppercase`、`require_lowercase`、`require_numbers`、`require_symbols`、`max_age_days`、`history_count`、`lockout_threshold`、`lockout_duration_mins` 字段

3. **更新密码策略**

```bash
curl -s -X PUT http://localhost:8080/api/v1/tenants/$TENANT_ID/password-policy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"min_length":12,"require_uppercase":true,"require_symbols":true}' | jq .
```

**预期**: 返回更新后的策略，`min_length` = 12，`require_symbols` = true

4. **管理员设置用户密码**

```bash
USER_ID=$(curl -s http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[0].id')

curl -s -X PUT http://localhost:8080/api/v1/users/$USER_ID/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password":"TempPass123!","temporary":true}' -w "\n%{http_code}" # pragma: allowlist secret
```

**预期**: 返回 200 或 204，密码设置成功

---

## 场景 2：Passkeys (WebAuthn) 子客户端 — 注册与认证

> **注意**: Passkeys 端点（`/api/v1/users/me/passkeys/*`）要求使用 **Identity Token**（非 Tenant Access Token）。
> 后端 handler 使用 `extract_identity_claims()` 只接受 Identity Token。
> 请使用 `gen-admin-token.sh` 生成的 token（即 Identity Token）访问这些端点。

### 步骤

1. **列出当前用户 Passkeys**

```bash
curl -s http://localhost:8080/api/v1/users/me/passkeys \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `data` 数组（可能为空）

2. **开始 Passkey 注册**

```bash
curl -s -X POST http://localhost:8080/api/v1/users/me/passkeys/register/start \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" | jq .
```

**预期**: 返回 `data.public_key` 对象，包含 WebAuthn challenge 信息

3. **开始 Passkey 认证（公开端点）**

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/webauthn/authenticate/start \
  -H "Content-Type: application/json" \
  -d '{}' | jq .
```

**预期**: 返回 `data.challenge_id` 和 `data.public_key` 对象

---

## 场景 3：Email OTP 子客户端 — 发送与验证

### 步骤

1. **发送 Email OTP**

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/email-otp/send \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}' -w "\n%{http_code}"
```

**预期**: 若 `email_otp_enabled` 已开启返回成功（防止邮箱枚举）；若未开启返回 404

2. **验证 Email OTP（错误验证码）**

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/email-otp/verify \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","code":"000000"}' -w "\n%{http_code}"
```

**预期**: 返回 400 或 401，验证失败

---

## 场景 4：Auth 流程辅助 — URL 构建与 Token 交换

### 步骤

1. **SDK URL 构建验证（不发 HTTP 请求）**

在 Node.js 环境中验证：

```bash
cd sdk/packages/core && node -e "
const { Auth9Client } = require('./dist/index.cjs');
const client = new Auth9Client({ baseUrl: 'https://auth9.example.com', apiKey: 'test' }); // pragma: allowlist secret
const url = client.auth.getAuthorizeUrl({ redirectUri: 'https://app.example.com/callback' });
console.log('Authorize URL:', url);
const logoutUrl = client.auth.getLogoutUrl({ postLogoutRedirectUri: 'https://app.example.com' });
console.log('Logout URL:', logoutUrl);
"
```

**预期**:
- Authorize URL 包含 `response_type=code`、`scope=openid+profile+email`、`redirect_uri=...`
- Logout URL 包含 `post_logout_redirect_uri=...`
- 两者均为同步返回，不发送 HTTP 请求

2. **UserInfo 端点**

```bash
curl -s http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `data.sub`、`data.email` 等用户信息字段

3. **Enterprise SSO Discovery**

```bash
curl -s -X POST "http://localhost:8080/api/v1/enterprise-sso/discovery?client_id=auth9-portal&redirect_uri=http://localhost:3000/callback&scope=openid+profile+email&state=test-state&response_type=code" \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com"}' -w "\n%{http_code}"
```

**预期**: 返回 SSO 发现结果（含 `authorize_url`）或 404（取决于是否配置了对应域名的 SSO 连接器）

---

## 场景 5：Organizations 子客户端 — 创建与查询

### 步骤

1. **获取当前用户的租户列表**

```bash
curl -s http://localhost:8080/api/v1/users/me/tenants \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `data` 数组，包含用户所属的租户信息

2. **创建 Organization（自助）**

```bash
curl -s -X POST http://localhost:8080/api/v1/organizations \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"SDK Test Org","slug":"sdk-test-org-phase3","domain":"sdk-test.example.com"}' | jq .
```

**预期**: 返回创建的 Organization，`data.name` = "SDK Test Org"，`data.slug` = "sdk-test-org-phase3"，`data.domain` = "sdk-test.example.com"，`data.status` = "pending"，创建者自动成为 owner

3. **带 service_id 过滤查询**

```bash
curl -s "http://localhost:8080/api/v1/users/me/tenants?service_id=nonexistent" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回空 `data` 数组（不存在的 `service_id` 正确返回空列表，此 bug 已修复）

> **注意**: `service_id` 参数期望的是 Service 对应的 `client_id` 值（如 `auth9-portal`），而非 Service 的 UUID。

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Password 密码重置与策略管理 | ☐ | | | |
| 2 | Passkeys WebAuthn 注册与认证 | ☐ | | | |
| 3 | Email OTP 发送与验证 | ☐ | | | |
| 4 | Auth 流程辅助 URL 构建与 Token | ☐ | | | |
| 5 | Organizations 创建与查询 | ☐ | | | |
