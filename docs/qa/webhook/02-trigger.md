# Webhook 管理 - 事件触发测试

**模块**: Webhook 管理
**测试范围**: 事件触发、签名验证
**场景数**: 5

---

## 支持的 Webhook 事件类型

| 事件类型 | 描述 |
|---------|------|
| user.created | 用户创建 |
| user.updated | 用户更新 |
| user.deleted | 用户删除 |
| session.created | 会话创建 |
| session.revoked | 会话撤销 |
| security_alert.created | 安全告警创建 |

---

## 场景 1：测试 Webhook 连接

### 初始状态
- 存在 Webhook id=`{webhook_id}`
- 目标 URL 的服务器正在运行

### 目的
验证 Webhook 测试功能

### 测试操作流程
1. 找到目标 Webhook
2. 点击「测试」按钮

### 预期结果
- 系统发送测试请求
- 显示测试结果（成功/失败）

### 预期数据状态
```sql
SELECT last_triggered_at FROM webhooks WHERE id = '{webhook_id}';
-- 可选：验证 last_triggered_at 是否更新
```

---

## 场景 2：用户创建事件触发

### 初始状态
- 存在启用的 Webhook，订阅了 `user.created` 事件
- 目标服务器准备接收请求

### 目的
验证用户创建时 Webhook 被正确触发

### 测试操作流程
1. 创建新用户
2. 检查目标服务器收到的请求

### 预期结果
- 目标服务器收到 POST 请求
- 请求体包含：
  ```json
  {
    "event": "user.created",
    "timestamp": "2024-01-01T00:00:00Z",
    "data": { "id": "...", "email": "..." }
  }
  ```

### 预期数据状态
```sql
SELECT last_triggered_at, failure_count FROM webhooks WHERE id = '{webhook_id}';
-- 预期: last_triggered_at 已更新，failure_count = 0
```

---

## 场景 3：Webhook 签名验证

### 初始状态
- Webhook secret 已知
- 目标服务器实现了签名验证

### 目的
验证 Webhook 请求包含有效签名

### 测试操作流程
1. 触发 Webhook 事件
2. 在目标服务器验证签名

### 预期结果
- 请求包含 `X-Webhook-Signature` 头
- 签名验证通过

### 签名验证代码示例
```javascript
const crypto = require('crypto');
function verifySignature(payload, signature, secret) {
  const expected = crypto.createHmac('sha256', secret)
    .update(JSON.stringify(payload)).digest('hex');
  return signature === expected;
}
```

---

## 场景 4：多个 Webhook 同时触发

### 初始状态
- 租户有 3 个 Webhook 都订阅了 `user.created` 事件

### 目的
验证多个 Webhook 同时触发

### 测试操作流程
1. 创建新用户
2. 检查所有 3 个目标服务器

### 预期结果
- 所有 3 个 Webhook 都收到请求

### 预期数据状态
```sql
SELECT id, last_triggered_at FROM webhooks
WHERE tenant_id = '{tenant_id}' AND events LIKE '%user.created%';
-- 预期: 所有记录的 last_triggered_at 都已更新
```

---

## 场景 5：选择性事件订阅

### 初始状态
- Webhook A 订阅：`user.created`, `user.deleted`
- Webhook B 订阅：`session.created`

### 目的
验证事件只发送给订阅了该事件的 Webhook

### 测试操作流程
1. 创建新用户
2. 检查 Webhook A 和 B

### 预期结果
- Webhook A 收到请求
- Webhook B 没有收到请求

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 测试 Webhook 连接 | ☐ | | | |
| 2 | 用户创建事件触发 | ☐ | | | |
| 3 | 签名验证 | ☐ | | | |
| 4 | 多 Webhook 同时触发 | ☐ | | | |
| 5 | 选择性事件订阅 | ☐ | | | |
