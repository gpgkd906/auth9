# OIDC Refresh Token Flow - Token 刷新

| 项目 | 值 |
|------|-----|
| 模块 | Refresh Token |
| 场景数 | 4 |
| 最后更新 | 2026-03-27 |

## 前置条件

1. 执行 `./scripts/reset-docker.sh --conformance` 重置环境至一致性测试状态
2. 确认 Auth9 Core 服务运行于 `http://localhost:8080`
3. 确认测试用 OAuth Client 已注册，持有有效的 `{client_id}` 和 `{client_secret}`
4. 确认 Client 的 `grant_types` 包含 `refresh_token`
5. 已通过 Authorization Code Flow 获取初始 refresh_token（场景 1 需浏览器交互）

---

## 场景 1：使用有效 refresh_token 获取新 access_token 和 id_token

> **注意**：此场景需先通过浏览器完成 Authorization Code Flow 获取初始 refresh_token。

**目的**：验证使用有效的 refresh_token 可以获取新的 access_token 和 id_token。

**步骤**：

1. （前置 - 需浏览器）完成 Authorization Code Flow，获取包含 refresh_token 的 token 响应
2. 使用 refresh_token 请求新 token：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token={refresh_token}&client_id={client_id}"
```

**预期结果**：

- HTTP 状态码 `200`
- 响应 JSON 包含新的 `access_token`
- 响应 JSON 包含新的 `id_token`
- `token_type` 值为 `Bearer`
- `expires_in` 大于 0

---

## 场景 2：Token 刷新返回新的 refresh_token（Token Rotation）

**目的**：验证 refresh 操作实施 Token Rotation，每次刷新签发新的 refresh_token 并废弃旧 token。

**步骤**：

1. 使用有效 refresh_token（记为 RT1）请求刷新：

```bash
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token={RT1}&client_id={client_id}")

RT2=$(echo "$RESPONSE" | jq -r '.refresh_token')
```

2. 验证返回了新的 refresh_token（RT2 != RT1）
3. 使用旧的 RT1 再次请求刷新（验证 replay protection）：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token={RT1}&client_id={client_id}"
```

**预期结果**：

- 第 1 步返回新的 `refresh_token`（RT2），与 RT1 不同
- RT2 是有效的 JWT，包含新的 `jti` claim
- 第 3 步使用旧 RT1 刷新失败（jti 已加入 blacklist），返回错误
- replay protection 生效：旧 token 的 jti 被列入黑名单

---

## 场景 3：使用无效/过期 refresh_token 返回错误

**目的**：验证无效或过期的 refresh_token 被正确拒绝。

**步骤**：

1. 使用伪造的 refresh_token：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token=invalid.jwt.token&client_id={client_id}"
```

2. 使用格式正确但签名无效的 JWT 作为 refresh_token：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token=eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJmYWtlIn0.invalidsig&client_id={client_id}"
```

**预期结果**：

- HTTP 状态码 `400` 或 `401`
- 响应 JSON 包含 `error` 字段，值为 `invalid_grant`
- 不返回任何 token

---

## 场景 4：使用不匹配的 client_id 刷新 token 返回错误

**目的**：验证 refresh_token 与 client_id 绑定，使用其他 client 的 ID 无法刷新。

**步骤**：

1. 使用有效的 refresh_token 但传入不同的 client_id：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token={refresh_token}&client_id=other_client_id"
```

2. 完全省略 client_id 参数：

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token={refresh_token}"
```

**预期结果**：

- 步骤 1：HTTP 状态码 `400` 或 `401`，`error` 为 `invalid_grant` 或 `invalid_client`
- 步骤 2：HTTP 状态码 `400`，`error` 为 `invalid_request`（缺少必需参数）
- 原 refresh_token 不受影响，后续仍可使用正确 client_id 刷新

---

## 检查清单

| # | 检查项 | 场景 | 预期 | 通过 |
|---|--------|------|------|------|
| 1 | 有效 refresh_token 获取新 access_token | 1 | HTTP 200，返回新 access_token | [ ] |
| 2 | 有效 refresh_token 获取新 id_token | 1 | HTTP 200，返回新 id_token | [ ] |
| 3 | Token Rotation 签发新 refresh_token | 2 | 新 RT 与旧 RT 不同 | [ ] |
| 4 | 新 refresh_token 包含新 jti | 2 | jti claim 不同 | [ ] |
| 5 | 旧 refresh_token 被废弃（replay protection） | 2 | 旧 RT 刷新失败 | [ ] |
| 6 | 伪造 refresh_token 被拒绝 | 3 | error=invalid_grant | [ ] |
| 7 | 签名无效的 JWT 被拒绝 | 3 | error=invalid_grant | [ ] |
| 8 | 不匹配 client_id 被拒绝 | 4 | error=invalid_grant 或 invalid_client | [ ] |
| 9 | 缺少 client_id 被拒绝 | 4 | error=invalid_request | [ ] |
