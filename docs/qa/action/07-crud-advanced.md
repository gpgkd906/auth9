# Action 管理 - CRUD 进阶测试

**模块**: Action 管理
**测试范围**: Action 详情、更新、启用/禁用、删除
**场景数**: 4

---
## 场景 5：详情入口可见性与查看 Action 详情

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
curl http://localhost:8080/api/v1/services/{service_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN"

# 获取统计信息
curl http://localhost:8080/api/v1/services/{service_id}/actions/{action_id}/stats \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 所有信息正确显示
- 统计数据准确
- 脚本代码格式化显示
- 日志按时间倒序排列

---

## 场景 6：更新 Action

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
curl -X PATCH http://localhost:8080/api/v1/services/{service_id}/actions/{action_id} \
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

## 场景 7：启用/禁用 Action

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
curl -X PATCH http://localhost:8080/api/v1/services/{service_id}/actions/{action_id} \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# 启用
curl -X PATCH http://localhost:8080/api/v1/services/{service_id}/actions/{action_id} \
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

## 场景 8：删除 Action

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
curl -X DELETE http://localhost:8080/api/v1/services/{service_id}/actions/{action_id} \
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


---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看 Action 详情 | ☐ | | | |
| 2 | 更新 Action | ☐ | | | |
| 3 | 启用/禁用 Action | ☐ | | | |
| 4 | 删除 Action | ☐ | | | |
