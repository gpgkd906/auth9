# Phase 5: Keycloak 全量退役（封闭开发）

**类型**: 架构收尾 + 代码清理
**严重程度**: High
**影响范围**: auth9-core, auth9-portal, auth9-keycloak-theme, auth9-keycloak-events, docker-compose, deploy, docs
**前置依赖**:
- `keycloak_phase1_identity_engine_abstraction.md` (CLOSED)
- `keycloak_phase2_hosted_login_and_session_frontend.md` (CLOSED)
- `keycloak_phase3_local_credentials_and_mfa.md` (CLOSED)
- `keycloak_phase4_external_identity_broker.md` (CLOSED)

---

## 背景

Phase 1-4 已完成全部功能替换：

- Phase 1: 身份引擎抽象层（IdentityEngine trait + 双后端 adapter）
- Phase 2: 托管登录 + 本地浏览器会话
- Phase 3: 本地凭据、密码、MFA、邮件动作
- Phase 4: 外部身份代理（Social / Enterprise OIDC / SAML / 联邦链接）

原 Phase 5 设计为渐进灰度切换（双栈运行、数据迁移、影子比对、回滚策略），适用于生产在线迁移场景。

**方案变更**：改为封闭开发，全量切换到 auth9_oidc，不保留 Keycloak 并行能力。这使得原 R1-R4 不再需要：

| 原需求 | 新方案 |
|--------|--------|
| R1 双栈运行 | 不需要 — 直接切换 |
| R2 数据迁移工具 | 不需要 — 全新部署 |
| R3 只读回放与结果比对 | 不需要 — 直接测试 auth9_oidc |
| R4 回滚策略 | 不需要 — 不保留回滚 |
| R5 退役 Keycloak | **主要工作** — 分解为 FR1-FR5 |
| R6 文档与运维收尾 | 保留 — FR6 |

---

## 期望行为

### R1: Auth9Oidc Adapter 完整可用

Auth9OidcIdentityEngineAdapter 的所有 sub-store（UserStore、ClientStore、SessionStore）不再有 "not implemented" 占位，能处理全部 IdentityEngine 操作。

### R2: IdentityEngine Trait 不依赖 Keycloak 类型

Trait 签名中不出现 `KeycloakOidcClient`、`RealmUpdate` 等 Keycloak 模块类型。

### R3: Keycloak 代码路径完全移除

- `IdentityBackend` 枚举删除，auth9_oidc 为唯一后端
- OIDC flow / logout 中的 Keycloak 分支删除
- `auth9-core/src/keycloak/` 模块删除
- `auth9-core/src/identity_engine/adapters/keycloak/` 删除
- `keycloak_client.rs` 删除

### R4: Config 中无 Keycloak 残留

- `KeycloakConfig` 重构为 `IdentityConfig`
- `KEYCLOAK_*` 环境变量不再出现

### R5: 基础设施中无 Keycloak 依赖

- docker-compose 中无 Keycloak service
- Portal 中无 Keycloak redirect 模式
- `auth9-keycloak-theme/` 和 `auth9-keycloak-events/` 删除
- K8s deploy 中无 Keycloak 资源

### R6: 文档同步到新架构

- CLAUDE.md、architecture.md、README.md 更新
- QA/security/uiux 文档同步

---

## 非目标

- 不保留对 Keycloak 数据结构的长期兼容
- 不保留 Keycloak 作为可选后端
- 不迁移已过期 session / token 的历史存量

---

## 子 FR 文档

| FR | 文件 | 内容 |
|----|------|------|
| FR1 | `keycloak_phase5_fr1_complete_auth9_oidc_adapter.md` | 补全 Auth9Oidc Adapter 缺失实现 |
| FR2 | `keycloak_phase5_fr2_decouple_keycloak_types.md` | 解耦 Keycloak 类型与 IdentityEngine Trait |
| FR3 | `keycloak_phase5_fr3_remove_keycloak_code_paths.md` | 移除 IdentityBackend 开关与 Keycloak 代码路径 |
| FR4 | `keycloak_phase5_fr4_refactor_config.md` | 重构 Config: KeycloakConfig → IdentityConfig |
| FR5 | `keycloak_phase5_fr5_infrastructure_cleanup.md` | 基础设施与 Portal 清理 |
| FR6 | `keycloak_phase5_fr6_documentation_update.md` | 文档更新 |

## 实现顺序

```
FR1 → FR2 → FR3 → FR4 → FR5 → FR6
```

每个 FR 独立可测试，`cargo test` 必须在每步之后通过。

---

## 验证方法

### 最终验证

```bash
# 源码中不应有 keycloak 运行时引用
rg -n "keycloak" auth9-core/src/ --type rust | grep -v "test" | grep -v "comment"

# 基础设施中不应有 keycloak
rg -n "keycloak" docker-compose.yml deploy/

# 所有测试通过
cd auth9-core && cargo test
cd auth9-portal && npm run test
```

### 端到端验证

1. `IDENTITY_BACKEND` 环境变量不再存在
2. `docker-compose up -d` 不启动 Keycloak 容器
3. Portal hosted login 完整可用
4. 密码登录 → token 签发 → refresh → userinfo 全链路通过
5. Social login / Enterprise SSO 全链路通过
