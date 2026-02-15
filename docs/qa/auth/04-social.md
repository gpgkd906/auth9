# 认证流程 - 社交登录与 OIDC 端点测试

**模块**: 认证流程
**测试范围**: 社交登录、身份关联、OIDC 元数据
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构，社交登录的入口在 Keycloak 登录页面上：

1. **社交登录按钮**（如「使用 Google 登录」）→ 显示在 Keycloak 登录页面上，由 auth9-keycloak-theme（基于 Keycloakify）定制外观，而非 Keycloak 原生 UI
2. **社交登录 OAuth 流程** → 用户点击按钮后，Keycloak 负责与第三方 IdP（Google、GitHub 等）的 OAuth 交互
3. **身份关联管理**（关联/解除社交账户）→ 在 Auth9 Portal 的设置页面操作，后端通过 Keycloak Admin API 完成

**页面归属**：
- 登录页面上的社交登录按钮 → Keycloak 托管（auth9-keycloak-theme 定制外观）
- 「设置 → 关联账户」管理页面 → Auth9 Portal 页面

---

## 场景 1：Google 登录

### 初始状态
- 系统配置了 Google Identity Provider
- 用户有 Google 账户

### 目的
验证 Google 社交登录

### 测试操作流程
1. 在 Keycloak 登录页面（auth9-keycloak-theme 定制外观）点击「使用 Google 登录」
2. 跳转到 Google 登录页
3. 完成 Google 授权
4. Google 回调到 Keycloak，Keycloak 完成身份映射后重定向回 Auth9

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

## 场景 2：关联社交账户

### 初始状态
- 用户已有 Auth9 账户（密码登录）
- 用户想关联 GitHub 账户

### 目的
验证社交账户关联功能

### 测试操作流程
1. 登录现有账户
2. 进入「设置」→「关联账户」
3. 点击「关联 GitHub」
4. 完成 GitHub 授权

### 预期结果
- GitHub 账户成功关联
- 以后可以用 GitHub 登录

### 预期数据状态
```sql
SELECT provider, external_id, created_at FROM linked_identities
WHERE user_id = '{user_id}' AND provider = 'github';
-- 预期: 存在记录
```

---

## 场景 3：解除社交账户关联

### 初始状态
- 用户已关联 GitHub 账户
- 用户有其他登录方式

### 目的
验证解除社交账户关联

### 测试操作流程
1. 进入「设置」→「关联账户」
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
| 2 | 关联社交账户 | ☐ | | | |
| 3 | 解除社交账户 | ☐ | | | |
| 4 | OIDC Discovery | ☐ | | | |
| 5 | JWKS 端点 | ☐ | | | |
