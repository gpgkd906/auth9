# 用户管理 - 高级操作测试

**模块**: 用户管理
**测试范围**: 删除、MFA、列表、租户关联
**场景数**: 5

---

## 场景 1：删除用户（级联删除）

### 初始状态
- 存在用户 id=`{user_id}`
- 该用户有以下关联数据：
  - 2 个租户关联
  - 3 个会话记录

### 目的
验证用户删除的级联处理

### 测试操作流程
1. 找到目标用户
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 用户从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
-- 预期: 0

SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}';
-- 预期: 0

-- Keycloak 验证：用户已被删除
```

---

## 场景 2：启用用户 MFA

### 初始状态
- 存在用户 id=`{user_id}`，mfa_enabled=false
- **目标用户必须在 Keycloak 中存在对应账户**（有 keycloak_id 映射）
- 管理员已登录且知道自己的密码（用于二次确认）

### 目的
验证 MFA 启用功能

### 测试操作流程
1. 在用户列表中找到目标用户
2. 点击该行末尾的「...」操作按钮
3. 选择「Enable MFA」
4. **在弹出的确认对话框中输入管理员自己的密码**（非目标用户密码）
5. 点击「Enable MFA」确认按钮
6. **等待页面自动刷新**（React Router 重新加载 loader 数据）

### 预期结果
- MFA 状态变为已启用
- Keycloak 中 MFA 配置同步

### 常见失败原因

| 现象 | 原因 | 解决方法 |
|------|------|----------|
| 对话框显示错误信息 | 管理员密码输入错误 | 检查输入的密码是否为当前登录管理员的密码 |
| 对话框显示 "not found" | 目标用户无 Keycloak 账户 | 确认用户是通过正常注册流程创建的，非直接插入 DB |
| 对话框关闭但状态未变 | 页面重新加载延迟 | 等待 1-2 秒或手动刷新页面后检查 |
| 无任何响应 | accessToken 缺失（session 过期） | 重新登录后再试 |
| API 返回 403 "Identity token is only allowed..." | 手动 API 测试使用了 Identity Token | **必须使用 Tenant Access Token**（`node scripts/qa/gen-access-token.js` 生成） |
| API 返回 405 Method Not Allowed | 手动 API 测试使用了错误的 HTTP 方法 | **启用 MFA 使用 POST，禁用使用 DELETE**（不是 PUT） |

### 手动 API 测试参考

```bash
# 启用 MFA (POST, 需要 Tenant Access Token)
TOKEN=$(node scripts/qa/gen-access-token.js)
curl -X POST "http://localhost:8080/api/v1/users/{user_id}/mfa" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"confirm_password":"your-admin-password"}'  # pragma: allowlist secret

# 禁用 MFA (DELETE, 需要 Tenant Access Token)
curl -X DELETE "http://localhost:8080/api/v1/users/{user_id}/mfa" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"confirm_password":"your-admin-password"}'  # pragma: allowlist secret
```

### 预期数据状态
```sql
SELECT mfa_enabled FROM users WHERE id = '{user_id}';
-- 预期: true
```

---

## 场景 3：禁用用户 MFA

### 初始状态
- 存在用户 id=`{user_id}`，mfa_enabled=true

### 目的
验证 MFA 禁用功能

### 测试操作流程
1. 进入用户详情页
2. 点击「禁用 MFA」
3. 确认操作

### 预期结果
- MFA 状态变为已禁用

### 预期数据状态
```sql
SELECT mfa_enabled FROM users WHERE id = '{user_id}';
-- 预期: false
```

---

## 场景 4：用户列表分页和搜索

### 初始状态
- 数据库中存在 50 个用户

### 目的
验证用户列表分页和搜索

### 测试操作流程
1. 打开用户管理页面
2. 验证分页（每页 20 条）
3. 搜索 `admin`

### 预期结果
- 分页正确显示
- 搜索正确过滤

### 预期数据状态
```sql
SELECT COUNT(*) FROM users;
-- 预期: 50

SELECT COUNT(*) FROM users WHERE email LIKE '%admin%' OR display_name LIKE '%admin%';
-- 用于验证搜索结果数量
```

---

## 场景 5：查看用户的租户列表

### 初始状态
- 用户 `{user_id}` 已加入 3 个租户

### 目的
验证用户租户关联正确显示

### 测试操作流程
1. 打开用户详情页
2. 查看「管理租户」

### 预期结果
- 显示 3 个租户
- 每个显示角色和加入时间

### 预期数据状态
```sql
SELECT t.name, tu.role_in_tenant, tu.joined_at
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}';
-- 预期: 3 条记录
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
| 1 | 删除用户（级联） | ☐ | | | |
| 2 | 启用 MFA | ☐ | | | |
| 3 | 禁用 MFA | ☐ | | | |
| 4 | 列表分页和搜索 | ☐ | | | |
| 5 | 查看用户租户列表 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
