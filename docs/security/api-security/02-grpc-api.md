# API 安全 - gRPC API 安全测试

**模块**: API 安全
**测试范围**: gRPC 服务安全
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-API-02
**OWASP ASVS 5.0**: V4.1,V4.2,V13.2
**回归任务映射**: Backlog #3, #20


---

## 背景知识

Auth9 gRPC API 概况：
- 端口: 50051
- 方法数: 4 个
- **当前状态**: 已支持 `GRPC_AUTH_MODE`（`none`/`api_key`/`mtls`），生产环境要求开启认证；主要风险为配置回归与绕过

关键方法：
- `ExchangeToken` - Identity Token → Tenant Access Token
- `ValidateToken` - 验证 Token 有效性
- `GetUserRoles` - 查询用户角色权限
- `IntrospectToken` - Token 内省 (调试用)

---

## 自动化回归脚本（推荐）

优先执行以下脚本进行 gRPC 安全基线回归（动态租户/用户夹具，避免硬编码误报）：

```bash
./scripts/qa/security_grpc_test.sh
```

脚本覆盖本页场景：
- 场景 1（未认证访问）✅
- 场景 2（跨租户 Token Exchange）✅
- 场景 5（TLS 端点拒绝明文）✅

场景 3/4 仍需按本文手工步骤做专项验证。

---

## 场景 1：未认证 gRPC 访问与认证配置回归

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
# 预期（生产安全基线）: 未带 x-api-key 时返回 UNAUTHENTICATED
# 回归风险（非生产/误配置）: 可能退化为业务错误而非 UNAUTHENTICATED

# 未认证调用 GetUserRoles
grpcurl -plaintext \
  -d '{"user_id":"user-uuid","tenant_id":"tenant-uuid"}' \
  localhost:50051 auth9.TokenExchange/GetUserRoles
# 预期（生产安全基线）: 未带 x-api-key 时返回 UNAUTHENTICATED
# 回归风险（非生产/误配置）: 可能返回业务层错误
```

### 修复建议
- 固化生产基线：`ENVIRONMENT=production` + `GRPC_AUTH_MODE=api_key` + `GRPC_API_KEYS` 非空
- mTLS 场景下补齐证书链校验与轮换流程
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

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| Exchange 成功但预期失败 | 用户实际是目标租户的成员 | **测试前先查数据库确认**: `SELECT * FROM tenant_users WHERE user_id = ?` 确保用户只属于源租户 |
| 使用 admin 用户测试失败 | admin 用户可能被自动加入多个租户 | 创建新的测试用户，确保只关联单个租户 |

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

### 实现说明：防枚举设计

GetUserRoles 端点对以下两种情况返回 **相同的错误信息**（`"User not found in tenant"`）：
1. user_id 在系统中完全不存在
2. user_id 存在但不属于目标 tenant

这意味着 **无法通过此端点区分「用户不存在」和「用户存在但不在该租户」**，从而防止全局用户枚举。

> **对授权调用方的说明**：该端点要求认证（API key 或 mTLS），因此授权调用方可以判断某个用户是否在特定租户中是预期行为——这属于正常的业务查询能力，不构成安全漏洞。测试时需注意区分「未认证的枚举」（应被拦截）和「已认证的合法查询」。

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

# 防枚举验证：以下两个请求应返回完全相同的错误
RANDOM_UUID=$(uuidgen | tr '[:upper:]' '[:lower:]')
REAL_USER_NOT_IN_TENANT="{real_user_id_not_in_target_tenant}"

grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d "{\"user_id\":\"$RANDOM_UUID\",\"tenant_id\":\"$TENANT_ID\"}" \
  localhost:50051 auth9.TokenExchange/GetUserRoles
# 预期: NOT_FOUND "User not found in tenant"

grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d "{\"user_id\":\"$REAL_USER_NOT_IN_TENANT\",\"tenant_id\":\"$TENANT_ID\"}" \
  localhost:50051 auth9.TokenExchange/GetUserRoles
# 预期: NOT_FOUND "User not found in tenant" (相同的错误信息)
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| PermissionDenied 但用户确实是成员 | user_id UUID 拼写错误（如 `f042-4f54` 与 `f042-f54a` 混淆） | 从数据库复制精确 UUID: `SELECT user_id FROM tenant_users WHERE tenant_id = ?` |
| UUID 查询失败但 slug 成功 | 使用了错误的 tenant UUID | 用 `SELECT id, slug FROM tenants` 确认正确的 UUID |
| GetUserRoles 测试全部成功 | gRPC API key 认证已开启时 API key 有特殊权限 | 确认 `GRPC_AUTH_MODE` 配置 |

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

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-02  
**适用控制**: V4.1,V4.2,V13.2  
**关联任务**: Backlog #3, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-API-02-C01 | 控制: V4.1 | 任务: #3, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-02-C02 | 控制: V4.2 | 任务: #3, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-02-C03 | 控制: V13.2 | 任务: #3, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
