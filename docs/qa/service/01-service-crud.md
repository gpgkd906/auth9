# 服务管理 - CRUD 操作测试

**模块**: 服务与客户端管理
**测试范围**: 服务创建、更新、删除
**场景数**: 5

---

## 数据库表结构参考

### services 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 ID |
| name | VARCHAR(255) | 服务名称 |
| base_url | TEXT | 服务基础 URL |
| redirect_uris | JSON | OIDC 回调 URL 列表 |
| logout_uris | JSON | 登出回调 URL 列表 |
| status | VARCHAR(20) | 状态 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：创建服务

### 初始状态
- 管理员已登录
- 不存在同名服务

### 目的
验证服务创建功能，包括 Keycloak OIDC 客户端同步

### 测试操作流程
1. 进入「服务管理」页面
2. 点击「注册服务」
3. 填写：
   - 服务名称：`My Web App`
   - Client ID：`my-web-app`
   - Base URL：`https://myapp.example.com`
   - Redirect URIs：`https://myapp.example.com/callback`
   - Logout URIs：`https://myapp.example.com/logout`
4. 点击「创建」

### 预期结果
- 显示创建成功
- 显示初始 Client Secret（仅此一次）
- 服务出现在列表中

### 预期数据状态
```sql
SELECT id, name, base_url, redirect_uris, status FROM services WHERE name = 'My Web App';
-- 预期: 存在记录

SELECT client_id FROM clients c JOIN services s ON s.id = c.service_id WHERE s.name = 'My Web App';
-- 预期: my-web-app

-- Keycloak 验证：存在 client_id = 'my-web-app' 的客户端
```

---

## 场景 2：创建重复名称的服务

### 初始状态
- 已存在名称为 `My Web App` 的服务

### 目的
验证服务名称唯一性

### 测试操作流程
1. 尝试创建同名服务

### 预期结果
- 显示错误：「服务名称已存在」

### 预期数据状态
```sql
SELECT COUNT(*) FROM services WHERE name = 'My Web App';
-- 预期: 1
```

---

## 场景 3：更新服务配置

### 初始状态
- 存在服务 id=`{service_id}`

### 目的
验证服务配置更新功能

### 测试操作流程
1. 找到目标服务
2. 点击「编辑」
3. 修改：
   - Base URL：`https://newdomain.example.com`
   - 添加 Redirect URI：`https://newdomain.example.com/callback`
4. 保存

### 预期结果
- 显示更新成功
- Keycloak 客户端配置同步

### 预期数据状态
```sql
SELECT base_url, redirect_uris, updated_at FROM services WHERE id = '{service_id}';
-- 预期: base_url = 'https://newdomain.example.com'
```

---

## 场景 4：删除服务（级联删除）

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务有：2 个客户端、3 个权限、2 个角色、1 个 Action、1 个 Service Branding

### 目的
验证服务删除的级联处理（含 Actions 和 Service Branding）

### 测试操作流程
1. 找到目标服务
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- Keycloak 中客户端被删除

### 预期数据状态
```sql
SELECT COUNT(*) FROM services WHERE id = '{service_id}';
-- 预期: 0

SELECT COUNT(*) FROM clients WHERE service_id = '{service_id}';
-- 预期: 0

SELECT COUNT(*) FROM permissions WHERE service_id = '{service_id}';
-- 预期: 0

SELECT COUNT(*) FROM roles WHERE service_id = '{service_id}';
-- 预期: 0

SELECT COUNT(*) FROM actions WHERE service_id = '{service_id}';
-- 预期: 0

SELECT COUNT(*) FROM service_brandings WHERE service_id = '{service_id}';
-- 预期: 0
```

---

## 场景 5：查看服务详情

### 初始状态
- 存在服务 id=`{service_id}`

### 目的
验证服务详情页正确显示

### 测试操作流程
1. 点击目标服务

### 预期结果
- 显示服务基本信息
- 显示 OIDC 配置
- 显示关联的客户端列表
- Tab 栏包含「Actions」标签页（显示该 Service 的 Action 列表）
- Tab 栏包含「Branding」标签页（显示 Service 级品牌配置）

### 预期数据状态
```sql
SELECT s.*,
       (SELECT COUNT(*) FROM clients WHERE service_id = s.id) as client_count,
       (SELECT COUNT(*) FROM permissions WHERE service_id = s.id) as permission_count
FROM services s WHERE s.id = '{service_id}';
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
| 1 | 创建服务 | ☐ | | | |
| 2 | 创建重复名称服务 | ☐ | | | |
| 3 | 更新服务配置 | ☐ | | | |
| 4 | 删除服务（级联） | ☐ | | | |
| 5 | 查看服务详情 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
