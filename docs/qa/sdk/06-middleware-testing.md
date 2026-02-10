# SDK - Next.js/Fastify 中间件与测试工具

**模块**: SDK
**测试范围**: Next.js middleware、Fastify plugin、createMockToken、createMockAuth9
**场景数**: 5
**优先级**: 中

---

## 背景说明

### 框架中间件

| 框架 | 导入路径 | 认证信息位置 |
|------|---------|-------------|
| Next.js | `@auth9/node/middleware/next` | Response headers（x-auth9-*） |
| Fastify | `@auth9/node/middleware/fastify` | `request.auth9` |

### 测试工具

`@auth9/node/testing` 提供免验证的 mock 工具，用于下游应用的单元测试：
- `createMockToken(claims?)` — 生成测试用 JWT（无签名验证）
- `createMockAuth9(config?)` — 创建跳过验证的 mock 中间件实例

---

## 场景 1：Next.js Middleware — 公开路径与保护路径

### 初始状态
- Next.js 应用已配置 auth9 middleware

### 目的
验证 Next.js middleware 的公开路径跳过和保护路径拦截

### 测试操作流程
1. 配置 Next.js middleware：
   ```typescript
   // middleware.ts
   import { auth9Middleware } from "@auth9/node/middleware/next";

   export default auth9Middleware({
     domain: "http://localhost:8080",
     audience: "my-service",
     publicPaths: ["/", "/login", "/api/health"],
   });
   ```
2. 请求公开路径（无 Token）：
   ```bash
   curl http://localhost:3000/
   curl http://localhost:3000/login
   curl http://localhost:3000/api/health
   ```
3. 请求保护路径（无 Token）：
   ```bash
   curl http://localhost:3000/api/users
   ```
4. 请求保护路径（带有效 Token）：
   ```bash
   curl -H "Authorization: Bearer {valid_token}" \
     http://localhost:3000/api/users
   ```

### 预期结果
- 公开路径：返回 200（不检查 Token）
- 保护路径无 Token：返回 401 JSON `{ "error": "unauthorized", "message": "Missing authorization token" }`
- 保护路径有效 Token：返回 200，Response headers 包含：
  - `x-auth9-user-id`: 用户 UUID
  - `x-auth9-email`: 用户邮箱
  - `x-auth9-token-type`: Token 类型
  - `x-auth9-tenant-id`: 租户 UUID（TenantAccess Token）
  - `x-auth9-roles`: JSON 数组字符串
  - `x-auth9-permissions`: JSON 数组字符串

---

## 场景 2：Fastify Plugin — request.auth9 注入

### 初始状态
- Fastify 应用已注册 auth9Plugin

### 目的
验证 Fastify plugin 正确解析 Token 并注入 `request.auth9`

### 测试操作流程
1. 配置 Fastify 应用：
   ```typescript
   import fastify from "fastify";
   import { auth9Plugin } from "@auth9/node/middleware/fastify";

   const app = fastify();
   await app.register(auth9Plugin, {
     domain: "http://localhost:8080",
     audience: "my-service",
   });

   app.get("/me", async (request) => {
     if (!request.auth9) {
       return { error: "Not authenticated" };
     }
     return {
       userId: request.auth9.userId,
       email: request.auth9.email,
       tokenType: request.auth9.tokenType,
       hasAdminRole: request.auth9.hasRole("admin"),
     };
   });
   ```
2. 无 Token 请求：
   ```bash
   curl http://localhost:3002/me
   ```
3. 有效 Token 请求：
   ```bash
   curl -H "Authorization: Bearer {valid_token}" \
     http://localhost:3002/me
   ```

### 预期结果
- 无 Token：`request.auth9` === `undefined`，返回 `{ error: "Not authenticated" }`
- 有效 Token：`request.auth9` 包含 userId、email、tokenType、roles、permissions
- `request.auth9.hasRole("admin")` 正确返回 boolean
- `request.auth9.hasPermission("user:read")` 正确返回 boolean
- 无效 Token：`request.auth9` === `undefined`（静默失败，不中断请求）

---

## 场景 3：createMockToken — 生成测试 Token

### 初始状态
- `@auth9/node` 包已安装

### 目的
验证 mock token 生成功能，用于下游应用的测试

### 测试操作流程
1. 生成默认 mock token：
   ```typescript
   import { createMockToken } from "@auth9/node/testing";

   const token = createMockToken();
   console.log(token); // eyJhb...
   ```
2. 验证 token 格式（三段式 JWT）
3. 解码 payload 检查默认值：
   ```typescript
   const parts = token.split(".");
   const payload = JSON.parse(Buffer.from(parts[1], "base64url").toString());
   console.log(payload.sub);       // "test-user-id"
   console.log(payload.email);     // "test@example.com"
   console.log(payload.tenantId);  // "test-tenant-id"
   console.log(payload.roles);     // ["user"]
   ```
4. 使用自定义 claims 生成：
   ```typescript
   const adminToken = createMockToken({
     sub: "admin-user-id",
     email: "admin@example.com",
     roles: ["admin", "user"],
     permissions: ["user:read", "user:write", "user:delete"],
     tenantId: "custom-tenant",
   });
   ```

### 预期结果
- 默认 token：三段式 JWT 格式
- 默认 claims：sub="test-user-id", email="test@example.com", tenantId="test-tenant-id", roles=["user"]
- 自定义 token：claims 与传入值一致
- `exp` > `iat`（token 未过期）

---

## 场景 4：createMockAuth9 — Mock 中间件

### 初始状态
- `@auth9/node` 包已安装
- 下游 Express 应用需要测试

### 目的
验证 mock Auth9 实例跳过真实验证，直接注入认证信息

### 测试操作流程
1. 创建 mock 实例并使用中间件：
   ```typescript
   import { createMockAuth9, createMockToken } from "@auth9/node/testing";

   const mockAuth9 = createMockAuth9({
     defaultUser: {
       sub: "test-user",
       email: "test@example.com",
       roles: ["admin"],
       permissions: ["user:read", "user:write"],
     },
   });

   // 在测试中替换真实中间件
   app.use(mockAuth9.middleware());
   ```
2. 不带 Token 请求（使用默认用户）：
   ```typescript
   const req = { headers: {} };
   const res = {};
   mockAuth9.middleware()(req, res, () => {
     console.log(req.auth.userId);     // "test-user"
     console.log(req.auth.email);      // "test@example.com"
     console.log(req.auth.roles);      // ["admin"]
   });
   ```
3. 带自定义 Token 请求（从 Token 解析用户）：
   ```typescript
   const customToken = createMockToken({ sub: "other-user", email: "other@test.com" });
   const req2 = { headers: { authorization: `Bearer ${customToken}` } };
   mockAuth9.middleware()(req2, res, () => {
     console.log(req2.auth.userId);  // "other-user"
   });
   ```
4. 使用 verifyToken：
   ```typescript
   const claims = mockAuth9.verifyToken(customToken);
   console.log(claims.sub);  // "other-user"
   ```

### 预期结果
- 无 Token：使用 defaultUser 配置填充 req.auth
- 有 Token：从 Token payload 解析 claims 填充 req.auth
- `req.auth.hasPermission()` / `req.auth.hasRole()` 正常工作
- `verifyToken()` 返回 Token payload（不验证签名）
- 整个过程不需要网络连接（无 JWKS 请求）

---

## 场景 5：SDK 构建输出格式验证

### 初始状态
- SDK 源码完整

### 目的
验证 SDK 双格式构建（ESM + CJS）和类型声明文件

### 测试操作流程
1. 构建两个包：
   ```bash
   cd sdk
   pnpm run --filter @auth9/core build
   pnpm run --filter @auth9/node build
   ```
2. 检查 @auth9/core 产物：
   ```bash
   ls sdk/packages/core/dist/
   # 应包含: index.js, index.cjs, index.d.ts, index.d.cts
   ```
3. 检查 @auth9/node 产物：
   ```bash
   ls sdk/packages/node/dist/
   # 应包含: index.js, index.cjs, index.d.ts
   # 以及 middleware/express.js, middleware/next.js, middleware/fastify.js
   # 以及 testing.js
   ```
4. 运行所有测试：
   ```bash
   pnpm run --filter @auth9/core test
   pnpm run --filter @auth9/node test
   ```
5. 在 CJS 环境中导入测试：
   ```javascript
   const { Auth9HttpClient, toSnakeCase } = require("@auth9/core");
   console.log(typeof Auth9HttpClient);  // "function"
   console.log(typeof toSnakeCase);      // "function"
   ```
6. 在 ESM 环境中导入测试：
   ```typescript
   import { Auth9, TokenVerifier } from "@auth9/node";
   import { auth9Middleware } from "@auth9/node/middleware/express";
   import { createMockToken } from "@auth9/node/testing";
   ```

### 预期结果
- @auth9/core 构建产物包含：`index.js`（ESM）、`index.cjs`（CJS）、`index.d.ts`、`index.d.cts`
- @auth9/node 构建产物包含 5 个入口点的双格式输出
- 所有测试通过（@auth9/core 40 tests + @auth9/node 8 tests = 48 total）
- CJS `require()` 和 ESM `import` 都能正确导入
- `.d.ts` 类型声明完整，IDE 类型提示可用

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Next.js Middleware 路径控制 | ☐ | | | |
| 2 | Fastify Plugin 注入 | ☐ | | | |
| 3 | createMockToken 生成 | ☐ | | | |
| 4 | createMockAuth9 中间件 | ☐ | | | |
| 5 | SDK 构建输出格式 | ☐ | | | |
