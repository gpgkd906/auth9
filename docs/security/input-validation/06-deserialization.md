# 输入验证 - 反序列化安全测试

**模块**: 输入验证
**测试范围**: JSON 反序列化、Protobuf 畸形消息、JWT 畸形数据
**场景数**: 3
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-INPUT-06
**OWASP ASVS 5.0**: V5.5,V1.1,V2.1
**回归任务映射**: Backlog #17, #20


---

## 背景知识

Auth9 涉及的反序列化处理：
- **REST API**: JSON 反序列化（serde_json），处理所有 HTTP 请求体
- **gRPC**: Protobuf 反序列化（prost），处理 Token Exchange 等关键操作
- **JWT**: Base64 + JSON 反序列化，处理所有认证 Token
- **Redis 缓存**: 序列化/反序列化缓存数据

反序列化攻击可导致：拒绝服务（CPU/内存耗尽）、崩溃（panic）、逻辑绕过。

---

## 场景 1：JSON 反序列化攻击

### 前置条件
- REST API 端点
- 有效的认证 Token

### 攻击目标
验证 JSON 反序列化是否能处理畸形、极端或恶意输入

### 攻击步骤
1. 发送深度嵌套 JSON（1000+ 层）测试栈溢出
2. 发送超大 JSON 体（>10MB）测试内存耗尽
3. 发送包含重复 key 的 JSON 测试处理行为
4. 发送特殊 Unicode 字符（零宽字符、RTL 标记）
5. 发送 JSON 中包含超长字符串字段
6. 发送包含 `__proto__` 等原型污染 key（虽然 Rust 不受影响，但验证行为）

### 预期安全行为
- 深度嵌套 JSON 被拒绝或限制解析深度
- 超大请求体在框架层被截断
- 重复 key 不导致未定义行为
- 特殊 Unicode 字符被正确处理
- 超长字符串字段被域模型验证拒绝
- 服务不崩溃（无 panic）

### 验证方法
```bash
# 深度嵌套 JSON
python3 -c "
depth = 1000
payload = '{\"a\":' * depth + '\"deep\"' + '}' * depth
print(payload)
" | curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @- http://localhost:8080/api/v1/tenants
# 预期: 400 Bad Request (不是 500 或服务崩溃)

# 超大 JSON 体
python3 -c "
import json
payload = json.dumps({'name': 'A' * 10_000_000})
print(payload)
" | curl -s -o /dev/null -w "%{http_code}" \
  -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @- http://localhost:8080/api/v1/tenants
# 预期: 413 或 400 (不是内存耗尽)

# 重复 key
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name": "first", "slug": "test", "name": "second"}'
# 预期: 使用最后一个值或报错，但不崩溃

# 超长字段值
curl -s -o /dev/null -w "%{http_code}" \
  -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d "{\"name\": \"$(python3 -c "print('A' * 100000)")\", \"slug\": \"test\"}"
# 预期: 400 - Name exceeds maximum length

# 零宽字符
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name": "test\u200b\u200c\u200d\ufeff", "slug": "zero-width"}'
# 预期: 接受或拒绝，但不产生显示异常

# NaN / Infinity (非标准 JSON)
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name": "test", "some_number": NaN}'
# 预期: 400 - Invalid JSON

# 检查服务存活
curl -s http://localhost:8080/health
# 预期: 200 (上述所有测试后服务仍正常)
```

### 修复建议
- 配置 serde_json 最大嵌套深度（默认 128 通常足够）
- axum 层设置请求体大小限制（如 1MB）
- 域模型层验证字符串最大长度
- 不对用户输入进行 `unwrap()`，使用 `?` 或 `match` 优雅错误处理
- 全局 panic handler 防止单请求 panic 导致进程退出

---

## 场景 2：gRPC Protobuf 畸形消息

### 前置条件
- gRPC 端点访问
- Protobuf 消息构造工具

### 攻击目标
验证 gRPC 服务对畸形 Protobuf 消息的处理

### 攻击步骤
1. 发送空 Protobuf 消息（所有字段缺失）
2. 发送包含超大 repeated 字段的消息（百万元素）
3. 发送包含 unknown field 的消息
4. 发送格式错误的 Protobuf 二进制数据
5. 发送超大单字段值（如 10MB 的 string 字段）
6. 发送原始 TCP 垃圾数据到 gRPC 端口

### 预期安全行为
- 空消息返回 INVALID_ARGUMENT 错误（proto3 中空字符串字段会作为 JWT 解析，返回 `Invalid identity token: JWT error: InvalidToken`）
- 超大消息被限制（gRPC max message size）
- Unknown fields 被忽略（Protobuf 默认行为）
- 格式错误的数据返回 INTERNAL 错误
- 服务不崩溃

### 验证方法
```bash
# 空消息
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d '{}' \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: ERROR - Invalid identity token: JWT error: InvalidToken
# 注意: proto3 没有 required 字段验证。空消息中 string 字段默认为 ""（空字符串），
# 不会触发 "Missing required field" 错误。空字符串作为 JWT 传入解析器后，
# 返回的是 JWT 解析错误而非字段缺失错误。

# 超大字段值
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d "{\"identity_token\": \"$(python3 -c "print('A' * 10000000)")\"}" \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: 错误 (消息过大或 token 无效)

# 随机二进制数据
echo -n "\x00\x01\x02\x03\xff\xfe\xfd" | \
  curl --http2-prior-knowledge -X POST \
  -H "Content-Type: application/grpc" \
  --data-binary @- \
  http://localhost:50051/auth9.TokenService/ExchangeToken
# 预期: gRPC 错误，不崩溃

# 检查 gRPC max message size
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d "{\"identity_token\": \"$(python3 -c "print('B' * 5000000)")\", \"tenant_id\": \"test\"}" \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: RESOURCE_EXHAUSTED 或 INVALID_ARGUMENT

# 验证服务存活
grpcurl -plaintext localhost:50051 list
# 预期: 服务仍响应
```

### 修复建议
- 配置 tonic/gRPC 的 `max_decoding_message_size`（如 4MB）
- 配置 `max_encoding_message_size`
- 在 handler 中验证必填字段
- 设置请求超时
- 使用 `tower` 中间件限制并发和请求大小

---

## 场景 3：JWT Payload 畸形数据

### 前置条件
- 能够构造自定义 JWT Token
- API 端点接受 JWT 认证

### 攻击目标
验证 JWT 解码器对畸形 payload 的处理

### 攻击步骤
1. 构造 JWT 包含超长 claims 值
2. 构造 JWT 包含深度嵌套的 JSON claims
3. 构造 JWT 缺少标准字段（无 `sub`, 无 `exp`）
4. 构造 JWT 包含非预期类型的 claims（`sub` 为数字而非字符串）
5. 构造无效的 Base64 编码的 JWT 部分
6. 构造 JWT header 指定不存在的 `kid`

### 预期安全行为
- 超长 claims 被大小限制截断或拒绝
- 缺少标准字段返回 401
- 类型不匹配返回 401
- 无效 Base64 返回 401
- 不存在的 kid 返回 401
- 所有情况下服务不崩溃

### 验证方法
```bash
# 构造畸形 JWT 的 Python 辅助脚本
python3 << 'PYEOF'
import base64, json, hmac, hashlib

def make_jwt(header, payload, secret="test"):
    h = base64.urlsafe_b64encode(json.dumps(header).encode()).rstrip(b'=')
    p = base64.urlsafe_b64encode(json.dumps(payload).encode()).rstrip(b'=')
    sig = base64.urlsafe_b64encode(
        hmac.new(secret.encode(), h + b'.' + p, hashlib.sha256).digest()
    ).rstrip(b'=')
    return (h + b'.' + p + b'.' + sig).decode()

# 超长 sub
token1 = make_jwt(
    {"alg": "HS256", "typ": "JWT"},
    {"sub": "A" * 100000, "exp": 9999999999}
)
print(f"LONG_SUB={token1[:100]}...")

# 嵌套 claims
token2 = make_jwt(
    {"alg": "HS256", "typ": "JWT"},
    {"sub": "user", "exp": 9999999999, "nested": {"a": {"b": {"c": {"d": "deep"}}}}}
)
print(f"NESTED={token2[:100]}...")

# 缺少 exp
token3 = make_jwt(
    {"alg": "HS256", "typ": "JWT"},
    {"sub": "user"}
)
print(f"NO_EXP={token3}")

# sub 为数字
token4 = make_jwt(
    {"alg": "HS256", "typ": "JWT"},
    {"sub": 12345, "exp": 9999999999}
)
print(f"NUM_SUB={token4}")
PYEOF

# 使用生成的 Token 测试
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $LONG_SUB_TOKEN" \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# 无效 Base64
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer not.valid.base64!!!" \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# 空 token
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer " \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# 只有两个部分
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer header.payload" \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# 不存在的 kid
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $(python3 -c "
import base64, json
h = base64.urlsafe_b64encode(json.dumps({'alg':'RS256','kid':'nonexistent-kid'}).encode()).rstrip(b'=').decode()
p = base64.urlsafe_b64encode(json.dumps({'sub':'user','exp':9999999999}).encode()).rstrip(b'=').decode()
print(f'{h}.{p}.fakesig')
")" \
  http://localhost:8080/api/v1/auth/userinfo
# 预期: 401

# 验证服务存活
curl -s http://localhost:8080/health
# 预期: 200
```

### 修复建议
- JWT 解析库配置最大 token 大小
- 验证所有必需 claims（sub, exp, iss, aud）
- 严格类型检查 claims 值
- 使用 `jsonwebtoken` crate 的严格验证模式
- 无效 JWT 统一返回 401，不泄露具体失败原因

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | JSON 反序列化攻击 | ☐ | | | |
| 2 | gRPC Protobuf 畸形消息 | ☐ | | | |
| 3 | JWT Payload 畸形数据 | ☐ | | | |

---

## 参考资料

- [OWASP Deserialization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Deserialization_Cheat_Sheet.html)
- [CWE-502: Deserialization of Untrusted Data](https://cwe.mitre.org/data/definitions/502.html)
- [CWE-400: Uncontrolled Resource Consumption](https://cwe.mitre.org/data/definitions/400.html)
- [Protobuf Security](https://protobuf.dev/programming-guides/proto3/#updating)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-INPUT-06  
**适用控制**: V5.5,V1.1,V2.1  
**关联任务**: Backlog #17, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 3

### 执行清单
- [ ] M-INPUT-06-C01 | 控制: V5.5 | 任务: #17, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-06-C02 | 控制: V1.1 | 任务: #17, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-06-C03 | 控制: V2.1 | 任务: #17, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
