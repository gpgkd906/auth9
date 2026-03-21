# 集成测试 - Phase 1 身份抽象层 Closure

**模块**: 集成测试
**测试范围**: `keycloak` 默认 backend、`auth9_oidc` stub backend、adapter 注入链、`identity_subject` / `provider_session_id` / `provider_alias` 中性字段主路径
**场景数**: 4
**优先级**: 高

---

## 背景说明

> **迁移已完成**: Keycloak 已被 Auth9 内置 OIDC 引擎完全替代。以下为历史迁移 closure 验收记录。

本用例用于对 Phase 1 的身份抽象层改造做最终 closure 验收，确认以下目标已同时成立：

- auth9-oidc backend 完成完整注入与 contract 回归
- 业务层与 adapter 注入链保持在 `IdentityEngine` 抽象边界内
- QA 主断言字段已经切到 `identity_subject`、`provider_session_id`、`provider_alias`

本文件是总验收文档，不替代分项回归文档 `integration/15`、`integration/16`、`integration/17`、`integration/18`。

---

## 场景 1：默认 `keycloak` backend 启动与注入链正常

### 初始状态
- 本地依赖服务已启动
- `auth9-core` 可执行 `cargo run`

### 目的
验证默认 `IDENTITY_BACKEND=keycloak` 下，`session_store`、`federation_broker`、`identity_engine` 注入链正常。

### 测试操作流程
1. 检查容器环境中的 backend 配置：
   ```bash
   docker inspect auth9-core --format '{{range .Config.Env}}{{println .}}{{end}}' | rg '^IDENTITY_BACKEND=' || echo 'IDENTITY_BACKEND not set (defaults to keycloak)'
   ```
2. 检查健康探针：
   ```bash
   curl -sf http://localhost:8080/health
   ```

### 预期结果
- 输出 `IDENTITY_BACKEND not set (defaults to keycloak)`，或显式出现 `IDENTITY_BACKEND=keycloak`
- `curl /health` 返回 `200`
- 当前运行实例在默认 `keycloak` backend 下健康响应

---

## 场景 2：auth9_oidc backend wiring 验证

> **已完成/已淘汰**: Phase 5 FR3 已完全移除 Keycloak 代码路径，`backend_switch_smoke_test` 测试目标已被删除。auth9_oidc 现在是唯一的 identity backend，无需再验证 backend 切换。以下改为验证 `identity_engine` 相关测试通过。

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test`

### 目的
验证 auth9_oidc backend 的 identity engine wiring 正常工作。

### 测试操作流程
1. 执行 identity engine 测试：
   ```bash
   cd auth9-core && cargo test identity_engine -- --nocapture
   ```
2. 验证健康端点：
   ```bash
   curl -sf http://localhost:8080/health
   ```

### 预期结果
- `cargo test identity_engine` 通过
- auth9_oidc 为当前唯一 identity backend，`session_store`、`federation_broker`、`identity_engine` 注入链正常
- `/health` 返回 200

---

## 场景 3：业务抽象边界保持稳定（Keycloak 已移除）

> **已更新**: Phase 5 FR3 已完全移除 Keycloak 代码路径，`keycloak_adapter_contract_test` 测试目标已被删除。本场景改为验证 identity engine 测试通过且业务层无 Keycloak 直接依赖残留。

### 初始状态
- Rust 依赖已安装
- 本地可执行 `cargo test` 与 `rg`

### 目的
验证 identity engine 抽象边界稳定，且目标业务层文件不包含已移除的 `KeycloakClient` 直接依赖。

### 测试操作流程
1. 执行 identity engine 测试：
   ```bash
   cd auth9-core && cargo test identity_engine -- --nocapture
   ```
2. 扫描目标服务文件的非测试区域，确认无 Keycloak 残留依赖：
   ```bash
   for f in \
     auth9-core/src/domains/tenant_access/service/user.rs \
     auth9-core/src/domains/identity/service/password.rs \
     auth9-core/src/domains/identity/service/webauthn.rs \
     auth9-core/src/domains/provisioning/service/scim.rs \
     auth9-core/src/domains/tenant_access/service/saml_application.rs; do
     echo "FILE:$f"
     sed '/#\[cfg(test)\]/,$d' "$f" | rg -n "KeycloakClient|Arc<KeycloakClient>" || true
   done
   ```

### 预期结果
- `cargo test identity_engine` 通过
- 目标服务实现文件的非测试区域无 `KeycloakClient` 命中
- Keycloak 代码路径已在 Phase 5 FR3 中完全移除，业务层仅通过 `IdentityEngine` 抽象访问身份服务

---

## 场景 4：中性字段成为 QA 主断言路径

### 初始状态
- QA 文档已同步到本 FR 实现版本
- 本地可执行 `rg`

### 目的
验证文档与集成回归主路径已切换到 `identity_subject`、`provider_session_id`、`provider_alias`。

### 测试操作流程
1. 检查中性字段引用：
   ```bash
   rg -n "identity_subject|provider_session_id|provider_alias" docs/qa
   ```
2. 检查旧字段是否仅保留在 migration / Keycloak 兼容说明中：
   ```bash
   rg -n "keycloak_id|keycloak_session_id|keycloak_alias" docs/qa docs/security docs/uiux
   ```

### 预期结果
- `docs/qa` 中存在对 `identity_subject`、`provider_session_id`、`provider_alias` 的主路径断言
- `docs/uiux` 无旧字段引用
- 旧字段仅出现在 migration period、底层 Keycloak 兼容、或专门验证 Keycloak 集成行为的文档中

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 默认 `keycloak` backend 启动与注入链正常 | ☑ | 2026-03-17 | Codex | `docker inspect` 未发现 `IDENTITY_BACKEND` 覆盖，默认回落 `keycloak`；`/health` 返回 healthy |
| 2 | auth9_oidc backend wiring 验证 | ☑ | 2026-03-17 | Codex | `backend_switch_smoke_test` 已在 Phase 5 FR3 中移除；改用 `cargo test identity_engine` 验证 |
| 3 | 业务抽象边界保持稳定（Keycloak 已移除） | ☑ | 2026-03-17 | Codex | `keycloak_adapter_contract_test` 已在 Phase 5 FR3 中移除；`identity_engine` 通过；非测试区域扫描无 `KeycloakClient` 命中 |
| 4 | 中性字段成为 QA 主断言路径 | ☑ | 2026-03-17 | Codex | `rg` 确认 `identity_subject` / `provider_session_id` / `provider_alias` 已成为 QA 主断言字段 |
