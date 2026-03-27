# OIDC 错误处理 - 异常请求验证

- **模块**: Error Handling
- **端点**: `GET /api/v1/auth/authorize`, `POST /api/v1/auth/token`
- **最后更新**: 2026-03-27

## 前置条件

1. 执行环境重置脚本初始化测试数据：
   ```bash
   ./scripts/reset-docker.sh --conformance
   ```
2. 确认 Auth9 Core 服务运行在 `http://localhost:8080`
3. 准备测试用 OAuth Client（`{client_id}`, `{client_secret}`, `{redirect_uri}`）
4. Scopes 白名单：`openid`, `profile`, `email`
5. 允许的 response_type：`code`, `token`, `id_token`

---

## 场景 1: Authorize 请求缺少必需参数（无 client_id）返回错误

**目的**: 验证 Authorize 端点在缺少 `client_id` 参数时返回正确的错误响应。

**步骤**:

```bash
# 缺少 client_id 参数
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
redirect_uri={redirect_uri}&\
scope=openid&\
state=test-state-001&\
response_type=code"

# 查看完整响应
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
redirect_uri={redirect_uri}&\
scope=openid&\
state=test-state-001&\
response_type=code" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `invalid_request`
- 错误描述指示缺少 `client_id` 参数

---

## 场景 2: Authorize 请求使用无效 client_id 返回错误

**目的**: 验证使用不存在的 client_id 时，Authorize 端点正确拒绝请求。

**步骤**:

```bash
# 使用不存在的 client_id
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id=non-existent-client-id-12345&\
redirect_uri={redirect_uri}&\
scope=openid&\
state=test-state-002&\
response_type=code"

# 查看完整响应
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id=non-existent-client-id-12345&\
redirect_uri={redirect_uri}&\
scope=openid&\
state=test-state-002&\
response_type=code" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400` 或 `401`
- 响应包含 `error` 字段，值为 `invalid_client` 或 `unauthorized_client`
- 不会重定向到 redirect_uri

---

## 场景 3: Authorize 请求使用未注册的 redirect_uri 返回错误

**目的**: 验证 redirect_uri 不在 client 注册列表中时，请求被拒绝而非盲目重定向。

**步骤**:

```bash
# 使用未注册的 redirect_uri
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri=https://attacker.example.com/steal-code&\
scope=openid&\
state=test-state-003&\
response_type=code"

# 查看完整响应
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri=https://attacker.example.com/steal-code&\
scope=openid&\
state=test-state-003&\
response_type=code" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `invalid_request`
- 错误描述指示 redirect_uri 未注册
- **不会**重定向到攻击者提供的 URI

---

## 场景 4: Token 请求使用不支持的 grant_type 返回错误

**目的**: 验证 Token 端点在收到不支持的 grant_type 时返回正确错误。

**步骤**:

```bash
# 使用不支持的 grant_type
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=urn:custom:unsupported&\
client_id={client_id}&\
client_secret={client_secret}"

# 查看完整响应
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=urn:custom:unsupported&\
client_id={client_id}&\
client_secret={client_secret}" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `unsupported_grant_type`
- 错误描述指示该 grant_type 不受支持

---

## 场景 5: Authorize 请求 scope 不包含 openid 返回错误（或被过滤）

**目的**: 验证 OIDC Authorize 请求在 scope 中不包含必需的 `openid` 时的行为。

**步骤**:

```bash
# scope 不包含 openid
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=profile%20email&\
state=test-state-005&\
response_type=code"

# 查看完整响应
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=profile%20email&\
state=test-state-005&\
response_type=code" \
  | jq .
```

**预期结果**（以下两种行为均可接受）:
- **方案 A（拒绝）**: HTTP 状态码 `400`，`error` 为 `invalid_scope`，指示缺少 `openid` scope
- **方案 B（过滤）**: 请求正常处理，但返回的 token 中 scope 被自动补充 `openid` 或仅作为普通 OAuth2 流程处理

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Authorize 缺少 client_id 返回错误 | | | | |
| 2 | Authorize 使用无效 client_id 返回错误 | | | | |
| 3 | Authorize 使用未注册 redirect_uri 返回错误 | | | | |
| 4 | Token 使用不支持的 grant_type 返回错误 | | | | |
| 5 | Authorize scope 不含 openid 返回错误或被过滤 | | | | |
