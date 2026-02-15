# 认证流程 - Client Credentials Grant 测试

**模块**: 认证流程
**测试范围**: 服务间认证（client_credentials 授权类型）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Client Credentials Grant 用于服务间（M2M）认证。服务通过 `client_id` 和 `client_secret` 直接获取 Access Token，无需用户参与。

Token 端点：`POST /api/v1/auth/token`

```json
{
  "grant_type": "client_credentials",
  "client_id": "my-service-client",
  "client_secret": "generated-secret"
}
```

响应格式：
```json
{
  "access_token": "<JWT>",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": null,
  "id_token": null
}
```

---

## 场景 1：使用有效凭证获取 Service Token

### 初始状态
- 已创建服务 `My API Service`，拥有有效的 client_id 和 client_secret

### 目的
验证 Client Credentials Grant 基本流程

### 测试操作流程
1. 使用服务的 client_id 和 client_secret 调用 Token 端点：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{
       "grant_type": "client_credentials",
       "client_id": "{client_id}",
       "client_secret": "{client_secret}"
     }'
   ```
2. 检查响应状态码和返回内容
3. 解码 JWT Token 检查 Claims

### 预期结果
- 状态码 200
- 返回 `access_token`（JWT 格式）
- `token_type` = `"Bearer"`
- `refresh_token` = `null`（client_credentials 不发放 refresh token）
- `id_token` = `null`
- JWT Claims 包含 `sub`（service_id）

### 预期数据状态
```sql
-- 验证服务存在
SELECT id, name FROM services WHERE id = '{service_id}';
-- 预期: 存在记录

-- 验证客户端存在
SELECT client_id FROM clients WHERE service_id = '{service_id}';
-- 预期: 存在记录
```

---

## 场景 2：使用错误的 Client Secret

### 初始状态
- 已创建服务，拥有有效的 client_id

### 目的
验证错误密钥被正确拒绝

### 测试操作流程
1. 使用正确的 client_id 但错误的 client_secret 调用：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{
       "grant_type": "client_credentials",
       "client_id": "{client_id}",
       "client_secret": "wrong-secret-12345"
     }'
   ```

### 预期结果
- 状态码 401 Unauthorized
- 错误信息：「Invalid client credentials」
- 不返回任何 Token

---

## 场景 3：使用不存在的 Client ID

### 初始状态
- 系统中不存在 client_id `non-existent-client`

### 目的
验证不存在的客户端被正确拒绝

### 测试操作流程
1. 使用不存在的 client_id 调用：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{
       "grant_type": "client_credentials",
       "client_id": "non-existent-client",
       "client_secret": "any-secret"
     }'
   ```

### 预期结果
- 状态码 404 Not Found 或 401 Unauthorized
- 错误信息表明客户端不存在
- 不返回任何 Token

---

## 场景 4：缺少必要参数

### 初始状态
- 无

### 目的
验证缺少 client_id 或 client_secret 时返回明确错误

### 测试操作流程
1. 只传 grant_type，不传 client_id 和 client_secret：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{"grant_type": "client_credentials"}'
   ```
2. 传 client_id 但不传 client_secret：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{
       "grant_type": "client_credentials",
       "client_id": "{client_id}"
     }'
   ```

### 预期结果
- 步骤 1：状态码 400，错误信息「Missing client_id」
- 步骤 2：状态码 400，错误信息「Missing client_secret」

---

## 场景 5：使用 Service Token 调用受保护 API

### 初始状态
- 已通过 client_credentials 获取有效的 Service Token

### 目的
验证 Service Token 可用于 API 调用和 Token 验证

### 测试操作流程
1. 使用 client_credentials 获取 Service Token
2. 使用该 Token 调用 gRPC ValidateToken：
   ```protobuf
   ValidateTokenRequest {
     token: "<Service Token>"
   }
   ```
3. 检查 Token 有效性和 Claims

### 预期结果
- Service Token 通过验证
- Token Claims 包含：
  - `sub`: service_id
  - `token_type`: `"service"`
  - `aud`: `"auth9-service"`
  - `tenant_id`: 服务关联的 tenant_id（如果有）
- Token 有过期时间

---

## 通用场景：不支持的 Grant Type

### 测试操作流程
1. 使用不支持的 grant_type：
   ```bash
   curl -X POST http://localhost:8080/api/v1/auth/token \
     -H "Content-Type: application/json" \
     -d '{"grant_type": "password", "username": "test", "password": "test"}'
   ```

### 预期结果
- 状态码 400
- 错误信息：「Unsupported grant type: password」

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 有效凭证获取 Token | ☐ | | | |
| 2 | 错误 Client Secret | ☐ | | | |
| 3 | 不存在的 Client ID | ☐ | | | |
| 4 | 缺少必要参数 | ☐ | | | |
| 5 | Service Token 调用 API | ☐ | | | |
| 6 | 不支持的 Grant Type | ☐ | | | |
