# 认证流程 - 社交登录与 OIDC 端点测试

**模块**: 认证流程
**测试范围**: 社交登录、身份关联、OIDC 元数据
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构，社交登录通过 Auth9 登录入口触发，认证页可由 `auth9-keycloak-theme` 承载：

1. **社交登录按钮**（如「使用 Google 登录」）→ 显示在 Auth9 品牌认证页上，而非 Portal `/login` 页面
2. **社交登录 OAuth 流程** → 用户点击按钮后，进入第三方 IdP（Google、GitHub 等）的 OAuth 交互
3. **身份关联管理**（查看/新增/解除）→ 在 Auth9 Portal 的 Account 页面操作

**进入社交登录的路径**：
- Portal `/login` → 点击「**Sign in with password**」→ 跳转到 Auth9 品牌认证页 → 页面底部显示社交登录按钮（如 Google、GitHub）

> **注意**：社交登录按钮不在 Portal `/login` 页面上，而是在 Auth9 品牌认证页上。QA 需要先点击「Sign in with password」进入托管认证页才能看到社交登录选项。

### 步骤 0: 验证 Identity Provider 已配置（场景 1-3 前置）

本地开发 Docker 环境默认不包含 IdP 配置。执行场景 1-3 前必须验证：

```bash
# 检查是否已配置 Identity Provider
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s http://localhost:8080/api/v1/identity-providers \
  -H "Authorization: Bearer $TOKEN" | jq '.[].alias'
# 必须输出至少一个 provider（如 "google", "github"）
# 如果输出为空数组 []，按以下步骤配置:
# 1. 进入 Auth9 Portal「设置」→「身份提供商」
# 2. 从团队统一密钥源（推荐 Vault）读取 OAuth Client ID/Secret，添加 IdP
# 如果没有真实的 OAuth credentials，场景 1-3 应标记为 SKIP（环境未配置）而非 FAILED
```

> **回归测试约定**：OAuth 凭据和测试账号凭据应保存在团队密钥系统（推荐 Vault）中。当前建议的标准命名为：
> - `secret/auth9-github-oauth`：保存 GitHub OAuth `client_id` / `client_secret`
> - `secret/auth9-github-test-account`：保存回归测试用 GitHub 账号凭据
>
> QA 执行时从 Vault 读取后临时填入 Portal 或测试浏览器使用，不要在 QA 文档、脚本或截图中直接记录明文凭据。

### GitHub OAuth App 配置（推荐保留为标准测试配置）

如需验证 GitHub 登录或已登录态主动关联 GitHub，建议准备一套长期可复用的 GitHub OAuth App 测试配置。

**GitHub 侧建议值**
- Application name: `Auth9 Local GitHub Test`（或对应测试环境名称）
- Homepage URL: `http://localhost:3000`（集群环境替换为实际 Portal 域名）
- Authorization callback URL: `http://localhost:8081/realms/auth9/broker/github/endpoint`

> **重要**：GitHub OAuth App 的 callback 必须指向 **Keycloak broker endpoint**，而不是 Auth9 Portal 页面。

如在非本地环境使用，callback URL 公式为：
- `{KEYCLOAK_PUBLIC_URL}/realms/{KEYCLOAK_REALM}/broker/{provider_alias}/endpoint`

当前默认值：
- `KEYCLOAK_PUBLIC_URL = http://localhost:8081`
- `KEYCLOAK_REALM = auth9`
- `provider_alias = github`

**页面归属**：
- Portal `/login` 页面 → 认证方式选择（Enterprise SSO / Password / Passkey）
- Auth9 品牌认证页 → 用户名密码表单 + 社交登录按钮（由 `auth9-keycloak-theme` 承载）
- 「Account → Linked Identities」管理页面 → Auth9 Portal 页面

> **补充回归建议**：本文件验证社交登录/关联的功能闭环；如需单独检查异常路径下是否出现未被主题接管的 Keycloak 原生 UI，请执行 [13-keycloak-ui-visibility-regression.md](./13-keycloak-ui-visibility-regression.md)。

---

## 场景 1：Google 登录

### 初始状态
- 系统配置了 Google Identity Provider
- 用户有 Google 账户

### 目的
验证 Google 社交登录

### 测试操作流程
1. 在 Portal `/login` 页面点击「**Sign in with password**」进入 Auth9 品牌认证页
2. 在认证页底部点击「使用 Google 登录」
2. 跳转到 Google 登录页
3. 完成 Google 授权
4. Google 回调到托管认证链路，完成身份映射后重定向回 Auth9

### 预期结果
- 用户成功登录
- 如果是新用户，自动创建账户
- Google 身份被关联

### 预期数据状态
```sql
SELECT provider, external_id FROM linked_identities
WHERE user_id = '{user_id}' AND provider = 'google';
-- 预期: 存在记录

SELECT event_type FROM login_events WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'social'
```

---

## 场景 2：已登录用户主动关联社交身份

### 初始状态
- 用户已有 Auth9 账户（密码登录）
- 系统已配置 GitHub 或其他可用 Identity Provider
- 待关联的提供商尚未出现在当前用户的已关联身份列表中

### 目的
验证已登录态可以主动发起新的社交身份关联

### 测试操作流程
1. 登录现有账户
2. 进入「Account」→「Linked Identities」
3. 确认页面显示「Link another identity」区域
4. 点击目标提供商的「Link GitHub / Link Google / ...」按钮
5. 完成对应第三方 IdP 授权
6. 浏览器返回「Account」→「Linked Identities」页面

### 预期结果
- 页面显示可发起关联的提供商入口（仅显示已启用且当前未关联的提供商）
- 完成授权后返回 Linked Identities 页面
- 新的社交身份出现在已关联身份列表中
- 页面仍允许执行「Unlink」

### 预期数据状态
```sql
SELECT provider, external_id, created_at FROM linked_identities
WHERE user_id = '{user_id}' AND provider = 'github';
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
2. 点击 GitHub 旁的「解除关联」
3. 确认操作

### 预期结果
- GitHub 账户解除关联
- 无法再用该 GitHub 登录

### 预期数据状态
```sql
SELECT COUNT(*) FROM linked_identities WHERE user_id = '{user_id}' AND provider = 'github';
-- 预期: 0
```

---

## 场景 4：OIDC Discovery 端点

### 初始状态
- Auth9 Core 正在运行

### 目的
验证 OIDC Discovery 元数据端点

### 测试操作流程
1. 访问 `/.well-known/openid-configuration`

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
1. 访问 `/.well-known/jwks.json`

### 预期结果
- 返回公钥集合
- 用于验证 JWT 签名

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Google 登录 | ☐ | | | |
| 2 | 已登录用户主动关联社交身份 | ☐ | | | 需在真实 IdP 凭据环境验证 |
| 3 | 解除社交账户 | ☐ | | | |
| 4 | OIDC Discovery | ☐ | | | |
| 5 | JWKS 端点 | ☐ | | | |
