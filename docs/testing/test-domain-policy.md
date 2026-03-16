# 测试域名使用策略

> **目的**: 约束所有 QA 文档、测试 fixture、自动化脚本中使用的域名，防止 AI agent 或人工引入未经批准的域名（如 `test-enterprise.com`）。

## 允许的域名清单

### 1. 标准测试域名（默认使用）

| 域名 | 用途 | 示例 |
|------|------|------|
| `example.com` | 通用测试邮箱、回调 URL | `test@example.com`, `https://app.example.com/callback` |
| `*.example.com` | 子域名场景（OIDC、SSO、webhook） | `https://sso.example.com`, `https://api.example.com` |
| `test.com` | 安全测试中的用户邮箱 | `user1@test.com`, `admin@test.com` |
| `auth9.local` | 平台管理员专用 | `admin@auth9.local` |

**规则**: 当你需要一个测试域名但不确定用哪个时，**一律使用 `example.com`**。

### 2. 安全攻击模拟域名（仅限 `docs/security/` 和安全测试脚本）

| 域名 | 用途 |
|------|------|
| `evil.com` | 通用恶意域名（XSS、CSRF、open redirect） |
| `attacker.com` | 攻击者控制的域名 |
| `attacker.example` | 攻击者域名（RFC 2606 保留 TLD） |
| `metadata.google.internal` | SSRF 云元数据攻击测试 |

**规则**: 这些域名**只能**出现在安全测试上下文中，不得用于普通 QA 测试。

### 3. 真实服务提供商域名（仅限 IdP/社交登录测试）

| 域名 | 用途 |
|------|------|
| `gmail.com` | Google OAuth 身份提供商测试 |

**规则**: 仅在模拟社交登录/IdP 链接场景时使用，不得作为通用测试邮箱域名。

### 4. 外部测试服务（仅限集成测试）

| 域名 | 用途 |
|------|------|
| `httpbin.org` | Webhook 目标、HTTP 请求测试 |

### 5. 组织/租户场景专用（B2B/SSO 测试）

| 域名 | 用途 |
|------|------|
| `acme.example.com` | B2B 组织域名 |
| `corp.example.com` | 企业 SSO 域名 |

**规则**: 组织/租户域名**必须**是 `example.com` 的子域名，不得使用 `acme.com`、`corp.com` 等独立域名。

## 禁止使用的域名

以下域名**绝对不允许**出现在任何测试文档或代码中：

- `test-enterprise.com` — 非标准，无 RFC 保留
- 任何未在上方清单中列出的 `.com` / `.io` / `.org` 域名
- 真实公司域名（`fb.com`、`google.com` 等，`gmail.com` IdP 场景除外）
- 随机编造的域名（`random-site.com`、`site1.com`、`app.com` 等）

## 迁移指南

将不合规域名替换为标准域名：

| 旧域名 | 替换为 |
|--------|--------|
| `acme.com` | `acme.example.com` |
| `acme.io` | `acme.example.com` |
| `acme.test` | `acme.example.com` |
| `corp.com` | `corp.example.com` |
| `fb.com` | `example.com`（或用 `gmail.com` 模拟社交登录） |
| `random-site.com` | `random.example.com` |
| `site1.com` / `site2.com` | `site1.example.com` / `site2.example.com` |
| `app.com` | `app.example.com` |
| `test-enterprise.com` | `enterprise.example.com` |

## Agent 指令

AI agent（包括 QA 测试 agent、文档生成 agent）在生成测试数据时**必须**遵守本策略：

1. **邮箱地址**: 只使用 `@example.com`、`@test.com`、`@auth9.local`
2. **回调/重定向 URL**: 只使用 `https://*.example.com/...`
3. **组织域名**: 只使用 `*.example.com`
4. **安全攻击场景**: 只使用 `evil.com`、`attacker.com`、`attacker.example`
5. **不确定时**: 使用 `example.com`
