# Webhook 管理 - 可靠性测试

**模块**: Webhook 管理
**测试范围**: 重试、自动禁用、密钥重新生成
**场景数**: 4

---

## 场景 1：Webhook 失败重试

### 初始状态
- 存在启用的 Webhook，URL 指向不可达地址（如 `http://192.0.2.1:9999/hook` TEST-NET-1）
- 目标服务器不可用

### 目的
验证 Webhook 失败后 failure_count 是否增加

### 测试操作流程
1. 创建 Webhook 并设置 URL 为不可达地址
2. **通过 UI 的 "Test" 按钮或 API `POST /api/v1/tenants/{id}/webhooks/{id}/test` 触发**
3. 等待请求超时（可能需要 10-30 秒）
4. 检查数据库 failure_count

> **⚠️ Test 按钮会更新 failure_count**。代码中 `webhook_service.test()` 调用 `webhook_repo.update_triggered(id, result.success)`，失败时 `failure_count = failure_count + 1`。如果 failure_count 未增加，可能是：
> - Webhook URL 实际可达（检查 URL 是否有误）
> - 请求仍在超时等待中（需等待更长时间）
> - 使用了 localhost 地址，容器内可能解析到容器自身

### 预期结果
- 发送失败
- failure_count 增加

### 预期数据状态
```sql
SELECT failure_count, last_triggered_at FROM webhooks WHERE id = '{webhook_id}';
-- 预期: failure_count > 0, last_triggered_at IS NOT NULL
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| failure_count = 0 且 last_triggered_at = NULL | Test 按钮请求可能仍在超时中 | 使用更快超时的不可达地址，或等待更长时间后再查询 |
| failure_count = 0 但 last_triggered_at 有值 | Webhook 发送成功（URL 可达） | 确认 URL 指向不可达地址（推荐 `http://192.0.2.1:9999/hook`） |
| UI 无错误提示 | Test 按钮的 UI 反馈可能不显示错误详情 | 通过数据库或 API 查看 failure_count |

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

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 失败重试 | ☐ | | | |
| 2 | 自动禁用 | ☐ | | | |
| 3 | 重新生成 Secret | ☐ | | | |
| 4 | 超时处理 | ☐ | | | |
| 5 | 认证状态检查 | ☐ | | | |
