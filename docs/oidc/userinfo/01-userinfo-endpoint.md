# OIDC UserInfo 端点 - 用户信息查询

- **模块**: UserInfo
- **端点**: `GET /api/v1/auth/userinfo`
- **最后更新**: 2026-03-27

## 前置条件

1. 执行环境重置脚本初始化测试数据：
   ```bash
   ./scripts/reset-docker.sh --conformance
   ```
2. 确认 Auth9 Core 服务运行在 `http://localhost:8080`
3. 准备测试用 OAuth Client（`{client_id}`, `{client_secret}`, `{redirect_uri}`）
4. 使用 Token 生成辅助脚本获取 access_token：
   ```bash
   source .claude/skills/tools/gen-admin-token.sh
   ```

---

## 场景 1: 使用有效 access_token 查询 UserInfo 返回用户数据

**目的**: 验证携带有效 access_token 时，UserInfo 端点返回完整用户信息。

**步骤**:

1. 通过 Authorization Code Flow 获取 access_token（scope 包含 `openid profile email`）
2. 使用 access_token 请求 UserInfo 端点

```bash
# 请求 UserInfo
curl -s -X GET http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `200`
- 响应包含 `sub` 字段（用户唯一标识）
- 响应包含 `name` 字段（profile scope）
- 响应包含 `email` 字段（email scope）

---

## 场景 2: 请求 openid+email scope 时返回 sub 和 email 字段

**目的**: 验证 scope 限制正确生效，仅返回授权范围内的字段。

**步骤**:

1. 通过 Authorization Code Flow 获取 access_token（scope 仅包含 `openid email`，不含 `profile`）

```bash
# Step 1: 发起授权请求（scope 仅含 openid email）
curl -s -X GET "http://localhost:8080/api/v1/auth/authorize?\
client_id={client_id}&\
redirect_uri={redirect_uri}&\
scope=openid%20email&\
state=test-state-123&\
response_type=code"

# Step 2: 使用获取的 code 换取 token
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
code=${AUTH_CODE}&\
client_id={client_id}&\
client_secret={client_secret}&\
redirect_uri={redirect_uri}"

# Step 3: 使用 access_token 请求 UserInfo
curl -s -X GET http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `200`
- 响应包含 `sub` 字段
- 响应包含 `email` 字段
- 响应**不**包含 `name` 字段（未请求 profile scope）

---

## 场景 3: 不携带 token 访问返回 401

**目的**: 验证未认证请求被正确拒绝。

**步骤**:

```bash
# 不携带 Authorization header 请求 UserInfo
curl -s -o /dev/null -w "%{http_code}" \
  -X GET http://localhost:8080/api/v1/auth/userinfo

# 查看完整响应
curl -s -X GET http://localhost:8080/api/v1/auth/userinfo | jq .
```

**预期结果**:
- HTTP 状态码 `401`
- 响应包含错误信息，指示缺少认证凭据

---

## 场景 4: 使用过期/无效 token 访问返回 401

**目的**: 验证无效或过期的 token 被正确拒绝。

**步骤**:

```bash
# 使用伪造的 token 请求
curl -s -o /dev/null -w "%{http_code}" \
  -X GET http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer invalid-token-value-12345"

# 使用格式错误的 token 请求
curl -s -X GET http://localhost:8080/api/v1/auth/userinfo \
  -H "Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.expired.signature" \
  | jq .
```

**预期结果**:
- HTTP 状态码 `401`
- 响应包含错误信息，指示 token 无效或已过期

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 使用有效 access_token 查询 UserInfo 返回用户数据 | | | | |
| 2 | 请求 openid+email scope 时返回 sub 和 email 字段 | | | | |
| 3 | 不携带 token 访问返回 401 | | | | |
| 4 | 使用过期/无效 token 访问返回 401 | | | | |
