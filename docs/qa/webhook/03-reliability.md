# Webhook 管理 - 可靠性测试

**模块**: Webhook 管理
**测试范围**: 重试、自动禁用、密钥重新生成
**场景数**: 4

---

## 场景 1：Webhook 失败重试

### 初始状态
- 存在启用的 Webhook
- 目标服务器暂时不可用

### 目的
验证 Webhook 失败后的重试机制

### 测试操作流程
1. 关闭目标服务器
2. 触发 Webhook 事件
3. 观察重试行为

### 预期结果
- 首次发送失败
- 系统按照重试策略进行重试
- failure_count 增加

### 预期数据状态
```sql
SELECT failure_count FROM webhooks WHERE id = '{webhook_id}';
-- 预期: failure_count > 0
```

---

## 场景 2：Webhook 自动禁用

### 初始状态
- Webhook 连续失败次数超过阈值（如 10 次）

### 目的
验证 Webhook 连续失败后自动禁用

### 测试操作流程
1. 模拟 Webhook 连续失败 10+ 次
2. 检查 Webhook 状态

### 预期结果
- Webhook 自动禁用
- 管理员收到通知（可选）

### 预期数据状态
```sql
SELECT enabled, failure_count FROM webhooks WHERE id = '{webhook_id}';
-- 预期: enabled = false，failure_count >= 10
```

---

## 场景 3：重新生成 Secret

### 初始状态
- 存在 Webhook，当前 secret 已泄露

### 目的
验证重新生成 Secret 功能

### 测试操作流程
1. 找到目标 Webhook
2. 点击「重新生成 Secret」
3. 确认操作

### 预期结果
- 显示新的 Secret
- 旧 Secret 立即失效

### 预期数据状态
```sql
SELECT secret FROM webhooks WHERE id = '{webhook_id}';
-- 预期: secret 与之前不同
```

---

## 场景 4：Webhook 超时处理

### 初始状态
- 目标服务器响应很慢（>30秒）

### 目的
验证请求超时处理

### 测试操作流程
1. 配置目标服务器延迟响应
2. 触发 Webhook 事件

### 预期结果
- 请求在超时后终止
- 计为失败，进入重试队列

---

## 测试数据准备 SQL

```sql
-- 准备测试租户
INSERT INTO tenants (id, name, slug, settings, status) VALUES
('tenant-wh-1111-1111-111111111111', 'Webhook Test', 'webhook-test', '{}', 'active');

-- 准备测试 Webhooks
INSERT INTO webhooks (id, tenant_id, name, url, secret, events, enabled, failure_count) VALUES
('wh-1111-1111-1111-111111111111', 'tenant-wh-1111-1111-111111111111',
 'User Events', 'https://httpbin.org/post', 'secret-123',
 '["user.created","user.updated"]', true, 0),
('wh-2222-2222-2222-222222222222', 'tenant-wh-1111-1111-111111111111',
 'Failed Webhook', 'https://httpbin.org/post', 'secret-456',
 '["user.created"]', false, 10);

-- 清理
DELETE FROM webhooks WHERE id LIKE 'wh-%';
DELETE FROM tenants WHERE id LIKE 'tenant-wh-%';
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 失败重试 | ☐ | | | |
| 2 | 自动禁用 | ☐ | | | |
| 3 | 重新生成 Secret | ☐ | | | |
| 4 | 超时处理 | ☐ | | | |
