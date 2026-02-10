# SDK - Express 中间件测试

**模块**: SDK
**测试范围**: auth9Middleware、requirePermission、requireRole、AuthInfo 对象
**场景数**: 5
**优先级**: 高

---

## 背景说明

Express 中间件提供三个功能：
1. `auth9Middleware(config)` — 验证 JWT Token，注入 `req.auth`
2. `requirePermission(permissions, options?)` — 检查用户权限
3. `requireRole(roles, options?)` — 检查用户角色

中间件参考了 `auth9-core/src/middleware/auth.rs` 的 `AuthUser` 结构和 RBAC helpers。

`req.auth` 包含的 helper 方法：
- `hasPermission(p)` — 单个权限检查
- `hasRole(r)` — 单个角色检查
- `hasAnyPermission(ps)` — 任一权限匹配
- `hasAllPermissions(ps)` — 所有权限匹配

测试方法：使用 Express + supertest 或直接构造 req/res 对象

---

## 场景 1：成功认证 — req.auth 注入

### 初始状态
- auth9-core 运行中（RSA 密钥配置）
- 已有有效的 Tenant Access Token（包含 roles 和 permissions）
- Express 应用配置了 auth9Middleware

### 目的
验证中间件从 Bearer Token 中正确提取用户信息到 `req.auth`

### 测试操作流程
1. 配置 Express 应用：
   ```typescript
   import express from "express";
   import { auth9Middleware } from "@auth9/node/middleware/express";

   const app = express();
   app.use(auth9Middleware({
     domain: "http://localhost:8080",
     audience: "{service_client_id}",
   }));

   app.get("/test", (req, res) => {
     res.json({
       userId: req.auth?.userId,
       email: req.auth?.email,
       tokenType: req.auth?.tokenType,
       tenantId: req.auth?.tenantId,
       roles: req.auth?.roles,
       permissions: req.auth?.permissions,
     });
   });
   ```
2. 发送带有效 Token 的请求：
   ```bash
   curl -H "Authorization: Bearer {tenant_access_token}" \
     http://localhost:3001/test
   ```

### 预期结果
- 状态码 200
- `userId` 是有效 UUID
- `email` 是有效邮箱
- `tokenType` === `"tenantAccess"`
- `tenantId` 是有效 UUID
- `roles` 是字符串数组（如 `["admin"]`）
- `permissions` 是字符串数组（如 `["user:read"]`）

---

## 场景 2：认证失败 — 无 Token / 无效 Token

### 初始状态
- Express 应用配置了非 optional 的 auth9Middleware

### 目的
验证缺少或无效的 Token 被正确拒绝

### 测试操作流程
1. 不带 Authorization header：
   ```bash
   curl http://localhost:3001/test
   ```
2. 带无效 Token：
   ```bash
   curl -H "Authorization: Bearer invalid-token" \
     http://localhost:3001/test
   ```
3. 带错误格式的 header：
   ```bash
   curl -H "Authorization: Basic dXNlcjpwYXNz" \
     http://localhost:3001/test
   ```

### 预期结果
- 无 Token：返回 401，错误信息「Missing authorization token」
- 无效 Token：返回 401，错误信息「Invalid or expired token」
- Basic Auth：返回 401（不是 Bearer scheme）
- `req.auth` 不被设置

---

## 场景 3：Optional 模式

### 初始状态
- Express 应用配置了 `optional: true` 的 auth9Middleware

### 目的
验证 optional 模式下无 Token 时请求继续处理（req.auth = undefined）

### 测试操作流程
1. 配置 optional 中间件：
   ```typescript
   app.use(auth9Middleware({
     domain: "http://localhost:8080",
     optional: true,
   }));

   app.get("/public-or-private", (req, res) => {
     if (req.auth) {
       res.json({ message: "Authenticated", user: req.auth.email });
     } else {
       res.json({ message: "Anonymous" });
     }
   });
   ```
2. 不带 Token 请求：
   ```bash
   curl http://localhost:3001/public-or-private
   ```
3. 带有效 Token 请求：
   ```bash
   curl -H "Authorization: Bearer {valid_token}" \
     http://localhost:3001/public-or-private
   ```
4. 带无效 Token 请求：
   ```bash
   curl -H "Authorization: Bearer invalid" \
     http://localhost:3001/public-or-private
   ```

### 预期结果
- 无 Token：状态码 200，返回 `{ message: "Anonymous" }`
- 有效 Token：状态码 200，返回 `{ message: "Authenticated", user: "..." }`
- 无效 Token：状态码 200，返回 `{ message: "Anonymous" }`（optional 模式不拒绝）

---

## 场景 4：requirePermission 权限控制

### 初始状态
- 用户 Token 包含 permissions: `["user:read", "user:write"]`
- Express 路由配置了 requirePermission 中间件

### 目的
验证权限检查的 all/any 模式

### 测试操作流程
1. 配置权限路由：
   ```typescript
   app.get("/users", requirePermission("user:read"), handler);
   app.post("/users", requirePermission(["user:read", "user:write"]), handler);
   app.delete("/users/:id", requirePermission("user:delete"), handler);

   // any 模式
   app.patch("/users/:id",
     requirePermission(["user:write", "user:admin"], { mode: "any" }),
     handler
   );
   ```
2. 测试各路由：
   - GET /users（有 user:read 权限）
   - POST /users（有 user:read + user:write 权限）
   - DELETE /users/1（没有 user:delete 权限）
   - PATCH /users/1（有 user:write，any 模式匹配）

### 预期结果
- GET /users：200（有 user:read 权限）
- POST /users：200（有 user:read + user:write，all 模式满足）
- DELETE /users/1：403 Forbidden，错误信息「Missing required permission(s): user:delete」
- PATCH /users/1：200（any 模式，user:write 匹配）

---

## 场景 5：requireRole 角色控制与 AuthInfo helpers

### 初始状态
- 用户 Token 包含 roles: `["admin", "user"]`，permissions: `["user:read", "user:write"]`

### 目的
验证角色检查和 AuthInfo 对象的 helper 方法

### 测试操作流程
1. 配置角色路由：
   ```typescript
   app.get("/admin", requireRole("admin"), handler);
   app.get("/superadmin", requireRole("superadmin"), handler);
   app.get("/any-admin",
     requireRole(["admin", "superadmin"], { mode: "any" }),
     handler
   );
   ```
2. 测试 AuthInfo helper 方法：
   ```typescript
   app.get("/check-helpers", (req, res) => {
     res.json({
       hasReadPerm: req.auth!.hasPermission("user:read"),
       hasDeletePerm: req.auth!.hasPermission("user:delete"),
       isAdmin: req.auth!.hasRole("admin"),
       isSuperAdmin: req.auth!.hasRole("superadmin"),
       hasAnyWritePerm: req.auth!.hasAnyPermission(["user:write", "user:admin"]),
       hasAllPerms: req.auth!.hasAllPermissions(["user:read", "user:write"]),
       hasAllPermsIncDelete: req.auth!.hasAllPermissions(["user:read", "user:delete"]),
     });
   });
   ```

### 预期结果
- GET /admin：200（有 admin 角色）
- GET /superadmin：403，错误信息「Missing required role(s): superadmin」
- GET /any-admin：200（any 模式，admin 匹配）
- Helper 方法返回值：
  - `hasReadPerm`: true
  - `hasDeletePerm`: false
  - `isAdmin`: true
  - `isSuperAdmin`: false
  - `hasAnyWritePerm`: true（user:write 匹配）
  - `hasAllPerms`: true（user:read + user:write 都有）
  - `hasAllPermsIncDelete`: false（缺少 user:delete）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 成功认证 req.auth 注入 | ☐ | | | |
| 2 | 认证失败 | ☐ | | | |
| 3 | Optional 模式 | ☐ | | | |
| 4 | requirePermission | ☐ | | | |
| 5 | requireRole 与 AuthInfo helpers | ☐ | | | |
