# Webhook 管理 - 边界测试

**模块**: Webhook 管理
**测试范围**: URL 验证、Payload 处理
**场景数**: 3

---

## 场景 1：无效 URL 验证

### 初始状态
- 用户尝试创建 Webhook

### 目的
验证 URL 格式验证

### 测试操作流程
测试以下 URL：
1. 有效 HTTPS：`https://api.example.com/webhook` ✓
2. 有效 localhost：`http://localhost:3000/webhook` ✓
3. 无效 HTTP（非本地）：`http://api.example.com/webhook` ✗
4. 无协议：`api.example.com/webhook` ✗
5. 内网 IP：`http://192.168.1.1/webhook` ✗（视安全策略）

### 预期结果
- 非法 URL 被拒绝

---

## 场景 2：大 Payload 处理

### 初始状态
- Webhook 订阅了会产生大 Payload 的事件

### 目的
验证大 Payload 的处理

### 测试操作流程
1. 触发产生大量数据的事件

### 预期结果
- Payload 被截断或简化
- 不会因为 Payload 过大导致发送失败

---

## 场景 3：无效端点响应处理

### 初始状态
- 目标服务器返回各种错误响应

### 目的
验证错误响应处理

### 测试操作流程
测试以下响应：
1. 200 OK - 成功
2. 301/302 重定向 - 视配置而定
3. 400 Bad Request - 记录失败
4. 401 Unauthorized - 记录失败
5. 500 Internal Server Error - 记录失败并重试
6. 超时 - 记录失败并重试

### 预期结果
- 各种错误被正确处理
- 失败计数正确更新

### 预期数据状态
```sql
SELECT failure_count FROM webhooks WHERE id = '{webhook_id}';
```

---

## Webhook 测试工具

可以使用以下工具测试 Webhook：

1. **httpbin.org**
   - URL: `https://httpbin.org/post`

2. **webhook.site**
   - https://webhook.site/

3. **本地 Mock 服务器**
   ```javascript
   const express = require('express');
   const app = express();
   app.use(express.json());

   app.post('/webhook', (req, res) => {
     console.log('Headers:', req.headers);
     console.log('Body:', JSON.stringify(req.body, null, 2));
     res.json({ received: true });
   });

   app.listen(3000);
   ```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | URL 格式验证 | ☐ | | | |
| 2 | 大 Payload 处理 | ☐ | | | |
| 3 | 错误响应处理 | ☐ | | | |
