# 认证流程 - 社交登录与 OIDC 端点测试

**模块**: 认证流程
**测试范围**: 社交登录 Broker、身份关联、OIDC 元数据
**场景数**: 5

---

## 架构说明

Auth9 接管社交登录 Broker 全链路。社交登录由 Auth9-core 直接执行 OAuth2 授权跳转、callback 接收、token 交换和 profile 映射。

**核心端点**:
- `GET /api/v1/social-login/providers` — 获取已启用的社交提供商列表（公开，无需认证）
- `GET /api/v1/social-login/authorize/{alias}?login_challenge={id}` — 发起社交登录 OAuth 跳转
- `GET /api/v1/social-login/callback?code=...&state=...` — 接收社交提供商回调
- `GET /api/v1/social-login/link/{alias}` — 已登录用户发起社交身份关联（需 JWT）
- `GET /api/v1/social-login/link/callback?code=...&state=...` — 关联回调

**支持的提供商**: Google、GitHub、Microsoft、通用 OIDC

**进入社交登录的路径**：
- Portal `/login?login_challenge={id}` → 页面显示社交登录按钮（Google、GitHub 等）→ 点击按钮跳转至第三方 IdP → callback 回到 Auth9 → 完成认证

### 步骤 0: 验证 Identity Provider 已配置（场景 1-3 前置）

```bash
# 检查是否已配置 Identity Provider
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" | jq '.data[].alias'
# 必须输出至少一个 provider（如 "google", "github"）

# 公开端点也可查看已启用的社交提供商
curl -s http://localhost:8080/api/v1/social-login/providers | jq '.data'

# 如果输出为空数组 []，按以下步骤配置:
# 1. 进入 Auth9 Portal「设置」→「身份提供商」
# 2. 添加 IdP（需提供 OAuth Client ID / Secret）
# 如果没有真实的 OAuth credentials，场景 1-3 应标记为 SKIP（环境未配置）而非 FAILED
```

> **回归测试约定**：OAuth 凭据应保存在团队密钥系统（推荐 Vault）中。
> - `secret/auth9-github-oauth`：保存 GitHub OAuth `client_id` / `client_secret`

### GitHub OAuth App 配置

**GitHub 侧建议值**
- Application name: `Auth9 Local GitHub Test`
- Homepage URL: `http://localhost:3000`
- Authorization callback URL: `http://localhost:8080/api/v1/social-login/callback`

> **重要**：GitHub OAuth App 的 callback 必须指向 **Auth9-core 社交登录 callback 端点**。

---

## 场景 1：Google 登录

### 初始状态
- 系统配置了 Google Identity Provider（alias = `google`）
- 用户有 Google 账户

### 目的
验证 Google 社交登录全链路（Auth9 直接执行 OAuth2 broker）

### 测试操作流程
1. 在 Portal `/login?login_challenge={id}` 页面确认显示 Google 社交登录按钮
2. 点击「Google」按钮
3. 浏览器跳转到 `https://accounts.google.com/o/oauth2/v2/auth?...`
4. 完成 Google 授权
5. Google 回调到 `http://localhost:8080/api/v1/social-login/callback?code=...&state=...`
6. Auth9 交换 code → 获取 access_token → 获取 userinfo → 映射 profile → 创建/查找用户 → 生成 authorization code → 重定向到 Portal callback

### 预期结果
- 用户成功登录，进入 Portal
- 如果是新用户，自动创建账户
- Google 身份被关联
- 浏览器地址栏不应暴露内部 broker endpoint

### 预期数据状态
```sql
SELECT provider_alias, external_user_id FROM linked_identities
WHERE user_id = '{user_id}' AND provider_alias = 'google';
-- 预期: 存在记录
```

---

## 场景 2：已登录用户主动关联社交身份

### 初始状态
- 用户已有 Auth9 账户（密码登录）
- 系统已配置 GitHub Identity Provider
- 待关联的提供商尚未出现在当前用户的已关联身份列表中

### 目的
验证已登录态可以通过 Auth9 社交 broker 主动发起新的社交身份关联

### 测试操作流程
1. 登录现有账户
2. 进入「Account」→「Linked Identities」
3. 确认页面显示「Link another identity」区域
4. 点击「Link GitHub」按钮
5. 浏览器通过 Auth9 `GET /api/v1/social-login/link/github` 跳转到 GitHub OAuth 页面
6. 完成 GitHub 授权
7. 浏览器返回「Account」→「Linked Identities」页面

### 预期结果
- 页面显示可发起关联的提供商入口（仅显示已启用且当前未关联的提供商）
- 完成授权后返回 Linked Identities 页面
- 新的社交身份出现在已关联身份列表中
- 关联流程由 Auth9 原生 broker 处理

### 预期数据状态
```sql
SELECT provider_alias, external_user_id, linked_at FROM linked_identities
WHERE user_id = '{user_id}' AND provider_alias = 'github';
-- 预期: 返回新关联记录
```

---

## 场景 3：解除社交账户关联

### 初始状态
- 用户已关联 GitHub 账户
- 用户有其他登录方式

### 目的
验证解除社交账户关联

### 测试操作流程
1. 进入「Account」→「Linked Identities」
2. 点击 GitHub 旁的「Unlink」
3. 确认操作

### 预期结果
- GitHub 账户解除关联
- 无法再用该 GitHub 登录

### 预期数据状态
```sql
SELECT COUNT(*) FROM linked_identities WHERE user_id = '{user_id}' AND provider_alias = 'github';
-- 预期: 0
```

---

## 场景 4：OIDC Discovery 端点

### 初始状态
- Auth9 Core 正在运行

### 目的
验证 OIDC Discovery 元数据端点

### 测试操作流程
```bash
curl -s http://localhost:8080/.well-known/openid-configuration | jq .
```

### 预期结果
- 返回 OIDC 元数据 JSON
- 包含：issuer, authorization_endpoint, token_endpoint, jwks_uri 等

---

## 场景 5：JWKS 端点

### 初始状态
- Auth9 Core 正在运行

### 目的
验证 JWKS 端点

### 测试操作流程
```bash
curl -s http://localhost:8080/.well-known/jwks.json | jq .
```

### 预期结果
- 返回公钥集合
- 用于验证 JWT 签名

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Google 登录（Auth9 Broker） | ☐ | | | 需要真实 OAuth credentials |
| 2 | 已登录用户主动关联社交身份 | ☐ | | | 需要真实 IdP 凭据环境验证 |
| 3 | 解除社交账户 | ☐ | | | |
| 4 | OIDC Discovery | ☐ | | | |
| 5 | JWKS 端点 | ☐ | | | |
