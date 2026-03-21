# MFA: TOTP 与 Recovery Code

> **模块**: auth (MFA)
> **前置条件**: 用户已注册，持有有效 access token
> **涉及端点**:
> - `GET /api/v1/mfa/status`
> - `POST /api/v1/mfa/totp/enroll`
> - `POST /api/v1/mfa/totp/enroll/verify`
> - `DELETE /api/v1/mfa/totp`
> - `POST /api/v1/mfa/recovery-codes/generate`
> - `GET /api/v1/mfa/recovery-codes/remaining`
> - `POST /api/v1/hosted-login/password`
> - `POST /api/v1/mfa/challenge/totp`
> - `POST /api/v1/mfa/challenge/recovery-code`

---

## 场景 1: TOTP 注册 → 验证 → 登录 MFA 挑战

### 步骤 0: Gate Check

- 确认用户 `test@example.com` 已通过密码认证获得 access token
- 确认 TOTP 尚未启用: `GET /api/v1/mfa/status` 返回 `totp_enabled: false`

### 步骤 1: 开始 TOTP 注册

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/totp/enroll \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" | jq .
```

**预期**: 返回 `setup_token`、`otpauth_uri`（以 `otpauth://totp/` 开头）、`secret`（base32 编码）

### 步骤 2: 使用 TOTP 验证码完成注册

使用 TOTP 应用（如 Google Authenticator）扫描 QR 码或手动输入 secret，获取当前验证码。

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/totp/enroll/verify \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"setup_token": "<SETUP_TOKEN>", "code": "<TOTP_CODE>"}' | jq .
```

**预期**: 返回 `totp_enabled: true`

### 步骤 3: 确认 MFA 状态已更新

```bash
curl -s http://localhost:8080/api/v1/mfa/status \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: `totp_enabled: true`

### 步骤 4: 使用密码登录触发 MFA 挑战

```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "TestPass1!"}  # pragma: allowlist secret' | jq . # pragma: allowlist secret
```

**预期**: 返回 `mfa_required: true`、`mfa_session_token`、`mfa_methods` 包含 `"totp"`

### 步骤 5: 使用 TOTP 完成 MFA 挑战

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/totp \
  -H "Content-Type: application/json" \
  -d '{"mfa_session_token": "<MFA_SESSION_TOKEN>", "code": "<TOTP_CODE>"}' | jq .
```

**预期**: 返回 `access_token`、`token_type: "Bearer"`

---

## 场景 2: Recovery Code 生成 → 使用 → 登录

### 步骤 0: Gate Check

- 确认用户已启用 TOTP（场景 1 已完成）

### 步骤 1: 生成 Recovery Codes

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/recovery-codes/generate \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" | jq .
```

**预期**: 返回包含 8 个 recovery code 的数组，每个 10 位小写字母+数字

### 步骤 2: 查看剩余 Recovery Codes

```bash
curl -s http://localhost:8080/api/v1/mfa/recovery-codes/remaining \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `8`

### 步骤 3: 使用 Recovery Code 完成 MFA 挑战

先触发 MFA 挑战（密码登录），然后用 recovery code 代替 TOTP：

```bash
# 触发 MFA
MFA_RESULT=$(curl -s -X POST http://localhost:8080/api/v1/hosted-login/password \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "TestPass1!"}  # pragma: allowlist secret')
MFA_TOKEN=$(echo $MFA_RESULT | jq -r .mfa_session_token)

# 使用 recovery code
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/recovery-code \
  -H "Content-Type: application/json" \
  -d "{\"mfa_session_token\": \"$MFA_TOKEN\", \"code\": \"<RECOVERY_CODE>\"}" | jq .
```

**预期**: 返回 `access_token`

### 步骤 4: 确认 Recovery Code 已消费

```bash
curl -s http://localhost:8080/api/v1/mfa/recovery-codes/remaining \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `7`

---

## 场景 3: TOTP 重放攻击被拒绝

### 步骤 1: 使用 TOTP 验证一次（成功）

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/totp \
  -H "Content-Type: application/json" \
  -d '{"mfa_session_token": "<TOKEN1>", "code": "<TOTP_CODE>"}' | jq .
```

**预期**: 成功

### 步骤 2: 在同一时间窗口内用相同代码再次验证

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/totp \
  -H "Content-Type: application/json" \
  -d '{"mfa_session_token": "<TOKEN2>", "code": "<SAME_TOTP_CODE>"}' | jq .
```

**预期**: 400 错误 "This TOTP code has already been used"

---

## 场景 4: MFA Session Token 过期

### 步骤 1: 触发 MFA 挑战获取 mfa_session_token

**预期**: 返回 `expires_in: 300`

### 步骤 2: 等待 token 过期（或手动清除 Redis key）后验证

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/totp \
  -H "Content-Type: application/json" \
  -d '{"mfa_session_token": "<EXPIRED_TOKEN>", "code": "123456"}' | jq .
```

**预期**: 401 错误 "MFA session expired or invalid"

---

## 场景 5: 消耗全部 Recovery Code 后验证失败

### 步骤 1: 生成 8 个 recovery codes 并逐个消耗

重复 8 次使用不同的 recovery code 完成 MFA 挑战。

### 步骤 2: 确认剩余数量为 0

```bash
curl -s http://localhost:8080/api/v1/mfa/recovery-codes/remaining \
  -H "Authorization: Bearer $TOKEN" | jq .
```

**预期**: 返回 `0`

### 步骤 3: 使用第 9 个 recovery code 尝试验证

```bash
curl -s -X POST http://localhost:8080/api/v1/mfa/challenge/recovery-code \
  -H "Content-Type: application/json" \
  -d '{"mfa_session_token": "<TOKEN>", "code": "invalidcode"}' | jq .
```

**预期**: 401 错误 "Invalid or already used recovery code"
