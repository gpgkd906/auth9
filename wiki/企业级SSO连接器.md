# 企业级 SSO 连接器

Auth9 支持**租户级别**的企业 SSO 连接器配置，允许每个组织（租户）独立配置自己的身份提供商，实现企业单点登录。

## 核心概念

### 企业 SSO vs 全局身份提供商

Auth9 支持两种级别的身份提供商配置：

| 类型 | 配置级别 | 适用场景 | 配置位置 |
|------|---------|---------|---------|
| **全局身份提供商** | 系统级别 | 所有租户共享（如 Google、GitHub） | Settings > Identity Providers |
| **企业 SSO 连接器** | 租户级别 | 单个租户独享（如企业 OIDC、SAML） | Tenants > [租户] > SSO |

**本文档主要介绍租户级别的企业 SSO 连接器。**

### 企业 SSO 连接器

企业 SSO 连接器（Enterprise SSO Connector）是租户级别的身份提供商配置，特点：

- **租户隔离** - 每个租户独立配置，互不影响
- **协议支持** - 支持 OIDC 和 SAML 2.0 协议
- **域名绑定** - 可以绑定企业域名，实现域名路由
- **优先级控制** - 支持多个连接器，可设置优先级
- **动态启用** - 可随时启用或禁用连接器

### 域名绑定

企业 SSO 连接器可以绑定一个或多个企业域名：

```
SSO 连接器: Acme OIDC
绑定域名:
  - acme.com (主域名)
  - acme.cn (次域名)
```

**用途**：

1. **域名路由** - 根据用户邮箱域名自动选择 SSO 连接器
2. **域名验证** - 验证组织对域名的所有权
3. **品牌识别** - 在登录页面显示企业品牌信息

## 支持的协议

### OIDC (OpenID Connect)

适用于对接现代身份提供商：

- **Azure AD / Entra ID** - 微软企业身份服务
- **Okta** - 企业身份管理平台
- **Auth0** - 身份认证即服务
- **Google Workspace** - Google 企业套件
- **自建 OIDC 服务** - 基于 Keycloak、ORY Hydra 等

**所需配置**：
- Client ID
- Client Secret
- Authorization Endpoint
- Token Endpoint
- User Info Endpoint
- JWKS URI（可选）

### SAML 2.0

适用于对接传统企业身份提供商：

- **ADFS** - Active Directory Federation Services
- **Shibboleth** - 学术机构常用
- **PingFederate** - 企业级身份联邦
- **传统企业 SSO** - 银行、政府等机构

**所需配置**：
- Entity ID
- SSO Service URL
- Logout Service URL（可选）
- 签名证书
- 加密证书（可选）

## 配置企业 SSO 连接器

### 通过管理界面配置

#### 步骤 1：进入租户 SSO 设置

1. 登录 Auth9 管理界面
2. 导航到 **Tenants**
3. 点击目标租户进入详情页
4. 点击 **SSO** 标签页

#### 步骤 2：创建 OIDC 连接器

1. 点击 **Add SSO Connector** 按钮
2. 选择 **OIDC** 协议
3. 填写配置信息：

```
基本信息：
- 别名（Alias）: acme-azure-ad
- 显示名称: Acme Azure AD
- 优先级: 100

OIDC 配置：
- Client ID: your-client-id
- Client Secret: your-client-secret
- Authorization Endpoint: https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize
- Token Endpoint: https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token
- User Info Endpoint: https://graph.microsoft.com/oidc/userinfo

高级选项：
- 启用状态: ✓ 已启用
- Scopes: openid profile email
- 声明映射:
  - 用户名: preferred_username
  - 邮箱: email
  - 姓名: name
```

4. 点击 **Create** 完成创建

#### 步骤 3：绑定企业域名

1. 在 SSO 连接器列表中找到刚创建的连接器
2. 点击 **Manage Domains**
3. 点击 **Add Domain**
4. 输入域名：`acme.com`
5. 设置为主域名：✓
6. 点击 **Add**

#### 步骤 4：测试连接器

1. 点击 SSO 连接器右侧的 **Test** 按钮
2. 系统会打开新窗口跳转到身份提供商
3. 使用企业账号登录
4. 验证能否成功回调并获取用户信息

### 通过 REST API 配置

#### 列出租户的 SSO 连接器

```bash
curl https://api.auth9.example.com/api/v1/tenants/{tenant-id}/sso-connectors \
  -H "Authorization: Bearer <admin_token>"
```

响应：

```json
{
  "data": [
    {
      "id": "connector-uuid",
      "tenant_id": "tenant-uuid",
      "alias": "acme-azure-ad",
      "display_name": "Acme Azure AD",
      "provider_type": "oidc",
      "enabled": true,
      "priority": 100,
      "config": {
        "clientId": "your-client-id",
        "authorizationEndpoint": "https://login.microsoftonline.com/.../authorize",
        "tokenEndpoint": "https://login.microsoftonline.com/.../token"
      },
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

#### 创建 SSO 连接器

```bash
curl -X POST https://api.auth9.example.com/api/v1/tenants/{tenant-id}/sso-connectors \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "acme-azure-ad",
    "display_name": "Acme Azure AD",
    "provider_type": "oidc",
    "enabled": true,
    "priority": 100,
    "config": {
      "clientId": "your-client-id",
      "clientSecret": "your-client-secret",
      "authorizationEndpoint": "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize",
      "tokenEndpoint": "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token",
      "userInfoEndpoint": "https://graph.microsoft.com/oidc/userinfo",
      "scopes": "openid profile email"
    }
  }'
```

#### 绑定域名到 SSO 连接器

```bash
curl -X POST https://api.auth9.example.com/api/v1/tenants/{tenant-id}/sso-connectors/{connector-id}/domains \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "domain": "acme.com",
    "is_primary": true
  }'
```

## 配置示例

### 示例 1：对接 Azure AD (OIDC)

**场景**：Acme 公司使用 Microsoft Azure AD 管理员工账号

**Azure AD 配置**：

1. 在 Azure Portal 中注册应用
2. 获取以下信息：
   - Application (client) ID: `abc123-def456-...`
   - Directory (tenant) ID: `xyz789-uvw012-...`
   - Client Secret: `your-secret`

**Auth9 配置**：

```json
{
  "alias": "acme-azure-ad",
  "display_name": "Acme Corporation Azure AD",
  "provider_type": "oidc",
  "enabled": true,
  "priority": 100,
  "config": {
    "clientId": "abc123-def456-...",
    "clientSecret": "your-secret",
    "authorizationEndpoint": "https://login.microsoftonline.com/xyz789-uvw012-.../oauth2/v2.0/authorize",
    "tokenEndpoint": "https://login.microsoftonline.com/xyz789-uvw012-.../oauth2/v2.0/token",
    "userInfoEndpoint": "https://graph.microsoft.com/oidc/userinfo",
    "scopes": "openid profile email",
    "claimMappings": {
      "username": "preferred_username",
      "email": "email",
      "firstName": "given_name",
      "lastName": "family_name"
    }
  }
}
```

**绑定域名**：`acme.com`

### 示例 2：对接 Okta (OIDC)

**场景**：Beta 公司使用 Okta 管理员工身份

**Okta 配置**：

1. 在 Okta 管理控制台创建应用
2. 选择 "Web Application"
3. 配置回调 URL：`https://auth9.example.com/realms/{tenant}/broker/{alias}/endpoint`
4. 获取配置信息

**Auth9 配置**：

```json
{
  "alias": "beta-okta",
  "display_name": "Beta Company Okta",
  "provider_type": "oidc",
  "enabled": true,
  "priority": 100,
  "config": {
    "clientId": "okta-client-id",
    "clientSecret": "okta-client-secret",
    "authorizationEndpoint": "https://beta.okta.com/oauth2/v1/authorize",
    "tokenEndpoint": "https://beta.okta.com/oauth2/v1/token",
    "userInfoEndpoint": "https://beta.okta.com/oauth2/v1/userinfo",
    "scopes": "openid profile email",
    "issuer": "https://beta.okta.com"
  }
}
```

**绑定域名**：`beta.com`

### 示例 3：对接 ADFS (SAML 2.0)

**场景**：Gamma 公司使用 Windows Server ADFS

**ADFS 配置**：

1. 在 ADFS 管理控制台添加信赖方信任
2. 配置 SAML 断言
3. 导出签名证书

**Auth9 配置**：

```json
{
  "alias": "gamma-adfs",
  "display_name": "Gamma Company ADFS",
  "provider_type": "saml",
  "enabled": true,
  "priority": 100,
  "config": {
    "entityId": "https://adfs.gamma.com/adfs/services/trust",
    "singleSignOnServiceUrl": "https://adfs.gamma.com/adfs/ls/",
    "singleLogoutServiceUrl": "https://adfs.gamma.com/adfs/ls/?wa=wsignout1.0",
    "signingCertificate": "-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----",
    "nameIdFormat": "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress",
    "attributeMappings": {
      "email": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
      "firstName": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname",
      "lastName": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname"
    }
  }
}
```

**绑定域名**：`gamma.com`

## 登录流程

### 标准登录流程（带域名绑定）

用户使用企业邮箱登录时：

```
1. 用户访问：https://auth9.example.com/login

2. 输入邮箱：zhangsan@acme.com

3. 系统检测邮箱域名：acme.com

4. 查找绑定的 SSO 连接器：Acme Azure AD

5. 自动跳转到 Azure AD 登录页

6. 用户在 Azure AD 完成认证

7. 回调到 Auth9，创建或关联用户

8. 返回 Identity Token（包含租户信息）

9. 应用可进行 Token Exchange 获取租户 Access Token
```

### 手动选择 SSO 连接器

如果未配置域名绑定或用户想手动选择：

1. 在登录页面点击 **Enterprise SSO**
2. 选择目标租户
3. 选择 SSO 连接器
4. 跳转到身份提供商登录

## 用户身份映射

### 首次登录

用户通过企业 SSO 首次登录时：

1. **创建本地用户** - 在 Auth9 数据库创建用户记录
2. **关联身份** - 将企业身份与本地用户关联
3. **分配租户** - 将用户添加到对应租户
4. **默认角色** - 分配默认角色（如果配置）

### 已有用户登录

如果用户已存在（通过邮箱匹配）：

1. **身份关联** - 关联企业身份到现有用户
2. **租户关联** - 如果用户未加入该租户，自动加入
3. **更新信息** - 从企业 SSO 更新用户信息（姓名、邮箱等）

### 声明/属性映射

配置如何从 SSO 提供商的声明映射到 Auth9 用户属性：

**OIDC 声明映射**：

```json
{
  "claimMappings": {
    "username": "preferred_username",
    "email": "email",
    "firstName": "given_name",
    "lastName": "family_name",
    "avatar": "picture"
  }
}
```

**SAML 属性映射**：

```json
{
  "attributeMappings": {
    "email": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
    "firstName": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname",
    "lastName": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname",
    "username": "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/upn"
  }
}
```

## 优先级和路由

### 优先级设置

当一个租户配置了多个 SSO 连接器时，通过优先级控制选择顺序：

```
连接器 A: 优先级 100 (最高)
连接器 B: 优先级 200
连接器 C: 优先级 300 (最低)
```

**规则**：

- 数字越小，优先级越高
- 当多个连接器都匹配时，选择优先级最高的
- 默认优先级：100

### 域名路由

通过域名自动路由到对应的 SSO 连接器：

```
用户邮箱          域名       SSO 连接器
──────────────  ────────  ──────────────
zhangsan@acme.com   → acme.com   → Acme Azure AD
lisi@acme.cn        → acme.cn    → Acme Azure AD (次域名)
wangwu@beta.com     → beta.com   → Beta Okta
zhaoliu@gamma.com   → gamma.com  → Gamma ADFS
```

**路由逻辑**：

1. 提取用户邮箱的域名部分
2. 查找该域名绑定的 SSO 连接器
3. 如果找到且已启用，自动使用该连接器
4. 如果未找到或已禁用，使用普通登录流程

## 安全配置

### 签名验证

**OIDC**：

- 验证 ID Token 签名（使用 JWKS）
- 验证 Token 的 issuer
- 验证 Token 的 audience

**SAML**：

- 验证 SAML 断言签名
- 验证响应签名（可选）
- 验证证书有效期

### 加密传输

**OIDC**：

- 使用 HTTPS 传输所有数据
- Client Secret 加密存储
- Token 使用 TLS 保护

**SAML**：

- 支持 SAML 响应加密
- 支持 SAML 断言加密
- 私钥加密存储

### 会话管理

- **会话超时** - 继承租户的会话策略
- **单点登出** - 支持 OIDC RP-Initiated Logout 和 SAML SLO
- **并发控制** - 限制同一用户的并发会话数

## 故障排查

### 常见问题

#### Q1: SSO 登录后无法获取用户信息

**排查步骤**：

1. 检查 User Info Endpoint 是否正确
2. 检查 Scopes 是否包含 `profile` 和 `email`
3. 检查声明映射配置是否正确
4. 查看 Auth9 日志：`docker logs auth9-core`

#### Q2: SAML 断言验证失败

**可能原因**：

1. 签名证书不正确或已过期
2. 时钟偏差（SAML 对时间敏感）
3. Entity ID 不匹配
4. Audience 限制不匹配

**解决方法**：

```bash
# 查看 SAML 响应详情
docker logs auth9-core | grep "SAML"

# 检查证书有效期
openssl x509 -in cert.pem -noout -dates
```

#### Q3: 域名路由不生效

**检查清单**：

- [ ] 域名已正确绑定到 SSO 连接器
- [ ] SSO 连接器已启用
- [ ] 用户邮箱域名与绑定域名完全匹配
- [ ] 没有更高优先级的连接器覆盖

#### Q4: 用户重复创建

**原因**：邮箱或用户名映射不一致

**解决**：

1. 检查声明/属性映射配置
2. 确保 `email` 字段始终被正确映射
3. 启用"根据邮箱匹配现有用户"选项

### 调试技巧

#### 启用详细日志

在 `auth9-core` 配置中启用 DEBUG 日志：

```bash
export RUST_LOG=debug
```

#### 查看 Keycloak 事件

1. 登录 Keycloak 管理控制台
2. 导航到 **Realm Settings** > **Events**
3. 启用事件记录
4. 查看登录事件和错误日志

#### 使用浏览器开发工具

1. 打开浏览器开发工具 (F12)
2. 切换到 Network 标签页
3. 观察 SSO 跳转和回调请求
4. 检查响应状态码和内容

## 最佳实践

### 1. 使用专用 Client Credentials

为每个租户创建独立的 Client ID 和 Secret：

- **不要**：所有租户共用一个 OIDC 应用
- **应该**：为每个租户在 Azure AD/Okta 中创建独立应用

### 2. 定期轮换密钥

- 定期更新 Client Secret
- 定期更新签名证书
- 保留旧证书一段时间以支持平滑过渡

### 3. 最小化声明/属性

只请求必要的用户信息：

```json
{
  "scopes": "openid email",  // 最小化 Scopes
  "claimMappings": {
    "email": "email"  // 只映射必要字段
  }
}
```

### 4. 配置登出回调

确保单点登出正常工作：

- 配置 Logout Redirect URI
- 实现 RP-Initiated Logout (OIDC)
- 配置 SLO Endpoint (SAML)

### 5. 监控和告警

- 监控 SSO 登录成功率
- 告警 SSO 连接器故障
- 记录异常登录行为

### 6. 用户体验优化

- 设置友好的显示名称
- 配置企业 Logo（通过品牌定制）
- 提供清晰的错误提示

## 相关文档

- [社交登录与SSO](社交登录与SSO.md) - 全局身份提供商配置
- [B2B 入驻与组织创建](B2B入驻与组织创建.md) - 组织创建和域名设置
- [认证流程](认证流程.md) - OIDC 认证流程详解
- [多租户管理](多租户管理.md) - 租户管理功能
- [最佳实践](最佳实践.md) - 安全配置建议

---

**最后更新**: 2026-02-17
