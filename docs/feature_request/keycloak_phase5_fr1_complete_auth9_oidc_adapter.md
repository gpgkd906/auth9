# Phase 5 FR1: 补全 Auth9Oidc Adapter 缺失实现

**类型**: 功能补全
**严重程度**: High
**影响范围**: `auth9-core/src/identity_engine/adapters/auth9_oidc/`
**前置依赖**: Phase 1-4 全部 CLOSED
**被依赖**:
- `keycloak_phase5_fr2_decouple_keycloak_types.md`
- `keycloak_phase5_fr3_remove_keycloak_code_paths.md`

---

## 背景

`Auth9OidcIdentityEngineAdapter` 在 Phase 1-3 中逐步实现了密码验证、凭据管理、Pending Actions、Email Verification、Federation Broker 等 sub-store。但 UserStore CRUD、ClientStore、SessionStore 仍为占位实现（返回 "not implemented"）。

在移除 Keycloak 代码前，必须先确保 auth9_oidc adapter 能完整处理所有 IdentityEngine 操作，因为以下业务代码直接依赖这些方法：

- `tenant_access/api/user.rs` — 调用 `create_user`, `update_user`, `delete_user`
- `tenant_access/api/invitation.rs` — 调用 `create_user`
- `provisioning/service/scim.rs` — 调用 `create_user`
- `authorization/api/service.rs` — 调用 ClientStore 全部 OIDC client 方法
- `tenant_access/service/saml_application.rs` — 调用 ClientStore SAML 方法
- `identity/service/password.rs` — 调用 `session_store().logout_user()`
- `identity/api/hosted_login.rs` — 调用 `session_store().delete_user_session()`

---

## 期望行为

### R1: UserStore CRUD 实现

补全 `Auth9OidcUserStore` 中以下方法：

#### `create_user(input: &IdentityUserCreateInput) -> Result<String>`

- 生成 UUID v4 作为 identity_subject
- 如果 `input.credentials` 包含 password，调用 `upsert_password_credential(identity_subject, password, temporary)` 存储到 `credentials` 表
- 返回 identity_subject

**设计依据**：在 auth9_oidc 模式下，auth9 自身即为身份提供者。调用方拿到 identity_subject 后会调用 `UserRepository::create(identity_subject, input)` 在 `users` 表创建记录。

#### `get_user(user_id: &str) -> Result<IdentityUserRepresentation>`

- 查询 `users` 表：`SELECT id, email, display_name, mfa_enabled FROM users WHERE identity_subject = ?`
- 映射到 `IdentityUserRepresentation`：
  - `id` → `Some(user.id.to_string())`
  - `username` → `user.email`
  - `email` → `Some(user.email)`
  - `first_name` → `user.display_name`
  - `last_name` → `None`
  - `enabled` → `true`（auth9 无 disabled 状态字段，locked_until 由业务层判断）
  - `email_verified` → 查询 `user_verification_status` 表
  - `attributes` → `HashMap::new()`
- 若用户不存在，返回 `AppError::NotFound`

#### `update_user(user_id: &str, input: &IdentityUserUpdateInput) -> Result<()>`

- 返回 `Ok(())`（no-op）
- **设计依据**：调用方在调用 identity_engine 后紧接着调用 `user_service().update()` 更新 auth9 数据库。在 auth9_oidc 模式下，auth9 数据库即为唯一数据源，无需额外同步。

#### `delete_user(user_id: &str) -> Result<()>`

- 清理 auth9-oidc 相关表中该用户的数据：
  - `DELETE FROM credentials WHERE user_id = ?`
  - `DELETE FROM pending_actions WHERE user_id = ?`
  - `DELETE FROM email_verification_tokens WHERE user_id = ?`
  - `DELETE FROM user_verification_status WHERE user_id = ?`
- 返回 `Ok(())`
- **设计依据**：auth9 users 表的记录由 `UserRepository::delete()` 负责，identity_engine 只负责清理身份后端关联数据。

### R2: ClientStore 实现

当前 `Auth9OidcClientStore` 为零尺寸结构体，需增加数据库连接。

#### 构造函数

增加 `pool: MySqlPool` 参数。

#### OIDC Client 方法

| 方法 | 实现 |
|------|------|
| `create_oidc_client` | 暂返回 `Ok(uuid::Uuid::new_v4().to_string())`。实际的 OIDC client 创建由 `ClientService` 在应用层完成，identity_engine 层的创建是 Keycloak 同步遗留。 |
| `get_client_secret` | 返回 `AppError::BadRequest("Client secrets are hashed and cannot be retrieved")`。auth9 使用 Argon2 存储 client secret hash，不可逆。 |
| `regenerate_client_secret` | 生成新随机 secret，hash 后更新 `clients.client_secret_hash`，返回明文 secret。 |
| `get_client_uuid_by_client_id` | 查询 `SELECT id FROM clients WHERE client_id = ?` |
| `get_client_by_client_id` | 查询 clients 表，映射到 `KeycloakOidcClient`（FR2 后改为 `OidcClientRepresentation`） |
| `update_oidc_client` | 暂返回 `Ok(())`（应用层已处理） |
| `delete_oidc_client` | 暂返回 `Ok(())`（应用层已处理） |

#### SAML Client 方法

| 方法 | 实现 |
|------|------|
| `create_saml_client` | 暂返回 `Ok(uuid::Uuid::new_v4().to_string())`（应用层已处理） |
| `update_saml_client` | 返回 `Ok(())`（应用层已处理） |
| `delete_saml_client` | 返回 `Ok(())`（应用层已处理） |
| `get_saml_idp_descriptor` | 返回 auth9 自身的 SAML IdP metadata XML |
| `get_active_signing_certificate` | 从 JWT key pair 导出公钥证书 PEM |
| `saml_sso_url()` | 返回 auth9 的 SAML SSO endpoint URL |

### R3: SessionStore 实现

当前 `Auth9OidcSessionStoreAdapter` 为 no-op。由于 auth9 的 session 管理已通过 `SessionRepository` 在应用层完成，identity_engine 的 SessionStore 在 auth9_oidc 模式下**保持 no-op 是正确行为**。

但为清晰起见，添加 tracing 日志：

```rust
async fn delete_user_session(&self, session_id: &str) -> Result<()> {
    tracing::debug!(session_id, "auth9_oidc: session deletion handled by application layer");
    Ok(())
}

async fn logout_user(&self, user_id: &str) -> Result<()> {
    tracing::debug!(user_id, "auth9_oidc: user logout handled by application layer");
    Ok(())
}
```

---

## 非目标

- 不修改 IdentityEngine trait 签名（FR2 负责）
- 不移除 Keycloak 代码路径（FR3 负责）
- 不重构 Config（FR4 负责）

---

## 关键文件

- `auth9-core/src/identity_engine/adapters/auth9_oidc/engine.rs` — 主要修改文件
- `auth9-core/src/identity_engine/adapters/auth9_oidc/session_store.rs` — SessionStore
- `auth9-core/src/identity_engine/mod.rs` — trait 定义（只读参考）
- `auth9-core/src/identity_engine/types.rs` — IdentityUserCreateInput 等类型（只读参考）
- `auth9-core/src/models/user.rs` — User model 和 CreateUserInput（只读参考）
- `auth9-core/src/repository/user/mod.rs` — UserRepository trait（只读参考）

---

## 验证方法

```bash
cd auth9-core && cargo test
cd auth9-core && cargo clippy
```

功能验证：

1. `IDENTITY_BACKEND=auth9_oidc` 模式下，创建用户不再报 "not implemented"
2. 用户 CRUD 通过 identity_engine 正常工作
3. 所有现有测试继续通过
