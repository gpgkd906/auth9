# Action 日志查询测试（详情）

**模块**: Action 执行日志
**测试范围**: 单条日志详情查看
**场景数**: 1

---
## 场景 6：日志详情入口可见性与查看

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


---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 日志详情查看 | ☐ | | | |
