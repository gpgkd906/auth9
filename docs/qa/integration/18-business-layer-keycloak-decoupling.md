> **本文档已归档** — Keycloak 解耦已完成，Auth9 已完全迁移至 auth9-oidc。此文档仅供历史参考。

---

# 集成测试 - 业务层身份引擎解耦

**模块**: 集成测试
**测试范围**: `UserService` / `PasswordService` / `WebAuthnService` / `ScimService` / `SamlApplicationService` 与 `tenant_access` handlers 依赖 `IdentityEngine` 抽象
**场景数**: 5
**优先级**: 高

---

## 背景说明

> **迁移已完成**: Keycloak 已被 Auth9 内置 OIDC 引擎完全替代。以下为历史迁移验证记录。

本用例覆盖 Phase 1 FR2 的主要回归点：

- 业务服务通过 `IdentityEngine` 抽象访问身份后端
- `tenant_access/api/user.rs` 与 `tenant_access/api/invitation.rs` 使用中性输入模型
- auth9-oidc backend 维持完整 contract

本用例聚焦后端抽象边界与回归测试，不覆盖 Portal UI。

---

## 场景 1：服务层不再直接持有 `KeycloakClient`

### 初始状态
- 仓库代码已切到本 FR 实现版本
- 本地可执行 `rg`

### 目的
验证目标服务实现文件已改为依赖 `IdentityEngine` 抽象，而不是 `KeycloakClient`。

### 测试操作流程
1. 执行以下命令：
   ```bash
   rg -n "KeycloakClient|Arc<KeycloakClient>" \
     auth9-core/src/domains/tenant_access/service/user.rs \
     auth9-core/src/domains/identity/service/password.rs \
     auth9-core/src/domains/identity/service/webauthn.rs \
     auth9-core/src/domains/provisioning/service/scim.rs \
     auth9-core/src/domains/tenant_access/service/saml_application.rs
   ```
2. 检查输出内容。

### 预期结果
- 上述 5 个服务实现文件无命中
- 允许测试模块中保留 `KeycloakClient` 构造用于 adapter/wiremock 回归

---

## 场景 2：handler 主业务路径不再直接拼装 `Keycloak*` DTO

### 初始状态
- 仓库代码已切到本 FR 实现版本
- 本地可执行 `rg`

### 目的
验证 `tenant_access` handlers 已切到中性输入模型。

### 测试操作流程
1. 执行以下命令：
   ```bash
   rg -n "CreateKeycloakUserInput|KeycloakCredential|KeycloakUserUpdate|KeycloakIdentityProvider" \
     auth9-core/src/domains/tenant_access/api/user.rs \
     auth9-core/src/domains/tenant_access/api/invitation.rs \
     auth9-core/src/domains/tenant_access/api/tenant_sso.rs
   ```
2. 检查输出内容。

### 预期结果
- `user.rs`、`invitation.rs` 无 `CreateKeycloakUserInput` / `KeycloakCredential` / `KeycloakUserUpdate` 命中
- `tenant_sso.rs` 维持 `IdentityProviderRepresentation` 抽象接口
- 业务主路径不再直接依赖 Keycloak DTO

---

## 场景 3：密码与 Passkeys 抽象调用路径回归

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 Password / WebAuthn 业务服务通过抽象接口完成回归。

### 测试操作流程
1. 执行以下命令：
   ```bash
   cd auth9-core && cargo test domains::identity::service::password -- --nocapture
   ```
2. 执行以下命令：
   ```bash
   cd auth9-core && cargo test domains::identity::service::webauthn -- --nocapture
   ```

### 预期结果
- 两组测试全部通过
- PasswordService 可通过 `IdentityUserStore` / `IdentitySessionStore` 完成改密路径
- WebAuthnService 可通过 `IdentityCredentialStore` 完成 Keycloak 凭据列表/删除兼容路径

---

## 场景 4：SCIM 与 SAML Application 抽象调用路径回归

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 SCIM user provisioning 与 SAML Application 管理已走抽象 backend。

### 测试操作流程
1. 执行以下命令：
   ```bash
   cd auth9-core && cargo test domains::provisioning -- --nocapture
   ```
2. 执行以下命令：
   ```bash
   cd auth9-core && cargo test domains::tenant_access::service::saml_application -- --nocapture
   ```

### 预期结果
- 两组测试全部通过
- SCIM create user 路径改用中性 user create input
- SAML Application CRUD / metadata 路径改由 `IdentityClientStore` 承载

---

## 场景 5：Keycloak adapter 与 `auth9_oidc` backend contract 保持稳定

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 Keycloak 解耦后，现有测试套件仍然通过，无 wiring panic。

> **注意**: 原文档引用的 `keycloak_adapter_contract_test` 和 `backend_switch_smoke_test` 两个测试目标已在后续重构中移除。当前应使用现有测试套件验证 contract 稳定性。

### 测试操作流程
1. 确认当前可用的集成测试目标：
   ```bash
   cd auth9-core && cargo test --test api_test -- --nocapture 2>&1 | tail -5
   ```
2. 运行全量单元测试验证无 wiring panic：
   ```bash
   cd auth9-core && cargo test --lib 2>&1 | grep "test result"
   ```

### 预期结果
- 现有集成测试通过（`api_test`）
- 单元测试通过，无 wiring panic
- `auth9_oidc` backend 路径正常运行

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| `no test target named keycloak_adapter_contract_test` | 旧测试目标已移除 | 使用 `cargo test --test api_test` 替代 |
| `no test target named backend_switch_smoke_test` | 旧测试目标已移除 | 使用 `cargo test --lib` 验证 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 服务层不再直接持有 `KeycloakClient` | ☑ | 2026-04-06 | QA Runner | `rg` 已验证目标服务文件无命中 |
| 2 | handler 主业务路径不再直接拼装 `Keycloak*` DTO | ☑ | 2026-04-06 | QA Runner | `rg` 已验证 `user.rs` / `invitation.rs` 无命中 |
| 3 | 密码与 Passkeys 抽象调用路径回归 | ☑ | 2026-04-06 | QA Runner | Password: 36, WebAuthn: 17 |
| 4 | SCIM 与 SAML Application 抽象调用路径回归 | ☑ | 2026-04-06 | QA Runner | SCIM: 22, SAML App: 35 |
| 5 | Keycloak adapter 与 `auth9_oidc` backend contract 保持稳定 | ☑ | 2026-04-06 | QA Runner | api_test: 622, lib: 2023 |
