# OIDC 安全参数 - State、Nonce、Redirect URI 验证

- **模块**: Security
- **端点**: `GET /api/v1/auth/authorize`, `POST /api/v1/auth/token`
- **最后更新**: 2026-03-27

## 前置条件

1. 执行环境重置脚本初始化测试数据：
   ```bash
   ./scripts/reset-docker.sh --conformance
   ```
2. 确认 Auth9 Core 服务运行在 `http://localhost:8080`
3. 准备测试用 OAuth Client（`{client_id}`, `{client_secret}`, `{redirect_uri}`）
4. State 参数为必需参数，不能为空
5. Authorization Code TTL 为 2 分钟

---

## 场景 1: 空 state 参数的 Authorize 请求被拒绝

**目的**: 验证 Authorize 端点强制要求非空的 state 参数，防止 CSRF 攻击。

**步骤**:

```bash
# state 参数为空字符串
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=openid&\
state=&\
response_type=code"

# 完全不携带 state 参数
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=openid&\
response_type=code"

# 查看完整响应
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=openid&\
state=&\
response_type=code" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `invalid_request`
- 错误描述指示 state 参数为必需且不能为空

---

## 场景 2: State 参数在 redirect 中原样透传

> **需浏览器**: 此场景需要浏览器环境完成完整的授权流程以观察 redirect 中的 state 值。

**目的**: 验证 state 参数在授权完成后的重定向中被原样返回，客户端可用于 CSRF 校验。

**步骤**:

1. 在浏览器中发起 Authorize 请求，携带特定的 state 值
2. 完成用户认证
3. 观察重定向 URL 中的 state 参数

```bash
# 发起授权请求（在浏览器中访问此 URL）
# http://localhost:8080/api/v1/auth/authorize?\
# client_id={client_id}&\
# redirect_uri={redirect_uri}&\
# scope=openid&\
# state=my-unique-csrf-token-abc123&\
# response_type=code

# 验证 redirect URL 中的 state（在 redirect 回调中检查）
# 预期 redirect URL 格式:
# {redirect_uri}?code=XXXX&state=my-unique-csrf-token-abc123
```

**预期结果**:
- 重定向 URL 中包含 `state=my-unique-csrf-token-abc123`
- State 值与请求时完全一致，未被修改或编码
- 同时包含 `code` 参数

---

## 场景 3: Redirect URI 必须与注册值精确匹配

**目的**: 验证 redirect_uri 必须与 client 注册时的值完全匹配，包括路径、查询参数和尾部斜杠。

**步骤**:

```bash
# 假设注册的 redirect_uri 为 {redirect_uri}

# 测试 1: 添加额外路径
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}/extra-path&\
scope=openid&\
state=test-redirect-001&\
response_type=code"

# 测试 2: 添加查询参数
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}?extra=param&\
scope=openid&\
state=test-redirect-002&\
response_type=code"

# 测试 3: 修改大小写
curl -s -o /dev/null -w "%{http_code}" \
  -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri=HTTP://LOCALHOST:3000/CALLBACK&\
scope=openid&\
state=test-redirect-003&\
response_type=code"
```

**预期结果**:
- 所有变体请求均返回 HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `invalid_request`
- 仅完全匹配注册值的 redirect_uri 被接受

---

## 场景 4: Authorization Code 只能使用一次（Replay Protection）

> **需浏览器**: 此场景需要浏览器环境完成授权流程获取 authorization code。

**目的**: 验证 authorization code 在首次兑换 token 后即失效，防止重放攻击。

**步骤**:

1. 在浏览器中完成授权流程，获取 authorization code
2. 第一次使用 code 兑换 token（应成功）
3. 第二次使用相同 code 兑换 token（应失败）

```bash
# 第一次兑换（应成功）
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}" \
  | jq .

# 第二次使用相同 code 兑换（应失败）
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}"

# 查看第二次的完整响应
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}" \
  | jq .
```

**预期结果**:
- 第一次兑换：HTTP 状态码 `200`，返回 access_token 和 id_token
- 第二次兑换：HTTP 状态码 `400`，`error` 为 `invalid_grant`
- 错误描述指示 authorization code 已被使用或无效

---

## 场景 5: Authorization Code 超过 TTL（2分钟）后失效

> **需浏览器**: 此场景需要浏览器环境完成授权流程获取 authorization code。

**目的**: 验证 authorization code 在超过 2 分钟 TTL 后自动失效。

**步骤**:

1. 在浏览器中完成授权流程，获取 authorization code
2. 等待超过 2 分钟（120 秒）
3. 尝试使用过期的 code 兑换 token

```bash
# 获取 authorization code 后等待 2 分钟以上
sleep 125

# 使用过期的 code 兑换 token
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${EXPIRED_AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}"

# 查看完整响应
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${EXPIRED_AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `400`
- 响应包含 `error` 字段，值为 `invalid_grant`
- 错误描述指示 authorization code 已过期

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 空 state 参数的 Authorize 请求被拒绝 | | | | |
| 2 | State 参数在 redirect 中原样透传 | | | | 需浏览器 |
| 3 | Redirect URI 必须与注册值精确匹配 | | | | |
| 4 | Authorization Code 只能使用一次 | | | | 需浏览器 |
| 5 | Authorization Code 超过 TTL 后失效 | | | | 需浏览器 |
