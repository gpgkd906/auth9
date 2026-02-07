# 高级攻击 - Webhook 伪造测试

**模块**: 高级攻击
**测试范围**: Webhook 签名验证、重放攻击、内容篡改
**场景数**: 2
**风险等级**: 🟠 高
**OWASP ASVS**: V2.10, V13.2

---

## 背景知识

Auth9 接收来自 Keycloak 的 Event Webhook，并向外部系统发送 Webhook 通知：

**入站 Webhook（Keycloak → Auth9）**:
- 端点: `POST /api/v1/webhooks/keycloak`
- 验证: `X-Webhook-Secret` 头与配置的 `KEYCLOAK_WEBHOOK_SECRET` 比较
- 用途: 接收用户登录事件、管理事件等

**出站 Webhook（Auth9 → 外部系统）**:
- 签名: HMAC-SHA256
- 事件: user.created, user.updated, login.success, login.failed, security.alert 等

Webhook 伪造可导致：虚假用户事件注入、安全告警绕过、业务逻辑篡改。

---

## 场景 1：入站 Webhook 签名伪造

### 前置条件
- 了解 Keycloak Webhook 端点路径
- 了解 Webhook 请求格式

### 攻击目标
验证 Keycloak Event Webhook 是否严格验证签名

### 攻击步骤
1. 发送无签名头的 Webhook 请求
2. 发送空签名的请求
3. 发送错误签名的请求
4. 发送正确格式但错误值的签名
5. 尝试暴力破解签名密钥
6. 测试时间侧信道（比较时间差异推断密钥）

### 预期安全行为
- 缺少签名头返回 401/403
- 空签名返回 401/403
- 错误签名返回 401/403
- 使用常数时间比较防止时间侧信道
- 暴力破解签名有速率限制

### 验证方法
```bash
# Keycloak 事件 payload
EVENT='{"type":"LOGIN","realmId":"auth9","userId":"test-user","time":1706000000}'

# 无签名头
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -d "$EVENT"
# 预期: 401 或 403

# 空签名
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: " \
  -d "$EVENT"
# 预期: 401 或 403

# 错误签名
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: wrong-secret-value" \
  -d "$EVENT"
# 预期: 401 或 403

# 时间侧信道测试
# 发送多个请求，密钥第一个字符正确 vs 全部错误
# 比较响应时间，如果差异显著则存在时间侧信道
python3 << 'PYEOF'
import requests, time, statistics

url = "http://localhost:8080/api/v1/webhooks/keycloak"
headers = {"Content-Type": "application/json"}
event = '{"type":"LOGIN","realmId":"auth9"}'

# 全错密钥
times_wrong = []
for _ in range(100):
    start = time.perf_counter()
    requests.post(url, headers={**headers, "X-Webhook-Secret": "AAAA"}, data=event)
    times_wrong.append(time.perf_counter() - start)

# 部分正确密钥（假设第一个字符正确）
times_partial = []
for _ in range(100):
    start = time.perf_counter()
    requests.post(url, headers={**headers, "X-Webhook-Secret": "correct-first-char-rest-wrong"}, data=event)
    times_partial.append(time.perf_counter() - start)

print(f"Wrong: mean={statistics.mean(times_wrong)*1000:.2f}ms, stdev={statistics.stdev(times_wrong)*1000:.2f}ms")
print(f"Partial: mean={statistics.mean(times_partial)*1000:.2f}ms, stdev={statistics.stdev(times_partial)*1000:.2f}ms")
# 预期: 两者响应时间无显著差异（常数时间比较）
PYEOF
```

### 修复建议
- 使用 `hmac::verify` 或等效的常数时间比较
- 签名验证失败记录审计日志（含来源 IP）
- Webhook 端点有速率限制
- 考虑使用 HMAC 签名替代简单 secret 比较

---

## 场景 2：Webhook 重放攻击

### 前置条件
- 能够截获合法的 Webhook 请求
- 了解 Webhook 签名机制

### 攻击目标
验证是否可以重放已截获的合法 Webhook 请求

### 攻击步骤
1. 截获一个合法的 Keycloak Webhook 请求（含正确签名）
2. 在 5 分钟后重放该请求
3. 在 1 小时后重放
4. 多次快速重放同一请求
5. 检查系统是否处理了重复事件

### 预期安全行为
- 事件包含时间戳，过期事件被拒绝
- 理想情况：实现 nonce/event ID 去重
- 重放的事件不产生重复业务操作
- 重放尝试记录日志

### 验证方法
```bash
# 获取合法 Webhook 响应
# (需要从 Keycloak 实际发出或使用正确的 secret)
VALID_SECRET="$KEYCLOAK_WEBHOOK_SECRET"
EVENT='{"type":"LOGIN","realmId":"auth9","userId":"test-user","time":1706000000,"id":"event-123"}'

# 使用正确 secret 发送（模拟合法请求）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: $VALID_SECRET" \
  -d "$EVENT"
# 预期: 200

# 立即重放（应被去重或接受但幂等）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: $VALID_SECRET" \
  -d "$EVENT"
# 预期: 200 (幂等处理) 或 409 (已处理)

# 发送过期事件（timestamp 很旧）
OLD_EVENT='{"type":"LOGIN","realmId":"auth9","userId":"test-user","time":1600000000,"id":"event-old"}'
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/webhooks/keycloak \
  -H "Content-Type: application/json" \
  -H "X-Webhook-Secret: $VALID_SECRET" \
  -d "$OLD_EVENT"
# 预期: 400 或 200 但不处理（事件过期）

# 检查是否产生了重复的登录事件记录
curl -s -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/audit?action=login&user_id=test-user&limit=10" | jq '.total'
# 预期: 重放不增加额外记录
```

### 修复建议
- 事件包含唯一 ID（event_id），服务端去重
- 事件包含时间戳，拒绝超过 5 分钟的过期事件
- 已处理的 event_id 存储在 Redis（TTL = 1 小时）
- 幂等处理：重复事件返回 200 但不执行业务逻辑

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 入站 Webhook 签名伪造 | ☐ | | | |
| 2 | Webhook 重放攻击 | ☐ | | | |

---

## 参考资料

- [OWASP Webhook Security](https://cheatsheetseries.owasp.org/cheatsheets/Webhook_Security_Cheat_Sheet.html)
- [CWE-345: Insufficient Verification of Data Authenticity](https://cwe.mitre.org/data/definitions/345.html)
- [CWE-294: Authentication Bypass by Capture-replay](https://cwe.mitre.org/data/definitions/294.html)
- [GitHub Webhook Signatures](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
