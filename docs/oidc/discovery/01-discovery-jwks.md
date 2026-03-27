# OIDC Discovery & JWKS - 端点验证

**模块**: Discovery
**测试范围**: OpenID Connect Discovery 端点与 JSON Web Key Set 验证
**场景数**: 5

---

## 前置条件

```bash
# 重置环境（含 Conformance Suite）
./scripts/reset-docker.sh --conformance

# 验证服务健康
curl -sf http://localhost:8080/health && echo "Core OK"
```

---

## 场景 1：Discovery 端点返回完整的 OpenID Configuration

### 初始状态
- auth9-core 运行中

### 目的
验证 `/.well-known/openid-configuration` 返回符合 OIDC Core 规范的 JSON

### 测试操作流程
1. 请求 Discovery 端点：
   ```bash
   curl -s http://localhost:8080/.well-known/openid-configuration | jq .
   ```
2. 验证所有必需字段存在

### 预期结果
- HTTP 200
- 返回 JSON 包含以下字段：

| 字段 | 预期值 |
|------|--------|
| `issuer` | `http://localhost:8080` |
| `authorization_endpoint` | `http://localhost:8080/api/v1/auth/authorize` |
| `token_endpoint` | `http://localhost:8080/api/v1/auth/token` |
| `userinfo_endpoint` | `http://localhost:8080/api/v1/auth/userinfo` |
| `jwks_uri` | `http://localhost:8080/.well-known/jwks.json` |
| `end_session_endpoint` | `http://localhost:8080/api/v1/auth/logout` |
| `response_types_supported` | `["code", "token", "id_token"]` |
| `grant_types_supported` | `["authorization_code", "client_credentials", "refresh_token"]` |
| `subject_types_supported` | `["public"]` |
| `id_token_signing_alg_values_supported` | `["RS256"]` |
| `scopes_supported` | `["openid", "profile", "email"]` |
| `token_endpoint_auth_methods_supported` | `["client_secret_basic", "client_secret_post"]` |
| `claims_supported` | `["sub", "email", "name", "iss", "aud", "exp", "iat"]` |

---

## 场景 2：Discovery 端点中所有 URL 可达

### 初始状态
- auth9-core 运行中

### 目的
验证 Discovery 文档中声明的所有端点 URL 均可访问（返回非 404）

### 测试操作流程
1. 从 Discovery 端点提取所有 URL 字段
2. 对每个端点发送 GET 请求：
   ```bash
   # authorization_endpoint（预期 302 或 400，因缺少参数）
   curl -s -o /dev/null -w '%{http_code}' "http://localhost:8080/api/v1/auth/authorize"

   # token_endpoint（预期 400 或 405，POST-only）
   curl -s -o /dev/null -w '%{http_code}' -X POST "http://localhost:8080/api/v1/auth/token"

   # userinfo_endpoint（预期 401，无 token）
   curl -s -o /dev/null -w '%{http_code}' "http://localhost:8080/api/v1/auth/userinfo"

   # jwks_uri
   curl -s -o /dev/null -w '%{http_code}' "http://localhost:8080/.well-known/jwks.json"
   ```

### 预期结果
- 所有端点返回非 404 状态码
- `jwks_uri` 返回 200
- `userinfo_endpoint` 返回 401（未认证）
- `token_endpoint` 返回 400（缺少参数）
- `authorization_endpoint` 返回 400 或 302（缺少必需参数）

---

## 场景 3：JWKS 端点返回有效的 JWK Set

### 初始状态
- auth9-core 运行中，配置了 RSA 签名密钥

### 目的
验证 `/.well-known/jwks.json` 返回符合 JWK 规范的密钥集

### 测试操作流程
1. 请求 JWKS 端点：
   ```bash
   curl -s http://localhost:8080/.well-known/jwks.json | jq .
   ```
2. 验证密钥格式

### 预期结果
- HTTP 200
- `keys` 数组非空（至少 1 个 key）
- 第一个 key 字段：

| 字段 | 预期值 |
|------|--------|
| `kty` | `RSA` |
| `use` | `sig` |
| `alg` | `RS256` |
| `kid` | `auth9-current` |
| `n` | 非空字符串（Base64url 编码的 RSA modulus） |
| `e` | 非空字符串（Base64url 编码的 RSA exponent） |

---

## 场景 4：JWKS 密钥可用于验证 Token 签名

### 初始状态
- auth9-core 运行中

### 目的
验证 JWKS 公钥能成功验证 auth9-core 签发的 JWT

### 测试操作流程
1. 获取一个有效 token（通过 `gen-admin-token.sh`）
2. 获取 JWKS 公钥
3. 使用公钥验证 token 签名：
   ```bash
   TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
   # 解码 header 确认 kid
   echo "$TOKEN" | cut -d. -f1 | base64 -d 2>/dev/null | jq .
   # 预期: {"alg":"RS256","kid":"auth9-current","typ":"JWT"}
   ```

### 预期结果
- Token header 中的 `alg` 为 `RS256`
- Token header 中的 `kid` 为 `auth9-current`
- 与 JWKS 中的 key `kid` 匹配

---

## 场景 5：Key Rotation — Previous Key 包含在 JWKS 中

### 初始状态
- auth9-core 运行中
- 配置了 `JWT_PREVIOUS_PUBLIC_KEY` 环境变量

### 目的
验证 Key Rotation 场景下 JWKS 同时暴露当前和旧密钥

### 测试操作流程
1. 请求 JWKS 端点：
   ```bash
   curl -s http://localhost:8080/.well-known/jwks.json | jq '.keys | length'
   ```
2. 验证密钥数量和 kid

### 预期结果
- 若配置了 `JWT_PREVIOUS_PUBLIC_KEY`：
  - `keys` 数组包含 2 个 key
  - kid 分别为 `auth9-current` 和 `auth9-previous`
  - 两个 key 的 `kty`、`use`、`alg` 一致
- 若未配置 previous key：
  - `keys` 数组包含 1 个 key（`auth9-current`）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Discovery 端点完整性 | ☐ | | | |
| 2 | 端点 URL 可达性 | ☐ | | | |
| 3 | JWKS 格式验证 | ☐ | | | |
| 4 | JWKS 签名验证 | ☐ | | | |
| 5 | Key Rotation | ☐ | | | |
