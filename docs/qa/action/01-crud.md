# Action 管理 - CRUD 操作测试

**模块**: Action 管理
**测试范围**: Action 导航入口、创建、列表、查看、更新、删除
**场景数**: 4

---

## 数据库表结构参考

### actions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属 Service ID |
| name | VARCHAR(255) | Action 名称 |
| description | TEXT | 描述 |
| trigger_id | VARCHAR(50) | 触发器类型 |
| script | TEXT | TypeScript 脚本 |
| enabled | BOOLEAN | 是否启用 |
| execution_order | INT | 执行顺序 |
| timeout_ms | INT | 超时时间（毫秒） |
| last_executed_at | TIMESTAMP | 最后执行时间 |
| execution_count | BIGINT | 执行次数 |
| error_count | BIGINT | 错误次数 |
| last_error | TEXT | 最后错误信息 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### action_executions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| action_id | CHAR(36) | Action ID |
| service_id | CHAR(36) | Service ID |
| trigger_id | VARCHAR(50) | 触发器类型 |
| user_id | CHAR(36) | 用户 ID（可选） |
| success | BOOLEAN | 是否成功 |
| duration_ms | INT | 执行时长（毫秒） |
| error_message | TEXT | 错误信息 |
| executed_at | TIMESTAMP | 执行时间 |

---

## 场景 1：Service 详情页 Actions Tab 入口可见性

### 初始状态
- 管理员已登录
- 存在 Service id=`{service_id}`，且该 Service 下已创建至少 1 个 Action

### 目的
验证用户可以从 Service 详情页的 Actions Tab 发现并访问 Actions 管理页面，而无需手动输入 URL

### 测试操作流程（Portal UI）
1. 进入 Service 详情页：`/dashboard/services/{service_id}`
2. 验证页面顶部 Tab 栏中存在「Actions」标签页
3. 点击「Actions」标签页
4. 验证 Actions 列表正确加载，显示该 Service 下的 Actions
5. 验证列表中每个 Action 显示名称、触发器类型、状态等信息

### 预期结果
- Service 详情页显示「Actions」Tab 入口
- 点击后成功加载 `/dashboard/services/{service_id}/actions` 页面
- Actions 列表页正确加载，显示该 Service 下的 Actions

---

## 场景 2：创建 Action（基础）

### 初始状态
- 管理员已登录
- 存在 Service id=`{service_id}`
- 已获取 API Token

### 目的
验证 Action 创建功能（API 和 Portal UI）

### 测试操作流程（Portal UI）
1. 进入 Service 详情页：`/dashboard/services/{service_id}`
2. 点击「Actions」Tab 进入 Actions 管理页面
3. 点击「New Action」按钮
4. 填写基本信息：
   - 名称：`Test Post Login Action`
   - 描述：`Add custom claims for testing`
   - 触发器：选择 `Post Login`
   - 执行顺序：`0`
   - 超时：`3000`
5. 填写脚本：
   ```typescript
   // Add custom claims
   context.claims = context.claims || {};
   context.claims.test_claim = "test_value";
   context;
   ```
6. 保持「Enabled」开关开启
7. 点击「Create Action」

### 测试操作流程（API）
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -X POST http://localhost:8080/api/v1/services/{service_id}/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Post Login Action",
    "description": "Add custom claims for testing",
    "trigger_id": "post-login",
    "script": "context.claims = context.claims || {}; context.claims.test_claim = \"test_value\"; context;",
    "enabled": true,
    "execution_order": 0,
    "timeout_ms": 3000
  }'
```

### 预期结果（Portal UI）
- 重定向到 Action 详情页
- 显示创建成功的 Action 信息
- Action 出现在列表中
- 状态显示为「Enabled」

### 预期结果（API）
- HTTP 200 OK
- 返回创建的 Action 对象（包含 id）

### 预期数据状态
```sql
SELECT id, name, trigger_id, enabled, script, execution_order, timeout_ms
FROM actions
WHERE name = 'Test Post Login Action' AND service_id = '{service_id}';
-- 预期:
-- - 存在记录
-- - enabled = true
-- - trigger_id = 'post-login'
-- - execution_order = 0
-- - timeout_ms = 3000
-- - script 包含 "test_claim"
```

---

## 场景 3：创建 Action（使用模板）

### 初始状态
- 管理员已登录
- 存在 Service id=`{service_id}`

### 目的
验证使用内置模板快速创建 Action

### 测试操作流程（Portal UI）
1. 从 Service 详情页点击「Actions」Tab 进入 Actions 管理页面，点击「New Action」
2. 在「Script Templates」下拉菜单中选择「Add Custom Claims」模板
3. 验证脚本自动填充
4. 填写基本信息：
   - 名称：`Department Claims Action`
   - 触发器：`Post Login`
5. 点击「Create Action」

### 预期结果
- Action 创建成功
- 脚本内容与模板匹配
- 包含 `context.claims.department` 等字段

### 预期数据状态
```sql
SELECT name, script FROM actions
WHERE name = 'Department Claims Action' AND service_id = '{service_id}';
-- 预期: script 包含 "department" 和 "tier"
```

---

## 场景 4：列表查看与筛选

### 初始状态
- 存在多个 Actions，包含不同触发器类型

### 目的
验证 Action 列表、搜索和筛选功能

### 测试操作流程（Portal UI）
1. 从 Service 详情页点击「Actions」Tab 进入 Actions 管理页面
2. 验证列表显示所有 Actions
3. 点击「Post Login」触发器筛选按钮
4. 验证仅显示 post-login 类型的 Actions
5. 在搜索框输入「Department」
6. 验证仅显示名称或描述包含「Department」的 Actions

### 测试操作流程（API）
```bash
# 列出所有 Actions
curl http://localhost:8080/api/v1/services/{service_id}/actions \
  -H "Authorization: Bearer $TOKEN"

# 按触发器筛选
curl http://localhost:8080/api/v1/services/{service_id}/actions?trigger_id=post-login \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果（Portal UI）
- 筛选器正确过滤结果
- 搜索功能正常工作
- 显示每个 Action 的关键信息：名称、触发器、状态、执行次数、成功率

### 预期结果（API）
- 返回 Actions 数组
- 筛选参数生效

---


---

## 说明

场景 5-8（详情/更新/启停/删除）已拆分到 `docs/qa/action/07-crud-advanced.md`。

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Service 详情页 Actions Tab 入口可见性 | ☐ | | | |
| 2 | 创建 Action（基础） | ☐ | | | |
| 3 | 创建 Action（使用模板） | ☐ | | | |
| 4 | 列表查看与筛选 | ☐ | | | |
