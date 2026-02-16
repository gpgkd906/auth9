# Auth9-Core 领域化重构计划与任务清单

## 1. 目标
在不改变现有 REST/gRPC 外部行为的前提下，将 `auth9-core` 从按技术层组织（api/service/repository）重构为按领域内聚（modules by domain），降低单体复杂度、提升可维护性和演进效率。

## 2. 已确定约束
- 形态：单仓单 crate（先模块化单体）
- 迁移策略：增量迁移 + 兼容层
- 兼容要求：API/gRPC 完全兼容
- 优先顺序：Identity 主链路优先
- 交付策略：重构期仅接受 bugfix，暂停新功能

## 3. 分域设计
当前规划领域：
- `identity`
- `tenant_access`
- `authorization`
- `platform`
- `integration`
- `security_observability`

## 4. 执行策略（低风险）
每个模块按以下步骤迁移：
1. 将实现文件迁移到 `src/domains/<domain>/...`
2. 在原路径保留 shim（`pub use ...`）确保兼容
3. 路由层只依赖领域本地 facade（`domains::<domain>::api`）
4. 通过 `cargo check` + `api_test` 回归
5. 更新重构任务清单

## 5. 任务清单

### 5.1 基础设施与骨架
- [x] 新增 `src/domains/*` 骨架
- [x] `server::build_full_router` 改为按领域路由组合
- [x] 新增领域 Context traits
- [x] 新增 `DomainRouterState` 聚合约束
- [x] 新增边界守卫脚本 `scripts/check-domain-boundaries.sh`

### 5.2 Identity 领域迁移
- [x] 路由改为依赖 `domains::identity::api` facade
- [x] 迁移 `auth9-core/src/api/auth.rs` -> `domains/identity/api/auth.rs`
- [x] 迁移 `auth9-core/src/api/session.rs` -> `domains/identity/api/session.rs`
- [x] 迁移 `auth9-core/src/api/password.rs` -> `domains/identity/api/password.rs`
- [x] 迁移 `auth9-core/src/api/webauthn.rs` -> `domains/identity/api/webauthn.rs`
- [x] 迁移 `auth9-core/src/api/identity_provider.rs` -> `domains/identity/api/identity_provider.rs`
- [x] 迁移 `auth9-core/src/service/session.rs` -> `domains/identity/service/session.rs`
- [x] 迁移 `auth9-core/src/service/password.rs` -> `domains/identity/service/password.rs`
- [x] 迁移 `auth9-core/src/service/webauthn.rs` -> `domains/identity/service/webauthn.rs`
- [x] 迁移 `auth9-core/src/service/identity_provider.rs` -> `domains/identity/service/identity_provider.rs`
- [x] 迁移 `auth9-core/src/service/keycloak_oidc.rs` -> `domains/identity/service/keycloak_oidc.rs`

### 5.3 其他领域迁移（展开）
#### 5.3.1 tenant_access
- [x] 迁移 API：`tenant.rs` -> `domains/tenant_access/api/tenant.rs`
- [x] 迁移 API：`user.rs` -> `domains/tenant_access/api/user.rs`
- [x] 迁移 API：`invitation.rs` -> `domains/tenant_access/api/invitation.rs`
- [x] 迁移 Service：`tenant.rs` -> `domains/tenant_access/service/tenant.rs`
- [x] 迁移 Service：`user.rs` -> `domains/tenant_access/service/user.rs`
- [x] 迁移 Service：`invitation.rs` -> `domains/tenant_access/service/invitation.rs`
- [x] 路由改为仅依赖 `domains::tenant_access::api`

#### 5.3.2 authorization
- [x] 迁移 API：`service.rs` -> `domains/authorization/api/service.rs`
- [x] 迁移 API：`role.rs` -> `domains/authorization/api/role.rs`
- [x] 迁移 API：`tenant_service.rs` -> `domains/authorization/api/tenant_service.rs`
- [x] 迁移 Service：`client.rs` -> `domains/authorization/service/client.rs`
- [x] 迁移 Service：`rbac.rs` -> `domains/authorization/service/rbac.rs`
- [x] 路由改为仅依赖 `domains::authorization::api`

#### 5.3.3 platform
- [x] 迁移 API：`system_settings.rs` -> `domains/platform/api/system_settings.rs`
- [x] 迁移 API：`email_template.rs` -> `domains/platform/api/email_template.rs`
- [x] 迁移 API：`branding.rs` -> `domains/platform/api/branding.rs`
- [x] 迁移 Service：`system_settings.rs` -> `domains/platform/service/system_settings.rs`
- [x] 迁移 Service：`email.rs` -> `domains/platform/service/email.rs`
- [x] 迁移 Service：`email_template.rs` -> `domains/platform/service/email_template.rs`
- [x] 迁移 Service：`branding.rs` -> `domains/platform/service/branding.rs`
- [x] 迁移 Service：`keycloak_sync.rs` -> `domains/platform/service/keycloak_sync.rs`
- [x] 路由改为仅依赖 `domains::platform::api`

#### 5.3.4 integration
- [x] 迁移 API：`webhook.rs` -> `domains/integration/api/webhook.rs`
- [x] 迁移 API：`keycloak_event.rs` -> `domains/integration/api/keycloak_event.rs`
- [x] 迁移 API：`action.rs` -> `domains/integration/api/action.rs`
- [x] 迁移 Service：`webhook.rs` -> `domains/integration/service/webhook.rs`
- [x] 迁移 Service：`action.rs` -> `domains/integration/service/action.rs`
- [x] 迁移 Service：`action_engine.rs` -> `domains/integration/service/action_engine.rs`
- [x] 路由改为仅依赖 `domains::integration::api`

#### 5.3.5 security_observability
- [x] 迁移 API：`analytics.rs` -> `domains/security_observability/api/analytics.rs`
- [x] 迁移 API：`security_alert.rs` -> `domains/security_observability/api/security_alert.rs`
- [x] 迁移 API：`audit.rs` -> `domains/security_observability/api/audit.rs`
- [x] 迁移 API：`health.rs` -> `domains/security_observability/api/health.rs`
- [x] 迁移 Service：`analytics.rs` -> `domains/security_observability/service/analytics.rs`
- [x] 迁移 Service：`security_detection.rs` -> `domains/security_observability/service/security_detection.rs`
- [x] 路由改为仅依赖 `domains::security_observability::api`

#### 5.3.6 兼容层与边界收口
- [x] 旧 `src/api/*.rs`（除 `metrics.rs`）全部降级为 shim `pub use crate::domains::...`
- [x] 旧 `src/service/*.rs`（除 `mod.rs`）全部降级为 shim `pub use crate::domains::...`
- [x] `server/mod.rs` 不再直接注册 `api::*` 路由（仅聚合领域路由）

### 5.4 质量与治理
- [x] `cargo check` 通过（当前改造集）
- [x] `cargo test --test api_test` 全绿（566 通过）
- [x] 边界守卫脚本通过
- [x] 将边界守卫接入 CI
- [x] 更新 `docs/architecture.md` 反映新结构
- [x] 输出迁移开发规范（新增模块模板、跨域调用规则）

## 6. 已完成里程碑（当前状态）
1. 完成路由层按领域拆分与组合。
2. 完成 Context 聚合约束，降低 server 层泛型复杂度。
3. 完成六大领域核心 API/Service 的物理迁移与兼容 shim。
4. 保持 API 行为与测试稳定（`api_test` 全绿）。

## 7. 已完成 / 未完成总览
- 已完成：5.1、5.2、5.3、5.4 全部任务。
- 未完成：无。

## 8. 下一步（进行中）
- 本轮领域化重构计划已 100% 完成；下一期可评估删除 shim 兼容层并收敛旧目录导出。
