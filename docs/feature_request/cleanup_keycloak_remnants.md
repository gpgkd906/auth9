# Keycloak 遗留清理计划

**类型**: 重构 / 技术债务清理
**严重程度**: Medium
**影响范围**: auth9-core (Backend), auth9-portal (Frontend), CI/CD, 文档
**前置依赖**: auth9-oidc 引擎已完成并上线
**被依赖**: 无

---

## 背景

Auth9 已完成核心 OIDC 引擎从 Keycloak 到 auth9-oidc 的迁移。身份后端已通过 `IdentityEngine` trait 实现抽象，`IDENTITY_BACKEND` 配置项可切换 `keycloak` / `auth9_oidc`。

然而代码库中仍残留 **202 个文件** 包含 Keycloak 引用，涵盖源代码命名、数据库列名、API 端点路径、用户可见文案、CI/CD 构建步骤和文档。这些遗留物增加新成员理解成本，且部分用户可见文案仍指向已不存在的 Keycloak 实例。

### 清理原则

- **不破坏运行时行为**: 每个阶段独立可部署，不引入 breaking change
- **数据库列保留到最后**: Schema 变更风险最高，放在最终阶段
- **先用户可见，后内部命名**: 优先修复用户会看到的文案和错误信息

---

## Phase 1: 用户可见文案与错误信息

> **风险**: 低 — 仅修改字符串常量和 i18n 文件，不涉及逻辑变更

### R1.1: Portal i18n 文案更新

将所有用户可见的 Keycloak 引用替换为中性表述：

| 文件 | 当前值 | 目标值 |
|------|--------|--------|
| `auth9-portal/app/i18n/locales/en-US.ts` | `keycloakUnavailable`: "Unable to retrieve - check Keycloak" | "Unable to retrieve client secret from identity backend" |
| `auth9-portal/app/i18n/locales/zh-CN.ts` | "无法获取，请检查 Keycloak" | "无法从身份后端获取客户端密钥" |
| `auth9-portal/app/i18n/locales/ja.ts` | 对应日文翻译 | 中性表述 |

同时将 i18n key 从 `keycloakUnavailable` 重命名为 `clientSecretUnavailable`，并更新引用处：

**涉及文件**:
- `auth9-portal/app/i18n/locales/en-US.ts`
- `auth9-portal/app/i18n/locales/zh-CN.ts`
- `auth9-portal/app/i18n/locales/ja.ts`
- `auth9-portal/app/components/services/service-integration-tab.tsx` — 引用 key 更新

### R1.2: 后端错误变体重命名

将 `"keycloak_error"` 错误码替换为 `"identity_backend_error"`：

```rust
// auth9-core/src/error/mod.rs
// 当前
"keycloak_error" => ...
// 目标
"identity_backend_error" => ...
```

> **注意**: 如果 Portal 前端有对 `"keycloak_error"` 的字符串匹配，需同步更新。

**涉及文件**:
- `auth9-core/src/error/mod.rs`
- `auth9-portal/app/lib/error-messages.ts` — 检查是否有对应匹配

### R1.3: OpenAPI / API 文档中的 Keycloak 引用

检查并更新 `auth9-core/src/openapi.rs` 中的描述文本。

**涉及文件**:
- `auth9-core/src/openapi.rs`

---

## Phase 2: 核心服务与模块重命名

> **风险**: 中 — 涉及 Rust 模块路径和结构体重命名，需全面 grep 确认引用

### R2.1: KeycloakSyncService 重命名

将 `KeycloakSyncService` 重命名为 `IdentitySyncService`，该服务通过 `IdentityEngine` trait 操作身份后端，命名不应绑定具体实现：

| 当前 | 目标 |
|------|------|
| `keycloak_sync.rs` | `identity_sync.rs` |
| `KeycloakSyncService` | `IdentitySyncService` |
| `keycloak_sync` 字段名 | `identity_sync` |

**涉及文件**:
- `auth9-core/src/domains/platform/service/keycloak_sync.rs` → 重命名为 `identity_sync.rs`
- `auth9-core/src/domains/platform/service/mod.rs` — 模块声明
- `auth9-core/src/server/mod.rs` — 实例化和注入
- `auth9-core/src/domains/identity/service/password.rs` — `keycloak_sync` 字段
- `auth9-core/src/domains/platform/service/branding.rs` — 依赖注入
- `auth9-core/src/domains/platform/service/system_settings.rs` — 依赖注入

### R2.2: seed_keycloak() 重命名

`seed_keycloak()` 实际功能是初始化 Portal/Demo 服务数据，已不再需要 Keycloak：

| 当前 | 目标 |
|------|------|
| `seed_keycloak()` | `seed_services()` |
| CLI 子命令 `seed` 的内部调用 | 保持 CLI 接口不变 |

**涉及文件**:
- `auth9-core/src/migration/mod.rs` — 函数重命名
- `auth9-core/src/main.rs` — 调用处更新

### R2.3: SAML/OIDC 构建函数重命名

| 当前 | 目标 |
|------|------|
| `build_keycloak_saml_client()` | `build_saml_client_representation()` |
| 相关测试函数名 | 同步更新 |

**涉及文件**:
- `auth9-core/src/domains/tenant_access/service/saml_application.rs`
- 对应测试模块

### R2.4: WebAuthn 辅助函数重命名

| 当前 | 目标 |
|------|------|
| `get_keycloak_user_id()` | `get_identity_subject()` |

**涉及文件**:
- `auth9-core/src/domains/identity/api/webauthn.rs`

### R2.5: Email 模型方法重命名

| 当前 | 目标 |
|------|------|
| `to_keycloak_smtp()` | `to_backend_smtp_config()` |

**涉及文件**:
- `auth9-core/src/models/email.rs`
- 所有调用处

---

## Phase 3: API 端点路径迁移

> **风险**: 中高 — 涉及公开 API 路径变更，需考虑向后兼容

### R3.1: Webhook 端点路径迁移

将 `/api/v1/keycloak/events` 迁移到 `/api/v1/identity/events`：

**策略**: 保留旧路径作为别名（转发到新处理器），设置 `Deprecation` 响应头，在后续版本中移除。

```rust
// 新路径
.route("/api/v1/identity/events", post(handle_identity_event))
// 旧路径（兼容，标记 deprecated）
.route("/api/v1/keycloak/events", post(handle_identity_event_deprecated))
```

**涉及文件**:
- `auth9-core/src/domains/integration/api/keycloak_event.rs` → 重命名为 `identity_event.rs`
- `auth9-core/src/domains/integration/api/mod.rs` — 模块声明
- `auth9-core/src/domains/integration/routes.rs` — 路由注册
- `auth9-core/src/server/mod.rs` — 路由挂载

### R3.2: Webhook 测试文件重命名

| 当前 | 目标 |
|------|------|
| `tests/domains/integration/keycloak_event_http_test.rs` | `identity_event_http_test.rs` |
| `tests/domains/integration/mod.rs` | 更新模块声明 |

**涉及文件**:
- `auth9-core/tests/domains/integration/keycloak_event_http_test.rs` → 重命名
- `auth9-core/tests/domains/integration/mod.rs`

---

## Phase 4: 配置与环境变量清理

> **风险**: 中 — 需要协调部署配置同步更新

### R4.1: 环境变量默认值更新

| 文件 | 变更 |
|------|------|
| `.env.example` | `IDENTITY_BACKEND=keycloak` → `IDENTITY_BACKEND=auth9_oidc` |
| `auth9-core/.env.example` | 将 `KEYCLOAK_*` 变量标记为 `# [DEPRECATED]`，添加注释说明仅在 `IDENTITY_BACKEND=keycloak` 时需要 |

**涉及文件**:
- `.env.example`
- `auth9-core/.env.example`

### R4.2: 配置读取代码清理

检查 `auth9-core/src/config/mod.rs` 中 Keycloak 相关配置项，确认哪些可在 `auth9_oidc` 模式下移除：

- `KEYCLOAK_URL` — auth9_oidc 模式不需要
- `KEYCLOAK_REALM` — auth9_oidc 模式不需要
- `KEYCLOAK_ADMIN_*` — auth9_oidc 模式不需要
- `KEYCLOAK_WEBHOOK_SECRET` → 重命名为 `IDENTITY_WEBHOOK_SECRET`（保留旧名作为 fallback）

**涉及文件**:
- `auth9-core/src/config/mod.rs`
- 对应的测试函数（如 `test_from_env_keycloak_webhook_secret`）

### R4.3: K8s Secrets 示例更新

更新 `deploy/k8s/secrets.yaml.example`，将 Keycloak 相关密钥标记为可选或移除。

**涉及文件**:
- `deploy/k8s/secrets.yaml.example`

---

## Phase 5: Portal 类型定义与 API 响应清理

> **风险**: 中 — 需要前后端同步变更

### R5.1: API 响应字段中性化

后端 API 响应中仍包含 `keycloak_*` 字段名，需迁移为中性名称：

| 当前字段 | 目标字段 | 涉及接口 |
|---------|---------|---------|
| `keycloak_client_id` | `backend_client_id` | SAML Application CRUD |
| `keycloak_alias` | `provider_alias` | Enterprise SSO |

**策略**: 响应中同时返回新旧字段名（兼容期），Portal 切换到新字段名后，后续版本移除旧字段。

**涉及文件**:
- `auth9-core/src/models/saml_application.rs` — 添加 `#[serde(alias)]`
- `auth9-core/src/models/identity_provider.rs`
- `auth9-portal/app/services/api/saml-application.ts` — 类型定义
- `auth9-portal/app/services/api/enterprise-sso.ts` — 类型定义

---

## Phase 6: 数据库 Schema 清理

> **风险**: 高 — 涉及生产数据库迁移，需要停机窗口或在线 DDL
> **前置条件**: Phase 2-5 全部完成且已在生产运行稳定

### R6.1: 移除旧列的 COALESCE 回退

确认所有查询已切换到中性列名后，移除 SQL 中的 `COALESCE(identity_subject, keycloak_id)` 等回退逻辑，直接使用中性列。

**涉及文件**:
- `auth9-core/src/repository/user/impl_repo.rs`
- `auth9-core/src/repository/session/impl_repo.rs`
- `auth9-core/src/domains/tenant_access/api/tenant_sso.rs`

### R6.2: 数据库迁移 — DROP 旧列

新增迁移文件，移除已被中性列替代的旧列：

```sql
-- 移除 users 表旧列
ALTER TABLE users DROP COLUMN keycloak_id;

-- 移除 sessions 表旧列
ALTER TABLE sessions DROP COLUMN keycloak_session_id;

-- 移除 enterprise_sso_providers 表旧列
ALTER TABLE enterprise_sso_providers DROP COLUMN keycloak_alias;

-- 移除 saml_applications 表旧列
ALTER TABLE saml_applications DROP COLUMN keycloak_client_id;
```

> **注意**: TiDB 的 `ALTER TABLE DROP COLUMN` 是在线 DDL，不会阻塞读写。但需确认所有 Repository 查询已不再引用旧列。

**涉及文件**:
- `auth9-core/migrations/` — 新增迁移文件

### R6.3: Repository 查询最终清理

移除所有 SQL 中对 `keycloak_*` 列的绑定和读取。

**涉及文件**:
- `auth9-core/src/repository/user/impl_repo.rs`
- `auth9-core/src/repository/session/impl_repo.rs`
- `auth9-core/src/repository/saml_application.rs`

---

## Phase 7: CI/CD 与工具链清理

> **风险**: 中 — 需确认 Keycloak 主题/事件插件是否仍在使用

### R7.1: 评估 Keycloak 扩展项目

确认以下子项目在 auth9-oidc 模式下是否仍需要：

| 项目 | 用途 | 决策 |
|------|------|------|
| `auth9-keycloak-theme` | 自定义登录主题 | auth9-oidc 有自己的 Hosted Login，不再需要 |
| `auth9-keycloak-events` | 事件监听 SPI 插件 | auth9-oidc 直接推送事件，不再需要 |

如确认不再需要：
- 移除 `.github/workflows/ci.yml` 中 `Build auth9-keycloak-theme image` 步骤
- 移除 `.github/workflows/cd.yml` 中 `THEME_IMAGE_NAME` 和 `EVENTS_IMAGE_NAME` 相关步骤
- 归档或删除子项目目录

### R7.2: Claude 技能文件清理

| 文件 | 处理 |
|------|------|
| `.claude/skills/keycloak-theme/SKILL.md` | 移除（Keycloakify 技能不再适用） |

### R7.3: Cursor 规则更新

更新 `.cursor/rules/` 中引用 Keycloak 的规则文件。

**涉及文件**:
- `.cursor/rules/testing-conventions.mdc`
- `.cursor/rules/project-overview.mdc`

---

## Phase 8: 文档与 Wiki 更新

> **风险**: 低 — 纯文档变更

### R8.1: Wiki 清理

| 文件 | 处理 |
|------|------|
| `wiki/Keycloak主题定制.md` | 归档或标记废弃（内容不再适用） |
| `wiki/架构设计.md` | 更新架构描述，移除 Keycloak 组件引用 |
| `wiki/认证流程.md` | 更新为 auth9-oidc 流程描述 |
| `wiki/快速开始.md` | 移除 Keycloak 配置步骤 |
| `wiki/本地开发.md` | 移除 Keycloak 容器相关说明 |
| `wiki/运维手册.md` | 移除 Keycloak 运维内容 |
| `wiki/配置说明.md` | 标记 Keycloak 配置项为废弃 |
| `wiki/故障排查.md` | 移除 Keycloak 相关排查项 |
| 其他 wiki 文件 | 逐一检查并更新 |

### R8.2: QA 文档清理

以下 QA 文档记录了 Keycloak 解耦过程，在清理完成后可归档：

- `docs/qa/integration/18-business-layer-keycloak-decoupling.md`
- `docs/qa/integration/22-config-keycloak-retirement.md`
- `docs/qa/identity_engine/decouple_keycloak_types.md`

其余 QA 文档中的 Keycloak 引用需逐一检查并更新为中性表述。

### R8.3: 项目根文档更新

- `docs/architecture.md` — 更新架构图和组件描述
- `README-zh.md` / `README-ja.md` — 移除 Keycloak 相关描述
- `AGENTS.md` — 更新 agent 上下文描述
- `auth9-threat-model.md` — 更新威胁模型中的 Keycloak 引用
- `userguide/USER_GUIDE.md` — 更新用户指南

---

## 实施顺序与里程碑

```
Phase 1 (用户可见文案)      ← 立即可做，零风险
  ↓
Phase 2 (核心服务重命名)    ← 纯重构，cargo test 验证
  ↓
Phase 3 (API 端点迁移)      ← 需兼容旧路径
  ↓
Phase 4 (配置清理)          ← 需协调部署配置
  ↓
Phase 5 (API 响应字段)      ← 前后端同步
  ↓
Phase 6 (数据库 Schema)     ← 最高风险，最后执行
  ↓
Phase 7 (CI/CD 清理)        ← 确认子项目废弃后执行
  ↓
Phase 8 (文档更新)          ← 可与任何阶段并行
```

Phase 1-2 可独立于部署执行。Phase 3-5 建议在同一个发布窗口完成。Phase 6 需要在 Phase 2-5 生产稳定后执行。Phase 7-8 可随时执行。

---

## 验证方法

### 每阶段通用验证

```bash
# 确认 Keycloak 引用减少
grep -ri "keycloak" auth9-core/src/ auth9-portal/app/ | wc -l

# 后端编译与测试
cd auth9-core && cargo build && cargo test

# 前端编译与测试
cd auth9-portal && npm run build && npm run test

# Lint 检查
cd auth9-core && cargo clippy
cd auth9-portal && npm run lint && npm run typecheck
```

### 最终验证（全部阶段完成后）

```bash
# 预期结果: 源代码目录中 Keycloak 引用为 0
grep -ri "keycloak" auth9-core/src/ auth9-portal/app/ --include="*.rs" --include="*.ts" --include="*.tsx" | grep -v "// DEPRECATED" | wc -l
# 期望输出: 0

# 迁移文件中的引用可保留（历史记录）
grep -ri "keycloak" auth9-core/migrations/ | wc -l
# 允许存在（迁移文件是不可变的历史记录）
```

---

## 参考

- 中性 Schema 迁移: `auth9-core/migrations/20260317000002_add_neutral_identity_columns.sql`
- IdentityEngine trait: `auth9-core/src/domains/platform/service/identity_engine.rs`（抽象层）
- 现有 QA 文档: `docs/qa/integration/22-config-keycloak-retirement.md`

---

## Implementation Log

### 2026-03-21 — Phase 3-5, 7-8

- **Fulfilled**: Phase 3 (R3.1, R3.2, R3.3), Phase 4 (R4.1, R4.2, R4.3, R4.4), Phase 5 (R5.1, R5.3), Phase 7 (R7.1, R7.2, R7.3, R7.4), Phase 8 (R8.1, R8.2, R8.3)
- **Deferred**: Phase 5 R5.2 (keycloak_client_id → backend_client_id, coupled to Phase 6), Phase 6 (DB schema DROP COLUMN)
- **Remaining**: Phase 6 (requires Phase 3-5 stable in production)
- **QA Status**: 5/5 passed (cargo test 2507/2507, npm test 1262/1262)
- **QA Document**: docs/qa/integration/25-keycloak-cleanup-phase3-5.md
- **Tickets**: None
- **Notes**: Phase 6 (DB DROP COLUMN) 需要 Phase 3-5 在生产环境稳定运行后执行。剩余 keycloak 源码引用主要为 DB 列名（COALESCE 回退）和内部注释。

### 2026-03-20 — Phase 1-2

- **Fulfilled**: Phase 1 (R1.1, R1.2, R1.3), Phase 2 (R2.1, R2.2, R2.3, R2.4, R2.5)
- **QA Status**: Pending (cargo test 620/620, npm test 1262/1262)
- **QA Document**: docs/qa/integration/24-keycloak-cleanup-phase1-2.md
- **Tickets**: None
- **Notes**: Phase 1+2 纯重构，所有现有测试通过。
