# Action 管理 - CRUD 操作测试

**模块**: Action 管理
**测试范围**: Action 创建、列表、查看、更新、删除
**场景数**: 7

---

## 数据库表结构参考

### actions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 ID |
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
| tenant_id | CHAR(36) | 租户 ID |
| trigger_id | VARCHAR(50) | 触发器类型 |
| user_id | CHAR(36) | 用户 ID（可选） |
| success | BOOLEAN | 是否成功 |
| duration_ms | INT | 执行时长（毫秒） |
| error_message | TEXT | 错误信息 |
| executed_at | TIMESTAMP | 执行时间 |

---

## 场景 1：创建 Action（基础）

### 初始状态
- 管理员已登录
- 存在租户 id=`{tenant_id}`
- 已获取 API Token

### 目的
验证 Action 创建功能（API 和 Portal UI）

### 测试操作流程（Portal UI）
1. 进入租户的「Actions」管理页面：`/dashboard/tenants/{tenant_id}/actions`
2. 点击「New Action」按钮
3. 填写基本信息：
   - 名称：`Test Post Login Action`
   - 描述：`Add custom claims for testing`
   - 触发器：选择 `Post Login`
   - 执行顺序：`0`
   - 超时：`3000`
4. 填写脚本：
   ```typescript
   // Add custom claims
   context.claims = context.claims || {};
   context.claims.test_claim = "test_value";
   context;
   ```
5. 保持「Enabled」开关开启
6. 点击「Create Action」

### 测试操作流程（API）
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -X POST http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
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
WHERE name = 'Test Post Login Action' AND tenant_id = '{tenant_id}';
-- 预期:
-- - 存在记录
-- - enabled = true
-- - trigger_id = 'post-login'
-- - execution_order = 0
-- - timeout_ms = 3000
-- - script 包含 "test_claim"
```

---

## 场景 2：创建 Action（使用模板）

### 初始状态
- 管理员已登录
- 存在租户 id=`{tenant_id}`

### 目的
验证使用内置模板快速创建 Action

### 测试操作流程（Portal UI）
1. 进入「Actions」管理页面，点击「New Action」
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
WHERE name = 'Department Claims Action' AND tenant_id = '{tenant_id}';
-- 预期: script 包含 "department" 和 "tier"
```

---

## 场景 3：列表查看与筛选

### 初始状态
- 存在多个 Actions，包含不同触发器类型

### 目的
验证 Action 列表、搜索和筛选功能

### 测试操作流程（Portal UI）
1. 进入「Actions」管理页面
2. 验证列表显示所有 Actions
3. 点击「Post Login」触发器筛选按钮
4. 验证仅显示 post-login 类型的 Actions
5. 在搜索框输入「Department」
6. 验证仅显示名称或描述包含「Department」的 Actions

### 测试操作流程（API）
```bash
# 列出所有 Actions
curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
  -H "Authorization: Bearer $TOKEN"

# 按触发器筛选
curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions?trigger_id=post-login \
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

## 场景 4：查看 Action 详情

### 初始状态
- 存在 Action id=`{action_id}`

### 目的
验证 Action 详情页显示完整信息

### 测试操作流程（Portal UI）
1. 在 Actions 列表中点击某个 Action 的「View Details」
2. 验证详情页显示：
   - 基本信息（名称、描述、触发器、状态）
   - 统计数据（执行次数、成功率、平均耗时、24小时执行次数）
   - Script 标签页：完整脚本代码
   - Logs 标签页：执行日志列表
   - Metadata：ID、创建时间、更新时间

### 测试操作流程（API）
```bash
# 获取 Action 详情
curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN"

# 获取统计信息
curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id}/stats \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 所有信息正确显示
- 统计数据准确
- 脚本代码格式化显示
- 日志按时间倒序排列

---

## 场景 5：更新 Action

### 初始状态
- 存在 Action id=`{action_id}`

### 目的
验证 Action 更新功能

### 测试操作流程（Portal UI）
1. 进入 Action 详情页，点击「Edit」
2. 修改：
   - 描述：添加更详细的描述
   - 脚本：修改 claims 内容
   - 执行顺序：改为 `10`
3. 点击「Save Changes」

### 测试操作流程（API）
```bash
curl -X PATCH http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Updated description",
    "script": "context.claims = context.claims || {}; context.claims.updated = true; context;",
    "execution_order": 10
  }'
```

### 预期结果（Portal UI）
- 重定向回详情页
- 显示更新成功
- 修改的内容正确显示

### 预期结果（API）
- HTTP 200 OK
- 返回更新后的 Action 对象

### 预期数据状态
```sql
SELECT description, execution_order, updated_at FROM actions
WHERE id = '{action_id}';
-- 预期:
-- - description 已更新
-- - execution_order = 10
-- - updated_at 时间戳更新
```

---

## 场景 6：启用/禁用 Action

### 初始状态
- 存在已启用的 Action id=`{action_id}`

### 目的
验证快速切换 Action 启用状态

### 测试操作流程（Portal UI）
1. 在 Actions 列表页找到目标 Action
2. 点击该 Action 卡片上的「Enabled」开关（关闭）
3. 验证状态立即变为「Disabled」
4. 再次点击开关（开启）
5. 验证状态变回「Enabled」

### 测试操作流程（API）
```bash
# 禁用
curl -X PATCH http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# 启用
curl -X PATCH http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

### 预期结果（Portal UI）
- 状态切换立即生效（Optimistic UI）
- Badge 颜色变化（Enabled=绿色，Disabled=灰色）
- 无需页面刷新

### 预期数据状态
```sql
SELECT enabled FROM actions WHERE id = '{action_id}';
-- 预期: enabled = false（禁用后）或 true（启用后）
```

---

## 场景 7：删除 Action

### 初始状态
- 存在 Action id=`{action_id}`

### 目的
验证 Action 删除功能

### 测试操作流程（Portal UI）
1. 在 Actions 列表页找到目标 Action
2. 点击「Delete」按钮
3. 在确认对话框中确认删除
4. 验证 Action 从列表中消失

### 测试操作流程（API）
```bash
curl -X DELETE http://localhost:8080/api/v1/tenants/{tenant_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果（Portal UI）
- 显示确认对话框
- 确认后 Action 立即从列表移除
- 显示删除成功消息

### 预期结果（API）
- HTTP 200 OK
- 返回成功消息

### 预期数据状态
```sql
SELECT COUNT(*) FROM actions WHERE id = '{action_id}';
-- 预期: COUNT = 0（已删除）

SELECT COUNT(*) FROM action_executions WHERE action_id = '{action_id}';
-- 预期: 执行日志保留（不级联删除）
```

---

## 边界条件测试

### 1. 重复名称
- **操作**: 创建同租户下同名 Action
- **预期**: 允许（名称不唯一）

### 2. 无效触发器
- **操作**: 使用不存在的 trigger_id
- **API 预期**: HTTP 400，返回错误信息
- **Portal 预期**: 下拉菜单限制，无法选择无效值

### 3. 脚本语法错误
- **操作**: 提交包含 TypeScript 语法错误的脚本
- **API 预期**: HTTP 400，返回编译错误
- **Portal 预期**: 可创建（语法检查在执行时）

### 4. 超时范围
- **操作**: 设置 timeout_ms = 100000（超过最大值 30000）
- **预期**: 自动限制为最大值或返回错误

### 5. 执行顺序冲突
- **操作**: 多个 Actions 使用相同 execution_order
- **预期**: 允许（按 ID 或创建时间排序）

---

## 性能测试

### 批量操作
```bash
# 批量创建 100 个 Actions
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"Action $i\", \"trigger_id\": \"post-login\", \"script\": \"context;\", \"enabled\": true}"
done

# 列表查询性能
time curl http://localhost:8080/api/v1/tenants/{tenant_id}/actions \
  -H "Authorization: Bearer $TOKEN"
```

**预期**:
- 批量创建：每个请求 < 200ms
- 列表查询（100条）: < 500ms
