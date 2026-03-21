# Keycloak 遗留清理 Phase 1+2 验证

**模块**: integration
**关联 FR**: `docs/feature_request/cleanup_keycloak_remnants.md` (Phase 1 + Phase 2)
**前置条件**: auth9-core 和 auth9-portal 已构建部署

---

## 场景 1: Portal 服务集成页面显示中性错误文案

**目的**: 验证当客户端密钥无法获取时，Portal 显示中性错误信息（不包含 "Keycloak" 字样）。

**类型**: UI 验证

### 步骤

1. 登录 Portal 管理后台
2. 导航至任意服务的「Integration」标签页
3. 查看 Confidential 类型客户端的密钥区域
4. 若密钥获取失败，确认显示的错误信息

### 预期结果

- 英文环境：显示 `"Unable to retrieve client secret from identity backend"`
- 中文环境：显示 `"无法从身份后端获取客户端密钥"`
- 日文环境：显示 `"クライアントシークレットを取得できません"`
- **不应出现** "Keycloak" 字样

### 代码验证

```bash
# 确认 i18n 文件中不包含 keycloakUnavailable key
grep -r "keycloakUnavailable" auth9-portal/app/i18n/locales/
# 预期输出: 无匹配

# 确认新 key 存在
grep -r "clientSecretUnavailable" auth9-portal/app/i18n/locales/
# 预期输出: 3 个匹配 (en-US, zh-CN, ja)
```

---

## 场景 2: API 错误码返回 identity_backend_error

**目的**: 验证后端身份后端错误返回 `identity_backend_error` 而非 `keycloak_error`。

**类型**: API 验证

### 步骤 0: Gate Check

```bash
# 确认 API 健康
curl -sf http://localhost:8080/health && echo "OK"
```

### 步骤

1. 获取管理员 Token

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

2. 确认错误码映射

```bash
# 代码级验证 — 确认 error/mod.rs 中已无 keycloak_error
grep -n "keycloak_error" auth9-core/src/error/mod.rs
# 预期输出: 无匹配

# 确认新错误码存在
grep -n "identity_backend_error" auth9-core/src/error/mod.rs
# 预期输出: 1 个匹配
```

3. 确认 Portal 错误映射同步

```bash
grep "identity_backend_error" auth9-portal/app/lib/error-messages.ts
# 预期输出: identity_backend_error: "apiErrors.authServiceError"

grep "keycloak_error" auth9-portal/app/lib/error-messages.ts
# 预期输出: 无匹配
```

### 预期结果

- 后端错误码 `"keycloak_error"` 已替换为 `"identity_backend_error"`
- Portal 错误映射已同步更新
- 错误日志中显示 `"Identity backend error:"` 而非 `"Keycloak error:"`

---

## 场景 3: IdentitySyncService 正常工作（密码策略同步）

**目的**: 验证重命名后的 `IdentitySyncService` 功能正常，密码策略更新能正确同步到身份后端。

**类型**: 单元测试验证

### 步骤

```bash
cd auth9-core

# 运行 identity_sync 相关测试
cargo test identity_sync -- --nocapture
# 预期: 所有测试通过

# 运行密码策略相关测试
cargo test password -- --nocapture 2>&1 | grep "test result"
# 预期: test result: ok

# 确认旧模块名不再存在
ls src/domains/platform/service/keycloak_sync.rs 2>&1
# 预期: No such file or directory

ls src/domains/platform/service/identity_sync.rs
# 预期: 文件存在
```

### 预期结果

- `identity_sync` 模块所有测试通过
- `password` 模块所有测试通过（含 `identity_sync` 集成）
- 旧文件 `keycloak_sync.rs` 已删除
- 新文件 `identity_sync.rs` 包含 `IdentitySyncService` 结构体

---

## 场景 4: CLI seed 命令正常执行

**目的**: 验证 `seed_services()` 函数（原 `seed_keycloak()`）正常工作。

**类型**: 代码 + 集成验证

### 步骤

```bash
cd auth9-core

# 代码级验证
grep -n "seed_keycloak" src/main.rs src/migration/mod.rs
# 预期输出: 无匹配

grep -n "seed_services" src/main.rs src/migration/mod.rs
# 预期输出: 3 个匹配 (main.rs x2, migration/mod.rs x1)

# 如有 Docker 环境，可执行实际 seed
# cargo run -- seed
# 预期: "Seeding services with default data..." 日志输出
```

### 预期结果

- CLI `init` 和 `seed` 子命令调用 `seed_services()` 而非 `seed_keycloak()`
- 日志输出 `"Seeding services with default data..."` 而非 `"Seeding Keycloak with default data..."`
- 种子数据正常写入数据库

### 预期数据状态

```sql
-- 验证 Portal 服务已种子
SELECT slug FROM services WHERE slug = 'auth9-portal';
-- 预期: 1 行

-- 验证 Demo 服务已种子
SELECT slug FROM services WHERE slug = 'auth9-demo';
-- 预期: 1 行
```
