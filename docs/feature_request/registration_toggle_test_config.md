# Registration Toggle 测试环境默认配置

**类型**: 配置 / 测试环境
**严重程度**: Medium
**影响范围**: auth9-core (seed data, branding config), QA 测试流程
**前置依赖**: 无
**被依赖**: auth_01-oidc-login_scenario2, auth_19-hosted-login-routes_scenario2

---

## 背景

多个 QA 测试票据（`auth_01-oidc-login_scenario2`、`auth_19-hosted-login-routes_scenario2`）因注册功能被关闭而阻塞。当前默认配置中 `allow_registration=false`（见 `src/models/branding.rs` 第 143 行），导致 `/register` 页面自动重定向至 `/login`，所有涉及注册流程的测试用例均无法执行。

### 当前行为

- `BrandingPageConfig::default()` 将 `allow_registration` 设为 `false`
- Docker seed 数据中未设置 demo tenant 的 branding 配置，因此沿用默认值
- QA 环境通过 `scripts/reset-docker.sh` 重置后，注册功能默认关闭
- `/register` 页面在 `allow_registration=false` 时重定向至 `/login`（见 `src/domains/tenant_access/api/user.rs` 第 394 行）

---

## 期望行为

### R1: Docker seed data 应为 demo tenant 启用注册

在 `scripts/reset-docker.sh` 或相关 seed 脚本中，为 demo tenant 设置 `allow_registration=true` 的 branding 配置，确保本地开发和 QA 环境中注册页面默认可访问。

**涉及文件**:
- `scripts/reset-docker.sh` -- 添加 branding 配置 seed 步骤
- 或相关 SQL seed 文件 / API 调用脚本

### R2: QA 环境默认启用注册

QA 测试环境重置后，demo tenant 的注册功能应默认开启，以便测试注册相关流程（包括但不限于 OIDC login 中的注册跳转、hosted login routes 中的注册页面渲染）。

**涉及文件**:
- `scripts/reset-docker.sh` -- 在环境重置流程中确保 branding 配置正确
- `.claude/skills/tools/` -- QA 辅助工具可能需要更新以反映新默认值

### R3: Registration toggle 应支持按 tenant 灵活配置

当前 `allow_registration` 已作为 `BrandingPageConfig` 字段存在于 per-tenant branding 配置中（`src/models/branding.rs`），但 seed 数据和测试辅助工具未利用此能力。应确保：

1. 测试辅助函数 `enable_registration`（`tests/support/http.rs` 第 483-487 行）在需要注册的测试场景中被调用
2. QA 测试脚本在执行注册相关场景前，通过 branding API (`PUT /api/branding`) 设置 `allow_registration=true`
3. 需要测试"注册关闭"场景时，可通过同一 API 将 `allow_registration` 切回 `false`

**涉及文件**:
- `src/models/branding.rs` -- `BrandingPageConfig` 定义（已有 `allow_registration` 字段）
- `src/domains/platform/api/branding.rs` -- branding API handler
- `tests/support/http.rs` -- 测试辅助函数 `enable_registration`

---

## 涉及文件

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `scripts/reset-docker.sh` | 修改 | 添加 demo tenant branding seed（`allow_registration=true`） |
| `src/models/branding.rs` | 无需修改 | 已有 `allow_registration` 字段，仅参考 |
| `src/domains/tenant_access/api/user.rs` | 无需修改 | 注册权限检查逻辑，仅参考 |
| `src/domains/platform/api/branding.rs` | 无需修改 | branding 更新 API，仅参考 |
| `tests/support/http.rs` | 无需修改 | 已有 `enable_registration` 辅助函数，仅参考 |

---

## 验证方法

### 自动验证

```bash
# 重置 Docker 环境
./scripts/reset-docker.sh

# 验证 demo tenant 的 branding 配置
curl -s http://localhost:8080/api/branding \
  -H "Authorization: Bearer <demo_tenant_token>" \
  | jq '.data.allow_registration'
# 期望输出: true

# 访问注册页面，确认不再重定向
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/register
# 期望输出: 200（而非 302）
```

### QA 场景验证

1. 执行 `auth_01-oidc-login_scenario2` -- 注册流程应正常完成
2. 执行 `auth_19-hosted-login-routes_scenario2` -- `/register` 页面应正常渲染
3. 通过 branding API 将 `allow_registration` 设为 `false`，确认 `/register` 重新重定向至 `/login`（验证 toggle 双向可控）

### 代码验证

```bash
# 确认 branding 配置中 allow_registration 字段存在
grep -n "allow_registration" auth9-core/src/models/branding.rs

# 确认注册权限检查逻辑
grep -n "allow_registration" auth9-core/src/domains/tenant_access/api/user.rs

# 运行 branding 相关测试
cd auth9-core && cargo test branding
```
