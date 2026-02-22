# 高级攻击 - Webhook 伪造测试

**模块**: 高级攻击
**测试范围**: Webhook 签名验证、重放攻击、内容篡改
**场景数**: 2
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-ADV-05
**OWASP ASVS 5.0**: V10.5,V13.2,V16.2
**回归任务映射**: Backlog #5, #20


---

## 背景知识

Auth9 接收来自 Keycloak 的 Event Webhook，并向外部系统发送 Webhook 通知：

**入站 Webhook（Keycloak → Auth9）**:
- 端点: `POST /api/v1/keycloak/events`
- 验证: `X-Keycloak-Signature` 头（HMAC-SHA256 签名，格式 `sha256=<hex>`）与 `KEYCLOAK_WEBHOOK_SECRET` 计算的签名比较
- 备用头: `X-Webhook-Signature`（兼容旧版）
- 用途: 接收用户登录事件、管理事件等
- **注意**: 使用常数时间比较（`hmac::verify_slice`）防止时间侧信道攻击

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

### 前置条件（重要）

**必须确保 `KEYCLOAK_WEBHOOK_SECRET` 已配置**，否则签名验证不会启用。

Docker 默认配置中已在 `docker-compose.yml` 中设置：
```yaml
KEYCLOAK_WEBHOOK_SECRET: ${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}
```

如需手动验证：
```bash
# 确认环境变量已生效
docker exec auth9-core env | grep KEYCLOAK_WEBHOOK_SECRET
# 预期输出: KEYCLOAK_WEBHOOK_SECRET=dev-webhook-secret-change-in-production
```

### 验证方法
```bash
# 设置 webhook secret（与 docker-compose.yml 一致）
export KEYCLOAK_WEBHOOK_SECRET="dev-webhook-secret-change-in-production"

# Keycloak 事件 payload
EVENT='{"type":"LOGIN","realmId":"auth9","userId":"test-user","time":1706000000}'

# 注意: 正确的端点是 /api/v1/keycloak/events（不是 /api/v1/webhooks/keycloak）
# 注意: 签名头是 X-Keycloak-Signature（不是 X-Webhook-Secret）
# 签名格式: sha256=<hex-encoded-hmac-sha256>

# 无签名头
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -d "$EVENT"
# 预期: 401 (Missing webhook signature)

# 空签名
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: " \
  -d "$EVENT"
# 预期: 401 (Missing webhook signature)

# 错误签名
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=0000000000000000000000000000000000000000000000000000000000000000" \
  -d "$EVENT"
# 预期: 401 (Invalid webhook signature)

# 正确签名（验证合法请求可通过）
SIGNATURE=$(echo -n "$EVENT" | openssl dgst -sha256 -hmac "$KEYCLOAK_WEBHOOK_SECRET" | awk '{print $2}')
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$SIGNATURE" \
  -d "$EVENT"
# 预期: 204 (No Content - 事件已接受)

# 时间侧信道测试
python3 << 'PYEOF'
import requests, time, statistics

url = "http://localhost:8080/api/v1/keycloak/events"
headers = {"Content-Type": "application/json"}
event = '{"type":"LOGIN","realmId":"auth9"}'

# 全错签名
times_wrong = []
for _ in range(100):
    start = time.perf_counter()
    requests.post(url, headers={**headers, "X-Keycloak-Signature": "sha256=0000000000000000000000000000000000000000000000000000000000000000"}, data=event)
    times_wrong.append(time.perf_counter() - start)

# 部分正确签名
times_partial = []
for _ in range(100):
    start = time.perf_counter()
    requests.post(url, headers={**headers, "X-Keycloak-Signature": "sha256=ff00000000000000000000000000000000000000000000000000000000000000"}, data=event)
    times_partial.append(time.perf_counter() - start)

print(f"Wrong: mean={statistics.mean(times_wrong)*1000:.2f}ms, stdev={statistics.stdev(times_wrong)*1000:.2f}ms")
print(f"Partial: mean={statistics.mean(times_partial)*1000:.2f}ms, stdev={statistics.stdev(times_partial)*1000:.2f}ms")
# 预期: 两者响应时间无显著差异（常数时间比较）
PYEOF
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 所有请求返回 204 | `KEYCLOAK_WEBHOOK_SECRET` 未配置 | 在 docker-compose.yml 或 .env 中设置 |
| 404 Not Found | 端点路径错误 | 使用 `/api/v1/keycloak/events`（不是 `/api/v1/webhooks/keycloak`） |
| 正确签名仍返回 401 | 签名格式错误 | 确保格式为 `sha256=<hex>`，使用 HMAC-SHA256 |

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

### 前置条件（重要）

**必须确保 `KEYCLOAK_WEBHOOK_SECRET` 已配置**，否则签名验证不会启用。Docker 默认配置已设置为 `dev-webhook-secret-change-in-production`。

**必须确保 Redis 正常运行**，否则去重机制使用内存缓存（仅进程内有效）。

**事件 payload 必须包含 `id` 字段**，否则去重机制不会生效（`id` 是可选字段）。

### 验证方法
```bash
# 重要: 必须先定义 EVENT，再计算签名（顺序不可颠倒）
VALID_SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret}"

# 使用当前时间戳，确保事件不过期（5 分钟窗口）
CURRENT_TIME=$(date +%s)
EVENT="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"test-user\",\"time\":${CURRENT_TIME},\"id\":\"event-replay-test-123\"}"

# 计算签名（必须在 EVENT 定义之后）
VALID_SIGNATURE=$(echo -n "$EVENT" | openssl dgst -sha256 -hmac "$VALID_SECRET" | awk '{print $2}')

# 第一次发送（应成功）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$VALID_SIGNATURE" \
  -d "$EVENT"
# 预期: 204 (No Content - 事件已接受)

# 立即重放同一事件（应被 Redis 去重拒绝）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$VALID_SIGNATURE" \
  -d "$EVENT"
# 预期: 204 (幂等返回，但不执行业务逻辑 - 日志显示 "Duplicate webhook event detected")

# 发送过期事件（timestamp 很旧，超出 5 分钟窗口）
OLD_EVENT='{"type":"LOGIN","realmId":"auth9","userId":"test-user","time":1600000000,"id":"event-old"}'
OLD_SIGNATURE=$(echo -n "$OLD_EVENT" | openssl dgst -sha256 -hmac "$VALID_SECRET" | awk '{print $2}')
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -H "X-Keycloak-Signature: sha256=$OLD_SIGNATURE" \
  -d "$OLD_EVENT"
# 预期: 400 (Event timestamp too old)
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 重放请求未被去重（两次都返回 204 且都执行了业务逻辑） | event payload 缺少 `id` 字段 | 确保 JSON 中包含 `"id": "event-xxx"` 字段 |
| 签名验证未生效（所有请求都返回 204） | `KEYCLOAK_WEBHOOK_SECRET` 未配置 | 在 docker-compose.yml 中设置该环境变量 |
| 签名不匹配（返回 401） | 签名计算在 EVENT 定义之前，或 EVENT 包含额外空白 | 先定义 EVENT，再计算签名；使用 `echo -n` 避免尾部换行 |
| 过期事件未被拒绝 | payload 中的 `time` 字段在 5 分钟窗口内 | 使用明确的旧时间戳（如 `1600000000`） |

### 已实现的安全机制
- **签名验证**: HMAC-SHA256 签名 + 常数时间比较（防时间侧信道）
- **时间戳验证**: 拒绝超过 5 分钟的过期事件
- **事件去重**: Redis SETNX（TTL=1h）+ 内存缓存降级
- **幂等处理**: 重复事件返回 204 但不执行业务逻辑

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

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-ADV-05  
**适用控制**: V10.5,V13.2,V16.2  
**关联任务**: Backlog #5, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 2

### 执行清单
- [ ] M-ADV-05-C01 | 控制: V10.5 | 任务: #5, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-ADV-05-C02 | 控制: V13.2 | 任务: #5, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-ADV-05-C03 | 控制: V16.2 | 任务: #5, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
