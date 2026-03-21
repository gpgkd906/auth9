# 认证流程 - Token Exchange 测试

**模块**: 认证流程
**测试范围**: Token Exchange、Token 验证
**场景数**: 5

---

## Token Exchange 说明

Token Exchange 流程：
1. 用户登录获得 Identity Token
2. 调用 gRPC TokenExchange.ExchangeToken
3. 获得包含角色和权限的 Tenant Access Token

---

## 场景 1：Token Exchange - 获取租户访问令牌

### 初始状态
- 用户已登录，持有有效的 Identity Token
- 用户是某租户的成员

### 目的
验证 Token Exchange 正常工作

### 测试操作流程
1. 调用 gRPC TokenExchange.ExchangeToken
   ```protobuf
   ExchangeTokenRequest {
     identity_token: "<Identity Token>"
     tenant_id: "{tenant_id}"
   }
   ```

### 预期结果
- 返回 Tenant Access Token
- Token 包含用户角色和权限

### 预期数据状态
```sql
SELECT tu.id FROM tenant_users tu JOIN users u ON u.id = tu.user_id
WHERE u.identity_subject = '{identity_subject}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 存在记录

-- Token 解码后应包含 roles 和 permissions
```

---

## 场景 2：Token Exchange - 用户不是租户成员

### 初始状态
- 用户已登录
- 用户不是请求的租户成员

> **重要**: 默认 seed 数据中 `admin@auth9.local` 同时属于 `auth9-platform` 和 `demo` 两个租户。
> 因此本场景 **不能使用 admin 用户** 测试，需要：
> - 创建一个新测试租户（如 `audit-test-tenant`），且 admin 不是其成员；或
> - 注册一个新用户，确保该用户只属于一个租户，然后请求另一个租户的 token。
>
> 在执行测试前，先验证数据库状态：
> ```sql
> SELECT tenant_id FROM tenant_users WHERE user_id = '{user_id}';
> -- 确认目标租户不在返回结果中
> ```

### 目的
验证非成员无法获取租户令牌

### 测试操作流程
1. 调用 Token Exchange，请求非成员租户的令牌

### 预期结果
- 返回错误：「用户不是该租户成员」
- 不返回任何 Token

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}' AND tenant_id = '{requested_tenant_id}';
-- 预期: 0
```

---

## 场景 3：Token 验证

### 初始状态
- 持有有效的 Tenant Access Token

### 目的
验证 Token 验证功能

### 测试操作流程
1. 调用 gRPC TokenExchange.ValidateToken
   ```protobuf
   ValidateTokenRequest {
     access_token: "<Tenant Access Token>"
     audience: "<service_id used in ExchangeToken>"
   }
   ```
   **注意**：`audience` 在生产环境（`ENVIRONMENT=production`）中为**必填**字段，须与 Token Exchange 时使用的 `service_id` 一致（如 `auth9-portal`）。

### 预期结果
- 返回 Token 有效
- 返回 Token 中的信息

---

## 场景 4：Token 过期验证

### 初始状态
- 持有已过期的 Token

### 目的
验证过期 Token 被正确拒绝

### 测试操作流程
1. 使用过期 Token 调用 ValidateToken，须包含 `audience` 参数
   ```protobuf
   ValidateTokenRequest {
     access_token: "<expired Token>"
     audience: "<service_id>"
   }
   ```
   **注意**：`audience` 在生产环境中为**必填**字段，否则会返回 `FailedPrecondition: audience is required in production`，掩盖真正的过期错误。

### 预期结果
- 返回错误：「Token 已过期」

---

## 场景 5：Token 内省

### 初始状态
- 服务持有 API Key
- 持有有效的 Access Token

### 目的
验证 Token 内省功能

### 测试操作流程
1. 调用 gRPC TokenExchange.IntrospectToken

### 预期结果
- 返回 Token 的详细信息
- 包含：用户信息、租户信息、角色、权限、过期时间

---

## Troubleshooting

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| ValidateToken 返回 `FailedPrecondition: audience is required in production` | Docker 环境为 `ENVIRONMENT=production`，`audience` 字段必填 | ValidateToken 请求中加入 `audience` 字段，值与 ExchangeToken 的 `service_id` 一致 |
| 场景 2 非成员测试通过了（应该失败） | `audit-test-tenant` 中 admin 用户已有 `tenant_users` 记录 | 测试前执行 `DELETE FROM tenant_users WHERE user_id='{admin_id}' AND tenant_id='{tenant_id}'` |
| ExchangeToken 返回 `PermissionDenied` 但预期成功 | 用户不在目标 tenant 中 | 检查 `tenant_users` 表确认用户-租户关系 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Token Exchange - 成功 | ✅ PASS | 2026-02-15 | Gemini | |
| 2 | Token Exchange - 非成员 | ✅ PASS | 2026-02-15 | Gemini | |
| 3 | Token 验证 | ✅ PASS | 2026-02-15 | Gemini | |
| 4 | Token 过期验证 | ✅ PASS | 2026-02-15 | Gemini | |
| 5 | Token 内省 | ✅ PASS | 2026-02-15 | Gemini | |
