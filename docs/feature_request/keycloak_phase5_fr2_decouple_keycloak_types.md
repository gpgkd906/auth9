# Phase 5 FR2: 解耦 Keycloak 类型与 IdentityEngine Trait

**类型**: 重构
**严重程度**: Medium
**影响范围**: `auth9-core/src/identity_engine/`, `auth9-core/src/keycloak/types.rs`, `auth9-core/src/models/email.rs`
**前置依赖**:
- `keycloak_phase5_fr1_complete_auth9_oidc_adapter.md`
**被依赖**:
- `keycloak_phase5_fr3_remove_keycloak_code_paths.md`

---

## 背景

当前 `IdentityEngine` trait 签名中直接引用了 Keycloak 模块类型：

- `IdentityClientStore::create_oidc_client(&self, client: &KeycloakOidcClient)` — 参数类型来自 `crate::keycloak`
- `IdentityClientStore::get_client_by_client_id(&self, ...) -> Result<KeycloakOidcClient>` — 返回类型来自 `crate::keycloak`
- `IdentityEngine::update_realm(&self, settings: &RealmUpdate)` — 参数类型来自 `crate::keycloak`
- `SmtpServerConfig` 在 `models/email.rs` 中引用自 `crate::keycloak::types`

这些类型耦合阻止了 FR3 直接删除 `keycloak` 模块。必须先用中性类型替换。

---

## 期望行为

### R1: 替换 `KeycloakOidcClient`

在 `auth9-core/src/identity_engine/types.rs` 新增：

```rust
/// Neutral OIDC client representation for IdentityClientStore operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OidcClientRepresentation {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub public_client: bool,
    pub redirect_uris: Vec<String>,
    pub web_origins: Vec<String>,
    pub secret: Option<String>,
    pub protocol: Option<String>,
}
```

更新 `IdentityClientStore` trait：
- `create_oidc_client(&self, client: &OidcClientRepresentation) -> Result<String>`
- `get_client_by_client_id(&self, client_id: &str) -> Result<OidcClientRepresentation>`
- `update_oidc_client(&self, client_uuid: &str, client: &OidcClientRepresentation) -> Result<()>`

两个 adapter 各自做类型转换：
- Keycloak adapter: `OidcClientRepresentation` ↔ `KeycloakOidcClient`
- Auth9Oidc adapter: 直接使用新类型

### R2: 替换 `RealmUpdate`

在 `auth9-core/src/identity_engine/types.rs` 新增：

```rust
/// Neutral realm settings update for IdentityEngine::update_realm().
#[derive(Debug, Clone, Default)]
pub struct RealmSettingsUpdate {
    pub registration_allowed: Option<bool>,
    pub reset_password_allowed: Option<bool>,
    pub smtp_server: Option<SmtpServerConfig>,
    pub password_policy: Option<String>,
    pub brute_force_protected: Option<bool>,
    pub max_login_failures: Option<u32>,
    pub wait_increment_seconds: Option<u32>,
}
```

更新 `IdentityEngine::update_realm(&self, settings: &RealmSettingsUpdate) -> Result<()>`

### R3: 迁移 `SmtpServerConfig`

将 `SmtpServerConfig` 从 `auth9-core/src/keycloak/types.rs` 迁移到 `auth9-core/src/models/email.rs`。

更新所有 `use crate::keycloak::SmtpServerConfig` 为 `use crate::models::email::SmtpServerConfig`。

---

## 非目标

- 不删除 Keycloak adapter 或模块（FR3 负责）
- 不修改 Config 结构（FR4 负责）
- 此阶段 Keycloak adapter 仍需编译通过

---

## 关键文件

- `auth9-core/src/identity_engine/mod.rs` — trait 定义，修改签名
- `auth9-core/src/identity_engine/types.rs` — 新增 `OidcClientRepresentation`, `RealmSettingsUpdate`
- `auth9-core/src/keycloak/types.rs` — SmtpServerConfig 迁出
- `auth9-core/src/models/email.rs` — SmtpServerConfig 迁入
- `auth9-core/src/identity_engine/adapters/keycloak/client_store.rs` — 适配新类型
- `auth9-core/src/identity_engine/adapters/auth9_oidc/engine.rs` — 适配新类型
- `auth9-core/src/domains/authorization/api/service.rs` — 调用方更新
- `auth9-core/src/domains/tenant_access/service/saml_application.rs` — 调用方更新

---

## 验证方法

```bash
cd auth9-core && cargo test
cd auth9-core && cargo clippy

# 确认 IdentityEngine trait 不再引用 keycloak 模块类型
rg "KeycloakOidcClient|RealmUpdate" auth9-core/src/identity_engine/mod.rs
# 期望：0 结果
```
