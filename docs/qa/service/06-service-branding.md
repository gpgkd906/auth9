# 服务管理 - Service 级别品牌设置测试

**模块**: 服务管理
**测试范围**: Service 级别 Branding 覆盖（API + Portal）、公开端点 client_id 查询、Keycloak 主题集成
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 支持两级品牌配置：
1. **系统级品牌** — 在「设置 → 登录页品牌」页面配置，作为全局默认
2. **Service 级品牌** — 在 Service 详情页「Branding」标签页配置，覆盖系统默认

当用户通过某个 Service 的 OIDC Client 访问登录页时，auth9-keycloak-theme 从公开端点获取品牌配置：
- `GET /api/v1/public/branding?client_id={client_id}` — 若该 client 所属 Service 有自定义品牌则返回 Service 品牌，否则返回系统默认品牌

端点：
- `GET /api/v1/services/{service_id}/branding` — 获取 Service 品牌配置
- `PUT /api/v1/services/{service_id}/branding` — 更新 Service 品牌配置
- `DELETE /api/v1/services/{service_id}/branding` — 删除 Service 品牌（恢复系统默认）
- `GET /api/v1/public/branding?client_id={client_id}` — 公开端点，支持 client_id 参数

---

## 数据库表结构参考

### service_brandings 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属 Service ID |
| config | JSON | BrandingConfig JSON |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 场景 1：Service 详情页 Branding Tab 入口可见性

### 初始状态
- 管理员已登录
- 存在 Service id=`{service_id}`

### 目的
验证用户可以从 Service 详情页的「Branding」标签页进入品牌配置界面

### 测试操作流程（Portal UI）
1. 进入 Service 详情页：`/dashboard/services/{service_id}`
2. 验证 Tab 栏中存在「Branding」标签页（位于 Actions 标签页之后）
3. 点击「Branding」标签页
4. 验证页面显示品牌配置表单

### 预期结果
- Tab 栏显示「Branding」入口
- 点击后显示品牌编辑区域
- 若 Service 无自定义品牌，显示 "Using system default branding" 提示和「Customize」按钮
- 若 Service 已有自定义品牌，显示编辑表单和「Reset to Default」按钮

### 预期数据状态
```sql
SELECT id, service_id, config FROM service_brandings
WHERE service_id = '{service_id}';
-- 可能为空（使用系统默认）或存在自定义配置
```

---

## 场景 2：创建 Service 级品牌配置

### 初始状态
- 管理员已登录
- 存在 Service id=`{service_id}`，该 Service 无自定义品牌

### 目的
验证通过 Portal 和 API 创建 Service 级品牌配置

### 测试操作流程（Portal UI）
1. 进入 Service 详情页 → 点击「Branding」标签页
2. 确认显示 "Using system default branding"
3. 点击「Customize」按钮
4. 修改品牌配置：
   - Primary Color：`#E74C3C`
   - Company Name：`Service Custom Brand`
   - Logo URL：`https://cdn.example.com/service-logo.png`（必须使用允许域名）
5. 点击「Save Branding」

### 测试操作流程（API）

> **注意 1**：`BrandingConfig` 中 `primary_color`、`secondary_color`、`background_color`、`text_color`
> 四个颜色字段均为**必填项**（非 Optional），必须提供完整的 `#RRGGBB` 格式颜色值。
> 缺少任一字段将导致 400 反序列化错误。
>
> **注意 2**：Logo URL 和 Favicon URL 的域名必须在 `BRANDING_ALLOWED_DOMAINS` 白名单中
> （默认 Docker 环境：`cdn.example.com`, `assets.example.com`）。
> 使用其他域名（如 `example.com`）会返回 422 验证错误。
>
> **注意 3**：此 API 需要 **Tenant Access Token**（不能使用 Identity Token）。
> Identity Token 在 `/api/v1/services/*` 路径上会被 `require_auth` 中间件以 403 拒绝。
> 使用 `gen-test-tokens.js tenant-owner` 生成正确的 Token。

```bash
TOKEN=$(node .claude/skills/tools/gen-test-tokens.js tenant-owner)
curl -X PUT http://localhost:8080/api/v1/services/{service_id}/branding \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "primary_color": "#E74C3C",
      "secondary_color": "#5856D6",
      "background_color": "#F5F5F7",
      "text_color": "#1D1D1F",
      "company_name": "Service Custom Brand",
      "logo_url": "https://cdn.example.com/service-logo.png"
    }
  }'
```

### 预期结果（Portal UI）
- 显示「Branding settings saved successfully」提示
- 表单显示已保存的自定义配置
- 出现「Reset to Default」按钮

### 预期结果（API）
- HTTP 200 OK
- 返回包含 `service_id` 和 `config` 的 ServiceBranding 对象

### 预期数据状态
```sql
SELECT service_id,
       JSON_EXTRACT(config, '$.primary_color') as primary_color,
       JSON_EXTRACT(config, '$.company_name') as company_name
FROM service_brandings
WHERE service_id = '{service_id}';
-- 预期: primary_color = "#E74C3C", company_name = "Service Custom Brand"
```

---

## 场景 3：公开端点按 client_id 返回 Service 品牌

### 初始状态
- 系统品牌已配置（primary_color = `#007AFF`）
- Service id=`{service_id}` 已配置自定义品牌（primary_color = `#E74C3C`）
- 该 Service 下存在 Client，client_id = `{client_id}`

### 目的
验证公开端点根据 `client_id` 参数返回对应 Service 的品牌配置，无 `client_id` 时返回系统默认

### 测试操作流程（API）
```bash
# 无 client_id — 返回系统默认品牌
curl http://localhost:8080/api/v1/public/branding

# 有 client_id — 返回 Service 品牌
curl "http://localhost:8080/api/v1/public/branding?client_id={client_id}"

# 无效 client_id — 返回系统默认品牌（降级）
curl "http://localhost:8080/api/v1/public/branding?client_id=nonexistent"
```

### 预期结果
- 无 `client_id`：返回系统默认品牌（`primary_color = "#007AFF"`）
- 有效 `client_id`：返回该 client 所属 Service 的品牌（`primary_color = "#E74C3C"`）
- 无效 `client_id`：返回系统默认品牌（降级，不报错）

### 预期数据状态
```sql
-- 确认 client_id 关联到 Service
SELECT c.client_id, s.id as service_id, s.name
FROM clients c JOIN services s ON c.service_id = s.id
WHERE c.client_id = '{client_id}';
-- 预期: service_id = '{service_id}'
```

---

## 场景 4：删除 Service 品牌（恢复系统默认）

### 初始状态
- Service id=`{service_id}` 已配置自定义品牌

### 目的
验证删除 Service 品牌后恢复系统默认

### 测试操作流程（Portal UI）
1. 进入 Service 详情页 → 「Branding」标签页
2. 确认当前显示自定义品牌配置
3. 点击「Reset to Default」按钮
4. 确认重置

### 测试操作流程（API）
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -X DELETE http://localhost:8080/api/v1/services/{service_id}/branding \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果（Portal UI）
- 显示「Branding reset to default」提示
- 恢复显示 "Using system default branding" 状态
- 重新出现「Customize」按钮

### 预期结果（API）
- HTTP 200 OK（或 204 No Content）

### 预期数据状态
```sql
SELECT COUNT(*) FROM service_brandings
WHERE service_id = '{service_id}';
-- 预期: COUNT = 0

-- 公开端点验证
-- curl "http://localhost:8080/api/v1/public/branding?client_id={client_id}"
-- 预期: 返回系统默认品牌
```

---

## 场景 5：Keycloak 主题按 client_id 加载 Service 品牌

### 初始状态
- Service id=`{service_id}` 已配置自定义品牌（primary_color = `#E74C3C`）
- 该 Service 的 Client 已配置 OIDC 回调

### 目的
验证 Keycloak 登录页根据 OIDC 请求中的 `client_id` 加载对应 Service 的品牌样式

### 测试操作流程
1. 通过该 Service 的 OIDC 入口触发登录流程（Portal 或直接 OIDC authorize URL）
2. 观察 Keycloak 登录页外观

### 预期结果
- 登录页使用 Service 自定义的 `primary_color`（`#E74C3C`，非系统默认 `#007AFF`）
- 显示 Service 自定义的 `company_name` 和 `logo_url`（如已配置）
- 若 Service 无自定义品牌，则显示系统默认品牌

### 验证方法
```bash
# 检查 Keycloak 主题是否请求了带 client_id 的品牌端点
# 在浏览器 DevTools Network 面板中观察：
# GET http://localhost:8080/api/v1/public/branding?client_id={client_id}
# 预期: 返回 Service 级别品牌配置
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Service 详情页 Branding Tab 入口可见性 | ☐ | | | |
| 2 | 创建 Service 级品牌配置 | ☐ | | | |
| 3 | 公开端点按 client_id 返回 Service 品牌 | ☐ | | | |
| 4 | 删除 Service 品牌（恢复系统默认） | ☐ | | | |
| 5 | Keycloak 主题按 client_id 加载 Service 品牌 | ☐ | | | |

---

## 常见问题排查

| 症状 | 原因 | 修复方法 |
|------|------|----------|
| API 返回 403 "Identity token is only allowed for tenant selection and exchange" | 使用了 Identity Token（`gen-admin-token.sh`）访问 `/api/v1/services/*` 路径 | 使用 Tenant Access Token：`node .claude/skills/tools/gen-test-tokens.js tenant-owner` |
| API 返回 422 "domain 'xxx' is not in the allowed domains list" | Logo/Favicon URL 域名不在 `BRANDING_ALLOWED_DOMAINS` 白名单中 | 使用允许域名（默认：`cdn.example.com`, `assets.example.com`），或留空 Logo URL |
| API 返回 400 反序列化错误 | 缺少必填颜色字段（`primary_color`, `secondary_color`, `background_color`, `text_color`） | 确保 JSON 包含全部 4 个颜色字段，格式为 `#RRGGBB` |
| Portal UI 保存返回 400 | 前端 action handler 将后端错误统一包装为 400 | 检查浏览器 DevTools Network 中实际 API 响应的 HTTP 状态码和错误消息 |
| API 返回 403 "Platform admin required" | 当前用户不是平台管理员 | 确保使用 `admin@auth9.local` 用户登录，或用户在 `auth9-platform` 租户中拥有 admin 角色 |
