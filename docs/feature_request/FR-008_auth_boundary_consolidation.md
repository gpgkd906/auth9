# FR-008 认证边界收口

**类型**: 安全加固（审计意见）
**严重程度**: Medium
**影响范围**: auth9-core (Backend)
**前置依赖**: 无

---

## 背景

审计发现 `AuthUser` 提取器与 `require_auth` 中间件的认证职责分散，存在绕过风险：

1. **`AuthUser` 提取器（`auth.rs:198-222`）**：按顺序尝试 service client → identity → tenant access token（any_audience），**不做 audience 校验**。如果某个 handler 未经过 `require_auth` 中间件（例如误挂在 public router），则任意 audience 的 token 都能通过。
2. **`require_auth` 中间件（`require_auth.rs:82-115`）**：做了 audience 动态校验（Redis cache `is_valid_audience`）+ identity token 路径限制。但这是外层防线，提取器本身无防护。
3. **No-cache fallback（`require_auth.rs:99`）**：无 Redis 时 `audience_valid` 默认为 `true`，测试便利但如果误入生产则形同虚设。

当前缓解措施：
- `require_auth` 中间件已覆盖所有 protected routes，提供了 audience 校验
- Identity token 被限制只能访问 tenant selection / exchange 路径
- 路由层（`server/mod.rs`）将 public 和 protected routes 显式分开

**涉及入口**:
- `auth9-core/src/middleware/auth.rs:198-222` — `AuthUser::from_request_parts`
- `auth9-core/src/middleware/require_auth.rs:82-115` — audience 校验 + token 类型限制
- `auth9-core/src/jwt/mod.rs:561-571` — `verify_tenant_access_token_any_audience`

---

## 需求

### R1: Audience 校验内聚到提取器

将 audience 校验从中间件下沉到 `AuthUser` 提取器，实现纵深防御。

- `AuthUser::from_request_parts` 在验证 tenant access token 时，同步校验 audience（通过 cache 或传入的合法 audience 列表）
- 提取器需要访问 cache（`CacheManager`），通过 `HasServices` trait 获取
- 中间件层的 audience 校验可保留作为额外防线，但提取器不再依赖中间件来保证 audience 合法性

**涉及文件**:
- `auth9-core/src/middleware/auth.rs` — `AuthUser::from_request_parts` 增加 audience 校验
- `auth9-core/src/middleware/auth.rs` — `HasServices` trait 可能需要暴露 cache 访问

### R2: 消除 No-Cache Fallback 风险

无 Redis 时 audience 校验不应默认通过。

- 生产环境：无 cache 时 audience 校验失败（fail-closed），返回 503
- 测试环境：通过 `NoOpCacheManager` 显式配置允许行为，而非隐式 fallback
- `require_auth.rs:99` 的 `true` fallback 改为 `false`，或引入显式的 `AllowAllAudiences` test helper

**涉及文件**:
- `auth9-core/src/middleware/require_auth.rs` — 修改 no-cache fallback 逻辑
- `auth9-core/src/cache/mod.rs` — `NoOpCacheManager::is_valid_audience` 返回 `true`（测试用）
- 受影响的测试文件 — 确保测试使用 `NoOpCacheManager` 而非依赖 no-cache path

### R3: Token 类型路径限制内聚

将 identity token 的路径限制（当前在中间件 `require_auth.rs:121`）同步体现在提取器层面。

- `AuthUser` 增加 `token_kind` 字段（`identity` / `tenant_access` / `service_client`）
- Handler 可通过 `auth_user.token_kind` 做细粒度访问控制，不完全依赖中间件路径匹配
- 或：提供 `TenantAuthUser` / `IdentityAuthUser` 类型化提取器，编译期保证 handler 接收正确的 token 类型

**涉及文件**:
- `auth9-core/src/middleware/auth.rs` — `AuthUser` 增加 token_kind 或拆分为类型化提取器
- 使用 `AuthUser` 的 handler 文件 — 视方案调整（如改用类型化提取器）

---

## 验收标准

- [ ] `AuthUser` 提取器独立完成 audience 校验，不挂中间件也能拒绝非法 audience
- [ ] 无 Redis 时 audience 校验 fail-closed（503），不默认放行
- [ ] `AuthUser` 携带 token 类型信息，handler 可据此做访问控制
- [ ] 现有 API 行为不变（所有 protected route 仍经过中间件 + 提取器双重校验）
- [ ] 单元测试覆盖：非法 audience 被提取器拒绝、no-cache fail-closed、token 类型区分
