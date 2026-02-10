# SDK - HTTP 客户端测试

**模块**: SDK
**测试范围**: Auth9HttpClient 请求构造、自动转换、错误映射、超时重试
**场景数**: 5
**优先级**: 高

---

## 背景说明

`Auth9HttpClient` 是 SDK 与 auth9-core REST API 交互的基础层，负责：
- 请求体自动 camelCase → snake_case
- 响应体自动 snake_case → camelCase
- Authorization header 自动注入
- HTTP 错误映射到类型化异常

测试方法：单元测试（mock fetch）或集成测试（连接 auth9-core）

```bash
# 单元测试
cd sdk && pnpm run --filter @auth9/core test

# 集成测试（需要 Docker 环境）
# 使用 Auth9HttpClient 直接调用 auth9-core API
```

---

## 场景 1：GET 请求与响应 camelCase 自动转换

### 初始状态
- auth9-core 运行中（集成测试）或 mock fetch（单元测试）
- 已有至少一个租户数据

### 目的
验证 GET 请求正确发送，响应体自动从 snake_case 转为 camelCase

### 测试操作流程
1. 创建 HTTP 客户端并发送 GET：
   ```typescript
   import { Auth9HttpClient } from "@auth9/core";

   const client = new Auth9HttpClient({
     baseUrl: "http://localhost:8080",
     accessToken: "{admin_token}",
   });

   const result = await client.get<{ data: { id: string; logoUrl?: string; createdAt: string } }>(
     "/api/v1/tenants/{tenant_id}"
   );
   ```
2. 检查返回值的 key 命名
3. 检查 Authorization header 是否携带

### 预期结果
- 返回对象的 key 为 camelCase（`logoUrl`、`createdAt`、不是 `logo_url`、`created_at`）
- 请求携带 `Authorization: Bearer {admin_token}`
- 返回的 pagination 字段（如有）也被转换：`perPage`（不是 `per_page`）、`totalPages`（不是 `total_pages`）

---

## 场景 2：POST 请求体自动 snake_case 转换

### 初始状态
- auth9-core 运行中
- 拥有有效的 admin token

### 目的
验证 POST 请求体自动从 camelCase 转为 auth9-core 期望的 snake_case

### 测试操作流程
1. 使用 camelCase 参数创建租户：
   ```typescript
   const client = new Auth9HttpClient({
     baseUrl: "http://localhost:8080",
     accessToken: "{admin_token}",
   });

   const result = await client.post("/api/v1/tenants", {
     name: "SDK Test Tenant",
     slug: "sdk-test",
     logoUrl: "https://example.com/logo.png",
   });
   ```
2. 验证 auth9-core 正确接收到 snake_case 请求体

### 预期结果
- 实际发送到 auth9-core 的请求体为 `{ "name": "SDK Test Tenant", "slug": "sdk-test", "logo_url": "https://example.com/logo.png" }`
- 创建成功（状态码 200）
- 返回值中 key 为 camelCase

### 预期数据状态
```sql
SELECT id, name, slug, logo_url FROM tenants WHERE slug = 'sdk-test';
-- 预期: 存在记录，logo_url = 'https://example.com/logo.png'

-- 清理
DELETE FROM tenants WHERE slug = 'sdk-test';
```

---

## 场景 3：HTTP 错误映射到类型化异常

### 初始状态
- auth9-core 运行中

### 目的
验证各 HTTP 状态码正确映射到 SDK 异常类型

### 测试操作流程
1. 触发 404 错误：
   ```typescript
   import { NotFoundError, UnauthorizedError, ConflictError } from "@auth9/core";

   try {
     await client.get("/api/v1/tenants/non-existent-id");
   } catch (err) {
     console.log(err instanceof NotFoundError);  // true
     console.log(err.statusCode);                 // 404
     console.log(err.code);                       // "not_found"
   }
   ```
2. 触发 401 错误（无 token）：
   ```typescript
   const noAuthClient = new Auth9HttpClient({ baseUrl: "http://localhost:8080" });
   try {
     await noAuthClient.get("/api/v1/tenants");
   } catch (err) {
     console.log(err instanceof UnauthorizedError);  // true
   }
   ```
3. 触发 409 冲突（重复 slug）：
   ```typescript
   await client.post("/api/v1/tenants", { name: "T1", slug: "default" });
   // 如果 slug "default" 已存在
   ```

### 预期结果
- 不存在的资源 → `NotFoundError`（statusCode=404, code="not_found"）
- 无认证令牌 → `UnauthorizedError`（statusCode=401）
- 重复冲突 → `ConflictError`（statusCode=409）
- 所有异常都包含 `message` 描述信息

---

## 场景 4：异步 Token Provider

### 初始状态
- `@auth9/core` 包已安装

### 目的
验证 `accessToken` 配置支持异步函数，每次请求自动调用获取最新 token

### 测试操作流程
1. 使用函数形式配置 token：
   ```typescript
   let callCount = 0;
   const client = new Auth9HttpClient({
     baseUrl: "http://localhost:8080",
     accessToken: async () => {
       callCount++;
       return await getLatestToken(); // 从某处获取动态 token
     },
   });

   await client.get("/api/v1/tenants");
   await client.get("/api/v1/users");
   ```
2. 验证每次请求都调用了 token 函数

### 预期结果
- `callCount` === 2（每次请求调用一次）
- 每次请求的 Authorization header 使用函数返回的 token
- 支持返回 `Promise<string>` 和直接 `string` 两种形式

---

## 场景 5：DELETE 请求与 204 No Content 处理

### 初始状态
- auth9-core 运行中
- 存在可删除的测试租户

### 目的
验证 DELETE 请求正确处理 204 No Content 响应

### 测试操作流程
1. 先创建一个测试租户：
   ```typescript
   const created = await client.post("/api/v1/tenants", {
     name: "To Delete",
     slug: "to-delete-sdk",
   });
   const tenantId = created.data.id;
   ```
2. 删除该租户：
   ```typescript
   const result = await client.delete(`/api/v1/tenants/${tenantId}`);
   ```
3. 验证返回值和数据库状态

### 预期结果
- `client.delete()` 返回 `undefined`（无返回体）
- 不抛出异常
- 再次 GET 该租户返回 404

### 预期数据状态
```sql
SELECT id FROM tenants WHERE slug = 'to-delete-sdk';
-- 预期: 无记录
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | GET 响应 camelCase 转换 | ☐ | | | |
| 2 | POST 请求体 snake_case 转换 | ☐ | | | |
| 3 | HTTP 错误映射 | ☐ | | | |
| 4 | 异步 Token Provider | ☐ | | | |
| 5 | DELETE 与 204 处理 | ☐ | | | |
