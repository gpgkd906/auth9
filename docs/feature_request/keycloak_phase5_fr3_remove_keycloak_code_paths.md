# Phase 5 FR3: 移除 IdentityBackend 开关与 Keycloak 代码路径

**类型**: 代码删除 + 重构
**严重程度**: High
**影响范围**: auth9-core 全局
**前置依赖**:
- `keycloak_phase5_fr1_complete_auth9_oidc_adapter.md`
- `keycloak_phase5_fr2_decouple_keycloak_types.md`
**被依赖**:
- `keycloak_phase5_fr4_refactor_config.md`

---

## 背景

Phase 1 引入了 `IdentityBackend` 枚举（Keycloak / Auth9Oidc）用于运行时后端切换。这导致 OIDC flow、logout 等核心路径中存在大量 `if config.identity_backend == IdentityBackend::Auth9Oidc { ... } else { ... }` 分支。

FR1 补全了 auth9_oidc adapter，FR2 解耦了 Keycloak 类型。现在可以：
1. 删除 `IdentityBackend` 枚举，auth9_oidc 成为唯一后端
2. 删除所有 Keycloak 代码分支
3. 删除 Keycloak adapter、Keycloak 模块、Keycloak client helpers

---

## 期望行为

### R1: 删除 IdentityBackend 枚举

文件：`auth9-core/src/config/mod.rs`

- 删除 `IdentityBackend` enum、`Display` impl、`as_str()` impl
- 删除 `Config.identity_backend` 字段
- 删除 `IDENTITY_BACKEND` 环境变量解析逻辑
- 更新 `Config::default()` 和测试中的 Config 构造

### R2: 清理 OIDC Flow 中的 Keycloak 分支

文件：`auth9-core/src/domains/identity/api/auth/oidc_flow.rs`

| 函数 | 操作 |
|------|------|
| `authorize()` | 删除 Keycloak 分支（~120 行），保留 auth9_oidc 分支作为唯一路径 |
| `token()` authorization_code | 删除 Keycloak 分支（~260 行），保留 auth9_oidc 分支 |
| `token()` refresh_token | 删除 Keycloak 分支（~140 行），保留 auth9_oidc 分支 |

删除后移除 `if state.config().identity_backend == IdentityBackend::Auth9Oidc` 判断，直接执行 auth9_oidc 逻辑。

### R3: 清理 Logout 中的 Keycloak 分支

文件：`auth9-core/src/domains/identity/api/auth/logout.rs`

- `logout_redirect()` — 删除 Keycloak 分支，保留 auth9_oidc 路径
- `logout()` — 删除 Keycloak 分支，保留 auth9_oidc 路径

### R4: 删除 keycloak_client.rs

文件：`auth9-core/src/domains/identity/api/auth/keycloak_client.rs`

- 删除整个文件
- 从 `auth/mod.rs` 移除 `mod keycloak_client`
- 被删除的函数：`exchange_code_for_tokens()`, `exchange_refresh_token()`, `fetch_userinfo()`

### R5: 清理 auth helpers

文件：`auth9-core/src/domains/identity/api/auth/helpers.rs`

- 删除 `build_keycloak_auth_url()`
- 删除 `build_keycloak_logout_url()`
- 删除 `KeycloakAuthUrlParams` 结构体
- 保留：`build_callback_url()`, `validate_redirect_uri()`, `LoginChallengeData`, `AuthorizationCodeData`, `verify_pkce_s256()`, `CallbackState`

### R6: 删除 Keycloak Adapter

删除整个目录：`auth9-core/src/identity_engine/adapters/keycloak/`

包含文件：
- `mod.rs`
- `engine.rs`
- `user_store.rs`
- `client_store.rs`
- `session_store.rs`
- `credential_store.rs`
- `event_source.rs`
- `federation_broker.rs`

从 `adapters/mod.rs` 移除 `pub mod keycloak`。

### R7: 删除 Keycloak 模块

删除整个目录：`auth9-core/src/keycloak/`

包含文件：
- `mod.rs`
- `client.rs` — KeycloakClient HTTP 客户端
- `seeder.rs` — KeycloakSeeder（启动时同步 realm/client）
- `types.rs` — Keycloak API 类型（SmtpServerConfig 已在 FR2 迁走）

从 `auth9-core/src/lib.rs` 移除 `pub mod keycloak`。

### R8: 清理 AppState 与 Server 启动

文件：`auth9-core/src/server/mod.rs`

- 删除 `select_identity_backend()` 函数
- 直接构建 `Auth9OidcIdentityEngineAdapter`
- 移除 `AppState` 中的 `keycloak_client` 字段（如果存在）
- 移除 `KeycloakClient::new()` 构造
- 移除 `KeycloakSeeder` 启动调用

### R9: 修复受影响的测试

- 删除 `identity_engine/mod.rs` 中的 `keycloak_adapter_exposes_identity_engine_surfaces` 测试
- 更新 ~15 个测试文件中构造 `Config` 的代码（移除 `identity_backend` 字段）
- 修复所有因删除 Keycloak 模块导致的编译错误

---

## 非目标

- 不重命名 `KeycloakConfig` 结构体（FR4 负责）
- 不删除 docker-compose 中的 Keycloak 服务（FR5 负责）
- 不更新文档（FR6 负责）

---

## 关键文件（修改/删除）

| 文件 | 操作 |
|------|------|
| `src/config/mod.rs` | 修改 — 删除 IdentityBackend |
| `src/domains/identity/api/auth/oidc_flow.rs` | 修改 — 删除 ~500 行 Keycloak 分支 |
| `src/domains/identity/api/auth/logout.rs` | 修改 — 删除 Keycloak 分支 |
| `src/domains/identity/api/auth/keycloak_client.rs` | 删除 |
| `src/domains/identity/api/auth/helpers.rs` | 修改 — 删除 Keycloak helpers |
| `src/identity_engine/adapters/keycloak/` | 删除整个目录 |
| `src/keycloak/` | 删除整个目录 |
| `src/server/mod.rs` | 修改 — 简化启动流程 |
| `src/lib.rs` | 修改 — 移除 mod keycloak |

---

## 验证方法

```bash
cd auth9-core && cargo test
cd auth9-core && cargo clippy

# 确认无 Keycloak 运行时代码残留
rg "KeycloakClient|keycloak_client|IdentityBackend" auth9-core/src/ --type rust
# 期望：0 结果（除了注释）

# 确认已删除的目录不存在
ls auth9-core/src/keycloak/ 2>&1        # 期望：不存在
ls auth9-core/src/identity_engine/adapters/keycloak/ 2>&1  # 期望：不存在
```
