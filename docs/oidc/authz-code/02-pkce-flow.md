# OIDC Authorization Code Flow - PKCE 验证

**模块**: Authorization Code (PKCE)
**测试范围**: Proof Key for Code Exchange (PKCE, RFC 7636) 的 S256 验证、Public Client 强制约束
**场景数**: 5

---

## 前置条件

```bash
# 重置环境（含 Conformance Suite）
./scripts/reset-docker.sh --conformance

# 验证服务健康
curl -sf http://localhost:8080/health && echo "Core OK"
```

OIDC 客户端通过 `scripts/oidc-conformance-setup.sh` 预置，文档中使用占位符：
- `{client_id}` — 测试客户端 ID
- `{client_secret}` — 测试客户端密钥
- `{redirect_uri}` — 已注册的回调地址

Issuer 地址：`http://localhost:8080`（Host 端测试）

### PKCE 参数生成

```bash
# 生成 code_verifier（43-128 字符的随机字符串）
CODE_VERIFIER=$(openssl rand -base64 32 | tr -d '=/+' | head -c 43)
echo "code_verifier: $CODE_VERIFIER"

# 生成 code_challenge（S256 = BASE64URL(SHA256(code_verifier))）
CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr '+/' '-_' | tr -d '=')
echo "code_challenge: $CODE_CHALLENGE"
```

---

## 场景 1：使用 code_challenge (S256) 的 Authorize 请求成功

### 初始状态
- auth9-core 运行中
- 测试客户端已配置

### 目的
验证 Authorize 端点接受 S256 method 的 PKCE code_challenge 参数

### 测试操作流程
1. 生成 PKCE 参数：
   ```bash
   CODE_VERIFIER=$(openssl rand -base64 32 | tr -d '=/+' | head -c 43)
   CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr '+/' '-_' | tr -d '=')
   ```
2. 发起带 PKCE 的 Authorize 请求：
   ```bash
   curl -s -o /dev/null -w '%{http_code}\n%{redirect_url}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid profile email&\
   state=pkce-test-state&\
   code_challenge=$CODE_CHALLENGE&\
   code_challenge_method=S256"
   ```

### 预期结果
- HTTP 307 重定向到 Hosted Login 页面（Axum `Redirect::temporary()` 返回 307，符合 RFC 7231）
- 重定向 URL 包含 `login_challenge` 参数
- 不返回任何 PKCE 相关错误

### 预期数据状态

> **注意**: PKCE state（login challenges 和 authorization codes）存储在 Redis 中并设置短 TTL，不存储在数据库表中。无需 SQL 验证，可通过 Redis CLI 检查：
>
> ```bash
> redis-cli KEYS "login_challenge:*"
> # 预期: 存在与当前请求关联的 key，包含 code_challenge 和 code_challenge_method 字段
> ```

---

## 场景 2：使用正确 code_verifier 的 Token Exchange 成功

### 初始状态
- 已通过场景 1 完成带 PKCE 的 Authorize 流程
- 已获取绑定了 code_challenge 的 authorization code
- 保留了原始 `code_verifier`

### 目的
验证使用正确 code_verifier 可成功换取 token

### 测试操作流程
1. 使用正确的 code_verifier 换取 token：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&\
   code=<auth_code>&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   code_verifier=$CODE_VERIFIER" | jq .
   ```

### 预期结果
- HTTP 200
- 返回 JSON 包含完整 token 集合：

| 字段 | 预期值 |
|------|--------|
| `access_token` | 非空 JWT 字符串 |
| `id_token` | 非空 JWT 字符串 |
| `token_type` | `Bearer` |
| `expires_in` | 正整数 |

> **注意**: 使用 PKCE 时，Public Client 不需要提供 `client_secret`（通过 `-u` 或 body），code_verifier 本身即为 proof of possession。

### 预期数据状态

> **注意**: PKCE state（login challenges 和 authorization codes）存储在 Redis 中并设置短 TTL，不存储在数据库表中。Auth code 被消费后会从 Redis 中删除。
>
> ```bash
> # 验证 auth code 已被消费（换取 token 后 key 应不存在）
> redis-cli GET "authorization_code:<auth_code>"
> # 预期: (nil) — 已被消费并删除
> ```

---

## 场景 3：使用错误 code_verifier 的 Token Exchange 失败

### 初始状态
- 已通过带 PKCE 的 Authorize 流程获取 authorization code
- 保留了原始 `code_verifier`

### 目的
验证 code_verifier 与 code_challenge 不匹配时 Token 端点拒绝请求

### 测试操作流程
1. 使用错误的 code_verifier：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&\
   code=<auth_code>&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   code_verifier=wrong-verifier-value-that-does-not-match" | jq .
   ```
2. 完全不提供 code_verifier（但 authorize 阶段使用了 PKCE）：
   ```bash
   curl -s -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code&\
   code=<auth_code>&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}" | jq .
   ```

### 预期结果
- 步骤 1：HTTP 400，S256 哈希验证失败

| 字段 | 预期值 |
|------|--------|
| `error` | `invalid_grant` |
| `error_description` | 包含 "code_verifier" 或 "PKCE" 相关描述 |

- 步骤 2：HTTP 400，缺少 code_verifier

| 字段 | 预期值 |
|------|--------|
| `error` | `invalid_grant` |
| `error_description` | 包含 "code_verifier" 缺失相关描述 |

---

## 场景 4：Public Client 不带 PKCE 的 Authorize 请求被拒绝

### 初始状态
- auth9-core 运行中
- 存在一个 Public Client（`public_client=true`）

### 目的
验证 Public Client 必须使用 PKCE，不带 code_challenge 的请求被服务端拒绝

### 测试操作流程
1. 使用 Public Client 发起不带 PKCE 的 Authorize 请求：
   ```bash
   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid&\
   state=no-pkce-state"
   ```
2. 使用同一 Public Client 发起带 PKCE 的请求作为对比：
   ```bash
   CODE_VERIFIER=$(openssl rand -base64 32 | tr -d '=/+' | head -c 43)
   CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr '+/' '-_' | tr -d '=')

   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid&\
   state=with-pkce-state&\
   code_challenge=$CODE_CHALLENGE&\
   code_challenge_method=S256"
   ```

### 预期结果
- 步骤 1：HTTP 400，Public Client 必须提供 PKCE 参数
- 步骤 2：HTTP 307，正常重定向到 Hosted Login

> **说明**: PKCE 仅对 Public Client（`public_client=true`）强制要求。Confidential Client 可选择性使用 PKCE。

### 预期数据状态
```sql
-- 确认客户端类型
SELECT id, client_id, public_client
FROM clients
WHERE client_id = '{client_id}';
-- 预期: public_client = 1（Public Client）
```

---

## 场景 5：不支持的 code_challenge_method (plain) 被拒绝

### 初始状态
- auth9-core 运行中
- 测试客户端已配置

### 目的
验证 Auth9 仅支持 S256 method，拒绝 `plain` 和其他非法 method

### 测试操作流程
1. 使用 `plain` method：
   ```bash
   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid&\
   state=plain-method-state&\
   code_challenge=plain-text-challenge-value&\
   code_challenge_method=plain"
   ```
2. 使用无效的 method 值：
   ```bash
   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid&\
   state=invalid-method-state&\
   code_challenge=some-challenge-value&\
   code_challenge_method=S512"
   ```
3. 使用正确的 S256 method 作为对比：
   ```bash
   CODE_VERIFIER=$(openssl rand -base64 32 | tr -d '=/+' | head -c 43)
   CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr '+/' '-_' | tr -d '=')

   curl -s -o /dev/null -w '%{http_code}' \
     "http://localhost:8080/api/v1/auth/authorize?\
   response_type=code&\
   client_id={client_id}&\
   redirect_uri={redirect_uri}&\
   scope=openid&\
   state=s256-method-state&\
   code_challenge=$CODE_CHALLENGE&\
   code_challenge_method=S256"
   ```

### 预期结果
- 步骤 1：HTTP 400，`plain` method 不被支持
- 步骤 2：HTTP 400，`S512` 为无效 method
- 步骤 3：HTTP 307，S256 正常接受，重定向到 Hosted Login

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | S256 code_challenge Authorize 请求 | ☐ | | | |
| 2 | 正确 code_verifier Token Exchange | ☐ | | | |
| 3 | 错误 code_verifier Token Exchange | ☐ | | | |
| 4 | Public Client 强制 PKCE | ☐ | | | |
| 5 | 不支持的 code_challenge_method | ☐ | | | |
