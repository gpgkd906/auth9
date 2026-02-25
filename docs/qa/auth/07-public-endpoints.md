# 认证流程 - 公开端点与 Userinfo 测试

**模块**: 认证流程
**测试范围**: 公开 Branding、Userinfo、gRPC GetUserRoles
**场景数**: 5
**优先级**: 中

---

## 场景 1：公开 Branding 端点（未登录访问）

### 初始状态
- 系统已配置品牌信息（logo、颜色、公司名称）
- 用户未登录

### 目的
验证未登录用户可以获取公开品牌信息（用于登录页面展示）

### 测试操作流程
1. 不带 Authorization 头调用公开 branding 端点：
   ```bash
   curl http://localhost:8080/api/v1/public/branding
   ```
2. 检查返回内容

### 预期结果
- 状态码 200
- 返回品牌配置信息，包含：
  - `primary_color`
  - `logo_url`
  - `allow_registration`（是否允许注册）
- 无需认证即可访问

---

## 场景 2：Branding 更新后公开端点即时反映

### 初始状态
- 当前品牌颜色为 `#1a73e8`
- 管理员已登录

### 目的
验证更新品牌设置后公开端点立即返回最新值

### 测试操作流程
1. 记录当前公开 branding：
   ```bash
   curl http://localhost:8080/api/v1/public/branding
   ```
2. 管理员更新品牌颜色（需要完整的 config 对象，所有颜色字段必填）：
   ```bash
   curl -X PUT http://localhost:8080/api/v1/system/branding \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer {admin_token}" \
     -d '{
       "config": {
         "primary_color": "#ff6600",
         "secondary_color": "#5856D6",
         "background_color": "#F5F5F7",
         "text_color": "#1D1D1F"
       }
     }'
   ```
3. 再次调用公开 branding：
   ```bash
   curl http://localhost:8080/api/v1/public/branding
   ```

### 预期结果
- 步骤 2：返回 200，branding 配置已更新
- 步骤 3 返回的 `primary_color` = `"#ff6600"`
- 变更即时生效，无缓存延迟

---

## 场景 3：Userinfo 端点

### 初始状态
- 用户已登录，持有有效的 Identity Token

### 目的
验证 userinfo 端点返回当前用户信息

### 测试操作流程
1. 使用 Identity Token 调用 userinfo：
   ```bash
   curl http://localhost:8080/api/v1/auth/userinfo \
     -H "Authorization: Bearer {identity_token}"
   ```
2. 检查返回内容

### 预期结果
- 状态码 200
- 返回包含用户信息的 JSON：
  - `sub`（用户 ID）
  - `email`
  - `name`（显示名称）

---

## 场景 4：Userinfo 端点 - 无效 Token

### 初始状态
- 不持有有效 Token

### 目的
验证无效 Token 被正确拒绝

### 测试操作流程
1. 使用无效 Token 调用 userinfo：
   ```bash
   curl http://localhost:8080/api/v1/auth/userinfo \
     -H "Authorization: Bearer invalid-token-12345"
   ```
2. 不带 Authorization 头调用：
   ```bash
   curl http://localhost:8080/api/v1/auth/userinfo
   ```

### 预期结果
- 步骤 1：状态码 401 Unauthorized
- 步骤 2：状态码 401 Unauthorized
- 不返回任何用户信息

---

## 场景 5：gRPC GetUserRoles 缓存行为

### 初始状态
- 用户属于租户 A，拥有角色 `editor`
- Redis 缓存可用

### 目的
验证 GetUserRoles 正确返回用户角色，并验证角色变更后缓存失效机制

### 重要说明：正确的 API 端点
- **角色分配**: `POST /api/v1/rbac/assign`（body 包含 `user_id`, `tenant_id`, `role_ids`）
- **角色移除**: `DELETE /api/v1/users/{user_id}/tenants/{tenant_id}/roles/{role_id}`
- **查询用户角色**: `GET /api/v1/users/{user_id}/tenants/{tenant_id}/roles`
- **⚠️ 不存在** `/api/v1/tenant-users/{id}/roles` 端点，请勿使用

### 测试操作流程
1. 首次调用 gRPC GetUserRoles：
   ```bash
   grpcurl -plaintext -d '{
     "user_id": "{user_id}",
     "tenant_id": "{tenant_id}",
     "service_id": "{service_id}"
   }' localhost:50051 auth9.TokenExchange/GetUserRoles
   ```
2. 记录返回的角色和权限
3. 通过 REST API 给用户新增角色 `admin`（需要 Tenant Access Token）：
   ```bash
   curl -X POST http://localhost:8080/api/v1/rbac/assign \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer {tenant_access_token}" \
     -d '{
       "user_id": "{user_id}",
       "tenant_id": "{tenant_id}",
       "role_ids": ["{admin_role_id}"]
     }'
   ```
4. 立即再次调用 GetUserRoles（同步骤 1）
5. 如果步骤 4 返回旧数据，等待缓存 TTL（默认 5 分钟）后再次调用
6. （可选）测试角色移除和缓存失效：
   ```bash
   curl -X DELETE http://localhost:8080/api/v1/users/{user_id}/tenants/{tenant_id}/roles/{admin_role_id} \
     -H "Authorization: Bearer {tenant_access_token}"
   ```
7. 再次调用 GetUserRoles 确认角色已移除

### 预期结果
- 步骤 1：返回角色 `editor` 及其关联权限
- 步骤 4：角色分配操作会主动失效缓存，应返回包含 `editor` 和 `admin` 的更新角色列表
- 步骤 5：如步骤 4 命中旧缓存，缓存 TTL 过期后应返回最新数据
- 步骤 7：角色移除操作会主动失效缓存（`invalidate_user_roles_for_tenant`），应仅返回 `editor`

### 预期数据状态
```sql
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 步骤 3 后预期: editor, admin
-- 步骤 6 后预期: editor
```

```bash
# 检查 Redis 缓存
redis-cli KEYS "auth9:user_roles:*"
```

### 故障排查

| 症状 | 原因 | 解决方案 |
|------|------|---------|
| 角色移除后 GetUserRoles 仍返回旧角色 | 使用了错误的 API 端点（如 `POST /tenant-users/...`） | 使用 `DELETE /api/v1/users/{user_id}/tenants/{tenant_id}/roles/{role_id}` |
| 角色分配返回 404 | 使用了错误的端点或 HTTP 方法 | 使用 `POST /api/v1/rbac/assign` 分配角色 |
| 缓存未失效 | Redis 连接问题 | 检查 auth9-core 日志中的 Redis 错误，确认 Redis 可达 |

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 公开 Branding 未登录访问 | ☐ | | | |
| 2 | Branding 更新即时反映 | ☐ | | | |
| 3 | Userinfo 正常返回 | ☐ | | | |
| 4 | Userinfo 无效 Token | ☐ | | | |
| 5 | gRPC GetUserRoles 缓存 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
