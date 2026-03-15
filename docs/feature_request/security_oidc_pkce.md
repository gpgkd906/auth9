# OIDC 授权流程实现 PKCE (RFC 7636)

**来源**: QA Ticket `security_oidc-scenario1_pkce_260315_235000`
**严重程度**: High
**类型**: 安全增强
**影响范围**: auth9-core (Backend), auth9-portal (Frontend), Keycloak 客户端配置

---

## 背景

Auth9 当前的 OIDC 授权流程（Authorization Code Flow）未实现 PKCE（Proof Key for Code Exchange, RFC 7636）。PKCE 是 OAuth 2.1 的强制要求，用于防止 Authorization Code 拦截攻击。

### 当前架构

```
Portal (login.tsx)
  → GET auth9-core /api/v1/auth/authorize
    → 302 Keycloak /protocol/openid-connect/auth
      (response_type, client_id, redirect_uri, scope, state, nonce)
      ← 用户登录 →
    ← 302 auth9-core /api/v1/auth/callback?code=...&state=...
  ← 302 Portal /auth/callback?code=...&state=...
  → POST auth9-core /api/v1/auth/token
    → POST Keycloak /protocol/openid-connect/token
      (grant_type, client_id, code, redirect_uri, [client_secret])
```

**缺失**: 整个流程中没有 `code_challenge`、`code_challenge_method`、`code_verifier` 参数。

### 风险

- **Public Client (auth9-demo)**: 无 `client_secret`，仅靠 authorization code 交换 token。如果 code 被拦截（中间人攻击、日志泄露、Referer 泄露），攻击者可直接换取 token。
- **Confidential Client (auth9-portal)**: 有 `client_secret` 保护，风险较低，但 PKCE 提供纵深防御。
- **合规性**: OAuth 2.1 (draft) 要求所有客户端使用 PKCE；OWASP 推荐所有 OAuth 2.0 流程使用 PKCE。

---

## 期望行为

### R1: auth9-core 授权端点支持 PKCE 参数

- `GET /api/v1/auth/authorize` 接受可选的 `code_challenge` 和 `code_challenge_method` 参数
- 将 `code_challenge` 和 `code_challenge_method` 透传到 Keycloak 授权 URL
- 将 `code_verifier` 的关联信息存储在 `CallbackState` 中（或通过 state 透传）

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/types.rs` — `AuthorizeRequest` 增加字段
- `auth9-core/src/domains/identity/api/auth/helpers.rs` — `KeycloakAuthUrlParams` 和 `build_keycloak_auth_url` 增加 PKCE 参数
- `auth9-core/src/domains/identity/api/auth/oidc_flow.rs` — `authorize` handler 传递 PKCE 参数

### R2: auth9-core Token 端点支持 code_verifier

- `POST /api/v1/auth/token` 接受可选的 `code_verifier` 参数
- 向 Keycloak token 端点透传 `code_verifier`

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/types.rs` — `TokenRequest` 增加 `code_verifier`
- `auth9-core/src/domains/identity/api/auth/keycloak_client.rs` — `exchange_code_for_tokens` 发送 `code_verifier`

### R3: auth9-portal 发起 PKCE 流程

- Portal 登录时生成 `code_verifier`（43-128 字符的 cryptographic random string）
- 计算 `code_challenge = BASE64URL(SHA256(code_verifier))`
- 将 `code_challenge` 和 `code_challenge_method=S256` 传入 authorize URL
- 将 `code_verifier` 存入 OAuth state cookie（与 state 一起）
- Callback 时从 cookie 读取 `code_verifier` 并传给 token 端点

**涉及文件**:
- `auth9-portal/app/routes/login.tsx` — `buildAuthorizeParams` 生成 PKCE 参数
- `auth9-portal/app/services/session.server.ts` — OAuth state cookie 增加 `code_verifier` 存储
- `auth9-portal/app/routes/auth.callback.tsx` — token 请求中发送 `code_verifier`

### R4: Keycloak 客户端配置 PKCE

- Public client（如 auth9-demo）**强制要求** PKCE：设置 `pkce.code.challenge.method = S256`
- Confidential client（如 auth9-portal）**推荐但不强制** PKCE

**涉及文件**:
- `auth9-core/src/keycloak/types.rs` — `KeycloakOidcClient` 的 `attributes` 增加 PKCE 配置
- `auth9-core/src/keycloak/seeder.rs` — Portal client 配置 PKCE
- `auth9-core/src/domains/authorization/api/service.rs` — `build_keycloak_client_for_create` 对 public client 设置 PKCE

### R5: 单元测试覆盖

- auth9-core OIDC 流程测试验证 PKCE 参数透传
- auth9-portal 单元测试验证 `code_verifier` 生成和 `code_challenge` 计算
- Keycloak wiremock 测试验证 token 请求包含 `code_verifier`

---

## 验证方法

### 代码验证

```bash
# 搜索 PKCE 相关实现
grep -r "code_challenge\|code_verifier\|pkce" auth9-core/src/ auth9-portal/app/

# 运行后端测试
cd auth9-core && cargo test

# 运行前端类型检查和测试
cd auth9-portal && npm run typecheck && npm run test
```

### 手动验证

1. 通过 Portal 发起登录，抓包检查 authorize URL 包含 `code_challenge` 和 `code_challenge_method=S256`
2. 检查 Keycloak token 请求包含 `code_verifier`
3. 验证 Public client 未提供 `code_challenge` 时被 Keycloak 拒绝

---

## 参考

- [RFC 7636 - Proof Key for Code Exchange](https://datatracker.ietf.org/doc/html/rfc7636)
- [OAuth 2.1 Draft](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1)
- [OWASP OAuth 2.0 Security](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
- QA 安全文档: `docs/security/authentication/01-oidc-security.md`
