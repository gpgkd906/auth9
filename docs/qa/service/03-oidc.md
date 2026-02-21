# 服务管理 - OIDC 配置测试

**模块**: 服务与客户端管理
**测试范围**: OIDC 配置、URI 验证
**场景数**: 5

---

## 场景 1：Redirect URI 验证

### 初始状态
- 服务配置了 redirect_uris: `["https://app.example.com/callback"]`

### 目的
验证 OIDC 重定向 URI 验证

### 测试操作流程
1. 使用已配置的 redirect_uri 进行授权
2. 使用未配置的 redirect_uri 进行授权

### 预期结果
- 已配置 URI：授权流程正常
- 未配置 URI：返回 `invalid_redirect_uri`

### 预期数据状态
```sql
SELECT redirect_uris FROM services WHERE id = '{service_id}';
```

---

## 场景 2：多 Redirect URI 配置

### 初始状态
- 服务需要支持多个回调地址

### 目的
验证多个 Redirect URI 的使用

### 测试操作流程
1. 编辑服务，添加多个 Redirect URIs：
   - `https://app.example.com/callback`
   - `https://app.example.com/auth/callback`
   - `https://localhost:3000/callback`
2. 保存
3. 分别使用每个 URI 测试

### 预期结果
- 所有配置的 URI 都可正常使用

### 预期数据状态
```sql
SELECT redirect_uris FROM services WHERE id = '{service_id}';
-- 预期: 包含 3 个 URI
```

---

## 场景 3：Logout URI 配置与验证

### 初始状态
- 服务配置了 logout_uris: `["https://test.example.com/logout"]`
- 客户端: `existing-client`

### 目的
验证登出后的重定向到配置的URI，未配置的URI应被拒绝

### 测试操作流程
1. 调用登出端点带有效URI和client_id:
   ```
   GET /api/v1/auth/logout?client_id=existing-client&post_logout_redirect_uri=https://test.example.com/logout
   ```
2. 调用登出端点带无效URI:
   ```
   GET /api/v1/auth/logout?client_id=existing-client&post_logout_redirect_uri=https://evil.com/logout
   ```
3. 调用登出端点带redirect_uri但不带client_id:
   ```
   GET /api/v1/auth/logout?post_logout_redirect_uri=https://test.example.com/logout
   ```

### 预期结果
- 步骤1: 307重定向到Keycloak登出页面
- 步骤2: 400错误，拒绝无效URI
- 步骤3: 400错误，要求提供client_id

---

## 场景 4：Client ID 自动生成验证

### 初始状态
- 已存在至少一个服务

### 目的
验证 Client ID 由系统自动生成（UUID），用户无法指定 client_id

### 测试操作流程
1. 调用创建客户端 API，仅提供 `name` 字段
2. 重复创建多个客户端

### 预期结果
- 每次创建成功，返回系统自动生成的 UUID 格式 client_id
- 每个 client_id 全局唯一
- API 忽略请求体中的 client_id 字段（如有）

### 预期数据状态
```sql
SELECT id, client_id, name FROM clients WHERE service_id = '{service_id}';
-- client_id 列均为 UUID 格式，且互不重复
```

---

## 场景 5：Redirect URI 格式验证

### 初始状态
- 用户尝试配置 Redirect URI

### 目的
验证 URI 格式验证

### 测试操作流程
测试以下 URI：
1. 有效 HTTPS：`https://app.example.com/callback` ✓
2. 有效 localhost HTTP：`http://localhost:3000/callback` ✓
3. 无效 HTTP（非本地）：`http://app.example.com/callback` ✗
4. 无协议：`app.example.com/callback` ✗

### 预期结果
- 非法格式被拒绝

---

## 测试数据准备 SQL

```sql
-- 准备测试服务
INSERT INTO services (id, name, base_url, redirect_uris, logout_uris, status) VALUES
('svc-test-1111-1111-111111111111', 'Test Service', 'https://test.example.com',
 '["https://test.example.com/callback"]', '["https://test.example.com/logout"]', 'active');

-- 准备测试客户端
INSERT INTO clients (id, service_id, client_id, client_secret_hash, name) VALUES
('client-1111-1111-1111-111111111111', 'svc-test-1111-1111-111111111111', 'existing-client',
 '$argon2id$...', 'Test Client');

-- 清理
DELETE FROM clients WHERE id LIKE 'client-%';
DELETE FROM services WHERE id LIKE 'svc-test-%';
```

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
| 1 | Redirect URI 验证 | ☐ | | | |
| 2 | 多 Redirect URI | ☐ | | | |
| 3 | Logout URI 配置 | ☐ | | | |
| 4 | Client ID 唯一性 | ☐ | | | |
| 5 | URI 格式验证 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
