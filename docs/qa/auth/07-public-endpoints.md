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
2. 管理员更新品牌颜色：
   ```bash
   PUT /api/v1/system/branding
   { "primary_color": "#ff6600" }
   ```
3. 再次调用公开 branding：
   ```bash
   curl http://localhost:8080/api/v1/public/branding
   ```

### 预期结果
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
验证 GetUserRoles 正确返回用户角色，并验证缓存行为

### 测试操作流程
1. 首次调用 gRPC GetUserRoles：
   ```protobuf
   GetUserRolesRequest {
     user_id: "{user_id}"
     tenant_id: "{tenant_id}"
     service_id: "{service_id}"
   }
   ```
2. 记录返回的角色和权限
3. 通过 API 给用户新增一个角色 `admin`
4. 立即再次调用 GetUserRoles
5. 等待缓存失效后再次调用

### 预期结果
- 步骤 1：返回角色 `editor` 及其关联权限
- 步骤 4：可能仍返回旧角色（缓存未失效）
- 步骤 5：返回更新后的角色列表（包含 `editor` 和 `admin`）

### 预期数据状态
```sql
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: editor, admin
```

```bash
# 检查 Redis 缓存
redis-cli KEYS "auth9:user_roles:*"
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

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
