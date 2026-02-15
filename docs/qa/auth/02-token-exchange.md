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
WHERE u.keycloak_id = '{keycloak_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 存在记录

-- Token 解码后应包含 roles 和 permissions
```

---

## 场景 2：Token Exchange - 用户不是租户成员

### 初始状态
- 用户已登录
- 用户不是请求的租户成员

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
     token: "<Tenant Access Token>"
   }
   ```

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
1. 使用过期 Token 调用 ValidateToken

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

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Token Exchange - 成功 | ✅ PASS | 2026-02-15 | Gemini | |
| 2 | Token Exchange - 非成员 | ✅ PASS | 2026-02-15 | Gemini | |
| 3 | Token 验证 | ✅ PASS | 2026-02-15 | Gemini | |
| 4 | Token 过期验证 | ✅ PASS | 2026-02-15 | Gemini | |
| 5 | Token 内省 | ✅ PASS | 2026-02-15 | Gemini | |
