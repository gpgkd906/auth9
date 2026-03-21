# 认证流程 - PKCE (RFC 7636) 安全增强

**模块**: 认证流程
**测试范围**: OIDC 授权码流程的 PKCE 参数透传与验证
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 的 OIDC 授权码流程已实现 PKCE（Proof Key for Code Exchange, RFC 7636）。PKCE 通过在授权请求中发送 `code_challenge`，在 token 请求中发送对应的 `code_verifier`，防止 Authorization Code 拦截攻击。

**架构要点**：
- Auth9 内置 OIDC 引擎负责 PKCE 验证（校验 `SHA256(code_verifier) == code_challenge`）
- Portal 负责生成 `code_verifier`/`code_challenge`，将 `code_verifier` 存入 `oauth_state` cookie

**涉及端点**：
- `GET /api/v1/auth/authorize` — 接受可选的 `code_challenge` + `code_challenge_method` 参数
- `POST /api/v1/auth/token` — 接受可选的 `code_verifier` 参数
- `POST /api/v1/enterprise-sso/discovery` — 同样透传 PKCE 参数

**涉及文件**：
- `auth9-core/src/domains/identity/api/auth/types.rs` — `AuthorizeRequest.code_challenge/code_challenge_method`、`TokenRequest.code_verifier`
- `auth9-core/src/domains/identity/api/auth/helpers.rs` — 构建授权 URL 时附加 PKCE 查询参数
- `auth9-core/src/domains/identity/api/auth/` — `exchange_code_for_tokens()` 发送 `code_verifier`
- `auth9-portal/app/routes/login.tsx` — `generatePkce()` 生成 PKCE 参数
- `auth9-portal/app/services/session.server.ts` — `OAuthStateData` 存储 `codeVerifier`
- `auth9-portal/app/routes/auth.callback.tsx` — token 请求中发送 `code_verifier`

---

## 场景 1：Portal 密码登录 — PKCE 参数透传验证

### 初始状态
- 用户未登录
- 所有服务健康（auth9-core、Redis）

### 目的
验证通过 Portal「Sign in with password」登录时，授权请求包含 PKCE 参数，token 交换包含 `code_verifier`

### 测试操作流程
1. 打开浏览器开发者工具 → Network 面板
2. 访问 `http://localhost:3000/login`
3. 点击「**Sign in with password**」
4. 观察 302 重定向到 `/api/v1/auth/authorize` 的 URL 参数
5. 继续完成登录流程
6. 观察 `/api/v1/auth/token` 的 POST 请求体

### 预期结果
- 步骤 4：授权 URL 包含以下参数：
  - `code_challenge=<base64url-encoded-value>`（43 字符的 Base64URL 字符串）
  - `code_challenge_method=S256`
- 步骤 4：OIDC 授权 URL 中包含 `code_challenge` 和 `code_challenge_method` 参数
- 步骤 6：Token 请求 JSON body 包含 `code_verifier` 字段（43 字符 Base64URL 字符串）
- 登录成功，正常进入 `/tenant/select` 或 `/dashboard`

### 验证方法
```bash
# 抓取授权重定向 URL，验证 PKCE 参数存在
curl -v "http://localhost:3000/login" \
  -d "intent=password-login" \
  -H "Cookie: auth9_locale=en-US" 2>&1 | grep -i "location:"
# 预期: Location URL 包含 code_challenge= 和 code_challenge_method=S256
```

---

## 场景 2：PKCE Cookie 存储与生命周期

### 初始状态
- 用户未登录
- 浏览器无 `oauth_state` cookie

### 目的
验证 `code_verifier` 正确存入 `oauth_state` cookie，并在 token 交换后清除

### 测试操作流程
1. 打开浏览器开发者工具 → Application → Cookies
2. 访问 `/login`，点击「**Sign in with password**」
3. 观察 `Set-Cookie: oauth_state=...` 的响应
4. 完成登录流程
5. 观察回调后的 Cookie 状态

### 预期结果
- 步骤 3：`oauth_state` cookie 被设置（加密值，`HttpOnly`、`SameSite=Lax`、`Max-Age=300`）
- 步骤 3：cookie 内部存储了 `{ state, codeVerifier }` 结构（加密后不可直接读取）
- 步骤 5：回调成功后 `oauth_state` cookie 被清除（`Max-Age=0`）

---

## 场景 3：auth9-core Authorize 端点 PKCE 参数透传

### 初始状态
- 有效的 service client（如 `auth9-portal`）已注册
- Auth9 服务健康

### 目的
验证 `/api/v1/auth/authorize` 端点正确处理 PKCE 参数

### 测试操作流程

#### 步骤 0: 验证环境状态
```bash
# 确认 auth9-core 健康
curl -sf http://localhost:8080/health
# 预期: HTTP 200
```

1. 发送带 PKCE 参数的授权请求：
```bash
curl -v "http://localhost:8080/api/v1/auth/authorize?\
response_type=code&\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/auth/callback&\
scope=openid+email+profile&\
state=test-pkce-state&\
code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&\
code_challenge_method=S256" 2>&1 | grep -i "location:"
```
2. 解析返回的 302 Location 头中的授权 URL

### 预期结果
- 返回 302 重定向到 Auth9 OIDC 授权端点
- Location URL 包含 `code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM`
- Location URL 包含 `code_challenge_method=S256`
- 其他标准参数（`response_type`、`client_id`、`scope`、`state`）正常处理

---

## 场景 4：无 PKCE 参数的向后兼容

### 初始状态
- auth9-core 服务健康
- `auth9-portal` client 已注册

### 目的
验证不带 PKCE 参数的授权请求仍然正常工作（向后兼容）

### 测试操作流程
1. 发送不带 PKCE 参数的授权请求：
```bash
curl -v "http://localhost:8080/api/v1/auth/authorize?\
response_type=code&\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/auth/callback&\
scope=openid+email+profile&\
state=test-no-pkce" 2>&1 | grep -i "location:"
```
2. 解析返回的 302 Location 头

### 预期结果
- 返回 302 重定向到 Auth9 OIDC 授权端点
- Location URL **不包含** `code_challenge` 或 `code_challenge_method` 参数
- 其他标准 OIDC 参数正常处理
- 登录流程正常完成（confidential client 不强制 PKCE）

---

## 场景 5：Demo Client (Public) PKCE 强制验证

### 初始状态
- auth9-demo client 已注册且为 public client（`public_client: true`）
- auth9-demo 的 `pkce.code.challenge.method` 已设置为 `S256`

### 目的
验证 public client（auth9-demo）被 Auth9 OIDC 引擎强制要求 PKCE

### 测试操作流程

#### 步骤 0: 验证客户端 PKCE 配置
```bash
# 通过 Auth9 管理 API 查询 auth9-demo 客户端配置
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s "http://localhost:8080/api/v1/services?client_id=auth9-demo" \
  -H "Authorization: Bearer $TOKEN" | jq '.data[0]'
# 预期: public_client = true, pkce 配置为 S256
# 若配置不正确，需执行 ./scripts/reset-docker.sh 重新 seed
```

1. 不带 PKCE 参数请求 demo client 授权（模拟攻击者）：
```bash
curl -v "http://localhost:8080/api/v1/auth/authorize?\
response_type=code&\
client_id=auth9-demo&\
redirect_uri=http://localhost:8080/api/v1/auth/callback&\
scope=openid+email+profile&\
state=no-pkce-demo" 2>&1 | grep -i "location:"
```
2. 完成登录后，观察 token exchange 是否被拒绝

### 预期结果
- Auth9 OIDC 引擎对未提供 `code_challenge` 的 public client 请求返回错误
- 或在 token exchange 阶段因缺少 `code_verifier` 而拒绝：返回 `400 Bad Request`，错误信息包含 PKCE 相关描述
- 这证明 public client 的 PKCE 强制配置生效

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Portal 密码登录 PKCE 参数透传 | ☐ | | | |
| 2 | PKCE Cookie 存储与生命周期 | ☐ | | | |
| 3 | Authorize 端点 PKCE 参数透传 | ☐ | | | |
| 4 | 无 PKCE 参数向后兼容 | ☐ | | | |
| 5 | Demo Client (Public) PKCE 强制 | ⏭️ 待实现 | | | **功能尚未实现**：`public_client` 字段存在但 PKCE 强制逻辑未编写，跳过此场景 |
