> **本文档已归档** — Keycloak 解耦已完成，Auth9 已完全迁移至 auth9-oidc。此文档仅供历史参考。

---

# 解耦旧身份引擎类型: IdentityEngine Trait 重构验证

> **迁移已完成**: Keycloak 已被 Auth9 内置 OIDC 引擎完全替代。以下为历史迁移验证记录。

**模块**: identity_engine
**关联 FR**: Phase 5 FR2
**前置条件**: auth9-core 代码已编译

---

## 场景 1: 全量单元测试通过

### 步骤

```bash
cd auth9-core
cargo test 2>&1 | tail -20
```

### 预期结果

- 所有测试通过，输出 `test result: ok`，失败数为 0
- 无 `FAILED` 行出现
- 退出码为 0

---

## 场景 2: Clippy 静态分析无新增警告

### 步骤

```bash
cd auth9-core
cargo clippy --all-targets --all-features 2>&1 | grep -c "warning:"
```

### 预期结果

- `cargo clippy` 退出码为 0
- 无新增 warning（允许既有的、已标注 `#[allow(...)]` 的警告）
- 不出现与 `identity_engine`、`OidcClientRepresentation`、`RealmSettingsUpdate` 相关的 warning

---

## 场景 3: IdentityEngine trait 不再引用旧身份引擎类型

### 步骤

```bash
cd auth9-core
# 检查 trait 定义文件中是否存在旧身份引擎专有类型引用
grep -n "KeycloakOidcClient\|RealmUpdate\|keycloak::.*Client\|keycloak::.*Realm" src/identity_engine/mod.rs || echo "PASS: 无旧身份引擎类型引用"
```

### 预期结果

- 输出 `PASS: 无旧身份引擎类型引用`
- `identity_engine/mod.rs` 的 trait 方法签名中仅使用中性类型（`OidcClientRepresentation`、`RealmSettingsUpdate` 等）
- 不出现 `KeycloakOidcClient`、`RealmUpdate` 等旧身份引擎专有结构体名

---

## 场景 4: 中性类型定义存在于 identity_engine/types.rs

### 步骤

```bash
cd auth9-core
# 验证新增的中性类型已定义
grep -n "pub struct OidcClientRepresentation" src/identity_engine/types.rs && echo "OidcClientRepresentation: OK"
grep -n "pub struct RealmSettingsUpdate" src/identity_engine/types.rs && echo "RealmSettingsUpdate: OK"
```

### 预期结果

- 两条 grep 均匹配，输出对应的行号和 `OK` 确认
- `OidcClientRepresentation` 和 `RealmSettingsUpdate` 均为 `pub struct`，定义在 `identity_engine/types.rs` 中

---

## 场景 5: SmtpServerConfig 定义在 models/email.rs 并从 keycloak/types.rs 重导出

### 步骤

```bash
cd auth9-core
# 验证 SmtpServerConfig 的规范定义位置
grep -n "pub struct SmtpServerConfig" src/models/email.rs && echo "models/email.rs: OK"

# 验证 keycloak/types.rs 重导出（re-export）
grep -n "SmtpServerConfig" src/keycloak/types.rs && echo "keycloak/types.rs re-export: OK"
```

### 预期结果

- `SmtpServerConfig` 的 `pub struct` 定义位于 `src/models/email.rs`
- `src/keycloak/types.rs` 通过 `pub use` 或等价方式重导出该类型
- 既有代码中通过 `keycloak::types::SmtpServerConfig` 路径引用的地方仍可编译通过（场景 1 已覆盖）
