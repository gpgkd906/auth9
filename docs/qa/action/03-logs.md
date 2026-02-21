# Action 日志查询测试

**模块**: Action 执行日志
**测试范围**: 日志查询、筛选、分页、性能
**场景数**: 6

---

## 前置条件

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

## 场景 1：查询所有执行日志

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

## 场景 6：日志详情查看

### 初始状态
- 存在执行日志

### 目的
验证日志详情查看功能

### 测试操作流程（Portal UI）
1. 在 Logs 标签页中点击某条失败的日志
2. 展开查看详细信息

### 预期结果（Portal UI）
- 显示完整错误堆栈（如果有）
- 显示执行上下文快照（如果记录）
- 显示执行时长
- 显示用户信息（如果有）

### 预期数据状态
```sql
SELECT * FROM action_executions
WHERE id = '{execution_id}';
-- 预期: 返回完整的执行记录
```

---

## 性能测试

### 1. 大量日志查询性能

**目标**: 查询 10,000 条日志 < 500ms

**准备数据**:
```bash
# 生成 10,000 条测试日志（通过模拟执行）
for i in {1..10000}; do
  mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
    INSERT INTO action_executions (id, action_id, service_id, trigger_id, success, duration_ms, executed_at)
    VALUES (UUID(), '$ACTION_ID', '$SERVICE_ID', 'post-login', TRUE, FLOOR(RAND() * 100), NOW() - INTERVAL FLOOR(RAND() * 86400) SECOND);
  "
done
```

**测试方法**:
```bash
time curl -s "http://localhost:8080/api/v1/services/{service_id}/actions/logs?limit=100" \
  -H "Authorization: Bearer $TOKEN" > /dev/null
```

**预期**: 响应时间 < 500ms

### 2. 复杂筛选查询性能

**测试方法**:
```bash
time curl -s "http://localhost:8080/api/v1/services/{service_id}/actions/logs?success=false&from=2026-01-01T00:00:00Z&to=2026-02-12T23:59:59Z&limit=100" \
  -H "Authorization: Bearer $TOKEN" > /dev/null
```

**预期**: 响应时间 < 800ms

### 3. 分页性能一致性

**测试方法**:
```bash
# 第 1 页
time curl -s "http://localhost:8080/api/v1/services/{service_id}/actions/logs?limit=50&offset=0" \
  -H "Authorization: Bearer $TOKEN" > /dev/null

# 第 100 页
time curl -s "http://localhost:8080/api/v1/services/{service_id}/actions/logs?limit=50&offset=4950" \
  -H "Authorization: Bearer $TOKEN" > /dev/null
```

**预期**: 两次查询时间差 < 200ms（说明分页性能稳定）

---

## 日志保留策略测试

### 1. 日志自动清理（如果实现）

**测试方法**:
```sql
-- 插入 90 天前的日志
INSERT INTO action_executions (id, action_id, service_id, trigger_id, success, duration_ms, executed_at)
VALUES (UUID(), '{action_id}', '{service_id}', 'post-login', TRUE, 10, NOW() - INTERVAL 90 DAY);

-- 等待清理任务运行

-- 验证是否被清理
SELECT COUNT(*) FROM action_executions
WHERE executed_at < NOW() - INTERVAL 90 DAY;
-- 预期: COUNT = 0（如果保留期为 90 天）
```

### 2. 日志存储空间监控

**测试方法**:
```sql
SELECT
  TABLE_NAME,
  ROUND((DATA_LENGTH + INDEX_LENGTH) / 1024 / 1024, 2) AS size_mb,
  TABLE_ROWS
FROM information_schema.TABLES
WHERE TABLE_SCHEMA = 'auth9' AND TABLE_NAME = 'action_executions';
```

**预期**: 10,000 条日志约占用 5-10 MB

---

## 边界条件测试

### 1. 空日志查询
- **操作**: 查询从未执行过的 Action 的日志
- **预期**: 返回空数组，total = 0

### 2. 无效筛选参数
- **操作**: 使用无效的 user_id 或 action_id
- **预期**: 返回空数组，不报错

### 3. 超大 limit 参数
- **操作**: `limit=10000`
- **预期**: 自动限制为最大值（如 1000）或返回错误

### 4. 负数 offset
- **操作**: `offset=-1`
- **预期**: HTTP 400 或自动修正为 0

### 5. 时间范围倒置
- **操作**: `from=2026-02-12&to=2026-02-11`
- **预期**: HTTP 400 或自动交换顺序

---

## 回归测试检查清单

- [ ] 日志按时间倒序排列
- [ ] 成功/失败筛选正确
- [ ] 时间范围筛选正确
- [ ] 用户筛选正确
- [ ] 全局日志查询跨 Actions
- [ ] 分页功能正常
- [ ] 日志详情显示完整
- [ ] 查询性能达标（< 500ms）
- [ ] 复杂筛选性能达标（< 800ms）
- [ ] 分页性能稳定
- [ ] 边界条件处理正确
- [ ] 日志保留策略生效（如果实现）
