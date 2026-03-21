# Public Client PKCE 强制验证

**类型**: 安全增强
**严重程度**: High
**影响范围**: auth9-core (Backend)
**前置依赖**: 无（PKCE 透传和 `public_client` 字段已实现）

---

## 背景

Auth9 OIDC 引擎已完整实现 PKCE（Proof Key for Code Exchange, RFC 7636）的 S256 透传与验证：当客户端在 `/authorize` 请求中提供 `code_challenge` 参数时，token 端点会要求并验证 `code_verifier`。

但当前 PKCE 对所有客户端类型均为**可选**。根据 OAuth 2.1 草案和 OIDC 最佳实践，Public Client（SPA、移动应用、CLI 工具）**必须**使用 PKCE 以防止授权码拦截攻击。

### 现状

| 组件 | 状态 | 位置 |
|------|------|------|
| `code_challenge` / `code_verifier` 透传 | 已实现 | `oidc_flow.rs:95-96, 396-413` |
| `verify_pkce_s256()` 验证函数 | 已实现 | `helpers.rs:53-57` |
| `clients.public_client` 字段 | 已存在 | 迁移 `20260320000002`；模型 `service.rs:137` |
| Public client 创建时 secret 为空 | 已实现 | `client.rs:160-161` |
| Authorize 端点检查 public_client 强制 PKCE | **未实现** | — |
| Token 端点检查 public_client 不验证 secret | **未实现** | — |

### 安全风险

Public client 无法安全存储 `client_secret`，因此依赖 PKCE 作为唯一的授权码交换保护机制。不强制 PKCE 意味着：
- 授权码可被中间人拦截并直接换取 token
- 不符合 OAuth 2.1 (draft) 和 FAPI 规范要求

---

## 期望行为

### R1: Authorize 端点 — Public Client 必须提供 PKCE

在 `authorize()` 处理流程中（`oidc_flow.rs`），获取 `Service` 后进一步查询 `Client` 记录：

```
1. 根据 params.client_id 查询 Client（当前仅查 Service）
2. 若 client.public_client == true 且 code_challenge 为空：
   → 返回 400 Bad Request: "Public clients must use PKCE (code_challenge required)"
3. 若 code_challenge_method 不为 S256：
   → 返回 400 Bad Request: "Only S256 code challenge method is supported"
```

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/oidc_flow.rs` — `authorize()` 函数（约 line 55-152）
- `auth9-core/src/domains/authorization/service/client.rs` — 需提供按 `client_id` 查询含 `public_client` 的方法（已有 `get_by_client_id`）

### R2: Token 端点 — Public Client 不验证 client_secret

当前 token 端点对所有客户端验证 `client_secret`。Public client 无 secret，需豁免：

```
1. 查询 Client 记录
2. 若 client.public_client == true：
   → 跳过 client_secret 验证
   → 仍要求 code_verifier（由 R1 保证 code_challenge 已存在）
3. 若 client.public_client == false 且 client_secret 不匹配：
   → 返回 401 Unauthorized
```

**涉及文件**:
- `auth9-core/src/domains/identity/api/auth/oidc_flow.rs` — `token()` 函数（约 line 350-571）

### R3: Client API — 支持设置 public_client

管理 API 需支持将客户端标记为 public：

```
POST /api/v1/services/{service_id}/clients
{
  "client_id": "my-spa",
  "public_client": true,
  "redirect_uris": ["http://localhost:3000/callback"]
}
```

当 `public_client = true` 时：
- `client_secret` 不生成（或生成后不返回）
- 响应中不包含 `client_secret` 字段

**涉及文件**:
- `auth9-core/src/domains/authorization/service/client.rs` — `create_client()` 已处理空 secret
- `auth9-core/src/domains/authorization/api/client.rs` — 确保 API 层传递 `public_client`

### R4: Portal UI — Client 配置页显示 Public Client 开关

在 Portal 的 Client 管理页面中添加 "Public Client" 开关：
- 开启时隐藏 Client Secret 相关 UI
- 显示提示："Public clients must use PKCE for authorization code flow"

**涉及文件**:
- `auth9-portal/app/routes/dashboard.services.$serviceId.clients*` — Client 管理页面

---

## 测试要求

### 单元测试

```
1. authorize() — public_client=true, 无 code_challenge → 400
2. authorize() — public_client=true, 有 code_challenge → 正常重定向
3. authorize() — public_client=false, 无 code_challenge → 正常重定向（向后兼容）
4. token() — public_client=true, 无 client_secret, 有 code_verifier → 成功
5. token() — public_client=true, 无 code_verifier → 400
6. token() — public_client=false, 无 client_secret → 401
```

### QA 场景

实现后启用 `docs/qa/auth/16-pkce-flow.md` 场景 5（当前标记为「⏭️ 待实现」）。

---

## 参考

- [RFC 7636 - Proof Key for Code Exchange](https://tools.ietf.org/html/rfc7636)
- [OAuth 2.1 Draft - Section 7.6](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1-11#section-7.6)
- QA 文档: `docs/qa/auth/16-pkce-flow.md`
