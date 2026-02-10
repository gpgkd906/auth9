# SDK - Core 类型定义与工具函数测试

**模块**: SDK
**测试范围**: @auth9/core 类型导出、snake_case ↔ camelCase 转换、错误类型体系
**场景数**: 5
**优先级**: 高

---

## 背景说明

`@auth9/core` 是 Auth9 SDK 的核心包，提供所有领域类型（camelCase 命名风格）、HTTP 客户端、错误层级和命名转换工具。SDK 对外使用 camelCase，内部自动与 auth9-core 的 snake_case API 互转。

测试方法：使用 vitest 单元测试，运行命令：

```bash
cd sdk && pnpm run --filter @auth9/core test
```

---

## 场景 1：snake_case → camelCase 转换

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证 `toCamelCase()` 正确递归转换 auth9-core API 返回的 snake_case 响应

### 测试操作流程
1. 构造模拟 API 响应（snake_case 格式）：
   ```typescript
   import { toCamelCase } from "@auth9/core";

   const apiResponse = {
     tenant_id: "abc-123",
     display_name: "Test User",
     mfa_enabled: true,
     created_at: "2024-01-01T00:00:00Z",
     nested_data: {
       role_in_tenant: "admin",
       logo_url: null
     }
   };

   const result = toCamelCase(apiResponse);
   ```
2. 验证所有 key 已转为 camelCase
3. 验证嵌套对象的 key 也被转换
4. 验证 value 未被修改

### 预期结果
- `result.tenantId` === `"abc-123"`
- `result.displayName` === `"Test User"`
- `result.mfaEnabled` === `true`
- `result.nestedData.roleInTenant` === `"admin"`
- `result.nestedData.logoUrl` === `null`
- 不存在任何 `_` 分隔的 key

---

## 场景 2：camelCase → snake_case 转换

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证 `toSnakeCase()` 正确将 SDK 请求体转换为 auth9-core 期望的 snake_case 格式

### 测试操作流程
1. 构造 SDK 端请求对象（camelCase 格式）：
   ```typescript
   import { toSnakeCase } from "@auth9/core";

   const input = {
     tenantName: "My Tenant",
     logoUrl: "https://example.com/logo.png",
     redirectUris: ["https://app.example.com/callback"],
     roleIds: ["role-1", "role-2"]
   };

   const result = toSnakeCase(input);
   ```
2. 验证 key 转换
3. 验证数组内的对象也被转换：
   ```typescript
   const arrayInput = [
     { userId: "u1", roleInTenant: "admin" },
     { userId: "u2", roleInTenant: "member" }
   ];
   const arrayResult = toSnakeCase(arrayInput);
   ```

### 预期结果
- `result.tenant_name` === `"My Tenant"`
- `result.logo_url` === `"https://example.com/logo.png"`
- `result.redirect_uris` 保持为数组 `["https://app.example.com/callback"]`
- `result.role_ids` 保持为数组 `["role-1", "role-2"]`
- `arrayResult[0].user_id` === `"u1"`
- `arrayResult[0].role_in_tenant` === `"admin"`

---

## 场景 3：转换边界条件

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证转换函数对 null、undefined、原始类型、空对象的处理

### 测试操作流程
1. 测试各类边界输入：
   ```typescript
   import { toSnakeCase, toCamelCase } from "@auth9/core";

   // null / undefined
   toCamelCase(null);
   toCamelCase(undefined);

   // 原始类型
   toCamelCase("hello");
   toCamelCase(42);
   toCamelCase(true);

   // 空对象和空数组
   toCamelCase({});
   toCamelCase([]);
   ```
2. 测试 roundtrip 一致性：
   ```typescript
   const original = { tenantId: "123", mfaEnabled: true, createdAt: "2024-01-01" };
   const roundtripped = toCamelCase(toSnakeCase(original));
   ```

### 预期结果
- `toCamelCase(null)` === `null`
- `toCamelCase(undefined)` === `undefined`
- `toCamelCase("hello")` === `"hello"`（原始类型直接返回）
- `toCamelCase(42)` === `42`
- `toCamelCase({})` deep equals `{}`
- `toCamelCase([])` deep equals `[]`
- Roundtrip 结果与原始对象 deep equal

---

## 场景 4：错误类型体系

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证错误类层级关系和 `createErrorFromStatus()` 映射

### 测试操作流程
1. 验证错误类继承关系：
   ```typescript
   import {
     Auth9Error, NotFoundError, UnauthorizedError,
     ForbiddenError, ValidationError, ConflictError,
     RateLimitError, BadRequestError, createErrorFromStatus
   } from "@auth9/core";

   const err = new NotFoundError("User not found");
   console.log(err instanceof Auth9Error);  // true
   console.log(err instanceof Error);       // true
   console.log(err.statusCode);             // 404
   console.log(err.code);                   // "not_found"
   console.log(err.name);                   // "NotFoundError"
   ```
2. 验证 `createErrorFromStatus()` 映射所有状态码：
   ```typescript
   const errors = [
     createErrorFromStatus(400, { message: "Bad" }),
     createErrorFromStatus(401, { message: "Unauth" }),
     createErrorFromStatus(403, { message: "Forbid" }),
     createErrorFromStatus(404, { message: "Not found" }),
     createErrorFromStatus(409, { message: "Conflict" }),
     createErrorFromStatus(422, { message: "Invalid" }),
     createErrorFromStatus(429, { message: "Rate limit" }),
     createErrorFromStatus(502, { error: "gateway", message: "Bad gateway" }),
   ];
   ```

### 预期结果
- 400 → `BadRequestError`
- 401 → `UnauthorizedError`
- 403 → `ForbiddenError`
- 404 → `NotFoundError`
- 409 → `ConflictError`
- 422 → `ValidationError`
- 429 → `RateLimitError`
- 502 → 基类 `Auth9Error`（statusCode = 502）
- 所有错误都是 `Auth9Error` 和 `Error` 的实例

---

## 场景 5：Token Claims 类型辨别

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证 `getTokenType()` 能正确区分三种 Token 类型

### 测试操作流程
1. 测试 Identity Token（aud = "auth9"）：
   ```typescript
   import { getTokenType } from "@auth9/core";
   import type { IdentityClaims, TenantAccessClaims, ServiceClientClaims } from "@auth9/core";

   const identity: IdentityClaims = {
     sub: "user-1", email: "test@example.com",
     iss: "https://auth9.test", aud: "auth9",
     iat: 1000000, exp: 1003600
   };
   getTokenType(identity); // "identity"
   ```
2. 测试 Tenant Access Token（aud = 自定义 service client_id）：
   ```typescript
   const tenantAccess: TenantAccessClaims = {
     sub: "user-1", email: "test@example.com",
     iss: "https://auth9.test", aud: "my-service",
     tenantId: "tenant-1", roles: ["admin"], permissions: ["user:read"],
     iat: 1000000, exp: 1003600
   };
   getTokenType(tenantAccess); // "tenantAccess"
   ```
3. 测试 Service Client Token（aud = "auth9-service"）：
   ```typescript
   const serviceClient: ServiceClientClaims = {
     sub: "service-1", email: "svc@auth9.local",
     iss: "https://auth9.test", aud: "auth9-service",
     iat: 1000000, exp: 1003600
   };
   getTokenType(serviceClient); // "serviceClient"
   ```

### 预期结果
- `aud === "auth9"` → `"identity"`
- `aud === "auth9-service"` → `"serviceClient"`
- 其他 `aud` 值 → `"tenantAccess"`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | snake_case → camelCase 转换 | ☐ | | | |
| 2 | camelCase → snake_case 转换 | ☐ | | | |
| 3 | 转换边界条件 | ☐ | | | |
| 4 | 错误类型体系 | ☐ | | | |
| 5 | Token Claims 类型辨别 | ☐ | | | |
