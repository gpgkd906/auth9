# 认证流程 - 社交登录 Broker API 测试

**模块**: 认证流程
**测试范围**: Social Login Broker API（provider 列表、authorize 跳转、callback 处理）
**场景数**: 5

---

## 场景 1：获取已启用社交提供商列表

### 初始状态
- Auth9 Core 正在运行
- 至少配置一个已启用的社交提供商

### 目的
验证公开端点返回已启用的非 link_only 社交提供商列表（不包含 secret）

### 测试操作流程
```bash
# 无需认证
curl -s http://localhost:8080/api/v1/social-login/providers | jq .
```

### 预期结果
- 返回 `{ "data": [...] }` 格式
- 每个 provider 包含 `alias`、`display_name`、`provider_id`
- 不包含 `config`（clientId/clientSecret 等敏感字段）
- 仅返回 `enabled = true` 且 `link_only = false` 的提供商

---

## 场景 2：社交登录 authorize 跳转（有效 login_challenge）

### 初始状态
- 配置了 Google 社交提供商（alias = `google`，包含 clientId）
- 存在一个有效的 login_challenge（通过 `/api/v1/auth/authorize` 生成）

### 目的
验证 authorize 端点正确跳转到社交提供商

### 测试操作流程
```bash
# 先获取 login_challenge（通过标准 OIDC authorize 流程）
# 然后访问：
curl -v "http://localhost:8080/api/v1/social-login/authorize/google?login_challenge={challenge_id}"
```

### 预期结果
- HTTP 302 重定向
- Location header 指向 `https://accounts.google.com/o/oauth2/v2/auth?...`
- URL 包含 `client_id`、`redirect_uri`、`scope=openid+email+profile`、`state`、`response_type=code`
- `redirect_uri` 指向 `http://localhost:8080/api/v1/social-login/callback`

---

## 场景 3：社交登录 authorize 跳转（无效 login_challenge）

### 初始状态
- Auth9 Core 正在运行

### 目的
验证无效的 login_challenge 被拒绝

### 测试操作流程
```bash
curl -s "http://localhost:8080/api/v1/social-login/authorize/google?login_challenge=invalid-id"
```

### 预期结果
- 返回 HTTP 400
- 错误消息指示 login challenge 无效或已过期

---

## 场景 4：社交登录 authorize 跳转（已禁用的提供商）

### 初始状态
- 配置了社交提供商但 `enabled = false`
- 存在有效的 login_challenge

### 目的
验证已禁用的提供商无法发起 authorize 跳转

### 测试操作流程
```bash
# 创建一个已禁用的提供商
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s -X POST http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"alias":"disabled-test","provider_id":"google","enabled":false,"config":{"clientId":"test","clientSecret":"test"}}' # pragma: allowlist secret

# 尝试通过该提供商发起授权
curl -s "http://localhost:8080/api/v1/social-login/authorize/disabled-test?login_challenge={valid_challenge}"
```

### 预期结果
- 返回 HTTP 400
- 错误消息指示提供商已禁用

---

## 场景 5：社交登录 callback 错误处理（提供商返回错误）

### 初始状态
- Auth9 Core 正在运行

### 目的
验证当社交提供商返回错误时（如用户取消授权），callback 正确重定向到 Portal login 页面

### 测试操作流程
```bash
# 模拟提供商返回错误（如用户取消）
curl -v "http://localhost:8080/api/v1/social-login/callback?error=access_denied&state=some-state"
```

### 预期结果
- HTTP 302 重定向到 Portal `/login?error=social_login_cancelled`
- 不会创建用户或会话

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 获取已启用社交提供商列表 | ☐ | | | |
| 2 | 有效 login_challenge authorize 跳转 | ☐ | | | 需配置 Google OAuth credentials |
| 3 | 无效 login_challenge 被拒绝 | ☐ | | | |
| 4 | 已禁用提供商无法发起跳转 | ☐ | | | |
| 5 | callback 错误处理 | ☐ | | | |
