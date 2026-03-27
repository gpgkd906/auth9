# OIDC Client Credentials Flow - 基本流程

| 项目 | 值 |
|------|-----|
| 模块 | Client Credentials |
| 场景数 | 4 |
| 最后更新 | 2026-03-27 |

## 前置条件

1. 执行 `./scripts/reset-docker.sh --conformance` 重置环境至一致性测试状态
2. 确认 Auth9 Core 服务运行于 `http://localhost:8080`
3. 确认测试用 OAuth Client 已注册，持有有效的 `{client_id}` 和 `{client_secret}`
4. 确认 Client 的 `grant_types` 包含 `client_credentials`

---

## 场景 1：使用 client_secret_basic (HTTP Basic Auth) 获取 access_token

**目的**：验证通过 HTTP Basic Auth 传递 client 凭证时，token endpoint 正确签发 access_token。

**步骤**：

1. 构造 Basic Auth header，值为 `Base64({client_id}:{client_secret})`
2. 发送请求：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -u "{client_id}:{client_secret}" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials"
```

**预期结果**：

- HTTP 状态码 `200`
- 响应 JSON 包含 `access_token`、`token_type`（值为 `Bearer`）、`expires_in`
- 不包含 `refresh_token`（client_credentials flow 不签发 refresh token）

---

## 场景 2：使用 client_secret_post (POST body) 获取 access_token

**目的**：验证通过 POST body 传递 client 凭证时，token endpoint 正确签发 access_token。

**步骤**：

1. 将 `client_id` 和 `client_secret` 放入 POST body
2. 发送请求：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials&client_id={client_id}&client_secret={client_secret}"
```

**预期结果**：

- HTTP 状态码 `200`
- 响应 JSON 包含 `access_token`、`token_type`（值为 `Bearer`）、`expires_in`
- 返回结构与 client_secret_basic 方式一致

---

## 场景 3：使用无效凭证返回 401

**目的**：验证错误的 client 凭证被正确拒绝，返回 OAuth 2.0 标准错误响应。

**步骤**：

1. 使用错误的 `client_secret` 发送请求：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -u "{client_id}:wrong_secret" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials"
```

2. 使用不存在的 `client_id` 发送请求：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -u "nonexistent_client:any_secret" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials"
```

**预期结果**：

- HTTP 状态码 `401`
- 响应 JSON 包含 `error` 字段，值为 `invalid_client`
- 不泄露具体失败原因（不区分 client_id 不存在与 secret 错误）

---

## 场景 4：返回的 token 包含正确的 audience 和 scope

**目的**：验证签发的 access_token 中 JWT claims 符合预期配置。

**步骤**：

1. 获取 access_token：

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -u "{client_id}:{client_secret}" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" | jq -r '.access_token')
```

2. 解码 JWT payload：

```bash
echo "$TOKEN" | cut -d. -f2 | base64 -d 2>/dev/null | jq .
```

**预期结果**：

- `iss` claim 值为 `http://localhost:8080`
- `sub` claim 值为 `{client_id}`
- `aud` claim 包含请求的 audience
- `exp` 在未来（与 `expires_in` 一致）
- `iat` 在过去（签发时间）
- JWT header 的 `alg` 为 `RS256`，`kid` 为 `auth9-current`

---

## 检查清单

| # | 检查项 | 场景 | 预期 | 通过 |
|---|--------|------|------|------|
| 1 | client_secret_basic 获取 access_token | 1 | HTTP 200，返回有效 token | [ ] |
| 2 | client_secret_post 获取 access_token | 2 | HTTP 200，返回有效 token | [ ] |
| 3 | 错误 client_secret 被拒绝 | 3 | HTTP 401，error=invalid_client | [ ] |
| 4 | 不存在的 client_id 被拒绝 | 3 | HTTP 401，error=invalid_client | [ ] |
| 5 | token_type 为 Bearer | 1, 2 | token_type=Bearer | [ ] |
| 6 | 不签发 refresh_token | 1, 2 | 响应中无 refresh_token | [ ] |
| 7 | JWT iss claim 正确 | 4 | iss=http://localhost:8080 | [ ] |
| 8 | JWT 使用 RS256 + kid=auth9-current | 4 | alg=RS256, kid=auth9-current | [ ] |
| 9 | exp 与 expires_in 一致 | 4 | exp - iat == expires_in | [ ] |
