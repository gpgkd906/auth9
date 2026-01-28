# Token 规范

本文档详细说明 Auth9 中使用的各种 Token 类型和格式。

## Token 类型

Auth9 使用两种主要的 Token：

1. **Identity Token** - 用户身份令牌
2. **Tenant Access Token** - 租户访问令牌

## JWT 结构

所有 Token 都遵循标准的 JWT (JSON Web Token) 格式：

```
<Header>.<Payload>.<Signature>
```

### Header

```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

支持的算法：
- `HS256` - HMAC SHA-256
- `HS384` - HMAC SHA-384
- `HS512` - HMAC SHA-512
- `RS256` - RSA SHA-256

## Identity Token

### 用途

Identity Token 是用户完成认证后获得的主令牌，用于：

- 证明用户身份
- 请求租户访问令牌
- 访问用户基本信息

### Payload 结构

```json
{
  "iss": "https://auth9.yourdomain.com",
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "aud": "auth9",
  "exp": 1640995200,
  "iat": 1640991600,
  "nbf": 1640991600,
  "jti": "unique-token-id",
  "email": "user@example.com",
  "email_verified": true,
  "name": "张三",
  "preferred_username": "zhangsan",
  "picture": "https://example.com/avatar.jpg"
}
```

### 字段说明

| 字段 | 类型 | 说明 | 必填 |
|------|------|------|------|
| `iss` | string | Token 签发者 | ✅ |
| `sub` | string | 用户唯一标识（UUID） | ✅ |
| `aud` | string | 受众，通常是 "auth9" | ✅ |
| `exp` | number | 过期时间（Unix 时间戳） | ✅ |
| `iat` | number | 签发时间（Unix 时间戳） | ✅ |
| `nbf` | number | 生效时间（Unix 时间戳） | ✅ |
| `jti` | string | Token 唯一标识 | ✅ |
| `email` | string | 用户邮箱 | ✅ |
| `email_verified` | boolean | 邮箱是否已验证 | ✅ |
| `name` | string | 显示名称 | 否 |
| `preferred_username` | string | 用户名 | 否 |
| `picture` | string | 头像 URL | 否 |

### 有效期

- 默认：1 小时（3600 秒）
- 可配置范围：5 分钟 - 24 小时
- 配置项：`JWT_EXPIRATION`

### 示例

完整的 Identity Token：

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJodHRwczovL2F1dGg5LnlvdXJkb21haW4uY29tIiwic3ViIjoiNTUwZTg0MDAtZTI5Yi00MWQ0LWE3MTYtNDQ2NjU1NDQwMDAwIiwiYXVkIjoiYXV0aDkiLCJleHAiOjE2NDA5OTUyMDAsImlhdCI6MTY0MDk5MTYwMCwibmJmIjoxNjQwOTkxNjAwLCJqdGkiOiJ1bmlxdWUtdG9rZW4taWQiLCJlbWFpbCI6InVzZXJAZXhhbXBsZS5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibmFtZSI6IuW8oOS4iSIsInByZWZlcnJlZF91c2VybmFtZSI6InpoYW5nc2FuIiwicGljdHVyZSI6Imh0dHBzOi8vZXhhbXBsZS5jb20vYXZhdGFyLmpwZyJ9.signature
```

## Tenant Access Token

### 用途

Tenant Access Token 是用户在特定租户中的访问令牌，用于：

- 访问租户资源
- 验证用户权限
- 执行授权操作

### Payload 结构

```json
{
  "iss": "https://auth9.yourdomain.com",
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "aud": "service-client-id",
  "exp": 1640995200,
  "iat": 1640991600,
  "nbf": 1640991600,
  "jti": "unique-token-id",
  "tenant_id": "tenant-uuid",
  "tenant_slug": "my-company",
  "scope": "read write",
  "roles": ["editor", "viewer"],
  "permissions": [
    "content:read",
    "content:write",
    "user:read"
  ],
  "resource_access": {
    "my-app": {
      "roles": ["editor"]
    },
    "another-app": {
      "roles": ["viewer"]
    }
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 | 必填 |
|------|------|------|------|
| `iss` | string | Token 签发者 | ✅ |
| `sub` | string | 用户唯一标识 | ✅ |
| `aud` | string | 目标服务的 client_id | ✅ |
| `exp` | number | 过期时间 | ✅ |
| `iat` | number | 签发时间 | ✅ |
| `nbf` | number | 生效时间 | ✅ |
| `jti` | string | Token 唯一标识 | ✅ |
| `tenant_id` | string | 租户 ID | ✅ |
| `tenant_slug` | string | 租户 Slug | ✅ |
| `scope` | string | 权限范围 | 否 |
| `roles` | array | 角色列表 | ✅ |
| `permissions` | array | 权限列表 | ✅ |
| `resource_access` | object | 资源访问权限 | 否 |

### 有效期

- 默认：1 小时（3600 秒）
- 可配置范围：5 分钟 - 24 小时
- 通过 Token Exchange 时可指定

### 获取方式

通过 gRPC Token Exchange 服务获取：

```protobuf
service TokenExchange {
  rpc ExchangeToken(ExchangeTokenRequest) returns (ExchangeTokenResponse);
}
```

请求示例：

```rust
let response = client.exchange_token(ExchangeTokenRequest {
    identity_token: "user-identity-token",
    tenant_id: "tenant-uuid",
    service_client_id: "my-service",
    scopes: vec!["read", "write"],
}).await?;

let access_token = response.access_token;
```

## Refresh Token

### 用途

用于刷新过期的 Access Token，无需用户重新登录。

### Payload 结构

```json
{
  "iss": "https://auth9.yourdomain.com",
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "aud": "auth9",
  "exp": 1643587200,
  "iat": 1640991600,
  "jti": "unique-refresh-token-id",
  "type": "refresh",
  "scope": "offline_access"
}
```

### 有效期

- 默认：30 天
- 可配置范围：1 天 - 90 天
- 配置项：`REFRESH_TOKEN_EXPIRATION`

### 使用方式

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "refresh-token-here"
  }'
```

响应：

```json
{
  "data": {
    "access_token": "new-access-token",
    "token_type": "Bearer",
    "expires_in": 3600,
    "refresh_token": "new-refresh-token"
  }
}
```

## Token 验证

### 验证流程

1. **格式验证** - 检查 JWT 格式
2. **签名验证** - 验证签名有效性
3. **时间验证** - 检查 exp, nbf, iat
4. **Issuer 验证** - 验证签发者
5. **Audience 验证** - 验证受众
6. **自定义验证** - 业务逻辑验证

### 本地验证

```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: usize,
    iat: usize,
}

fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["https://auth9.yourdomain.com"]);
    validation.set_audience(&["auth9"]);
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &validation,
    )?;
    
    Ok(token_data.claims)
}
```

### 远程验证（gRPC）

```rust
let response = client.validate_token(ValidateTokenRequest {
    token: "token-to-validate",
    expected_audience: "my-service",
}).await?;

if response.valid {
    println!("Token 有效");
    println!("用户 ID: {}", response.user_id);
    println!("租户 ID: {}", response.tenant_id);
} else {
    println!("Token 无效: {}", response.error_message);
}
```

## Token 撤销

### 撤销机制

Auth9 支持两种 Token 撤销方式：

#### 1. Token Blacklist

将 Token 加入黑名单：

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/auth/revoke \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "token": "token-to-revoke",
    "token_type_hint": "access_token"
  }'
```

#### 2. Token Version

通过递增用户的 token version 使所有旧 Token 失效：

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/users/{user_id}/invalidate-tokens \
  -H "Authorization: Bearer <token>"
```

## Token 安全

### 存储建议

| 环境 | 推荐方式 | 不推荐 |
|------|---------|--------|
| Web 浏览器 | HttpOnly Cookie | LocalStorage |
| 移动 App | Secure Storage (Keychain/KeyStore) | SharedPreferences |
| 服务端 | 内存 + Redis | 文件系统 |

### 传输安全

✅ **推荐做法**：

- 使用 HTTPS 传输
- 使用 Authorization Header
- 启用 HSTS
- 验证 TLS 证书

❌ **不推荐做法**：

- URL 参数传递 Token
- HTTP 明文传输
- 跨域存储 Token

### Token 泄露应对

1. **立即撤销** Token
2. **强制用户重新登录**
3. **审查异常活动**
4. **通知用户**
5. **更新安全策略**

## JWKS (JSON Web Key Set)

### 获取公钥

对于 RS256 等非对称加密算法：

```bash
curl https://auth9.yourdomain.com/.well-known/jwks.json
```

响应：

```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "2024-01-key-1",
      "alg": "RS256",
      "n": "...",
      "e": "AQAB"
    }
  ]
}
```

### 使用公钥验证

```rust
use jsonwebtoken::{decode, DecodingKey, Validation};

let jwks = fetch_jwks("https://auth9.yourdomain.com/.well-known/jwks.json").await?;
let key = jwks.find_key_by_kid("2024-01-key-1")?;

let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)?;
let token_data = decode::<Claims>(token, &decoding_key, &Validation::new(Algorithm::RS256))?;
```

## 性能优化

### Token 缓存

```rust
// 缓存已验证的 Token
let cache_key = format!("token:valid:{}", token_hash);
let cached = redis.get::<Option<bool>>(&cache_key).await?;

if let Some(is_valid) = cached {
    return Ok(is_valid);
}

// 验证并缓存
let is_valid = verify_token(token).await?;
redis.set_ex(&cache_key, is_valid, 300).await?; // 5 分钟 TTL

Ok(is_valid)
```

### 批量验证

```rust
// 使用 Pipeline 批量验证
let tokens = vec!["token1", "token2", "token3"];
let results = validate_tokens_batch(tokens).await?;
```

## 调试工具

### jwt.io

在线 JWT 调试工具：https://jwt.io

### 命令行工具

```bash
# 解码 JWT
echo "your-jwt-token" | base64 -d

# 使用 jq 格式化
echo "payload" | base64 -d | jq .
```

### 验证脚本

```bash
#!/bin/bash
TOKEN="your-token-here"

# 分离 Header 和 Payload
IFS='.' read -ra PARTS <<< "$TOKEN"

# 解码 Header
echo "Header:"
echo ${PARTS[0]} | base64 -d | jq .

# 解码 Payload
echo "Payload:"
echo ${PARTS[1]} | base64 -d | jq .
```

## 常见问题

### Q: Token 过期了怎么办？

A: 使用 Refresh Token 刷新，或引导用户重新登录。

### Q: 如何判断 Token 是否快过期？

A: 检查 `exp` 字段，建议在过期前 5-10 分钟开始刷新。

### Q: 可以延长 Token 有效期吗？

A: 不建议。应该使用 Refresh Token 机制。

### Q: Token 可以跨租户使用吗？

A: Identity Token 可以，但 Tenant Access Token 只能在指定租户使用。

### Q: 如何防止 Token 重放攻击？

A: 使用 `jti` (JWT ID) 和 Token Blacklist 机制。

## 相关文档

- [认证流程](认证流程.md)
- [REST API](REST-API.md)
- [gRPC API](gRPC-API.md)
- [安全最佳实践](最佳实践.md)
