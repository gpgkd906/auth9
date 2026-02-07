# 服务管理 - 租户服务关联测试

**模块**: 服务与客户端管理
**测试范围**: 租户-服务启用/禁用、已启用服务查询
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 支持将全局服务（`tenant_id IS NULL` 的服务）关联到具体租户，通过 `tenant_services` 表管理启用/禁用状态。

### 相关 API
- `GET /api/v1/tenants/{tenant_id}/services` — 列出所有全局服务及其在该租户的启用状态
- `POST /api/v1/tenants/{tenant_id}/services` — 启用/禁用服务
- `GET /api/v1/tenants/{tenant_id}/services/enabled` — 仅获取已启用的服务

### 请求格式
```json
POST /api/v1/tenants/{tenant_id}/services
{
  "service_id": "550e8400-...",
  "enabled": true
}
```

---

## 场景 1：查看租户可用服务列表

### 初始状态
- 存在租户 id=`{tenant_id}`
- 系统中存在 3 个全局服务：`Auth API`、`Admin API`、`Public API`
- 该租户尚未启用任何服务

### 目的
验证服务列表正确展示所有全局服务及其启用状态

### 测试操作流程
1. 调用 API：
   ```bash
   GET /api/v1/tenants/{tenant_id}/services
   ```
2. 检查返回的服务列表

### 预期结果
- 返回 3 个服务
- 每个服务包含 `id`、`name`、`base_url`、`status`、`enabled` 字段
- 所有服务的 `enabled` = `false`

### 预期数据状态
```sql
SELECT COUNT(*) FROM services WHERE tenant_id IS NULL;
-- 预期: 3

SELECT COUNT(*) FROM tenant_services WHERE tenant_id = '{tenant_id}';
-- 预期: 0
```

---

## 场景 2：为租户启用服务

### 初始状态
- 存在租户 id=`{tenant_id}`
- 存在全局服务 `Auth API`，id=`{service_id}`
- 该服务尚未为该租户启用

### 目的
验证启用服务功能

### 测试操作流程
1. 调用 API 启用服务：
   ```bash
   POST /api/v1/tenants/{tenant_id}/services
   {
     "service_id": "{service_id}",
     "enabled": true
   }
   ```
2. 调用已启用服务列表 API 验证：
   ```bash
   GET /api/v1/tenants/{tenant_id}/services/enabled
   ```

### 预期结果
- 步骤 1 返回更新后的服务列表，`Auth API` 的 `enabled` = `true`
- 步骤 2 返回列表中包含 `Auth API`

### 预期数据状态
```sql
SELECT enabled FROM tenant_services
WHERE tenant_id = '{tenant_id}' AND service_id = '{service_id}';
-- 预期: enabled = TRUE
```

---

## 场景 3：为租户禁用已启用的服务

### 初始状态
- 存在租户 id=`{tenant_id}`
- 服务 `Auth API` 已为该租户启用

### 目的
验证禁用服务功能

### 测试操作流程
1. 调用 API 禁用服务：
   ```bash
   POST /api/v1/tenants/{tenant_id}/services
   {
     "service_id": "{service_id}",
     "enabled": false
   }
   ```
2. 调用已启用服务列表 API 验证：
   ```bash
   GET /api/v1/tenants/{tenant_id}/services/enabled
   ```

### 预期结果
- 步骤 1 返回更新后的列表，`Auth API` 的 `enabled` = `false`
- 步骤 2 返回空列表（或不包含 `Auth API`）

### 预期数据状态
```sql
SELECT enabled FROM tenant_services
WHERE tenant_id = '{tenant_id}' AND service_id = '{service_id}';
-- 预期: enabled = FALSE
```

---

## 场景 4：启用不存在的服务

### 初始状态
- 存在租户 id=`{tenant_id}`
- 不存在 service_id=`99999999-9999-9999-9999-999999999999` 的全局服务

### 目的
验证启用不存在的服务时返回正确错误

### 测试操作流程
1. 调用 API 启用不存在的服务：
   ```bash
   POST /api/v1/tenants/{tenant_id}/services
   {
     "service_id": "99999999-9999-9999-9999-999999999999",
     "enabled": true
   }
   ```

### 预期结果
- 状态码 404
- 错误信息：「Global service ... not found」

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_services
WHERE tenant_id = '{tenant_id}' AND service_id = '99999999-9999-9999-9999-999999999999';
-- 预期: 0
```

---

## 场景 5：重复启用同一服务（幂等性）

### 初始状态
- 存在租户 id=`{tenant_id}`
- 服务 `Auth API` 已为该租户启用

### 目的
验证重复启用操作的幂等性

### 测试操作流程
1. 再次调用启用服务：
   ```bash
   POST /api/v1/tenants/{tenant_id}/services
   {
     "service_id": "{service_id}",
     "enabled": true
   }
   ```
2. 检查 `tenant_services` 表记录数

### 预期结果
- 请求成功，不报错
- `tenant_services` 表中仍然只有 1 条记录（ON DUPLICATE KEY UPDATE）
- `updated_at` 被更新

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_services
WHERE tenant_id = '{tenant_id}' AND service_id = '{service_id}';
-- 预期: 1（不是 2）

SELECT updated_at FROM tenant_services
WHERE tenant_id = '{tenant_id}' AND service_id = '{service_id}';
-- 预期: 接近当前时间
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
| 1 | 查看租户可用服务列表 | ☐ | | | |
| 2 | 为租户启用服务 | ☐ | | | |
| 3 | 为租户禁用服务 | ☐ | | | |
| 4 | 启用不存在的服务 | ☐ | | | |
| 5 | 重复启用（幂等性） | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
