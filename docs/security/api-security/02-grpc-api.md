# API 安全 - gRPC API 安全测试

**模块**: API 安全
**测试范围**: gRPC 服务安全
**场景数**: 5
**风险等级**: 🔴 极高

---

## 背景知识

Auth9 gRPC API 概况：
- 端口: 50051
- 方法数: 4 个
- **当前状态**: 全部无认证保护 (P0 安全风险)

关键方法：
- `ExchangeToken` - Identity Token → Tenant Access Token
- `ValidateToken` - 验证 Token 有效性
- `GetUserRoles` - 查询用户角色权限
- `IntrospectToken` - Token 内省 (调试用)

---

## 场景 1：未认证 gRPC 访问

### 前置条件
- gRPC 端口可访问
- grpcurl 或 gRPC 客户端

### 攻击目标
验证 gRPC 服务是否可被未授权访问

### 攻击步骤
1. 连接 gRPC 服务
2. 列出所有可用服务和方法
3. 不带任何认证调用每个方法
4. 检查是否可获取敏感数据

### 预期安全行为
- 需要 mTLS 或 API Key 认证
- 未认证请求返回 UNAUTHENTICATED
- 不泄露服务信息

### 验证方法
```bash
# 列出服务 (不应在生产环境公开)
grpcurl -plaintext localhost:50051 list
# 预期: 需要认证或禁用反射

# 未认证调用 ExchangeToken
grpcurl -plaintext \
  -d '{"identity_token":"dummy"}' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
# 当前: 可能返回 invalid token 错误
# 预期修复后: UNAUTHENTICATED

# 未认证调用 GetUserRoles
grpcurl -plaintext \
  -d '{"user_id":"user-uuid","tenant_id":"tenant-uuid"}' \
  localhost:50051 auth9.TokenExchange/GetUserRoles
# 当前: 可能返回用户角色信息
# 预期修复后: UNAUTHENTICATED
```

### 修复建议
- 实现 mTLS (推荐生产环境)
- 或实现 API Key Interceptor
- 禁用 gRPC 反射 (生产环境)
- 添加调用方身份日志

---

## 场景 2：Token Exchange 滥用

### 前置条件
- 有效的 Identity Token

### 攻击目标
验证 Token Exchange 是否可被滥用

### 攻击步骤
1. 使用有效 Identity Token 调用 ExchangeToken
2. 请求不属于用户的 tenant_id
3. 请求不存在的 service_id
4. 检查返回的权限

### 预期安全行为
- 验证用户与租户的关联
- 验证服务存在且属于租户
- 不能获取未授权的权限

### 验证方法
```bash
# 有效请求
grpcurl -plaintext \
  -d '{
    "identity_token": "'$VALID_IDENTITY_TOKEN'",
    "tenant_id": "'$USER_TENANT_ID'",
    "service_id": "'$VALID_SERVICE_ID'"
  }' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
# 预期: 返回有效的 Tenant Access Token

# 未授权租户
grpcurl -plaintext \
  -d '{
    "identity_token": "'$VALID_IDENTITY_TOKEN'",
    "tenant_id": "'$OTHER_TENANT_ID'",
    "service_id": "'$VALID_SERVICE_ID'"
  }' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
# 预期: PERMISSION_DENIED "User not member of tenant"

# 不存在的服务
grpcurl -plaintext \
  -d '{
    "identity_token": "'$VALID_IDENTITY_TOKEN'",
    "tenant_id": "'$USER_TENANT_ID'",
    "service_id": "non-existent-service"
  }' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
# 预期: NOT_FOUND "Service not found"
```

### 修复建议
- 验证所有输入参数
- 从数据库查询实际关联
- 返回的权限与数据库一致
- 记录所有 Exchange 操作

---

## 场景 3：用户角色枚举

### 前置条件
- gRPC 服务可访问

### 攻击目标
验证是否可以枚举用户角色信息

### 攻击步骤
1. 调用 GetUserRoles 枚举用户：
   - 遍历常见 user_id
   - 遍历 tenant_id
2. 收集用户角色信息
3. 分析权限分布

### 预期安全行为
- 需要认证
- 调用方只能查询授权范围内的用户
- 不存在的用户返回相同错误

### 验证方法
```bash
# 枚举用户角色
for user_id in user1 user2 user3 admin; do
  grpcurl -plaintext \
    -d "{\"user_id\":\"$user_id\",\"tenant_id\":\"tenant1\"}" \
    localhost:50051 auth9.TokenExchange/GetUserRoles
done

# 检查响应是否泄露用户存在性
# 不存在的用户应返回相同错误
```

### 修复建议
- 需要调用方认证
- 验证调用方查询权限
- 统一错误响应 (防枚举)
- 限制请求频率

---

## 场景 4：Token 内省安全

### 前置条件
- gRPC 服务可访问
- 有效的 Token

### 攻击目标
验证 IntrospectToken 是否泄露敏感信息

### 攻击步骤
1. 调用 IntrospectToken 检查各种 Token
2. 分析返回的详细信息
3. 检查生产环境是否开放

### 预期安全行为
- 生产环境应禁用或严格限制
- 不返回签名密钥等敏感信息
- 需要管理员权限

### 验证方法
```bash
# 调用 IntrospectToken
grpcurl -plaintext \
  -d '{"token": "'$TOKEN'"}' \
  localhost:50051 auth9.TokenExchange/IntrospectToken

# 检查返回内容
# 不应包含:
# - 签名密钥
# - 敏感的内部字段
# - 其他用户信息

# 生产环境应禁用
curl -s http://production:50051/health
# 检查 IntrospectToken 是否可用
```

### 修复建议
- 生产环境禁用 IntrospectToken
- 或限制为内部网络
- 需要管理员认证
- 脱敏返回数据

---

## 场景 5：gRPC 传输安全

### 前置条件
- 网络访问权限
- 流量捕获工具

### 攻击目标
验证 gRPC 通信是否加密

### 攻击步骤
1. 检查是否使用 TLS
2. 捕获 gRPC 流量
3. 尝试中间人攻击
4. 验证证书配置

### 预期安全行为
- 使用 TLS 加密
- 验证服务器证书
- 生产环境使用 mTLS

### 验证方法
```bash
# 检查是否支持 plaintext
grpcurl -plaintext localhost:50051 list
# 预期: 生产环境应拒绝

# 检查 TLS 连接
grpcurl -insecure localhost:50051 list
# 或
grpcurl -cacert ca.crt localhost:50051 list

# 证书信息
openssl s_client -connect localhost:50051 </dev/null 2>/dev/null | \
  openssl x509 -text -noout

# 使用 Wireshark 捕获流量
# 如果是 TLS，应该看到加密数据
```

### 修复建议
- 生产环境强制 TLS
- 使用 mTLS 双向认证
- 禁用不安全的连接
- 定期轮换证书

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 未认证 gRPC 访问 | ☐ | | | |
| 2 | Token Exchange 滥用 | ☐ | | | |
| 3 | 用户角色枚举 | ☐ | | | |
| 4 | Token 内省安全 | ☐ | | | |
| 5 | gRPC 传输安全 | ☐ | | | |

---

## gRPC 安全测试工具

```bash
# grpcurl - gRPC 命令行客户端
brew install grpcurl

# 列出服务
grpcurl -plaintext localhost:50051 list

# 描述方法
grpcurl -plaintext localhost:50051 describe auth9.TokenExchange

# 调用方法
grpcurl -plaintext -d '{"field":"value"}' \
  localhost:50051 service/Method

# ghz - gRPC 压力测试
brew install ghz
ghz --insecure --call auth9.TokenExchange/ExchangeToken \
  -d '{"identity_token":"..."}' \
  -n 1000 -c 10 localhost:50051
```

---

## 推荐的认证方案

### mTLS (生产环境推荐)

```rust
use tonic::transport::{Server, ServerTlsConfig, Identity, Certificate};

let server_identity = Identity::from_pem(cert, key);
let client_ca = Certificate::from_pem(ca_cert);

let tls = ServerTlsConfig::new()
    .identity(server_identity)
    .client_ca_root(client_ca);

Server::builder()
    .tls_config(tls)?
    .add_service(service)
    .serve(addr)
    .await?;
```

### API Key Interceptor (开发环境)

```rust
impl Interceptor for ApiKeyAuth {
    fn call(&mut self, req: Request<()>) -> Result<Request<()>, Status> {
        let api_key = req.metadata()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or(Status::unauthenticated("Missing API key"))?;

        if !self.valid_keys.contains(api_key) {
            return Err(Status::unauthenticated("Invalid API key"));
        }
        Ok(req)
    }
}
```

---

## 参考资料

- [gRPC Authentication](https://grpc.io/docs/guides/auth/)
- [Tonic TLS Guide](https://github.com/hyperium/tonic/tree/master/examples/src/tls)
- [API Access Control 文档](../../api-access-control.md)
- [CWE-306: Missing Authentication](https://cwe.mitre.org/data/definitions/306.html)
