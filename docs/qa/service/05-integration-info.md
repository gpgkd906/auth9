# 服务集成信息 - Integration Info API & Portal 页面

**模块**: 服务与客户端
**测试范围**: Service Integration Info API 端点 + Portal Integration 标签页
**场景数**: 5
**优先级**: 高

---

## 背景说明

开发者在 auth9-portal 创建 Service 后，需要知道如何配置 SDK 接入 Auth9。Integration 功能提供：

1. **API 端点**：`GET /api/v1/services/{id}/integration` — 返回完整集成信息 JSON
2. **Portal 页面**：Service 详情页新增 "Integration" 标签页，展示客户端凭据、环境变量、OAuth 端点、SDK 代码示例

响应结构：
```json
{
  "data": {
    "service": { "id", "name", "base_url", "redirect_uris", "logout_uris" },
    "clients": [{ "client_id", "name", "public_client", "client_secret", "created_at" }],
    "endpoints": { "auth9_domain", "auth9_public_url", "authorize", "token", "callback", "logout", "userinfo", "openid_configuration", "jwks" },
    "grpc": { "address", "auth_mode" },
    "environment_variables": [{ "key", "value", "description" }]
  }
}
```

---

## 场景 1：API 返回 Confidential Client 的完整集成信息

### 初始状态
- 已创建 Service，包含至少一个 Confidential Client（非 Public Client）
- 已获取有效的 Admin Access Token

### 目的
验证 Integration API 返回完整的集成信息，包含 Keycloak 中的真实 client_secret

### 测试操作流程
1. 获取 Admin Access Token
2. 调用 Integration API：
   ```bash
   curl -s http://localhost:8080/api/v1/services/{service_id}/integration \
     -H "Authorization: Bearer {access_token}" | jq .
   ```
3. 检查响应 JSON 结构

### 预期结果
- 状态码 `200`
- `data.service` 包含正确的 `id`、`name`、`base_url`
- `data.clients[0].client_id` 非空
- `data.clients[0].public_client` 为 `false`
- `data.clients[0].client_secret` 非空（真实 secret，非哈希）
- `data.endpoints.authorize` 包含 `/realms/{tenant}/protocol/openid-connect/auth`
- `data.endpoints.token` 包含 `/realms/{tenant}/protocol/openid-connect/token`
- `data.endpoints.openid_configuration` 以 `/.well-known/openid-configuration` 结尾
- `data.grpc.address` 格式为 `host:port`
- `data.environment_variables` 包含 `AUTH9_DOMAIN`、`AUTH9_CLIENT_ID`、`AUTH9_CLIENT_SECRET` 等

### 预期数据状态
```sql
-- 验证 Service 存在
SELECT id, name, base_url FROM services WHERE id = '{service_id}';
-- 预期: 1 行

-- 验证 Client 存在
SELECT client_id, name FROM clients WHERE service_id = '{service_id}';
-- 预期: 至少 1 行
```

---

## 场景 2：API 处理 Public Client（无 Secret）

### 初始状态
- 已创建 Service，包含一个 Public Client
- 已获取有效的 Admin Access Token

### 目的
验证 Public Client 不返回 client_secret，环境变量中也不包含 SECRET 行

### 测试操作流程
1. 创建或使用已有的 Public Client Service
2. 调用 Integration API：
   ```bash
   curl -s http://localhost:8080/api/v1/services/{service_id}/integration \
     -H "Authorization: Bearer {access_token}" | jq .
   ```
3. 检查 `clients` 数组中 Public Client 的字段
4. 检查 `environment_variables` 是否省略了 `AUTH9_CLIENT_SECRET`

### 预期结果
- `data.clients[].public_client` 为 `true` 的客户端，`client_secret` 为 `null`
- `data.environment_variables` 中不包含 `AUTH9_CLIENT_SECRET` 条目
- `data.environment_variables` 中 `AUTH9_CLIENT_ID` 正确对应该 Public Client
- 其他端点信息（endpoints、grpc）正常返回

---

## 场景 3：API 鉴权 — 未授权请求被拒绝

### 初始状态
- 已创建 Service
- 无有效 Token 或使用无权限 Token

### 目的
验证 Integration API 需要有效的 Admin 认证，未认证请求返回 401

### 测试操作流程
1. 不带 Authorization Header 调用 API：
   ```bash
   curl -s -o /dev/null -w "%{http_code}" \
     http://localhost:8080/api/v1/services/{service_id}/integration
   ```
2. 带无效 Token 调用 API：
   ```bash
   curl -s -o /dev/null -w "%{http_code}" \
     http://localhost:8080/api/v1/services/{service_id}/integration \
     -H "Authorization: Bearer invalid-token-xxx"
   ```

### 预期结果
- 不带 Token：状态码 `401`
- 无效 Token：状态码 `401`
- 响应体包含错误信息，不泄露任何服务配置

---

## 场景 4：Portal Integration 标签页展示与交互

### 初始状态
- 已登录 auth9-portal（http://localhost:3000）
- 已创建至少一个 Service，包含 Confidential Client

### 目的
验证 Portal Service 详情页的 Integration 标签页正确展示所有集成信息

### 测试操作流程
1. 登录 Portal，进入 Dashboard → Services
2. 点击任意 Service 进入详情页
3. 观察页面顶部是否有「Configuration」和「Integration」两个标签
4. 点击「Integration」标签
5. 检查以下区域：
   - **Clients & Credentials** 卡片：显示 client_id 和隐藏的 secret（`••••••••`）
   - 点击「Reveal」按钮，检查是否显示真实 secret
   - 点击 client_id 旁的复制按钮，检查是否复制到剪贴板
6. 检查 **Environment Variables** 区域：
   - `.env` 格式代码块包含 `AUTH9_DOMAIN`、`AUTH9_CLIENT_ID`、`AUTH9_CLIENT_SECRET` 等
   - 点击「Copy All」按钮复制全部
7. 检查 **OAuth/OIDC Endpoints** 区域：
   - 表格列出 Authorize、Token、Callback、Logout、UserInfo、OIDC Discovery、JWKS URL
   - 每行有复制按钮
8. 检查 **SDK Initialization** 区域：
   - 包含 TypeScript 代码示例（Auth9 Client、Express Middleware、gRPC Token Exchange）
   - 代码块有深色背景和复制按钮
9. 切换回「Configuration」标签，确认原有配置表单和客户端卡片仍然正常

### 预期结果
- 两个标签切换顺畅，无页面闪烁
- Secret 默认隐藏，点击 Reveal 后显示真实值
- 所有复制按钮正常工作（复制到系统剪贴板）
- 环境变量块中的值与 API 返回一致
- OAuth 端点 URL 完整且格式正确
- SDK 代码示例中的 `domain`、`audience` 使用真实值
- Configuration 标签内容不受影响

---

## 场景 5：Service 无 Client 时 Integration 页面的降级处理

### 初始状态
- 已创建 Service，但未创建任何 Client
- 已登录 Portal

### 目的
验证没有 Client 时 Integration 页面和 API 的降级展示

### 测试操作流程
1. 调用 API 验证：
   ```bash
   curl -s http://localhost:8080/api/v1/services/{service_id}/integration \
     -H "Authorization: Bearer {access_token}" | jq .
   ```
2. 在 Portal 进入该 Service 的 Integration 标签页

### 预期结果
- API 返回 `200`，`data.clients` 为空数组 `[]`
- `data.environment_variables` 中 `AUTH9_CLIENT_ID` 和 `AUTH9_CLIENT_SECRET` 使用占位符或默认值
- Portal Integration 页面不崩溃，Clients & Credentials 区域显示空状态或提示信息
- 端点信息和 gRPC 信息正常显示（不依赖 Client）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Confidential Client 完整集成信息 | ☐ | | | |
| 2 | Public Client 无 Secret | ☐ | | | |
| 3 | 未授权请求被拒绝 | ☐ | | | |
| 4 | Portal Integration 标签页展示与交互 | ☐ | | | |
| 5 | 无 Client 降级处理 | ☐ | | | |
