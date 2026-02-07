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
本项目目前**没有稳定、可配置**的“天然大 payload”业务事件（例如带大量列表字段的事件）。
因此这里补充一个（依赖安全检测规则的）**可复现**方案：通过 Keycloak events webhook 写入超长 `user_agent`，触发 `security.alert` 的 `new_device` 告警事件（其 `data.details.user_agent` 会包含该超长字符串），从而构造大 payload。

前置条件：
- 存在启用的 webhook，订阅 `security.alert`，URL 指向可接收的端点（建议 `webhook.site`）。
- Auth9 Core 已启动（默认 `http://localhost:8080`）。
- 注意：`login_events.user_agent` 列类型为 `TEXT`，单条最大约 64KB；因此本场景建议把 `User-Agent` 控制在 `60000` 字符以内。

步骤：
1. （准备）创建一次“成功登录”事件，建立已知设备：
   - 发送一次 `type=LOGIN`，带一个正常长度的 `User-Agent`。
2. （触发 new_device + 制造大 payload）再次发送 `type=LOGIN`，同一 `userId`，但 `User-Agent` 替换为 60KB 超长字符串。
3. 检查接收端（如 `webhook.site`）收到的 `security.alert` webhook body 体积明显增大（包含 `data.details.user_agent`），并检查系统侧 `failure_count` 是否维持为 0。

注意：
- 该方案依赖 `new_device` 告警能够被触发；如果你发现始终没有产生 `security.alert(new_device)`，优先检查安全检测实现是否已修复“当前事件被当作已知设备”导致无法触发的逻辑问题。

示例命令（仅供 QA 复现，`userId` 使用任意 UUID 字符串即可）：
```bash
# 生成约 60KB 的 User-Agent
UA="$(python3 - <<'PY'
print("A" * 60000)
PY
)"

# 1) 先写入一个“正常 UA”的成功登录事件（建立已知设备）
curl -sS -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "User-Agent: qa-small-ua" \
  -d '{
    "type": "LOGIN",
    "time": 0,
    "userId": "00000000-0000-0000-0000-000000000001",
    "ipAddress": "203.0.113.10",
    "details": { "email": "qa-big-payload@example.com" }
  }' >/dev/null

# 2) 再写入一个“超长 UA”的成功登录事件，期望触发 security.alert(new_device)
curl -sS -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "User-Agent: ${UA}" \
  -d '{
    "type": "LOGIN",
    "time": 0,
    "userId": "00000000-0000-0000-0000-000000000001",
    "ipAddress": "203.0.113.10",
    "details": { "email": "qa-big-payload@example.com" }
  }' >/dev/null
```

说明：
- 如果环境配置了 `keycloak.webhook_secret`，还需要携带 `x-keycloak-signature` / `x-webhook-signature`；否则本地 dev 默认会跳过签名校验。
  - 签名算法见 `auth9-core/src/api/keycloak_event.rs`（HMAC-SHA256，hex）。

### 预期结果
需要明确“截断/简化”的规则和验收点，否则会变成不可测的主观判断。这里给出一套**当前实现现状**与**建议验收标准**：

当前实现现状（以代码为准）：
- Webhook payload 直接使用 `serde_json::to_string(WebhookEvent)`，**不会主动截断/简化**。
- HTTP client timeout 为 30 秒（`auth9-core/src/service/webhook.rs`）。

建议的验收标准（可作为后续实现的规范）：
1. 当 payload 序列化后 `<= 256KB`：
   - 必须投递成功（接收端返回 2xx）
   - `webhooks.failure_count` 维持/重置为 0
2. 当 payload `> 256KB`（或达到系统上限）：
   - 系统应**简化** payload（而不是失败）：
     - 仅保留必要字段（如 `event_type/timestamp`、关键 ID 字段）
     - 对超长字符串字段做截断（例如每个 string 字段最多 4096 字符）
     - 在 payload 中加入 `meta.truncated=true` 与 `meta.original_size_bytes`
   - 或者（如果产品选择 hard fail）：
     - 明确返回失败并可观测（日志/metrics），并增加 `failure_count`
3. 签名验证：
   - 如果发送了 `X-Webhook-Signature`，接收端以**实际发送的 body**计算签名应通过。

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

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | URL 格式验证 | ☐ | | | |
| 2 | 大 Payload 处理 | ☐ | | | |
| 3 | 错误响应处理 | ☐ | | | |
| 4 | 认证状态检查 | ☐ | | | |
