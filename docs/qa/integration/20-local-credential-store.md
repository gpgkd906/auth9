# 20 - 本地 Credential Store (Phase 3 FR1)

| 项目 | 值 |
|------|-----|
| **模块** | auth9-oidc / credential |
| **关联 FR** | `keycloak_phase3_fr1_local_credential_store.md` |
| **前置依赖** | Phase 1 FR1/FR2/FR4 已完成 |
| **涉及表** | `credentials`, `user_verification_status`, `pending_actions` |

---

## 场景 1: Credential 模型中性语义验证

**目的**: 验证 credential 数据模型不包含任何 `keycloak_*` 语义命名

### 步骤

1. 在 `auth9-oidc/src/` 下搜索 keycloak 相关命名：
```bash
rg -i "keycloak" auth9-oidc/src/ --type rust
```

2. 验证 `CredentialType` 枚举值均为中性命名：
```bash
rg "CredentialType" auth9-oidc/src/models/credential.rs
```

### 预期结果

- 步骤 1: 除注释外无任何 `keycloak` 引用
- 步骤 2: 枚举值为 `Password`, `Totp`, `RecoveryCode`, `WebAuthn`

---

## 场景 2: Migration 文件完整性验证

**目的**: 验证 OIDC 相关 migration 文件包含所有必需表结构

> **注意**: OIDC 相关 migration 文件位于 `auth9-core/migrations/`（非 `auth9-oidc/migrations/`），文件名以 `oidc_` 前缀标识。这是因为 auth9-oidc 共享 auth9-core 的 TiDB 数据库，由 auth9-core 的 `sqlx::migrate!()` 统一管理。

### 步骤

1. 检查 migration 文件存在：
```bash
ls auth9-core/migrations/*oidc*
```

2. 验证 `credentials` 表包含必需字段：
```bash
rg "credential_type|credential_data" auth9-core/migrations/20260318000001_oidc_create_credentials.sql
```

3. 验证 `user_verification_status` 表包含 email 验证字段：
```bash
rg "email_verified" auth9-core/migrations/20260318000002_oidc_create_user_verification_status.sql
```

4. 验证 `pending_actions` 表包含 action 状态字段：
```bash
rg "action_type|status|completed_at" auth9-core/migrations/20260318000003_oidc_create_pending_actions.sql
```

### 预期结果

- 4 个 migration 文件存在（`credentials`、`user_verification_status`、`pending_actions`、`email_verification_tokens`）
- `credentials` 表包含 `credential_type`, `credential_data`, `is_active` 字段
- `user_verification_status` 表包含 `email_verified`, `email_verified_at` 字段
- `pending_actions` 表包含 `action_type`, `status`, `completed_at` 字段
- `email_verification_tokens` 表包含 `token_hash`, `expires_at`, `used_at` 字段
- 所有表无 `FOREIGN KEY` 约束 (TiDB 规则)

---

## 场景 3: Repository Trait 契约完整性

**目的**: 验证 repository 层提供完整的 CRUD + 状态切换能力

### 步骤

1. 验证 `CredentialRepository` trait 方法覆盖：
```bash
rg "async fn" auth9-oidc/src/repository/credential.rs | head -20
```

2. 验证 `VerificationRepository` trait 方法：
```bash
rg "async fn" auth9-oidc/src/repository/verification.rs | head -10
```

3. 验证 `PendingActionRepository` trait 方法：
```bash
rg "async fn" auth9-oidc/src/repository/pending_action.rs | head -10
```

### 预期结果

- `CredentialRepository`: create, find_by_id, find_by_user_and_type, update_data, deactivate, activate, delete, delete_all_by_user, delete_by_user_and_type
- `VerificationRepository`: get_or_create, set_email_verified
- `PendingActionRepository`: create, find_pending_by_user, complete, cancel, delete_by_user

---

## 场景 4: Contract Tests 通过

**目的**: 验证所有 credential 相关合约测试通过

### 步骤 0 (Gate Check)

确保 Rust 工具链可用：
```bash
rustc --version && cargo --version
```

### 步骤

1. 运行 credential 相关测试：
```bash
cd auth9-oidc && cargo test credential 2>&1
```

2. 运行 verification 相关测试：
```bash
cd auth9-oidc && cargo test verification 2>&1
```

3. 运行 pending_action 相关测试：
```bash
cd auth9-oidc && cargo test pending_action 2>&1
```

4. 运行全量测试确认无回归：
```bash
cd auth9-oidc && cargo test 2>&1
```

### 预期结果

- 步骤 1: 至少 7 个 credential 测试通过
- 步骤 2: 至少 3 个 verification 测试通过
- 步骤 3: 至少 5 个 pending_action 测试通过
- 步骤 4: 全部 39 测试通过，0 失败

---

## 场景 5: 与 Auth9 内置 OIDC 引擎路径无冲突

**目的**: 验证新增代码不破坏 auth9-core 现有运行路径

### 步骤

1. 验证 auth9-core identity engine adapter 未被修改：
```bash
git diff --name-only auth9-core/src/identity_engine/
```

2. 验证 auth9-core 编译和测试不受影响：
```bash
cd auth9-core && cargo build 2>&1 | tail -5
cd auth9-core && cargo test --lib 2>&1 | tail -10
```

### 预期结果

- 步骤 1: 无文件被修改
- 步骤 2: auth9-core 编译成功，现有测试全部通过
