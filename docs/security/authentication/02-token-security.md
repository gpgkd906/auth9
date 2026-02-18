# 认证安全 - JWT Token 安全测试

**模块**: 认证安全
**测试范围**: JWT Token 签发、验证和存储安全
**场景数**: 3
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-AUTH-02
**OWASP ASVS 5.0**: V9.1,V9.2,V9.3,V6.2
**回归任务映射**: Backlog #1, #20


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

## 场景 3：Token 声明篡改

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

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | JWT 签名算法混淆攻击 | ☐ | | | |
| 2 | JWT 密钥泄露测试 | ☐ | | | |
| 3 | Token 声明篡改 | ☐ | | | |

---

## 参考资料

- [RFC 7519 - JWT](https://datatracker.ietf.org/doc/html/rfc7519)
- [JWT Security Best Practices](https://curity.io/resources/learn/jwt-best-practices/)
- [CWE-347: Improper Verification of Cryptographic Signature](https://cwe.mitre.org/data/definitions/347.html)
- [Auth0 JWT Handbook](https://auth0.com/resources/ebooks/jwt-handbook)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTH-02  
**适用控制**: V9.1,V9.2,V9.3,V6.2  
**关联任务**: Backlog #1, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 3

### 执行清单
- [ ] M-AUTH-02-C01 | 控制: V9.1 | 任务: #1, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-02-C02 | 控制: V9.2 | 任务: #1, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-02-C03 | 控制: V9.3 | 任务: #1, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-02-C04 | 控制: V6.2 | 任务: #1, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
