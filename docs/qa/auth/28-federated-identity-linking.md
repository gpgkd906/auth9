# 认证流程 - Federated Identity Linking 测试

**模块**: 认证流程
**测试范围**: Federated Identity Linking（社交登录身份关联、Unlink/Re-link、first_login_policy 策略控制、confirm-link 过期）
**场景数**: 5

---

## 场景 1：正常路径 — 社交登录创建 linked identity

> **[DEFERRED - pending FR: social_login_google_idp.md]** No social identity providers are configured.

### 步骤 0：Gate Check

```bash
# 确认 Auth9 Core 正在运行
curl -sf http://localhost:8080/health | jq .

# 确认至少有一个已启用的社交提供商（默认 first_login_policy = auto_merge）
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" | jq '.data[] | select(.provider_id == "google")'

# 确认 linked_identities 表可查询
mysql -h 127.0.0.1 -P 4000 -u root -D auth9 -e "SELECT COUNT(*) FROM linked_identities;"
```

### 初始状态
- Auth9 Core 正在运行，配置了 Google 社交提供商（alias = `google`，`enabled = true`，`first_login_policy = auto_merge`）
- 存在一个测试用户 `user@example.com`，该用户尚无 linked identity

### 目的
验证通过社交登录（Google）成功认证后，`linked_identities` 表自动写入正确的 provider_alias、external_user_id 记录

### 测试操作流程
1. 使用浏览器访问 Auth9 Portal 登录页，点击 "Sign in with Google"
2. 在 Google OAuth 页面完成授权（使用 `user@gmail.com`）
3. 登录完成后回到 Portal

### 预期结果
- 登录成功，用户进入 Portal Dashboard
- `linked_identities` 表新增一条记录

### 预期数据状态

```sql
-- 验证 linked identity 已创建
SELECT id, user_id, provider_type, provider_alias, external_user_id, external_email, linked_at
FROM linked_identities
WHERE provider_alias = 'google'
  AND external_email = 'user@gmail.com'
ORDER BY linked_at DESC
LIMIT 1;
```

**断言**:
- `provider_type` = `google`
- `provider_alias` = `google`
- `external_user_id` 非空（Google 返回的 sub claim）
- `external_email` = `user@gmail.com`
- `linked_at` 为最近时间

---

## 场景 2：正常路径 — Unlink 后可重新 Link

> **[DEFERRED - pending FR: social_login_google_idp.md]** No social identity providers are configured.

### 步骤 0：Gate Check

```bash
# 确认当前用户至少有一个 linked identity
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/users/me/linked-identities \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

### 初始状态
- 场景 1 已通过，当前用户已有一条 Google linked identity
- 用户已登录，持有有效的 Identity Token

### 目的
验证 Unlink API 正确删除 linked identity，且之后可通过社交登录 link 流程重新关联

### 测试操作流程

```bash
# 1. 获取当前用户的 linked identities
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
IDENTITIES=$(curl -s http://localhost:8080/api/v1/users/me/linked-identities \
  -H "Authorization: Bearer $TOKEN")
echo "$IDENTITIES" | jq '.data'

# 2. 提取要删除的 identity ID
IDENTITY_ID=$(echo "$IDENTITIES" | jq -r '.data[0].id')

# 3. 执行 Unlink
curl -s -X DELETE "http://localhost:8080/api/v1/users/me/linked-identities/$IDENTITY_ID" \
  -H "Authorization: Bearer $TOKEN" | jq .

# 4. 验证已删除
curl -s http://localhost:8080/api/v1/users/me/linked-identities \
  -H "Authorization: Bearer $TOKEN" | jq '.data'
```

### 预期结果
- DELETE 返回 `{ "message": "Identity unlinked successfully." }`
- 再次 GET linked-identities 时列表中不再包含该 identity

### 预期数据状态

```sql
-- 验证 linked identity 已从数据库删除
SELECT COUNT(*) AS cnt
FROM linked_identities
WHERE id = '{IDENTITY_ID}';
-- 预期 cnt = 0
```

**重新 Link 验证**:
1. 在 Portal Account 页面点击 "Link Google Account"（触发 `GET /api/v1/social-login/link/google`）
2. 完成 Google OAuth 授权
3. 回调 `GET /api/v1/social-login/link/callback` 成功

```sql
-- 验证重新关联成功
SELECT id, provider_alias, external_user_id, external_email, linked_at
FROM linked_identities
WHERE user_id = '{USER_ID}' AND provider_alias = 'google';
-- 预期有一条新记录，linked_at 为最新时间
```

---

## 场景 3：安全 — prompt_confirm 策略阻止静默 takeover

> **[DEFERRED - pending FR: social_login_google_idp.md]** No social identity providers are configured.

### 步骤 0：Gate Check

```bash
# 确认社交提供商的 first_login_policy 已设置为 prompt_confirm
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" | jq '.data[] | select(.alias == "google") | .first_login_policy'
# 如果不是 prompt_confirm，先更新：
curl -s -X PUT http://localhost:8080/api/v1/identity-providers/google \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"first_login_policy":"prompt_confirm"}'
```

### 初始状态
- 社交提供商 Google 的 `first_login_policy` = `prompt_confirm`
- 数据库中已存在用户 `user@example.com`
- Google OAuth 返回的邮箱与该用户匹配（模拟 email 碰撞场景）

### 目的
验证当 `first_login_policy = prompt_confirm` 时，社交登录回调不会静默合并到已有账号，而是重定向到 Portal 的 `/login/confirm-link` 页面，并在 Redis 缓存中存储 PendingMergeData

### 测试操作流程
1. 使用新浏览器（未登录状态）访问 Auth9 Portal 登录页
2. 点击 "Sign in with Google"
3. 在 Google OAuth 页面使用 `user@gmail.com` 完成授权（该邮箱对应 `user@example.com` 用户）
4. 观察回调后的重定向行为

### 预期结果
- 浏览器**不会**直接登录到 Dashboard
- 浏览器重定向到 Portal `/login/confirm-link?token={merge_token}` 页面
- 页面显示确认提示，告知用户将关联到已有账号 `user@example.com`
- Redis 中存储了 `pending_merge:{merge_token}` 键，TTL 约 600 秒

### 验证缓存

```bash
# 通过 Redis CLI 验证 pending merge 数据
redis-cli GET "pending_merge:{merge_token}"
# 预期返回 JSON，包含:
# - existing_user_id: 已有用户的 ID
# - existing_email: "user@example.com"
# - provider_alias: "google"
# - external_user_id: Google 返回的 sub
# - login_challenge_id: 原始 login challenge ID
```

**关键安全断言**:
- `linked_identities` 表中**没有**新记录（尚未确认）
- 用户**没有**获得 session 或 authorization code

---

## 场景 4：安全 — create_new 策略创建独立账号

> **[DEFERRED - pending FR: social_login_google_idp.md]** No social identity providers are configured.

### 步骤 0：Gate Check

```bash
# 确认社交提供商的 first_login_policy 已设置为 create_new
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s -X PUT http://localhost:8080/api/v1/identity-providers/google \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"first_login_policy":"create_new"}'
```

### 初始状态
- 社交提供商 Google 的 `first_login_policy` = `create_new`
- 数据库中已存在用户 `existing@example.com`
- Google OAuth 返回的邮箱与已有用户匹配

### 目的
验证当 `first_login_policy = create_new` 时，即使邮箱匹配已有用户，系统也会创建一个全新的独立账号，不进行任何关联

### 测试操作流程
1. 记录当前 `users` 表用户数量
2. 使用新浏览器（未登录状态）访问 Auth9 Portal 登录页
3. 点击 "Sign in with Google"
4. 在 Google OAuth 页面使用与 `existing@example.com` 同邮箱的 Google 账号完成授权

### 预期结果
- 登录成功，用户直接进入 Portal Dashboard
- `users` 表新增一条**独立用户**记录
- `linked_identities` 表新增一条记录，`user_id` 指向**新用户**（非已有用户）

### 预期数据状态

```sql
-- 验证新用户被创建（identity_subject 不同于已有用户）
SELECT id, email, identity_subject, created_at
FROM users
WHERE email = 'existing@example.com'
ORDER BY created_at DESC;
-- 预期有 2 条记录，最新一条是新创建的独立用户

-- 验证 linked identity 指向新用户
SELECT li.user_id, li.provider_alias, li.external_user_id, u.identity_subject
FROM linked_identities li
JOIN users u ON u.id = li.user_id
WHERE li.provider_alias = 'google'
ORDER BY li.linked_at DESC
LIMIT 1;
-- 预期 li.user_id 对应新创建的用户，而非已有用户
```

**安全断言**:
- 已有用户的 `linked_identities` 不变
- 已有用户的 session 未受影响

---

## 场景 5：错误 — confirm-link token 过期

### 初始状态
- Auth9 Core 正在运行
- 不存在有效的 pending merge token

### 目的
验证使用无效或过期的 confirm-link token 时，API 返回 400 错误

### 测试操作流程

```bash
# 使用无效 token 调用 confirm-link
curl -s -X POST http://localhost:8080/api/v1/auth/confirm-link \
  -H "Content-Type: application/json" \
  -d '{"token": "expired-or-invalid-token-12345"}' | jq .

# 使用空 token
curl -s -X POST http://localhost:8080/api/v1/auth/confirm-link \
  -H "Content-Type: application/json" \
  -d '{"token": ""}' | jq .
```

### 预期结果
- HTTP 400 Bad Request
- 错误消息包含 `"Link confirmation token expired or invalid. Please try logging in again."`
- 不会创建 session、authorization code 或 linked identity

### 预期数据状态

```sql
-- 验证没有新的 linked identity
SELECT COUNT(*) AS cnt
FROM linked_identities
WHERE linked_at >= NOW() - INTERVAL 1 MINUTE;
-- 预期 cnt 不增加
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 社交登录创建 linked identity | ☐ | | | 需配置 Google OAuth credentials |
| 2 | Unlink 后可重新 Link | ☐ | | | 依赖场景 1 |
| 3 | prompt_confirm 策略阻止静默 takeover | ☐ | | | 需修改 first_login_policy |
| 4 | create_new 策略创建独立账号 | ☐ | | | 需修改 first_login_policy |
| 5 | confirm-link token 过期 | ☐ | | | 公开端点，无需认证 |
