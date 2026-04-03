# Google Social Login (社交登录 - Google IdP)

**类型**: 新功能
**严重程度**: Medium
**影响范围**: auth9-core (social provider CRUD, OAuth2 callback), auth9-portal (login page, settings page)
**前置依赖**: 无

---

## 背景

QA 测试期望登录页面显示 "Sign in with Google" 按钮，并期望 `social_providers` 表中有已配置的数据。当前系统未实现任何社交身份提供商（Social IdP）功能，无法通过 Google OAuth2/OIDC 进行第三方登录。

---

## 期望行为

### R1: Google OAuth2/OIDC IdP 配置管理

管理员可在系统设置中配置 Google IdP，包括 Client ID、Client Secret、回调 URL 等 OAuth2 参数。配置存储在 `social_providers` 表中。

**涉及文件**:
- `auth9-core/migrations/` — `social_providers` 表迁移
- `auth9-core/src/domains/identity/` — Social Provider CRUD service 和 API
- `auth9-portal/app/routes/` — 管理员设置页面中的 IdP 配置 UI

### R2: 登录页显示已配置的社交登录按钮

Portal 登录页根据已配置的 social providers 动态渲染对应的社交登录按钮（如 "Sign in with Google"）。

**涉及文件**:
- `auth9-portal/app/routes/login.tsx` — 社交登录按钮渲染

### R3: 社交登录 OAuth2 流程

完整的 OAuth2 授权码流程：重定向到 Google 授权页 → 用户授权 → 回调处理 → 账户关联（或自动注册）→ 签发 Identity Token。

**涉及文件**:
- `auth9-core/src/domains/identity/` — OAuth2 callback handler、账户关联逻辑
- `auth9-portal/app/routes/` — OAuth2 回调路由

---

## 验证方法

### 手动验证

1. 在管理设置中配置 Google IdP（Client ID、Client Secret）
2. 访问登录页，确认显示 "Sign in with Google" 按钮
3. 点击按钮，确认重定向到 Google 授权页
4. 授权后确认回调处理正确，用户成功登录

### 代码验证

```bash
grep -r "social_provider\|SocialProvider\|google\|oauth2_callback" auth9-core/src/ auth9-portal/app/
cd auth9-core && cargo test social
```
