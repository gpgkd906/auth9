# Migration 统一治理

**类型**: 技术债务清理
**严重程度**: Medium
**影响范围**: auth9-core, auth9-oidc, database schema, Docker
**前置依赖**:
- `keycloak_phase3_local_credentials_and_mfa.md` (Phase 3 全部 FR 关闭)
**被依赖**:
- `keycloak_phase5_cutover_and_keycloak_retirement.md`

---

## 背景

auth9-core 和 auth9-oidc 共享同一个 `auth9` 数据库，但使用两套独立的 migration 机制：

| 项目 | 机制 | 跟踪表 | Rollback | 版本管理 |
|------|------|--------|----------|----------|
| auth9-core | `sqlx::migrate!()` | `_sqlx_migrations` | 支持 | 有序编号 |
| auth9-oidc | `include_str!` + raw SQL | 无 | 不支持 | 无 |

这导致以下问题：

1. **无版本跟踪**: auth9-oidc 的表变更无法追溯何时应用、是否已应用
2. **无 rollback**: auth9-oidc 的 schema 变更不可回退
3. **表归属模糊**: `credentials`、`pending_actions` 等表物理上在 `auth9` 数据库，但逻辑上属于 auth9-oidc
4. **冲突风险**: 两个 migration 系统无法共享 `_sqlx_migrations` 跟踪表

---

## 期望行为

### R1: 统一 migration 入口

要求：

- 所有 migration 由单一入口管理（auth9-core 的 `init` 子命令或独立的 migration runner）
- auth9-oidc 的 migration 文件移入统一管理的 migration 目录
- `_sqlx_migrations` 跟踪所有表的版本状态

### R2: auth9-oidc 去除 raw SQL migration

要求：

- 删除 `auth9-oidc/src/db.rs` 中的 `include_str!` + raw SQL 逻辑
- auth9-oidc 启动时不再自行建表，仅连接已初始化的数据库
- `auth9-oidc/Dockerfile` 不再需要 COPY migrations 目录

### R3: Schema 归属清晰化

评估两个方案并选择其一：

**方案 A: 单数据库、统一 migration**
- 所有表保留在 `auth9` 数据库
- auth9-oidc 的 migration 文件合入 `auth9-core/migrations/`
- 命名约定区分来源（如 `YYYYMMDD_oidc_*.sql`）

**方案 B: 独立数据库、独立 migration**
- auth9-oidc 使用独立的 `auth9_oidc` 数据库
- auth9-oidc 恢复使用 `sqlx::migrate!()` + 独立 `_sqlx_migrations` 表
- 跨库查询通过 service 层协调

### R4: Docker init 流程覆盖

要求：

- `auth9-init` 容器在启动时执行所有 migration（包括 auth9-oidc 的表）
- 幂等性保证：重复运行 init 不会失败
- `docker-compose up` 后所有表就绪，无需手动干预

---

## 非目标

- 本 FR 不要求数据迁移（仅 schema 管理方式变更）
- 本 FR 不要求 auth9-core 和 auth9-oidc 代码合并
- 本 FR 不要求变更现有表结构

---

## 执行时机

Phase 3 全部 FR 关闭后、Phase 4 开始前执行。此时 auth9-oidc 的表结构基本稳定。

---

## 验证方法

```bash
# 1. 全新环境启动后所有表就绪
docker-compose down -v
docker-compose up -d
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "SHOW TABLES;"
# 预期: 包含 credentials, pending_actions, email_verification_tokens, user_verification_status

# 2. Migration 跟踪完整
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "SELECT version, description FROM _sqlx_migrations ORDER BY version;"
# 预期: 包含 auth9-oidc 的 migration 记录

# 3. 重复启动幂等
docker-compose restart auth9-init
# 预期: 无错误

# 4. 单元测试通过
cd auth9-core && cargo test
cd auth9-oidc && cargo test
```
