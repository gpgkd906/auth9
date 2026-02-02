# 认证安全 - JWT Token 安全测试

**模块**: 认证安全
**测试范围**: JWT Token 签发、验证和存储安全
**场景数**: 5
**风险等级**: 🔴 极高

---

## 背景知识

Auth9 使用两种 Token：
- **Identity Token**: 用户身份凭证，包含基础用户信息
- **Tenant Access Token**: Token Exchange 后获得，包含租户角色和权限

Token 结构示例：
```json
{
  "iss": "https://auth9.example.com",
  "sub": "user-uuid",
  "aud": "service-client-id",
  "exp": 1234567890,
  "tenant_id": "tenant-uuid",
  "roles": ["editor"],
  "permissions": ["user:read", "user:write"]
}
```

---

## 场景 1：JWT 签名算法混淆攻击

### 前置条件
- 获取一个有效的 JWT Token

### 攻击目标
验证是否可以通过算法混淆攻击伪造 Token

### 攻击步骤
1. 解码获取的 JWT Token
2. 尝试以下攻击：
   - 将 `alg` 改为 `none`
   - 将 RS256 改为 HS256 (用公钥作为密钥签名)
   - 将 `alg` 改为不支持的算法
3. 使用修改后的 Token 访问 API

### 预期安全行为
- 服务端应验证算法白名单
- `alg: none` 应被拒绝
- 算法不匹配应返回 401

### 验证方法
```bash
# 原始 Token
TOKEN="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# 构造 alg:none 的 Token
# Header: {"alg":"none","typ":"JWT"}
# Payload: {...原始内容...}
# Signature: (空)

FORGED_TOKEN="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.{payload}."

curl -H "Authorization: Bearer $FORGED_TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 401 Unauthorized
```

### 修复建议
- 明确配置允许的算法白名单
- 禁用 `none` 算法
- 验证时指定期望的算法
- 使用非对称签名 (RS256/ES256)

---

## 场景 2：JWT 密钥泄露测试

### 前置条件
- 系统运行中
- 能够访问各种端点

### 攻击目标
检测 JWT 签名密钥是否可能泄露

### 攻击步骤
1. 检查以下潜在泄露点：
   - 错误响应中是否包含密钥信息
   - `/.well-known/jwks.json` 是否包含私钥
   - 配置端点是否暴露密钥
   - 日志文件是否记录密钥
2. 尝试通过弱密钥暴力破解 (HS256)
3. 检查密钥轮换机制

### 预期安全行为
- JWKS 端点仅暴露公钥
- 错误信息不泄露密钥
- 使用足够强度的密钥 (>= 256 bits)

### 验证方法
```bash
# 检查 JWKS 端点
curl http://localhost:8080/.well-known/jwks.json | jq .
# 确认仅包含 "kty", "n", "e" (公钥部分)
# 不应包含 "d", "p", "q" (私钥部分)

# 对于 HS256，尝试弱密钥
# 使用 jwt-cracker 或 hashcat
```

### 修复建议
- 使用非对称加密 (RS256/ES256)
- JWKS 仅暴露公钥
- 密钥存储在安全位置 (K8s Secrets, Vault)
- 实现密钥轮换

---

## 场景 3：Token 有效期与刷新测试

### 前置条件
- 正常用户会话

### 攻击目标
验证 Token 过期机制是否正确实现

### 攻击步骤
1. 获取有效 Token
2. 检查 Token 过期时间 (exp claim)
3. 等待 Token 过期后使用
4. 测试 refresh token 机制：
   - 过期的 refresh token 是否可用
   - refresh token 是否可重放
   - 吊销后 refresh token 是否仍有效

### 预期安全行为
- Access Token 过期后立即失效
- Refresh Token 一次性使用
- 支持 Token 吊销

### 验证方法
```bash
# 解析 Token 获取过期时间
echo $TOKEN | cut -d'.' -f2 | base64 -d | jq .exp

# 过期后使用
curl -H "Authorization: Bearer $EXPIRED_TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 401 {"error": "token_expired"}

# 测试 refresh token 重放
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -d "refresh_token=$USED_REFRESH_TOKEN"
# 预期: 400 {"error": "invalid_grant"}
```

### 修复建议
- Access Token 有效期: 15-60 分钟
- Refresh Token 有效期: 7-30 天
- 实现 Token Rotation (每次刷新生成新的 refresh token)
- 支持 Token 黑名单 (Redis)

---

## 场景 4：Token 声明篡改

### 前置条件
- 有效的 JWT Token

### 攻击目标
验证是否可以篡改 Token 中的 claims

### 攻击步骤
1. 解码 JWT Token
2. 尝试修改以下 claims：
   - `sub` - 更改为其他用户 ID
   - `tenant_id` - 更改为其他租户
   - `roles` - 添加 `admin` 角色
   - `permissions` - 添加额外权限
   - `exp` - 延长过期时间
3. 重新签名 (如果有密钥) 或直接使用

### 预期安全行为
- 任何篡改都应导致签名验证失败
- 返回 401 错误

### 验证方法
```bash
# 使用 jwt.io 或脚本修改 payload
# 修改 roles: ["admin"]
# 重新编码但保持原签名

TAMPERED_TOKEN="eyJ...tampered_payload...original_signature"

curl -H "Authorization: Bearer $TAMPERED_TOKEN" \
  http://localhost:8080/api/v1/tenants
# 预期: 401 {"error": "invalid_signature"}
```

### 修复建议
- 始终验证签名
- 服务端验证 claims 合理性
- 敏感操作从数据库重新获取权限

---

## 场景 5：Token Exchange 安全测试

### 前置条件
- 有效的 Identity Token
- gRPC 客户端工具

### 攻击目标
验证 Token Exchange 流程的安全性

### 攻击步骤
1. 使用有效 Identity Token 请求 Token Exchange
2. 尝试请求未授权的 tenant_id
3. 尝试请求未授权的 service_id
4. 检查返回的 Tenant Access Token 权限

### 预期安全行为
- 仅能交换用户实际所属租户的 Token
- 拒绝未授权的 tenant_id 请求
- Token 中的权限与数据库一致

### 验证方法
```bash
# 使用 grpcurl 测试 Token Exchange
grpcurl -plaintext \
  -d '{
    "identity_token": "valid_token_here",
    "tenant_id": "unauthorized_tenant_id",
    "service_id": "test-service"
  }' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
# 预期: gRPC 错误 "User not member of tenant"

# 验证返回的 Token 权限
# 解析并确认权限与数据库一致
```

### 修复建议
- 验证用户与租户的关联关系
- 从数据库实时查询权限 (不信任请求参数)
- 记录 Token Exchange 审计日志
- 实现调用方认证 (mTLS/API Key)

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | JWT 签名算法混淆攻击 | ☐ | | | |
| 2 | JWT 密钥泄露测试 | ☐ | | | |
| 3 | Token 有效期与刷新测试 | ☐ | | | |
| 4 | Token 声明篡改 | ☐ | | | |
| 5 | Token Exchange 安全测试 | ☐ | | | |

---

## 参考资料

- [RFC 7519 - JWT](https://datatracker.ietf.org/doc/html/rfc7519)
- [JWT Security Best Practices](https://curity.io/resources/learn/jwt-best-practices/)
- [CWE-347: Improper Verification of Cryptographic Signature](https://cwe.mitre.org/data/definitions/347.html)
- [Auth0 JWT Handbook](https://auth0.com/resources/ebooks/jwt-handbook)
