# 25 - Auth9 本地 OIDC Token 签发与授权码流程

> **模块**: auth (Identity)
> **前置条件**: `IDENTITY_BACKEND=auth9_oidc`，Docker 环境运行中
> **覆盖需求**: Phase 3 FR5 (Token Issuance & Auth Server Core)

---

## 场景 1: 授权码完整流程（authorize → hosted login → token）

### 步骤 0 - Gate Check

```bash
# 验证环境
curl -sf http://localhost:8080/health && echo "OK"
# 验证 identity backend 配置
# IDENTITY_BACKEND=auth9_oidc 需要在 docker-compose 环境变量中设置
```

> **重要**: 如果测试用户 `admin@auth9.local` 已启用 MFA（TOTP），密码登录将返回 `mfa_required` 响应而非 identity token。测试前请确认：
> 1. 使用 **未启用 MFA** 的测试用户（如 `test@auth9.local`），或
> 2. 执行 `./scripts/reset-docker.sh` 重置环境（会禁用 MFA），或
> 3. 通过数据库手动禁用 MFA：`UPDATE users SET mfa_enabled = 0 WHERE email = 'admin@auth9.local';`

### 步骤 1 - 发起 authorize 请求

```bash
# 应该重定向到 portal 的 hosted login 页面（带 login_challenge）
curl -v "http://localhost:8080/api/v1/auth/authorize?response_type=code&client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&scope=openid+email+profile&state=test-state-123&nonce=test-nonce-456" 2>&1 | grep -i "location:"
```

### 预期行为

- HTTP 302 重定向
- Location 包含 `/login?login_challenge=` 前缀
- login_challenge 是有效的 UUID

### 步骤 2 - 使用密码登录获取 identity token

```bash
# 通过 hosted-login 获取 identity token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@auth9.local","password":"<admin_password>"}' | jq -r '.access_token')
echo "Identity Token: ${TOKEN:0:50}..."
```

### 步骤 3 - 完成授权（authorize_complete）

```bash
# 从步骤 1 的 Location 中提取 login_challenge
CHALLENGE_ID="<从步骤1提取的login_challenge>"
RESULT=$(curl -s -X POST http://localhost:8080/api/v1/auth/authorize/complete \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d "{\"login_challenge_id\":\"$CHALLENGE_ID\"}")
echo "$RESULT" | jq .
```

### 预期行为

- 返回 `{ "data": { "redirect_url": "http://localhost:3000/auth/callback?code=<uuid>&state=test-state-123" } }`
- redirect_url 包含 `code` 和原始 `state` 参数

### 步骤 4 - 用授权码换取 token

```bash
CODE="<从步骤3的redirect_url中提取的code>"
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"authorization_code\",\"client_id\":\"auth9-portal\",\"code\":\"$CODE\",\"redirect_uri\":\"http://localhost:3000/auth/callback\"}" | jq .
```

### 预期行为

- 返回 `access_token`（Auth9 identity JWT）
- 返回 `id_token`（OIDC id token，包含 nonce、at_hash）
- 返回 `refresh_token`（Auth9 OIDC refresh JWT）
- `token_type` = "Bearer"

---

## 场景 2: 授权码 Replay 防护

### 步骤

使用场景 1 中已消费的 `code` 再次调用 token endpoint：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"authorization_code\",\"client_id\":\"auth9-portal\",\"code\":\"$CODE\",\"redirect_uri\":\"http://localhost:3000/auth/callback\"}"
```

### 预期行为

- HTTP 400
- 错误信息: "Invalid or expired authorization code"

---

## 场景 3: PKCE 验证

### 步骤 0 - Gate Check

需要在 authorize 请求中包含 `code_challenge` 和 `code_challenge_method=S256`。

### 步骤 1 - 生成 PKCE 参数

```bash
# 生成 code_verifier 和 code_challenge
CODE_VERIFIER=$(openssl rand -base64 32 | tr -d '=' | tr '+/' '-_')
CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr -d '=' | tr '+/' '-_')
echo "Verifier: $CODE_VERIFIER"
echo "Challenge: $CODE_CHALLENGE"
```

### 步骤 2 - 发起带 PKCE 的 authorize

```bash
curl -v "http://localhost:8080/api/v1/auth/authorize?response_type=code&client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&scope=openid&state=pkce-test&code_challenge=$CODE_CHALLENGE&code_challenge_method=S256"
```

### 步骤 3 - 完成登录并获取 code（同场景 1 步骤 2-3）

### 步骤 4 - 用正确的 code_verifier 换取 token

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"authorization_code\",\"client_id\":\"auth9-portal\",\"code\":\"$CODE\",\"redirect_uri\":\"http://localhost:3000/auth/callback\",\"code_verifier\":\"$CODE_VERIFIER\"}" | jq .
```

### 预期行为（正确 verifier）

- HTTP 200，返回完整 token set

### 步骤 5 - 用错误的 code_verifier（另一次请求）

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"authorization_code\",\"client_id\":\"auth9-portal\",\"code\":\"$NEW_CODE\",\"redirect_uri\":\"http://localhost:3000/auth/callback\",\"code_verifier\":\"wrong-verifier\"}"
```

### 预期行为（错误 verifier）

- HTTP 400
- 错误信息: "PKCE verification failed"

---

## 场景 4: Refresh Token 轮转

### 步骤 1 - 从场景 1 获取 refresh_token

### 步骤 2 - 使用 refresh_token 刷新

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"refresh_token\",\"client_id\":\"auth9-portal\",\"refresh_token\":\"$REFRESH_TOKEN\"}" | jq .
```

### 预期行为

- HTTP 200
- 返回新的 `access_token`、`id_token`、`refresh_token`
- 新的 `refresh_token` 与旧的不同（轮转）

### 步骤 3 - 使用旧 refresh_token 再次刷新（replay）

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d "{\"grant_type\":\"refresh_token\",\"client_id\":\"auth9-portal\",\"refresh_token\":\"$REFRESH_TOKEN\"}"
```

### 预期行为

- HTTP 400 (invalid_grant)
- 错误信息: "Refresh token has already been used"

> **说明**: Refresh token replay 检测在 session 绑定校验之前触发。旧文档中记录的 HTTP 401 / "Refresh token is not bound to an active session" 是 session 绑定校验的错误，但 replay 检测优先级更高，属于正确的安全行为。

---

## 场景 5: ID Token Claims 正确性

### 步骤

解码场景 1 中获得的 `id_token`：

```bash
echo "$ID_TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
```

### 预期行为

- `sub` = 用户 ID（UUID 格式）
- `email` = 用户邮箱
- `aud` = "auth9-portal"（client_id，非 "auth9"）
- `iss` = Auth9 issuer URL
- `token_type` = "id_token"
- `nonce` = 授权请求中传入的 nonce 值
- `at_hash` = access_token 的 SHA-256 左 128 位 base64url 编码
- `iat` 和 `exp` 合理（exp > iat）
