# 日志与监控 - 日志安全测试

**模块**: 错误处理与日志安全
**测试范围**: 日志注入、审计完整性、检测系统有效性
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-LOG-01
**OWASP ASVS 5.0**: V16.1,V16.2,V16.3,V16.4
**回归任务映射**: Backlog #5, #12, #19, #20


---

## 背景知识

Auth9 的日志与监控体系：
- **应用日志**: Rust `tracing` 框架，结构化日志输出
- **审计日志**: `audit_logs` 表，记录所有安全敏感操作
- **安全检测**: `SecurityDetectionService` - 暴力破解、密码喷洒、不可能旅行检测
- **安全告警**: `security_alerts` 表，按严重级别分类

日志安全问题可导致：攻击痕迹消除、日志伪造误导调查、敏感数据泄露。

---

## 场景 1：日志注入攻击

### 前置条件
- 能够在 API 请求中传递用户可控数据
- 访问应用日志输出

### 攻击目标
验证用户输入是否可注入恶意日志条目，伪造日志记录

### 攻击步骤
1. 在用户名字段中注入换行符和伪造的日志条目
2. 在搜索参数中注入日志格式化字符串
3. 在 HTTP 头（User-Agent, X-Forwarded-For）中注入
4. 检查应用日志是否包含注入的内容
5. 尝试注入 ANSI 颜色代码混淆日志输出

### 预期安全行为
- 日志框架对用户输入自动转义换行符
- 结构化日志（JSON 格式）中用户数据作为值而非结构
- CRLF 注入不会产生新的日志行
- 日志查看工具不会执行注入的格式化字符串

### 验证方法
```bash
# CRLF 注入 - 尝试伪造日志条目
# 使用忘记密码入口（用户可控 email 字段）
curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"admin%0a[INFO] Login successful for admin from 127.0.0.1"}'
# 检查日志: 注入内容应在同一日志字段内，不产生新行

# HTTP Header 注入
curl -H "User-Agent: Mozilla/5.0\r\n[WARN] Suspicious activity detected" \
  http://localhost:8080/health
# 检查日志: User-Agent 应被完整记录为单个字段值

# ANSI 转义序列注入
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?search=%1B%5B31m%5BCRITICAL%5D%20System%20compromised%1B%5B0m"
# 检查日志: ANSI 代码应被转义，不影响显示

# 检查日志格式
docker logs auth9-core 2>&1 | tail -20
# 验证: 日志使用结构化格式 (JSON)，用户输入在引号内
```

### 修复建议
- 使用结构化日志格式（JSON），用户数据作为字段值
- 对日志中的用户输入进行转义（至少转义 `\n`, `\r`）
- 日志查看工具过滤 ANSI 转义序列
- 生产环境日志发送到集中式日志系统（ELK, Loki）

---

## 场景 2：审计日志完整性验证

### 前置条件
- **Tenant Access Token**（非 Identity Token）。审计日志端点位于受保护路由，需要通过 Token Exchange 获取的 Tenant Access Token。使用 Identity Token 会返回 401。
- 了解审计日志 API

### 攻击目标
验证审计日志是否不可篡改，且覆盖所有安全敏感操作

### 攻击步骤
1. 执行多个安全敏感操作（创建用户、分配角色、更改密码等）
2. 查询审计日志确认每个操作都被记录
3. 尝试通过 API 删除或修改审计日志
4. 尝试通过 SQL 注入修改审计记录
5. 验证审计日志包含足够的上下文信息

### 预期安全行为
- 无 API 端点可删除或修改审计日志
- 审计日志记录：操作类型、操作者、资源、时间戳、IP、新旧值
- 所有安全敏感操作都有对应审计记录
- 审计日志不可通过应用层接口篡改

### 验证方法
```bash
# 执行安全敏感操作
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users \
  -d '{"email": "audit-test@test.com", "name": "Audit Test"}'

# 检查审计日志
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/audit-logs?resource_type=user&limit=5"
# 预期: 包含 user.create 记录，含 actor_id, resource_id, ip_address
# 注意: ip_address 通过以下优先级获取:
#   1. X-Forwarded-For 头（反向代理场景）
#   2. X-Real-IP 头（反向代理场景）
#   3. TCP 连接的 socket 地址（直连场景，由 inject_client_ip 中间件自动注入）

# 尝试删除审计日志（不应存在此端点）
curl -X DELETE -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/audit-logs/some-audit-id
# 预期: 404 或 405 Method Not Allowed

# 尝试修改审计日志
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/audit-logs/some-audit-id \
  -d '{"action": "modified"}'
# 预期: 404 或 405

# 验证覆盖完整性 - 以下操作都应有审计记录
OPERATIONS=(
  "user.create" "user.update" "user.delete"
  "role.create" "role.update" "role.delete"
  "role.assign" "role.unassign"
  "tenant.create" "tenant.update" "tenant.delete"
  "service.create" "service.update" "service.delete"
  "password.change" "password.reset"
  "system.email.update" "system.branding.update"
  "invitation.create" "invitation.accept"
)
for op in "${OPERATIONS[@]}"; do
  echo -n "$op: "
  curl -s -H "Authorization: Bearer $TOKEN" \
    "http://localhost:8080/api/v1/audit-logs?action=$op&limit=1" | jq '.total'
done
```

### 修复建议
- 审计日志表不提供 DELETE/UPDATE API
- 数据库级别可使用只追加表（如果 TiDB 支持）
- 关键审计记录同步到外部不可变存储
- 定期审查审计日志覆盖完整性

---

## 场景 3：敏感数据日志泄露

### 前置条件
- 访问应用日志输出
- 能够触发各种 API 请求

### 攻击目标
验证日志中是否意外记录了敏感信息

### 攻击步骤
1. 执行密码相关操作，检查日志中是否出现密码明文
2. 执行 Token 操作，检查日志中是否出现完整 JWT
3. 触发错误，检查错误日志中的敏感信息
4. 检查 HTTP 请求日志中是否记录了 Authorization 头
5. 检查内部服务通信日志中是否泄露 client_secret

### 预期安全行为
- 密码、Token、API Key 不出现在日志中
- Authorization 头内容被脱敏（如 `Bearer ***`）
- 错误日志不包含数据库连接字符串
- PII 数据（邮箱、电话）根据策略脱敏
- 内部签名密钥不出现在日志中

### 验证方法
```bash
# 触发密码操作
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com"}'

curl -X POST http://localhost:8080/api/v1/password/change \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"current_password": "OldPass123!", "new_password": "NewPass456!"}'

# 检查日志中的敏感信息
docker logs auth9-core 2>&1 | grep -i "password\|secret\|OldPass\|NewPass"
# 预期: 无明文密码

docker logs auth9-core 2>&1 | grep -i "eyJ"
# 预期: 无完整 JWT token (eyJ 是 base64 编码的 JWT 头部前缀)

docker logs auth9-core 2>&1 | grep -i "database_url\|redis_url\|connection"
# 预期: 连接字符串已脱敏或不出现

# 触发错误路径
curl -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d '{"grant_type":"client_credentials","client_id":"invalid-client","client_secret":"invalid-secret"}'
docker logs auth9-core 2>&1 | tail -5
# 预期: 错误日志不包含密码值

# 检查配置输出
docker logs auth9-core 2>&1 | grep -i "REDACTED\|<REDACTED>"
# 预期: 敏感配置值显示为 <REDACTED>
```

### 修复建议
- 使用 tracing 的 `skip` 或 `#[instrument(skip(password))]` 跳过敏感字段
- 实现日志中间件自动脱敏 Authorization 头
- 配置结构体的 Debug trait 实现中脱敏敏感字段（已实现）
- 定期扫描日志文件检测敏感数据泄露

---

## 场景 4：安全告警系统有效性

### 前置条件
- 了解 `SecurityDetectionService` 的检测阈值
- 能够模拟各类攻击模式
- **auth9-core 已启动**。登录事件通过内部事件系统推送到安全检测服务，auth9-core 接收后触发安全检测。确认 webhook secret 已配置。

### 攻击目标
验证安全检测与告警系统是否正确识别攻击行为

### 攻击步骤
1. **暴力破解检测**: 对同一账户连续 5 次错误登录，检查是否生成 HIGH 告警
2. **密码喷洒检测**: 从同一 IP 对 5+ 不同账户尝试登录，检查是否生成 CRITICAL 告警
3. **新设备检测**: 使用不同 User-Agent 登录，检查是否生成 Medium 告警
4. **检测规避**: 使用低速攻击（每 3 分钟 1 次），验证是否绕过检测
5. **检查告警列表是否正确展示**

### 预期安全行为
- 暴力破解: 5 次失败 / 10 分钟 → HIGH 告警
- 密码喷洒: 5+ 不同账户 / 同一 IP / 10 分钟 → CRITICAL 告警
- 新设备: 新 IP+UA 组合 → Medium 告警（`AlertSeverity` 枚举值为 Low/Medium/High/Critical，无 Info 级别）
- 告警可通过 API 查询
- 检测不影响正常用户体验（低误报率）

### 验证方法
```bash
# 说明：Auth9 不支持 /api/v1/auth/token + grant_type=password。
# 当前默认链路使用 Webhook 注入登录事件，验证检测逻辑与告警产出。
# 以下使用 Webhook 端点直接注入事件进行测试。

SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
send_webhook_event() {
  local body="$1"
  local sig=$(echo -n "$body" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)
  curl -s -X POST http://localhost:8080/api/v1/identity/events \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=$sig" \
    -d "$body" >/dev/null
  sleep 1
}

# 暴力破解检测测试
for i in $(seq 1 6); do
  send_webhook_event "{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440000\",\"ipAddress\":\"192.168.1.10\",\"error\":\"invalid_user_credentials\",\"time\":$(date +%s)000,\"details\":{\"username\":\"test@test.com\",\"email\":\"test@test.com\"}}"
  echo " - attempt $i"
  sleep 1
done

# 检查是否生成告警
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security/alerts?type=brute_force&limit=5"
# 预期: 至少 1 条 HIGH 级别告警

# 密码喷洒检测测试
# 重要: 每个事件必须使用不同的 userId（不仅仅是不同的 email）。
# 检测逻辑使用 COALESCE(user_id, email) 计算 distinct 用户数量。
# 如果所有事件共用同一个 userId，即使 email 不同，distinct count 仍为 1，不会触发阈值。
SPRAY_USERS=("550e8400-0001-0001-0001-000000000001" "550e8400-0001-0001-0001-000000000002" "550e8400-0001-0001-0001-000000000003" "550e8400-0001-0001-0001-000000000004" "550e8400-0001-0001-0001-000000000005" "550e8400-0001-0001-0001-000000000006")
SPRAY_EMAILS=(user1@test.com user2@test.com user3@test.com user4@test.com user5@test.com user6@test.com)
for i in $(seq 0 5); do
  send_webhook_event "{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"${SPRAY_USERS[$i]}\",\"ipAddress\":\"203.0.113.20\",\"error\":\"invalid_user_credentials\",\"time\":$(date +%s)000,\"details\":{\"username\":\"${SPRAY_EMAILS[$i]}\",\"email\":\"${SPRAY_EMAILS[$i]}\"}}"
  echo " - ${SPRAY_EMAILS[$i]}"
  sleep 0.5
done

# 检查密码喷洒告警
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security/alerts?type=password_spray&limit=5"
# 预期: CRITICAL 级别告警 (alert_type: "password_spray")

# 新设备检测测试
# 重要: 新设备检测要求该用户已有历史成功登录记录（已知设备基线）。
# 如果环境刚重置或该用户从未登录过，所有登录都是"首次"，不会产生"新设备"告警。
# 先注入至少 1 条成功登录事件建立基线，再用不同 IP/UA 触发新设备检测。
send_webhook_event "{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440000\",\"ipAddress\":\"192.168.1.100\",\"time\":$(date +%s)000,\"details\":{\"username\":\"test@test.com\",\"email\":\"test@test.com\",\"user_agent\":\"KnownDevice/1.0 (Linux)\"}}"
echo " - baseline login (known device)"
sleep 2
# 然后用新 IP + 新 UA 登录触发告警
send_webhook_event "{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440000\",\"ipAddress\":\"198.51.100.88\",\"time\":$(date +%s)000,\"details\":{\"username\":\"test@test.com\",\"email\":\"test@test.com\",\"user_agent\":\"NewDevice/1.0 (Unknown OS)\"}}"

curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://localhost:8080/api/v1/security/alerts?type=new_device&limit=5"
# 预期: Medium 级别告警（AlertSeverity 无 INFO，实际为 Medium）

# 检测规避测试 - 低速攻击
for i in $(seq 1 10); do
  send_webhook_event "{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440000\",\"ipAddress\":\"192.168.1.10\",\"error\":\"invalid_user_credentials\",\"time\":$(date +%s)000,\"details\":{\"username\":\"test@test.com\",\"email\":\"test@test.com\"}}"
  sleep 180  # 每 3 分钟一次
done
# 检查是否仍然触发告警（根据滑动窗口设计）
```

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| 查询告警返回 `{"error":"not_found"}` | 使用了错误的 API 路径 `/api/v1/security-alerts`（带连字符） | 正确路径为 `/api/v1/security/alerts`（带斜杠） |
| Webhook 事件发送返回 404 | 使用了旧的端点路径 `/api/v1/keycloak/events` | 正确路径为 `/api/v1/identity/events`（仍接受 `x-keycloak-signature` 头） |
| 告警查询返回空列表 | 未等待足够时间让事件落库，或环境刚重置无历史事件 | 确保 `sleep 1` 在每次事件发送后执行，且发送满 5+ 次再查询 |
| 401 Unauthorized | 查询告警 API 需要 Platform Admin Token | 使用 `gen-test-tokens.js identity`（平台管理员 Identity Token）或具有 PlatformAdmin 权限的 token |

### 修复建议
- 支持可配置的检测阈值
- 实现滑动窗口而非固定窗口（防止边界绕过）
- 低速攻击检测需要更大的时间窗口（如 24 小时聚合分析）
- 告警触发后的自动响应（如临时封禁 IP）

---

## 场景 5：错误处理信息泄露

### 前置条件
- 能够触发各种错误条件

### 攻击目标
验证错误响应是否泄露内部实现细节

### 攻击步骤
1. 发送畸形请求触发 400 错误
2. 访问不存在的端点触发 404 错误
3. 发送导致服务器错误的请求 (500)
4. 检查错误响应中的信息
5. 比较不同错误条件下的响应格式一致性

### 预期安全行为
- 错误响应不包含堆栈跟踪 (stack trace)
- 错误响应不暴露内部文件路径
- 错误响应不暴露数据库查询或连接信息
- 错误响应不暴露内部服务信息
- 所有错误使用统一格式

### 验证方法

> **统一错误格式说明**：
> Auth9 所有 HTTP 错误响应（4xx/5xx）均使用统一格式：
> ```json
> {"error": "<error_type>", "message": "<human_readable_message>"}
> ```
> 其中 `error` 是小写蛇形命名的错误类型（如 `"unauthorized"`、`"not_found"`、`"bad_request"`），
> `message` 是人类可读的描述信息。SCIM 端点例外，使用 RFC 7644 格式。

```bash
# 畸形 JSON
curl -s -X POST http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"invalid json'
# 预期: {"error": "bad_request", "message": "Invalid request body"} (无内部细节)

# 不存在的端点
curl -s http://localhost:8080/api/v1/nonexistent
# 预期: {"error": "not_found", "message": "Not found"} (无路由泄露)

# 未授权请求
curl -s http://localhost:8080/api/v1/tenants
# 预期: {"error": "unauthorized", "message": "..."} (无内部细节)

# 超大请求体
python3 -c "print('A' * 10_000_000)" | curl -s -X POST \
  -H "Content-Type: application/json" \
  -d @- http://localhost:8080/api/v1/tenants
# 预期: 413 Payload Too Large (无崩溃信息)

# 无效 UUID
curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/tenants/not-a-uuid
# 预期: {"error": "bad_request", "message": "..."} 或 {"error": "not_found", "message": "..."} (无 SQL 错误)

# 检查所有错误响应格式一致性 - 所有响应应包含 "error" 和 "message" 字段
for code in 400 401 403 404 409 422 429 500; do
  echo "=== HTTP $code ==="
  # 触发各状态码并检查响应格式
done
```

### 修复建议
- 统一错误响应格式：`{"error": "<type>", "message": "<description>"}`
- 生产环境禁用详细错误信息
- 500 错误仅返回通用消息，详细信息记录到日志
- 实现全局错误处理中间件（已实现：`normalize_error_response`）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 日志注入攻击 | ☐ | | | |
| 2 | 审计日志完整性验证 | ☐ | | | |
| 3 | 敏感数据日志泄露 | ☐ | | | |
| 4 | 安全告警系统有效性 | ☐ | | | |
| 5 | 错误处理信息泄露 | ☐ | | | |

---

## 参考资料

- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
- [CWE-117: Improper Output Neutralization for Logs](https://cwe.mitre.org/data/definitions/117.html)
- [CWE-532: Insertion of Sensitive Information into Log File](https://cwe.mitre.org/data/definitions/532.html)
- [CWE-209: Generation of Error Message Containing Sensitive Information](https://cwe.mitre.org/data/definitions/209.html)
- [OWASP Error Handling](https://owasp.org/www-community/Improper_Error_Handling)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-LOG-01  
**适用控制**: V16.1,V16.2,V16.3,V16.4  
**关联任务**: Backlog #5, #12, #19, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-LOG-01-C01 | 控制: V16.1 | 任务: #5, #12, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-LOG-01-C02 | 控制: V16.2 | 任务: #5, #12, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-LOG-01-C03 | 控制: V16.3 | 任务: #5, #12, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-LOG-01-C04 | 控制: V16.4 | 任务: #5, #12, #19, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
