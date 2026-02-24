# Action 日志查询测试

**模块**: Action 执行日志
**测试范围**: 日志查询、筛选、分页、性能
**场景数**: 5

---

## 前置条件

### Token 要求

**重要**：Action 日志查询 API（`/api/v1/services/{service_id}/actions/*/logs`）需要 **tenant access token**，不接受 identity token。

| 测试方式 | Token 类型 | 说明 |
|----------|-----------|------|
| Portal UI | 自动 (tenant access token) | 浏览器登录后 Portal 自动处理 token exchange |
| API 直接测试 | 需 tenant access token | 通过 Token Exchange 流程获取，或使用 Portal Network 面板复制 |
| 数据库验证 | 不需要 token | 直接查询 `action_executions` 表 |

> `gen-admin-token.sh` 生成的是 identity token，只能用于 Action 创建（`/api/v1/services/*/actions`），不能用于日志查询端点。

### 测试数据准备

使用 API 或 Portal 创建测试 Actions 并触发执行，生成足够的日志数据：

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
SERVICE_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services LIMIT 1;")

# 创建测试 Action
ACTION_ID=$(curl -s -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Logging Action",
    "trigger_id": "post-login",
    "script": "context.claims = context.claims || {}; context.claims.test = true; context;",
    "enabled": true
  }' | jq -r '.data.id')

# 触发多次执行（通过登录）
for i in {1..20}; do
  # 登录操作，自动触发 post-login Action
  echo "Triggering execution $i..."
  # 实际登录流程或使用测试工具
done
```

### 验证日志数据存在

```sql
SELECT COUNT(*) FROM action_executions
WHERE action_id = '{action_id}';
-- 预期: COUNT >= 20
```

---

## 场景 1：日志入口可见性与查询所有执行日志

### 初始状态
- 存在多条执行日志
- 用户已登录 Portal

### 目的
验证日志列表查询和分页功能

### 测试操作流程（Portal UI）
1. 进入 Action 详情页：`/dashboard/services/{service_id}/actions/{action_id}`
2. 点击「Logs」标签页
3. 验证日志列表显示

### 测试操作流程（API）
```bash
# 查询最近 50 条日志
curl http://localhost:8080/api/v1/services/{service_id}/actions/{action_id}/logs?limit=50 \
  -H "Authorization: Bearer $TOKEN"

# 分页查询
curl http://localhost:8080/api/v1/services/{service_id}/actions/{action_id}/logs?limit=10\&offset=10 \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果（Portal UI）
- 日志按时间倒序排列（最新的在最上方）
- 每条日志显示：
  - 成功/失败状态图标
  - 执行时间（duration_ms）
  - 执行时间戳（executed_at）
  - 错误信息（如果失败）
  - User ID（如果有）
- 颜色区分：成功（绿色）、失败（红色）

### 预期结果（API）
- HTTP 200 OK
- 返回日志数组，包含完整字段：
  ```json
  {
    "data": [
      {
        "id": "log-uuid",
        "action_id": "action-uuid",
        "service_id": "service-uuid",
        "trigger_id": "post-login",
        "user_id": "user-uuid",
        "success": true,
        "duration_ms": 15,
        "error_message": null,
        "executed_at": "2026-02-12T10:30:00Z"
      }
    ],
    "total": 20,
    "has_more": false
  }
  ```

### 预期数据状态
```sql
SELECT COUNT(*) FROM action_executions
WHERE action_id = '{action_id}';
-- 预期: 返回实际执行次数

SELECT * FROM action_executions
WHERE action_id = '{action_id}'
ORDER BY executed_at DESC
LIMIT 10;
-- 预期: 按时间倒序返回最近 10 条
```

---

## 场景 2：按成功/失败筛选日志

### 初始状态
- 存在成功和失败的执行日志

### 目的
验证日志筛选功能

### 测试操作流程（API）
```bash
# 查询所有成功的执行
curl "http://localhost:8080/api/v1/services/{service_id}/actions/{action_id}/logs?success=true&limit=50" \
  -H "Authorization: Bearer $TOKEN"

# 查询所有失败的执行
curl "http://localhost:8080/api/v1/services/{service_id}/actions/{action_id}/logs?success=false&limit=50" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 仅返回符合 success 参数的日志
- 成功日志：error_message = NULL
- 失败日志：error_message 不为空

### 预期数据状态
```sql
-- 验证成功日志
SELECT COUNT(*) FROM action_executions
WHERE action_id = '{action_id}' AND success = true;
-- 预期: 与 API 返回的 total 一致

-- 验证失败日志
SELECT COUNT(*) FROM action_executions
WHERE action_id = '{action_id}' AND success = false;
-- 预期: 与 API 返回的 total 一致
```

---

## 场景 3：按时间范围查询日志

### 初始状态
- 存在跨越多天的执行日志

### 目的
验证时间范围筛选功能

### 测试操作流程（API）
```bash
# 查询最近 24 小时的日志
FROM=$(date -u -d '1 day ago' +%Y-%m-%dT%H:%M:%SZ)
TO=$(date -u +%Y-%m-%dT%H:%M:%SZ)

curl "http://localhost:8080/api/v1/services/{service_id}/actions/logs?from=$FROM&to=$TO&limit=100" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 仅返回指定时间范围内的日志
- 所有日志的 executed_at 在 [from, to] 区间内

### 预期数据状态
```sql
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_id}'
  AND executed_at >= '2026-02-11 10:00:00'
  AND executed_at <= '2026-02-12 10:00:00';
-- 预期: 与 API 返回的 total 一致
```

---

## 场景 4：按用户查询日志

### 初始状态
- 存在多个用户触发的执行日志

### 目的
验证按用户筛选日志

### 测试操作流程（API）
```bash
# 查询特定用户的所有执行日志
USER_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = 'test@example.com';")

curl "http://localhost:8080/api/v1/services/{service_id}/actions/logs?user_id=$USER_ID&limit=100" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 仅返回该用户触发的日志
- 所有日志的 user_id 字段匹配查询参数

### 预期数据状态
```sql
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_id}'
  AND user_id = '{user_id}';
-- 预期: 与 API 返回的 total 一致
```

---

## 场景 5：全局日志查询（跨 Actions）

### 初始状态
- Service 下存在多个 Actions
- 每个 Action 都有执行日志

### 目的
验证跨 Action 的全局日志查询

### 测试操作流程（API）
```bash
# 查询Service 下所有 Actions 的日志
curl "http://localhost:8080/api/v1/services/{service_id}/actions/logs?limit=100" \
  -H "Authorization: Bearer $TOKEN"

# 查询特定触发器的所有日志
curl "http://localhost:8080/api/v1/services/{service_id}/actions/logs?trigger_id=post-login&limit=100" \
  -H "Authorization: Bearer $TOKEN"
```

### 预期结果
- 返回Service 下所有符合条件的日志
- 日志来自不同的 Actions
- 按时间倒序排列

### 预期数据状态
```sql
-- 验证全局日志查询
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_id}';
-- 预期: 与 API 返回的 total 一致

-- 验证触发器筛选
SELECT COUNT(*) FROM action_executions
WHERE service_id = '{service_id}'
  AND trigger_id = 'post-login';
-- 预期: 与 API 返回的 total 一致
```

---


---

## 说明

场景 6（日志详情查看）已拆分到 `docs/qa/action/09-logs-detail.md`。

---

## 故障排除

| 症状 | 原因 | 解决方案 |
|------|------|----------|
| `403 Forbidden` "Identity token is only allowed for tenant selection and exchange" | 使用 identity token 访问日志 API | 日志 API 需要 tenant access token，通过 Portal UI 测试或先进行 Token Exchange |
| `401 Unauthorized` | Token 过期或无效 | 重新生成 token |

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查询所有执行日志 | ☐ | | | |
| 2 | 按成功/失败筛选日志 | ☐ | | | |
| 3 | 按时间范围查询日志 | ☐ | | | |
| 4 | 按用户查询日志 | ☐ | | | |
| 5 | 全局日志查询（跨 Actions） | ☐ | | | |
