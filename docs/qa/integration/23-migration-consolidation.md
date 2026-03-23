# Migration 统一治理

**模块**: Integration / Infrastructure
**测试范围**: 验证 auth9-oidc migration 统一至 auth9-core 后，全量启动、幂等性、表完整性均正常
**场景数量**: 4
**优先级**: Medium

---

## 背景

auth9-core 与 auth9-oidc 共享 `auth9` 数据库。原先 auth9-oidc 通过 `include_str!` + raw SQL 在启动时自行建表，无 `_sqlx_migrations` 跟踪。本次治理将 auth9-oidc 的 4 个 migration 文件迁入 `auth9-core/migrations/`，由 `auth9-core init` 统一管理。

### 涉及的 migration 文件

| 文件 | 来源 |
|------|------|
| `20260318000001_oidc_create_credentials.sql` | auth9-oidc |
| `20260318000002_oidc_create_user_verification_status.sql` | auth9-oidc |
| `20260318000003_oidc_create_pending_actions.sql` | auth9-oidc |
| `20260318000004_oidc_create_email_verification_tokens.sql` | auth9-oidc |

### 数据库验证 SQL

```sql
-- 验证 OIDC 表存在
SHOW TABLES LIKE 'credentials';
SHOW TABLES LIKE 'user_verification_status';
SHOW TABLES LIKE 'pending_actions';
SHOW TABLES LIKE 'email_verification_tokens';

-- 验证 _sqlx_migrations 包含 OIDC migration 记录
SELECT version, description FROM _sqlx_migrations
WHERE description LIKE '%oidc%'
ORDER BY version;
```

---

## 场景 1: 全新环境启动后所有表就绪

**初始状态**: 无数据库（`docker-compose down -v` 清除所有卷）

**目的**: 验证 `auth9-init` 容器执行 `auth9-core init` 后，auth9-oidc 的 4 张表被正确创建

### 步骤

1. 清除所有 Docker 卷并重新启动

```bash
docker-compose down -v
docker-compose up -d
```

2. 等待 `auth9-init` 完成

```bash
docker wait auth9-init
```

3. 验证 OIDC 表已创建

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT TABLE_NAME FROM information_schema.TABLES
WHERE TABLE_SCHEMA = 'auth9'
AND TABLE_NAME IN ('credentials', 'user_verification_status', 'pending_actions', 'email_verification_tokens')
ORDER BY TABLE_NAME;
"
```

### 预期结果

- 返回 4 行，包含：`credentials`、`email_verification_tokens`、`pending_actions`、`user_verification_status`

### 预期数据状态

```sql
-- _sqlx_migrations 应包含 OIDC migration 记录
SELECT COUNT(*) AS oidc_migration_count FROM _sqlx_migrations
WHERE description LIKE '%oidc%';
-- 预期: oidc_migration_count = 4
```

---

## 场景 2: Migration 版本跟踪完整

**初始状态**: 场景 1 完成后

**目的**: 验证 `_sqlx_migrations` 表中记录了 auth9-oidc 的 4 个 migration，且版本号和描述正确

### 步骤

1. 查询 migration 跟踪表

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT version, description, success
FROM _sqlx_migrations
WHERE version IN (20260318000001, 20260318000002, 20260318000003, 20260318000004)
ORDER BY version;
"
```

### 预期结果

| version | description | success |
|---------|-------------|---------|
| 20260318000001 | oidc create credentials | 1 |
| 20260318000002 | oidc create user verification status | 1 |
| 20260318000003 | oidc create pending actions | 1 |
| 20260318000004 | oidc create email verification tokens | 1 |

---

## 场景 3: 重复启动幂等

**初始状态**: 场景 1 完成后（表已存在）

**目的**: 验证重复执行 `auth9-core init` 不会报错，migration 记录不会重复

### 步骤

1. 重新执行 init 容器

```bash
docker-compose restart auth9-init
```

2. 等待完成并检查退出码

```bash
EXIT_CODE=$(docker wait auth9-init)
echo "Exit code: $EXIT_CODE"
```

3. 验证 migration 记录未重复

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "
SELECT version, COUNT(*) AS cnt FROM _sqlx_migrations
WHERE version IN (20260318000001, 20260318000002, 20260318000003, 20260318000004)
GROUP BY version
HAVING cnt > 1;
"
```

### 预期结果

- init 容器退出码为 `0`
- 第 3 步查询返回空结果（无重复记录）

---

## 场景 4: auth9-oidc 启动不再自行建表

**初始状态**: 场景 1 完成后

**目的**: 验证 auth9-oidc 容器启动时仅连接数据库、不再执行 migration SQL

### 步骤

1. 查看 auth9-oidc 容器日志

```bash
docker-compose logs auth9-oidc 2>&1 | grep -E "(database|migration|table)"
```

### 预期结果

- 日志中包含 `auth9-oidc database connected`
- 日志中**不包含** `auth9-oidc database tables ensured`（旧的 migration 日志消息）

---

## 清单

- [x] 场景 1: 全新环境启动后所有表就绪
- [x] 场景 2: Migration 版本跟踪完整
- [x] 场景 3: 重复启动幂等
- [x] 场景 4: auth9-oidc 启动不再自行建表

**测试日期**: 2026-03-23
**测试人员**: opencode
**备注**:
