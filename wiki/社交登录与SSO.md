# 社交登录与企业 SSO

Auth9 支持多种身份提供商集成，包括社交登录（Google、GitHub）和企业 SSO（OIDC、SAML）。

**本文档介绍系统级别的全局身份提供商配置**。如需配置租户级别的企业 SSO，请参考 [企业级 SSO 连接器](企业级SSO连接器.md)。

## 身份提供商级别

Auth9 支持两种级别的身份提供商配置：

| 类型 | 配置级别 | 适用场景 | 配置位置 |
|------|---------|---------|---------|
| **全局身份提供商** | 系统级别 | 所有租户共享（如 Google、GitHub） | Settings > Identity Providers |
| **企业 SSO 连接器** | 租户级别 | 单个租户独享（如企业 OIDC、SAML） | Tenants > [租户] > SSO |

**选择建议**：

- 使用**全局身份提供商**：公共社交登录（Google、GitHub、Microsoft）
- 使用**企业 SSO 连接器**：企业专用身份提供商（Azure AD、Okta、ADFS）

详见 [企业级 SSO 连接器](企业级SSO连接器.md) 文档了解租户级别的配置。

## 核心概念

### 身份提供商类型

| 类型 | 协议 | 适用场景 |
|------|------|---------|
| **Google** | OAuth 2.0 / OIDC | 消费者应用 |
| **GitHub** | OAuth 2.0 | 开发者应用 |
| **Microsoft** | OAuth 2.0 / OIDC | 企业应用 |
| **OIDC** | OpenID Connect | 通用 OIDC 提供商 |
| **SAML** | SAML 2.0 | 企业级 SSO |

### 集成架构

```
用户 → Auth9 → Keycloak → 身份提供商
                   ↓
           Identity Brokering
                   ↓
            用户身份关联
```

Auth9 通过 Keycloak 的 Identity Brokering 功能实现身份提供商集成。

## 配置身份提供商

### 通过管理界面

1. 导航到 **Settings** > **Identity Providers**
2. 点击 **Add Provider** 按钮
3. 选择提供商类型
4. 填写配置信息
5. 点击 **Create**

### 通过 REST API

#### 列出所有身份提供商

```bash
curl https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>"
```

响应：

```json
{
  "data": [
    {
      "id": "google",
      "alias": "google",
      "displayName": "Google",
      "providerId": "google",
      "enabled": true,
      "config": {
        "clientId": "xxx.apps.googleusercontent.com"
      },
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 创建身份提供商

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "google",
    "displayName": "Sign in with Google",
    "providerId": "google",
    "enabled": true,
    "config": {
      "clientId": "your-client-id.apps.googleusercontent.com",
      "clientSecret": "your-client-secret"
    }
  }'
```

#### 更新身份提供商

```bash
curl -X PUT https://api.auth9.yourdomain.com/api/v1/identity-providers/{alias} \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "displayName": "Google Login",
    "enabled": true
  }'
```

#### 删除身份提供商

```bash
curl -X DELETE https://api.auth9.yourdomain.com/api/v1/identity-providers/{alias} \
  -H "Authorization: Bearer <admin_token>"
```

## Google 登录配置

### 1. 创建 Google OAuth 应用

1. 访问 [Google Cloud Console](https://console.cloud.google.com/)
2. 创建新项目或选择现有项目
3. 进入 **APIs & Services** > **Credentials**
4. 点击 **Create Credentials** > **OAuth client ID**
5. 选择 **Web application**
6. 配置：
   - **Name**: Auth9
   - **Authorized redirect URIs**:
     ```
     https://keycloak.yourdomain.com/realms/auth9/broker/google/endpoint
     ```
7. 保存 Client ID 和 Client Secret

### 2. 在 Auth9 中配置

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "google",
    "displayName": "Sign in with Google",
    "providerId": "google",
    "enabled": true,
    "config": {
      "clientId": "xxx.apps.googleusercontent.com",
      "clientSecret": "xxx"
    }
  }'
```

### Google 配置参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `clientId` | ✅ | Google OAuth Client ID |
| `clientSecret` | ✅ | Google OAuth Client Secret |
| `defaultScope` | ❌ | 请求的范围（默认：openid email profile） |
| `hostedDomain` | ❌ | 限制登录的 Google Workspace 域 |

## GitHub 登录配置

### 1. 创建 GitHub OAuth 应用

1. 访问 [GitHub Developer Settings](https://github.com/settings/developers)
2. 点击 **New OAuth App**
3. 配置：
   - **Application name**: Auth9
   - **Homepage URL**: `https://auth9.yourdomain.com`
   - **Authorization callback URL**:
     ```
     https://keycloak.yourdomain.com/realms/auth9/broker/github/endpoint
     ```
4. 保存 Client ID 和 Client Secret

### 2. 在 Auth9 中配置

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "github",
    "displayName": "Sign in with GitHub",
    "providerId": "github",
    "enabled": true,
    "config": {
      "clientId": "your-github-client-id",
      "clientSecret": "your-github-client-secret"
    }
  }'
```

### GitHub 配置参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `clientId` | ✅ | GitHub OAuth Client ID |
| `clientSecret` | ✅ | GitHub OAuth Client Secret |
| `defaultScope` | ❌ | 请求的范围（默认：user:email） |

## Microsoft / Azure AD 配置

### 1. 在 Azure AD 注册应用

1. 访问 [Azure Portal](https://portal.azure.com/)
2. 进入 **Azure Active Directory** > **App registrations**
3. 点击 **New registration**
4. 配置：
   - **Name**: Auth9
   - **Supported account types**: 选择合适的选项
   - **Redirect URI**:
     ```
     https://keycloak.yourdomain.com/realms/auth9/broker/microsoft/endpoint
     ```
5. 创建 Client Secret
6. 记录 Application (client) ID 和 Client Secret

### 2. 在 Auth9 中配置

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "microsoft",
    "displayName": "Sign in with Microsoft",
    "providerId": "microsoft",
    "enabled": true,
    "config": {
      "clientId": "your-azure-application-id",
      "clientSecret": "your-azure-client-secret"
    }
  }'
```

## 通用 OIDC 配置

用于连接任何兼容 OIDC 的身份提供商。

### 配置示例

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "custom-oidc",
    "displayName": "Enterprise SSO",
    "providerId": "oidc",
    "enabled": true,
    "config": {
      "clientId": "auth9-client",
      "clientSecret": "client-secret",
      "authorizationUrl": "https://idp.example.com/oauth2/authorize",
      "tokenUrl": "https://idp.example.com/oauth2/token",
      "userInfoUrl": "https://idp.example.com/oauth2/userinfo",
      "logoutUrl": "https://idp.example.com/oauth2/logout",
      "issuer": "https://idp.example.com",
      "defaultScope": "openid email profile"
    }
  }'
```

### OIDC 配置参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `clientId` | ✅ | OIDC Client ID |
| `clientSecret` | ✅ | OIDC Client Secret |
| `authorizationUrl` | ✅ | 授权端点 URL |
| `tokenUrl` | ✅ | Token 端点 URL |
| `userInfoUrl` | ❌ | UserInfo 端点 URL |
| `logoutUrl` | ❌ | 登出端点 URL |
| `issuer` | ❌ | Token 颁发者 |
| `defaultScope` | ❌ | 请求的范围 |
| `validateSignature` | ❌ | 是否验证签名 |
| `useJwksUrl` | ❌ | 是否使用 JWKS URL |
| `jwksUrl` | ❌ | JWKS URL |

## SAML 2.0 配置

用于企业级 SSO 集成。

### 配置示例

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/identity-providers \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "corporate-saml",
    "displayName": "Corporate SSO",
    "providerId": "saml",
    "enabled": true,
    "config": {
      "entityId": "https://idp.corp.example.com/saml/metadata",
      "singleSignOnServiceUrl": "https://idp.corp.example.com/saml/sso",
      "singleLogoutServiceUrl": "https://idp.corp.example.com/saml/slo",
      "nameIDPolicyFormat": "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress",
      "signingCertificate": "MIIDXTCCAkWgAwIBAgI...",
      "wantAuthnRequestsSigned": true,
      "wantAssertionsSigned": true,
      "wantAssertionsEncrypted": false
    }
  }'
```

### SAML 配置参数

| 参数 | 必填 | 说明 |
|------|------|------|
| `entityId` | ✅ | IdP Entity ID |
| `singleSignOnServiceUrl` | ✅ | SSO 端点 URL |
| `singleLogoutServiceUrl` | ❌ | SLO 端点 URL |
| `nameIDPolicyFormat` | ❌ | NameID 格式 |
| `signingCertificate` | ✅ | IdP 签名证书 |
| `wantAuthnRequestsSigned` | ❌ | 是否签名请求 |
| `wantAssertionsSigned` | ❌ | 是否要求签名断言 |
| `wantAssertionsEncrypted` | ❌ | 是否加密断言 |

### Auth9 SAML 元数据

提供给 IdP 的 SP 元数据：

```
https://keycloak.yourdomain.com/realms/auth9/broker/{alias}/endpoint/descriptor
```

## 用户身份关联

### 自动关联

当用户通过身份提供商登录时，系统自动：
1. 检查是否存在相同邮箱的用户
2. 如果存在，关联身份
3. 如果不存在，创建新用户

### 查看关联身份

**通过管理界面**：
1. 导航到 **Settings** > **Linked Accounts**
2. 查看已关联的第三方账户

**通过 REST API**：

```bash
curl https://api.auth9.yourdomain.com/api/v1/linked-identities \
  -H "Authorization: Bearer <access_token>"
```

响应：

```json
{
  "data": [
    {
      "id": "linked-identity-uuid",
      "provider": "google",
      "providerUserId": "123456789",
      "providerUsername": "user@gmail.com",
      "created_at": "2024-01-01T10:00:00Z"
    }
  ]
}
```

### 关联新身份

```bash
curl -X POST https://api.auth9.yourdomain.com/api/v1/linked-identities/link \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "github"
  }'
```

响应：

```json
{
  "data": {
    "linkUrl": "https://keycloak.yourdomain.com/realms/auth9/broker/github/link?..."
  }
}
```

### 解绑身份

```bash
curl -X DELETE https://api.auth9.yourdomain.com/api/v1/linked-identities/{provider} \
  -H "Authorization: Bearer <access_token>"
```

## 数据库结构

### 关联身份表

```sql
CREATE TABLE linked_identities (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  provider VARCHAR(100) NOT NULL,
  provider_user_id VARCHAR(255) NOT NULL,
  provider_username VARCHAR(255),
  provider_email VARCHAR(255),
  access_token TEXT,
  refresh_token TEXT,
  token_expires_at TIMESTAMP,
  raw_profile JSONB,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  UNIQUE (provider, provider_user_id),
  INDEX idx_user_id (user_id),
  INDEX idx_provider (provider)
);
```

## Keycloak Admin API

### 列出身份提供商

```bash
curl https://keycloak.yourdomain.com/admin/realms/auth9/identity-provider/instances \
  -H "Authorization: Bearer <admin_token>"
```

### 创建身份提供商

```bash
curl -X POST https://keycloak.yourdomain.com/admin/realms/auth9/identity-provider/instances \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "google",
    "displayName": "Google",
    "providerId": "google",
    "enabled": true,
    "config": {
      "clientId": "xxx",
      "clientSecret": "xxx"
    }
  }'
```

### 获取用户关联身份

```bash
curl https://keycloak.yourdomain.com/admin/realms/auth9/users/{user_id}/federated-identity \
  -H "Authorization: Bearer <admin_token>"
```

## 审计日志

身份提供商相关操作会记录审计日志：

| 事件类型 | 描述 |
|---------|------|
| `idp.created` | 创建身份提供商 |
| `idp.updated` | 更新身份提供商 |
| `idp.deleted` | 删除身份提供商 |
| `idp.login_success` | 通过 IdP 登录成功 |
| `idp.login_failed` | 通过 IdP 登录失败 |
| `identity.linked` | 关联身份 |
| `identity.unlinked` | 解绑身份 |

## 安全建议

### 1. 提供商配置

- ✅ 使用 HTTPS 回调 URL
- ✅ 限制 OAuth 范围到最小必要
- ✅ 定期轮换 Client Secret
- ✅ 验证 Token 签名

### 2. 用户关联

- ✅ 验证邮箱再关联
- ✅ 发送关联通知邮件
- ✅ 允许用户解绑
- ✅ 保留至少一种登录方式

### 3. 企业 SSO

- ✅ 使用 SAML 签名
- ✅ 验证断言签名
- ✅ 配置合理的会话超时
- ✅ 启用单点登出

## 故障排查

### 登录失败

| 错误 | 原因 | 解决方案 |
|------|------|---------|
| invalid_client | Client ID/Secret 错误 | 检查配置 |
| redirect_uri_mismatch | 回调 URL 不匹配 | 更新提供商配置 |
| access_denied | 用户拒绝授权 | 用户需重新授权 |
| invalid_grant | Token 过期或已使用 | 重新发起登录 |

### SAML 问题

| 错误 | 原因 | 解决方案 |
|------|------|---------|
| Invalid signature | 签名验证失败 | 检查证书配置 |
| Invalid audience | Audience 不匹配 | 检查 Entity ID |
| Invalid status | IdP 返回错误状态 | 查看 IdP 日志 |

## 常见问题

### Q: 如何限制只允许特定域名的邮箱登录？

A: 对于 Google，使用 `hostedDomain` 配置；对于其他提供商，可以在用户属性映射中添加验证逻辑。

### Q: 用户可以关联多个相同提供商的账户吗？

A: 不可以。每个提供商只能关联一个账户。

### Q: 如何迁移现有用户到 SSO？

A:
1. 配置身份提供商
2. 通知用户通过 SSO 登录
3. 系统自动基于邮箱关联
4. 可选：禁用密码登录

### Q: SAML 和 OIDC 如何选择？

A:
- **OIDC**：更现代、更简单、适合新项目
- **SAML**：企业标准、兼容性好、适合遗留系统集成

## 相关文档

- [认证流程](认证流程.md)
- [多租户管理](多租户管理.md)
- [WebAuthn与Passkey](WebAuthn与Passkey.md)
- [REST API](REST-API.md)
