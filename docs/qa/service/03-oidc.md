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

## 场景 3：Logout URI 配置

### 初始状态
- 服务配置了 logout_uris

### 目的
验证登出后的重定向

### 测试操作流程
1. 用户登录
2. 用户点击登出
3. 验证重定向

### 预期结果
- 登出后正确重定向到配置的 URI

---

## 场景 4：Client ID 唯一性验证

### 初始状态
- 已存在 client_id=`existing-client`

### 目的
验证 Client ID 全局唯一性

### 测试操作流程
1. 尝试创建 client_id=`existing-client` 的新客户端

### 预期结果
- 显示错误：「Client ID 已存在」

### 预期数据状态
```sql
SELECT COUNT(*) FROM clients WHERE client_id = 'existing-client';
-- 预期: 1
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
| 1 | Redirect URI 验证 | ☐ | | | |
| 2 | 多 Redirect URI | ☐ | | | |
| 3 | Logout URI 配置 | ☐ | | | |
| 4 | Client ID 唯一性 | ☐ | | | |
| 5 | URI 格式验证 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
