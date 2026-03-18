# Phase 5 FR4: 重构 Config — KeycloakConfig → IdentityConfig

**类型**: 重构
**严重程度**: Medium
**影响范围**: auth9-core 全局（~50 处引用）
**前置依赖**:
- `keycloak_phase5_fr3_remove_keycloak_code_paths.md`
**被依赖**:
- `keycloak_phase5_fr5_infrastructure_cleanup.md`

---

## 背景

FR3 删除了 Keycloak 模块和代码路径，但 `Config` 结构体中仍保留着 `KeycloakConfig` 及其字段。业务代码通过 `config().keycloak.portal_url` 和 `config().keycloak.core_public_url` 访问这些字段（约 50+ 处），且这些字段并非 Keycloak 专属——它们是 Auth9 自身的公网 URL。

---

## 期望行为

### R1: 重命名 KeycloakConfig

将 `KeycloakConfig` 重命名为通用结构，保留仍在使用的字段：

**新结构**（建议放在 Config 顶层或新建 `IdentityConfig`）：

```rust
// Option A: 提升到 Config 顶层（推荐，因为只有 2 个字段）
pub struct Config {
    // ... 现有字段 ...
    /// Auth9 Core 公网 URL（用于 OIDC callback、SAML metadata 等）
    pub core_public_url: Option<String>,
    /// Auth9 Portal 公网 URL（用于邮件链接、登录重定向等）
    pub portal_url: Option<String>,
    // 不再需要 keycloak: KeycloakConfig
}
```

**删除的 Keycloak 专属字段**：

| 字段 | 状态 |
|------|------|
| `url` | 删除 — Keycloak 内部 URL |
| `public_url` | 删除 — Keycloak 公网 URL |
| `realm` | 删除 — Keycloak realm 名称 |
| `admin_client_id` | 删除 — Keycloak Admin API client |
| `admin_client_secret` | 删除 — Keycloak Admin API secret |
| `ssl_required` | 删除 — Keycloak SSL 配置 |
| `webhook_secret` | 删除 — Keycloak Event SPI webhook |
| `core_public_url` | 保留 → 提升到 Config 顶层 |
| `portal_url` | 保留 → 提升到 Config 顶层 |

### R2: 更新环境变量

| 旧环境变量 | 新环境变量 | 备注 |
|-----------|-----------|------|
| `KEYCLOAK_URL` | 删除 | |
| `KEYCLOAK_PUBLIC_URL` | 删除 | |
| `KEYCLOAK_REALM` | 删除 | |
| `KEYCLOAK_ADMIN_CLIENT_ID` | 删除 | |
| `KEYCLOAK_ADMIN_CLIENT_SECRET` | 删除 | |
| `KEYCLOAK_SSL_REQUIRED` | 删除 | |
| `KEYCLOAK_WEBHOOK_SECRET` | 删除 | |
| `KEYCLOAK_ADMIN` | 删除 | |
| `KEYCLOAK_ADMIN_PASSWORD` | 删除 | |
| `AUTH9_CORE_PUBLIC_URL` | 保留（已存在） | 映射到 `config.core_public_url` |
| `AUTH9_PORTAL_URL` | 保留（已存在） | 映射到 `config.portal_url` |

### R3: 全局替换引用

将所有 `config().keycloak.portal_url` 替换为 `config().portal_url`。
将所有 `config().keycloak.core_public_url` 替换为 `config().core_public_url`。

**受影响的文件**（约 15 个）：
- `domains/identity/api/auth/oidc_flow.rs`
- `domains/identity/api/auth/helpers.rs`
- `domains/identity/api/hosted_login.rs`
- `domains/identity/api/enterprise_common.rs`
- `domains/identity/api/social_broker.rs`
- `domains/identity/service/email_verification.rs`
- `domains/identity/service/password.rs`
- `domains/tenant_access/service/saml_application.rs`
- `domains/integration/api/keycloak_event.rs`（若仍存在，一并删除）
- `migration/mod.rs`
- `server/mod.rs`
- 其他引用 `config().keycloak.*` 的文件

### R4: 更新测试 Config 构造

所有测试中构造 `Config` 的地方需要：
- 移除 `keycloak: KeycloakConfig { ... }` 字段
- 改为设置 `core_public_url` 和 `portal_url` 顶层字段

---

## 非目标

- 不修改 docker-compose 中的环境变量（FR5 负责）
- 不更新部署脚本（FR5 负责）

---

## 关键文件

- `auth9-core/src/config/mod.rs` — 主要修改文件
- 所有引用 `config().keycloak.*` 的 ~15 个源文件
- 所有构造 `Config` 的测试文件

---

## 验证方法

```bash
cd auth9-core && cargo test
cd auth9-core && cargo clippy

# 确认无 KeycloakConfig 残留
rg "KeycloakConfig|keycloak\." auth9-core/src/ --type rust
# 期望：0 结果

# 确认无 KEYCLOAK_ 环境变量引用
rg "KEYCLOAK_" auth9-core/src/ --type rust
# 期望：0 结果
```
