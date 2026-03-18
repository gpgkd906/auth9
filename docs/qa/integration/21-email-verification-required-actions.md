# 集成测试 - Phase 3 FR3 邮箱验证与 Required Actions

**模块**: Integration
**测试范围**: auth9-oidc 邮箱验证/Required Actions 数据库 schema、adapter 契约、Identity Token 白名单、Backend 切换 smoke test
**场景数**: 5
**优先级**: 高

---

## 背景说明

Phase 3 FR3 实现了 Auth9 自管的邮箱验证和 Required Actions，替代 Keycloak required actions 页面。核心变更：

- **auth9-oidc 新增 4 张表**：`credentials`、`user_verification_status`、`email_verification_tokens`、`pending_actions`
- **auth9-oidc 启动时自动建表**：通过 `include_str!` 嵌入 migration SQL + `CREATE TABLE IF NOT EXISTS` 幂等执行
- **Identity Token 白名单扩展**：`/api/v1/hosted-login/pending-actions` 和 `/api/v1/hosted-login/complete-action` 允许 identity token 访问
- **Backend 隔离**：邮箱验证和 required actions 仅在 `IDENTITY_BACKEND=auth9_oidc` 下可用

---

## 场景 1：auth9-oidc 数据库 Schema 完整性

### 步骤 0（Gate Check）
- TiDB 运行中：`mysql -h 127.0.0.1 -P 4000 -u root -e "SELECT 1"`
- auth9-oidc 服务运行中：`curl -sf http://localhost:8090/health`

### 初始状态
- auth9-oidc 已启动并自动执行 migration

### 目的
验证 auth9-oidc 的 4 张表结构完整性

### 测试操作流程
1. 检查表是否存在：
```sql
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'auth9'
AND table_name IN ('credentials', 'user_verification_status', 'email_verification_tokens', 'pending_actions')
ORDER BY table_name;
```

2. 检查 `credentials` 表结构：
```sql
DESCRIBE auth9.credentials;
-- 预期列: id, user_id, credential_type, credential_data(JSON), user_label, is_active, created_at, updated_at
```

3. 检查 `email_verification_tokens` 表结构：
```sql
DESCRIBE auth9.email_verification_tokens;
-- 预期列: id, user_id, token_hash, expires_at, used_at, created_at
```

4. 检查 `pending_actions` 表结构：
```sql
DESCRIBE auth9.pending_actions;
-- 预期列: id, user_id, action_type, status(default 'pending'), metadata(JSON), created_at, completed_at
```

5. 检查索引存在：
```sql
SHOW INDEX FROM auth9.credentials WHERE Key_name LIKE 'idx_%';
SHOW INDEX FROM auth9.email_verification_tokens WHERE Key_name LIKE 'idx_%';
SHOW INDEX FROM auth9.pending_actions WHERE Key_name LIKE 'idx_%';
```

### 预期结果
- 4 张表全部存在
- 列定义与 migration SQL 一致
- 索引：`credentials` 有 `idx_credentials_user_id` 和 `idx_credentials_user_type`；`email_verification_tokens` 有 `idx_ev_tokens_user_id` 和 `idx_ev_tokens_expires`；`pending_actions` 有 `idx_pending_actions_user` 和 `idx_pending_actions_user_status`

---

## 场景 2：auth9-oidc Migration 幂等性

### 初始状态
- 场景 1 已通过（表已存在）

### 目的
验证 auth9-oidc 重启后不会因表已存在而失败（`CREATE TABLE IF NOT EXISTS` 幂等性）

### 测试操作流程
1. 重启 auth9-oidc：
```bash
docker compose restart auth9-oidc
```

2. 等待服务就绪：
```bash
sleep 3 && curl -sf http://localhost:8090/health | jq .
```

3. 检查日志确认 migration 成功：
```bash
docker logs auth9-oidc 2>&1 | tail -5
```

### 预期结果
- 服务正常启动，health 返回 `{"status": "healthy"}`
- 日志中出现 `auth9-oidc database tables ensured`（无错误）
- 表结构不变（无新表创建）

---

## 场景 3：IdentityVerificationStore / IdentityActionStore Adapter 契约

### 步骤 0（Gate Check）
- auth9-core 编译通过：`cd auth9-core && cargo build`

### 初始状态
- 代码库最新状态

### 目的
验证 auth9-oidc adapter 实现了 `IdentityVerificationStore` 和 `IdentityActionStore` trait 所有方法，单元测试覆盖核心逻辑

### 测试操作流程
1. 运行 email verification 相关单元测试：
```bash
cd auth9-core && cargo test email_verification -- --nocapture
```

2. 运行 required actions 相关单元测试：
```bash
cd auth9-core && cargo test required_action -- --nocapture
```

3. 运行 auth9-oidc adapter 契约测试：
```bash
cd auth9-core && cargo test adapter_contract -- --nocapture
```

### 预期结果
- email_verification 测试：7 passed（token 生成唯一性、hash 确定性、link 格式、request 反序列化）
- required_action 测试：5 passed（redirect URL 映射、serialization、request 反序列化）
- 无编译错误（trait 实现完整）

---

## 场景 4：Identity Token 白名单 — Pending Actions 端点

### 步骤 0（Gate Check）
- auth9-core 编译通过

### 初始状态
- 代码库最新状态

### 目的
验证 `is_identity_token_path_allowed` 白名单包含 required actions 端点，且单元测试覆盖

### 测试操作流程
1. 运行白名单测试：
```bash
cd auth9-core && cargo test test_identity_token_path_allowed -- --nocapture
```

2. 检查白名单代码（`src/middleware/require_auth.rs`）：
```bash
grep -A2 "hosted-login" auth9-core/src/middleware/require_auth.rs
```

### 预期结果
- 测试 `test_identity_token_path_allowed_existing` 通过
- 白名单包含以下两条规则：
  - `path == "/api/v1/hosted-login/pending-actions"`
  - `path == "/api/v1/hosted-login/complete-action"`
- 非白名单路径（如 `/api/v1/users`、`/api/v1/roles`）仍被拒绝

---

## 场景 5：Keycloak Backend 下 Email Verification 端点 Graceful 处理

### 步骤 0（Gate Check）
- auth9-core 以 `IDENTITY_BACKEND=keycloak` 运行（默认）

### 初始状态
- auth9-core 使用 Keycloak backend

### 目的
验证在 Keycloak backend 下调用 email verification 端点不会返回 200 假成功，而是返回明确的错误

### 测试操作流程
1. 确认 backend 为 keycloak：
```bash
docker exec auth9-core env | grep IDENTITY_BACKEND
# 预期: IDENTITY_BACKEND=keycloak
```

2. 调用发送验证端点（已存在用户）：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/send-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@auth9.local"}'
```

3. 调用发送验证端点（不存在用户）：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/send-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "nonexistent@example.com"}'
```

### 预期结果
- 存在用户：HTTP 500（`internal_error`），因为 Keycloak adapter 不支持 verification tokens
  - **注意**：理想情况应返回 501 Not Implemented 或 200 + 统一消息（防枚举），当前行为为 500 — 这是已知的待优化项
- 不存在用户：HTTP 200 + 统一消息 `"If an account exists with this email..."`（防枚举逻辑在 user 查找之前就返回）
- 日志中出现 `verification tokens not supported in keycloak backend` 错误信息（仅对存在用户的请求）
