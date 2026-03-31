# Password Max-Age Enforcement (登录时密码过期拦截)

**类型**: 安全 / 认证
**严重程度**: Medium
**影响范围**: auth9-oidc (OIDC Engine), auth9-core (Identity Domain)
**前置依赖**: 无（max_age_days 字段已存在于 password_policies 配置中）
**被依赖**: 无

---

## 背景

`max_age_days` 字段已存储在 `tenants.password_policy` JSON 中，可通过 `PUT /api/v1/tenants/{id}/password-policy` 设置。但当前 auth9-oidc 登录流程中并未检查用户密码是否已超过 `max_age_days` 天未更改。

现状：
- `password_policy.max_age_days` 可正常读写（API 和数据库层面完整）
- `credentials.created_at` 和 `users.password_changed_at` 字段记录了密码最后修改时间
- 登录时 auth9-oidc 不检查密码年龄，过期密码可以正常登录
- `UPDATE_PASSWORD` required action 机制已存在（管理员设置临时密码时使用），可复用

---

## 期望行为

### R1: 登录时检查密码年龄

在 auth9-oidc 密码认证成功后、签发 Identity Token 前，检查:

```
current_time - password_changed_at > max_age_days
```

若密码已过期，不签发 Token，而是创建 `UPDATE_PASSWORD` required action。

**涉及文件**:
- `auth9-oidc/src/` — OIDC 引擎认证流程（密码验证后的 post-authentication hook）
- `auth9-core/src/domains/identity/` — 登录流程中密码年龄检查逻辑

### R2: 创建 UPDATE_PASSWORD required action

密码过期时：
1. 在用户的 pending actions 中插入 `UPDATE_PASSWORD`
2. 返回 MFA/required-action 风格的中间响应，而非直接签发 Token
3. 前端检测到 `UPDATE_PASSWORD` action 后引导用户到强制修改密码页面

**涉及文件**:
- `auth9-oidc/src/` — required action 创建逻辑
- `auth9-core/src/domains/identity/` — pending actions 管理

### R3: 引导用户到强制修改密码页面

Portal 托管认证流程中：
1. 检测 `UPDATE_PASSWORD` required action
2. 展示密码修改表单（需输入新密码，新密码须符合当前密码策略）
3. 修改成功后清除 `UPDATE_PASSWORD` action，继续签发 Token 完成登录
4. 更新 `password_changed_at` 为当前时间

**涉及文件**:
- `auth9-portal/app/routes/` — 强制密码修改页面（可复用 reset-password 组件）

---

## 验证方法

### 手动验证

1. 设置租户密码策略 `max_age_days=1`
2. 创建测试用户并设置密码
3. 等待密码过期（或通过 SQL 手动调整 `password_changed_at` 到过去）
4. 使用过期密码尝试登录
5. 确认被拦截并引导到密码修改页面
6. 修改密码后确认可正常登录

### 数据库验证

```sql
-- 检查密码修改时间
SELECT password_changed_at FROM users WHERE email = 'test@example.com';

-- 检查 pending actions
SELECT * FROM user_required_actions
WHERE user_id = '{user_id}' AND action = 'UPDATE_PASSWORD';
```

### 代码验证

```bash
# 搜索 max_age_days 相关实现
grep -r "max_age_days\|password_changed_at" auth9-oidc/src/ auth9-core/src/domains/identity/

# 运行相关测试
cd auth9-core && cargo test password_age
```

---

## 参考

- 密码策略 QA 文档: `docs/qa/integration/02-password-policy.md`（场景 3 已标记 DEFERRED）
- 管理员临时密码流程（已实现 UPDATE_PASSWORD action）: 密码策略场景 5
- OIDC 引擎认证流程: `auth9-oidc/src/`
