# 集成测试 - Identity Engine 能力面补齐与 State 清理

**模块**: 集成测试
**测试范围**: `IdentityUserStore` / `IdentityClientStore` / `IdentityCredentialStore` 最小能力面、`HasServices` 抽象出口、Auth9 OIDC wiring 回归
**场景数**: 3
**优先级**: 高

---

## 背景说明

> **迁移已完成**: Keycloak 已被 Auth9 内置 OIDC 引擎完全替代。以下为历史迁移验证记录。

本用例用于验证 Phase 1 FR1 完成后的关键回归点：

- `state` 通过 `IdentityEngine` 抽象向业务层暴露身份能力
- `identity_engine` 抽象具备当前业务层已使用的 user/client/credential 最小能力面
- auth9-oidc backend 完成完整 wiring 与 contract 回归

本用例聚焦后端注入链与 contract，不覆盖 Portal UI。

---

## 场景 1：`state` 仅暴露抽象身份后端

### 初始状态
- 仓库代码已切到本 FR 实现版本
- 本地可执行 `rg`

### 目的
验证 `HasServices` 与生产 `AppState` 不再暴露 `keycloak_client()` trait 出口。

### 测试操作流程
1. 执行以下命令：
   ```bash
   rg -n "fn keycloak_client|KeycloakClient" auth9-core/src/state.rs auth9-core/src/server/mod.rs
   ```
2. 检查输出内容。

### 预期结果
- `auth9-core/src/state.rs` 中不存在 `fn keycloak_client(&self)` trait 定义
- `auth9-core/src/server/mod.rs` 中不存在 `impl HasServices for AppState` 的 `keycloak_client()` 实现
- 允许保留 server 组装层自己的 `KeycloakClient` 字段或构造代码

---

## 场景 2：Keycloak adapter contract 覆盖新增能力面

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 Keycloak adapter 通过 `IdentityUserStore` / `IdentityClientStore` / `IdentityCredentialStore` 暴露最小能力，并通过 contract test。

### 测试操作流程
1. 执行以下命令：
   ```bash
   cd auth9-core && cargo test identity_engine -- --nocapture
   ```
2. 执行以下命令：
   ```bash
   cd auth9-core && cargo test keycloak_adapter_contract_test -- --nocapture
   ```

### 预期结果
- `identity_engine` 相关测试通过
- `keycloak_adapter_contract_test` 通过
- 输出中包含新增 user/client/credential store contract 回归

---

## 场景 3：`auth9_oidc` stub backend 保持最小 wiring

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 `auth9_oidc` 分支在补齐能力面后仍可完成最小 wiring，并对未实现操作返回明确错误而不是 panic。

### 测试操作流程
1. 执行以下命令：
   ```bash
   cd auth9-core && cargo test backend_switch_smoke_test -- --nocapture
   ```

### 预期结果
- `backend_switch_smoke_test` 通过
- `IdentityBackend::Auth9Oidc` 分支可完成 `session_store`、`federation_broker`、`identity_engine` 注入
- 未实现的 user/client 操作返回显式错误，不出现 wiring panic

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | `state` 仅暴露抽象身份后端 | ☑ | 2026-03-17 | Codex | `rg` 已验证 |
| 2 | Keycloak adapter contract 覆盖新增能力面 | ☑ | 2026-03-17 | Codex | `cargo test identity_engine` 与 `cargo test keycloak_adapter_contract_test` 已通过 |
| 3 | `auth9_oidc` stub backend 保持最小 wiring | ☑ | 2026-03-17 | Codex | `cargo test backend_switch_smoke_test` 已通过 |
