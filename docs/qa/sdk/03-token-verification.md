# SDK - Token 验证测试

**模块**: SDK
**测试范围**: @auth9/node TokenVerifier JWKS 验证、三种 Token 类型解析、Auth9 主类
**场景数**: 5
**优先级**: 高

---

## 背景说明

`TokenVerifier` 使用 `jose` 库从 `{domain}/.well-known/jwks.json` 获取公钥，本地验证 JWT 签名。支持三种 Token 类型：

| Token 类型 | aud 值 | 包含字段 |
|-----------|--------|---------|
| Identity | `"auth9"` | sub, email, name?, sid? |
| TenantAccess | service client_id | sub, email, tenantId, roles, permissions |
| ServiceClient | `"auth9-service"` | sub, email, tenantId? |

测试方法：
- 单元测试：mock JWKS endpoint
- 集成测试：连接 auth9-core（需要 RSA 密钥配置）

```bash
cd sdk && pnpm run --filter @auth9/node test
```

---

## 场景 1：验证 Identity Token

### 初始状态
- auth9-core 运行中，配置了 RSA 密钥对
- 已通过登录获取 Identity Token
- JWKS 端点可用：`GET http://localhost:8080/.well-known/jwks.json`

### 目的
验证 TokenVerifier 能正确验证 Identity Token 并返回类型化 claims

### 测试操作流程
1. 获取 Identity Token（通过登录或 gen-admin-token.sh）
2. 使用 TokenVerifier 验证：
   ```typescript
   import { TokenVerifier } from "@auth9/node";

   const verifier = new TokenVerifier({
     domain: "http://localhost:8080",
   });

   const { claims, tokenType } = await verifier.verify(identityToken);
   ```
3. 检查返回的 claims 和 tokenType

### 预期结果
- `tokenType` === `"identity"`
- `claims.aud` === `"auth9"`
- `claims.sub` 是有效的 UUID
- `claims.email` 是有效的邮箱
- `claims.iss` 匹配 auth9-core 配置的 issuer
- `claims.iat` 和 `claims.exp` 是有效的 Unix 时间戳
- `claims.exp > claims.iat`

---

## 场景 2：验证 Tenant Access Token

### 初始状态
- 已通过 gRPC Token Exchange 获取 Tenant Access Token
- 用户在该租户有 `admin` 角色和相关权限

### 目的
验证 TenantAccess Token 验证后正确提取 roles 和 permissions

### 测试操作流程
1. 通过 gRPC 获取 Tenant Access Token
2. 验证 Token：
   ```typescript
   const verifier = new TokenVerifier({
     domain: "http://localhost:8080",
     audience: "{service_client_id}",
   });

   const { claims, tokenType } = await verifier.verify(tenantAccessToken);
   ```
3. 检查 roles 和 permissions

### 预期结果
- `tokenType` === `"tenantAccess"`
- `claims.aud` === `"{service_client_id}"`（匹配配置的 audience）
- `claims.tenantId` 是有效的租户 UUID
- `claims.roles` 是字符串数组（如 `["admin"]`）
- `claims.permissions` 是字符串数组（如 `["user:read", "user:write"]`）
- `claims.sub` 是用户 UUID

### 预期数据状态
```sql
-- 验证用户确实有这些角色
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{claims.sub}' AND tu.tenant_id = '{claims.tenantId}';
-- 预期: 与 claims.roles 一致
```

---

## 场景 3：Token 签名验证失败

### 初始状态
- TokenVerifier 已初始化

### 目的
验证篡改或伪造的 Token 被正确拒绝

### 测试操作流程
1. 使用无效 Token 字符串：
   ```typescript
   const verifier = new TokenVerifier({
     domain: "http://localhost:8080",
   });

   // 完全无效的字符串
   try {
     await verifier.verify("not-a-jwt-token");
   } catch (err) {
     console.log(err.message); // 应包含验证失败信息
   }
   ```
2. 篡改合法 Token 的 payload：
   ```typescript
   // 取合法 token，修改 payload 部分
   const parts = validToken.split(".");
   const payload = JSON.parse(atob(parts[1]));
   payload.email = "hacker@evil.com";
   parts[1] = btoa(JSON.stringify(payload)).replace(/=/g, "");
   const tamperedToken = parts.join(".");

   try {
     await verifier.verify(tamperedToken);
   } catch (err) {
     // 签名验证应失败
   }
   ```
3. 使用过期 Token（exp 在过去）

### 预期结果
- 无效字符串：抛出异常
- 篡改 Token：签名验证失败，抛出异常
- 过期 Token：exp 验证失败，抛出异常
- 所有情况都不返回 claims

---

## 场景 4：Audience 验证

### 初始状态
- 拥有有效的 Tenant Access Token（aud = "my-service"）

### 目的
验证配置了 audience 时，不匹配的 Token 被拒绝

### 测试操作流程
1. 创建配置了特定 audience 的 verifier：
   ```typescript
   const verifier = new TokenVerifier({
     domain: "http://localhost:8080",
     audience: "other-service",  // 与 Token aud 不匹配
   });
   ```
2. 验证一个 aud="my-service" 的 Token：
   ```typescript
   try {
     await verifier.verify(tokenForMyService);
   } catch (err) {
     // audience 不匹配，应拒绝
   }
   ```
3. 不配置 audience 时应接受所有 Token：
   ```typescript
   const permissiveVerifier = new TokenVerifier({
     domain: "http://localhost:8080",
     // 不设置 audience
   });
   const result = await permissiveVerifier.verify(tokenForMyService);
   // 应成功
   ```

### 预期结果
- audience 不匹配 → 抛出异常
- 不配置 audience → 接受所有有效签名的 Token

---

## 场景 5：Auth9 主类统一入口

### 初始状态
- auth9-core 运行中

### 目的
验证 `Auth9` 主类的 `verifyToken()` 方法和 `getServiceToken()` 方法

### 测试操作流程
1. 初始化 Auth9 主类：
   ```typescript
   import { Auth9 } from "@auth9/node";

   const auth9 = new Auth9({
     domain: "http://localhost:8080",
     audience: "{service_client_id}",
     clientId: "{client_id}",
     clientSecret: "{client_secret}",
   });
   ```
2. 验证 Token：
   ```typescript
   const claims = await auth9.verifyToken(validToken);
   ```
3. 获取 Service Token：
   ```typescript
   const serviceToken = await auth9.getServiceToken();
   ```
4. 未配置 credentials 时调用 getServiceToken()：
   ```typescript
   const auth9NoCredentials = new Auth9({
     domain: "http://localhost:8080",
   });
   try {
     await auth9NoCredentials.getServiceToken();
   } catch (err) {
     // 应提示未配置 clientId/clientSecret
   }
   ```

### 预期结果
- `verifyToken()` 返回类型化 claims
- `getServiceToken()` 返回有效的 JWT 字符串
- 未配置 credentials 时 `getServiceToken()` 抛出明确错误信息：「Client credentials not configured」

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Identity Token 验证 | ☐ | | | |
| 2 | Tenant Access Token 验证 | ☐ | | | |
| 3 | Token 签名验证失败 | ☐ | | | |
| 4 | Audience 验证 | ☐ | | | |
| 5 | Auth9 主类统一入口 | ☐ | | | |
