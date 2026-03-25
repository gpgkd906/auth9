# FR-007 Portal 会话架构加固

**类型**: 安全加固（审计意见）
**严重程度**: Medium
**影响范围**: auth9-portal (Frontend/BFF)
**前置依赖**: 无

---

## 背景

审计发现 Portal BFF 层将 `accessToken`、`refreshToken`、`idToken` 直接存入 cookie session（signed httpOnly secure）。虽然当前实现有基本保护（签名、httpOnly、secure、sameSite），但存在以下风险：

1. **Refresh token 落 cookie**：refresh token 长期有效，cookie 被盗（如 SSRF/日志泄漏）后可长期冒用
2. **Cookie 容量风险**：多 token 存储接近 4KB 限制（代码已做 compact 优化但治标不治本）
3. **Token 撤销延迟**：cookie 中的 token 在过期前始终有效，服务端无法即时撤销

当前缓解措施：
- Cookie 已配置 `httpOnly: true, secure: true, sameSite: "lax"`
- `commitSession` 做了字段精简（删除可重建的 `accessToken`、`tenantAccessToken`）
- Portal 是 BFF 架构（React Router 7 SSR），cookie 内容不暴露给浏览器 JS

**涉及入口**:
- `auth9-portal/app/services/session.server.ts:34-53` — SessionData 定义 + cookie 创建
- `auth9-portal/app/services/session.server.ts:115-128` — commitSession compact 逻辑

---

## 需求

### R1: 服务端 Session Store

将 session 数据从 cookie 迁移到服务端存储（Redis），cookie 仅存 opaque session ID。

- 新增 `SessionStore` 模块，使用 Redis 存储 session 数据
- Cookie 内容缩减为 `{ sessionId: string }` — 签名后远小于 4KB
- Session TTL 与当前 `SESSION_MAX_AGE` 保持一致
- 提供 `getSession(request)` / `commitSession(data)` 同签名的替换实现，最小化调用方改动
- 支持服务端即时 session 撤销（删除 Redis key）

**涉及文件**:
- `auth9-portal/app/services/session.server.ts` — 重构为 Redis-backed store
- `auth9-portal/app/services/redis.server.ts`（新建）— Redis 客户端封装
- `docker-compose.dev.yml` — Portal 连接 Redis（可复用 auth9-core 的 Redis 实例）

### R2: Refresh Token 不落浏览器

Refresh token 仅存服务端 session store，不通过 Set-Cookie 下发到浏览器。

- `SessionData` 中 `refreshToken` 仅写入 Redis，不出现在 cookie
- Token 刷新流程在 BFF 服务端完成，浏览器无感知
- 刷新失败时清除 session 并重定向到登录页

**涉及文件**:
- `auth9-portal/app/services/session.server.ts` — 拆分 cookie 字段 vs store-only 字段
- `auth9-portal/app/services/auth.server.ts`（如存在）— token 刷新逻辑

### R3: Session 即时撤销能力

支持服务端主动销毁 session，用于登出、密码修改、安全事件响应。

- 登出时删除 Redis session key + cookie
- 密码修改后可选清除该用户所有 session（通过 user_id → session_ids 索引）
- 管理员可通过 API 撤销指定用户的所有 session

**涉及文件**:
- `auth9-portal/app/services/session.server.ts` — `destroySession(sessionId)`
- `auth9-portal/app/routes/` — 登出 action 调用 destroySession

---

## 验收标准

- [ ] Cookie 仅包含 opaque session ID（签名后），不含任何 token
- [ ] Refresh token 仅存在于 Redis，浏览器 DevTools 中 cookie 不可见 refresh token
- [ ] 删除 Redis session key 后，使用该 cookie 的请求立即失效（返回登录页）
- [ ] 现有登录/登出/租户切换流程功能不变
- [ ] Redis 不可用时 graceful degradation（返回登录页而非 500）
