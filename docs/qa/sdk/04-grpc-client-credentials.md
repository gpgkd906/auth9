# SDK - gRPC 客户端与 Client Credentials 测试

**模块**: SDK
**测试范围**: Auth9GrpcClient（4 个 RPC 方法）、ClientCredentials（M2M 认证、Token 缓存）
**场景数**: 5
**优先级**: 高

---

## 背景说明

### gRPC 客户端

`Auth9GrpcClient` 封装了 `auth9.proto` 中的 `TokenExchange` 服务（4 个方法），使用 `@grpc/grpc-js` 动态加载 proto 文件。

| 方法 | 用途 |
|------|------|
| `exchangeToken()` | Identity Token → Tenant Access Token |
| `validateToken()` | 验证 Access Token 有效性 |
| `getUserRoles()` | 查询用户在租户中的角色/权限 |
| `introspectToken()` | Token 内省（详细信息） |

### Client Credentials

`ClientCredentials` 封装 M2M（Machine-to-Machine）认证流程，自动缓存 Token 并在到期前 30 秒刷新。

测试方法：集成测试需要 Docker 环境（auth9-core + gRPC 端口 50051）

---

## 场景 1：gRPC Token Exchange 完整流程

### 初始状态
- auth9-core 运行中，gRPC 端口 50051 可访问
- 用户已登录，拥有有效的 Identity Token
- 用户是目标租户的成员，在该租户有角色分配

### 目的
验证 SDK gRPC 客户端完成 Token Exchange，获取包含角色和权限的 Tenant Access Token

### 测试操作流程
1. 创建 gRPC 客户端并调用 exchangeToken：
   ```typescript
   import { Auth9 } from "@auth9/node";

   const auth9 = new Auth9({ domain: "http://localhost:8080" });
   const grpc = auth9.grpc({ address: "localhost:50051" });

   const result = await grpc.exchangeToken({
     identityToken: "{identity_token}",
     tenantId: "{tenant_id}",
     serviceId: "{service_id}",
   });

   console.log(result.accessToken);   // JWT 字符串
   console.log(result.tokenType);     // "Bearer"
   console.log(result.expiresIn);     // 秒数
   console.log(result.refreshToken);  // JWT 字符串

   grpc.close();
   ```
2. 解码 accessToken 检查 claims
3. 验证返回字段命名为 camelCase

### 预期结果
- `result.accessToken` 是有效的 JWT（三段式）
- `result.tokenType` === `"Bearer"`
- `result.expiresIn` > 0
- 解码后 claims 包含 `tenant_id`、`roles`、`permissions`
- 字段名为 camelCase（`accessToken` 不是 `access_token`）

### 预期数据状态
```sql
-- 验证用户是租户成员
SELECT tu.id FROM tenant_users tu
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 存在记录
```

---

## 场景 2：gRPC ValidateToken 与 IntrospectToken

### 初始状态
- 已通过场景 1 获取有效的 Tenant Access Token
- gRPC 端口可用

### 目的
验证 Token 验证和内省 API

### 测试操作流程
1. 验证 Token：
   ```typescript
   const grpc = auth9.grpc({ address: "localhost:50051" });

   const validateResult = await grpc.validateToken({
     accessToken: "{tenant_access_token}",
   });
   console.log(validateResult.valid);     // true
   console.log(validateResult.userId);    // UUID
   console.log(validateResult.tenantId);  // UUID
   ```
2. 内省 Token：
   ```typescript
   const introspectResult = await grpc.introspectToken({
     token: "{tenant_access_token}",
   });
   console.log(introspectResult.active);       // true
   console.log(introspectResult.sub);           // user UUID
   console.log(introspectResult.email);         // email
   console.log(introspectResult.roles);         // string[]
   console.log(introspectResult.permissions);   // string[]

   grpc.close();
   ```
3. 使用无效 Token 调用 validateToken：
   ```typescript
   const invalidResult = await grpc.validateToken({
     accessToken: "invalid-token",
   });
   ```

### 预期结果
- 有效 Token：`valid === true`，userId 和 tenantId 非空
- introspect：`active === true`，包含完整用户信息、roles、permissions
- 无效 Token：`valid === false`，error 字段包含错误信息

---

## 场景 3：gRPC GetUserRoles

### 初始状态
- 用户在租户中已分配角色和权限

### 目的
验证 GetUserRoles API 返回的角色和权限与数据库一致

### 测试操作流程
1. 调用 GetUserRoles：
   ```typescript
   const grpc = auth9.grpc({ address: "localhost:50051" });

   const result = await grpc.getUserRoles({
     userId: "{user_id}",
     tenantId: "{tenant_id}",
   });

   console.log(result.roles);        // Array<{ id, name, serviceId }>
   console.log(result.permissions);   // string[]

   grpc.close();
   ```
2. 比对数据库查询结果

### 预期结果
- `result.roles` 每个元素包含 `id`（UUID）、`name`（角色名）、`serviceId`（服务 UUID）
- `result.permissions` 是派生的权限代码列表
- 字段名为 camelCase（`serviceId` 不是 `service_id`）

### 预期数据状态
```sql
SELECT r.id, r.name, r.service_id FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 与 result.roles 一致

SELECT DISTINCT p.code FROM user_tenant_roles utr
JOIN role_permissions rp ON rp.role_id = utr.role_id
JOIN permissions p ON p.id = rp.permission_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 与 result.permissions 一致
```

---

## 场景 4：Client Credentials Token 获取与缓存

### 初始状态
- auth9-core 运行中
- 已创建服务，拥有有效的 client_id 和 client_secret

### 目的
验证 ClientCredentials 获取 M2M Token 并正确缓存

### 测试操作流程
1. 创建 ClientCredentials 实例并获取 Token：
   ```typescript
   import { ClientCredentials } from "@auth9/node";

   const creds = new ClientCredentials({
     domain: "http://localhost:8080",
     clientId: "{client_id}",
     clientSecret: "{client_secret}",
   });

   const token1 = await creds.getToken();
   console.log(token1); // JWT 字符串
   ```
2. 再次获取 Token，验证缓存生效：
   ```typescript
   const token2 = await creds.getToken();
   console.log(token1 === token2); // true，使用缓存
   ```
3. 清除缓存后重新获取：
   ```typescript
   creds.clearCache();
   const token3 = await creds.getToken();
   console.log(token3 !== token1); // true，新 Token
   ```

### 预期结果
- 首次调用：发出 HTTP 请求，返回有效 JWT
- 第二次调用：不发出 HTTP 请求（缓存命中），返回相同 Token
- `clearCache()` 后：重新发出 HTTP 请求，返回新 Token
- Token 是三段式 JWT 格式

---

## 场景 5：Client Credentials 错误处理

### 初始状态
- auth9-core 运行中

### 目的
验证无效凭证被正确拒绝

### 测试操作流程
1. 使用错误的 client_secret：
   ```typescript
   const badCreds = new ClientCredentials({
     domain: "http://localhost:8080",
     clientId: "{valid_client_id}",
     clientSecret: "wrong-secret",
   });

   try {
     await badCreds.getToken();
   } catch (err) {
     console.log(err.statusCode); // 401
   }
   ```
2. 使用不存在的 client_id：
   ```typescript
   const noCreds = new ClientCredentials({
     domain: "http://localhost:8080",
     clientId: "non-existent",
     clientSecret: "any",
   });

   try {
     await noCreds.getToken();
   } catch (err) {
     console.log(err.statusCode); // 401 或 404
   }
   ```
3. 使用错误的 domain：
   ```typescript
   const wrongDomain = new ClientCredentials({
     domain: "http://localhost:9999",
     clientId: "any",
     clientSecret: "any",
   });

   try {
     await wrongDomain.getToken();
   } catch (err) {
     // 连接错误
   }
   ```

### 预期结果
- 错误凭证：抛出 `UnauthorizedError`（statusCode=401）
- 不存在的客户端：抛出 `UnauthorizedError` 或 `NotFoundError`
- 连接失败：抛出网络错误
- 所有错误都有明确的 message 描述

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | gRPC Token Exchange | ☐ | | | |
| 2 | gRPC Validate/Introspect | ☐ | | | |
| 3 | gRPC GetUserRoles | ☐ | | | |
| 4 | Client Credentials 缓存 | ☐ | | | |
| 5 | Client Credentials 错误 | ☐ | | | |
