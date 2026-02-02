# FAQ - 常见问题

## 一般问题

### Q: Auth9 是什么？

A: Auth9 是一个开源的自托管身份认证和访问管理平台，提供企业级的 SSO、多租户管理和 RBAC 功能，是 Auth0 等商业解决方案的替代品。

### Q: Auth9 和 Keycloak 的关系是什么？

A: Auth9 使用 Keycloak 作为底层的认证引擎，但提供了更简洁的管理界面和更灵活的多租户架构。Keycloak 负责核心的 OIDC 协议和用户认证，Auth9 在此基础上增加了租户管理、动态 RBAC 和 Token 交换等功能。

### Q: Auth9 收费吗？

A: Auth9 是完全开源和免费的，采用 MIT 许可证。您可以免费使用、修改和部署。

### Q: Auth9 支持哪些认证方式？

A: 
- 用户名/密码认证
- OIDC/OAuth2 SSO
- 社交登录（Google、GitHub 等）
- 多因素认证（TOTP、SMS、Email）
- SAML 2.0（通过 Keycloak）

### Q: Auth9 的最低硬件要求是什么？

A: 
- **开发环境**：4GB 内存，2 核 CPU，20GB 磁盘
- **小型生产**：8GB 内存，4 核 CPU，50GB 磁盘
- **中型生产**：16GB 内存，8 核 CPU，100GB 磁盘

## 安装和部署

### Q: 如何快速体验 Auth9？

A: 使用 Docker Compose 是最快的方式：

```bash
git clone https://github.com/gpgkd906/auth9.git
cd auth9
docker-compose up -d
```

然后访问 http://localhost:3000

详见：[快速开始](快速开始.md)

### Q: 可以在 Windows 上运行吗？

A: 可以，但推荐使用 Docker Desktop 或 WSL2。不建议直接在 Windows 上编译运行。

### Q: 支持哪些数据库？

A: 
- **推荐**：TiDB（MySQL 兼容）
- **支持**：MySQL 5.7+、MariaDB 10.3+
- **未来支持**：PostgreSQL

### Q: 必须使用 Keycloak 吗？

A: 当前版本是的。Keycloak 是核心认证引擎。未来版本可能会支持其他认证提供商。

### Q: 如何升级到新版本？

A: 
1. 备份数据库
2. 拉取新镜像
3. 运行数据库迁移
4. 滚动更新服务

详见：[安装部署](安装部署.md#5-升级指南)

## 多租户

### Q: 一个用户可以属于多个租户吗？

A: 可以。Auth9 支持用户跨租户，一个用户可以同时属于多个租户，并在不同租户中拥有不同的角色。

### Q: 租户之间的数据会相互影响吗？

A: 不会。Auth9 在数据库、API 和缓存层都实现了完整的租户隔离。

### Q: 如何切换租户？

A: 
- **Web UI**：点击顶部的租户选择器
- **API**：在请求头中设置 `X-Tenant-ID`
- **gRPC**：通过 Token Exchange 获取特定租户的 Token

### Q: 租户数量有限制吗？

A: 没有硬性限制，但建议根据硬件资源合理规划。单实例支持数千个租户。

### Q: 可以删除租户吗？

A: 当前版本只支持禁用租户，不支持物理删除。禁用后租户数据会保留但无法访问。

## 用户和认证

### Q: 如何重置用户密码？

A: 
- **管理员**：在管理界面或通过 API 重置
- **用户自助**：使用"忘记密码"功能

```bash
curl -X POST /api/v1/users/{user_id}/reset-password \
  -H "Authorization: Bearer <token>" \
  -d '{"new_password": "NewPass123!"}'
```

### Q: 支持邮箱验证吗？

A: 支持。可以配置要求用户验证邮箱后才能登录。

### Q: 如何邀请用户加入租户？

A: Auth9 提供邀请系统：

```bash
curl -X POST /api/v1/tenants/{tenant_id}/invitations \
  -H "Authorization: Bearer <token>" \
  -d '{
    "email": "newuser@example.com",
    "role_ids": ["role-uuid"],
    "expires_in_days": 7
  }'
```

系统会自动发送邀请邮件，用户点击链接即可加入租户。详见 [多租户管理](多租户管理.md#通过邀请添加用户)。

### Q: 邀请链接有效期多久？

A: 默认 7 天，可以在创建邀请时通过 `expires_in_days` 参数自定义（1-30 天）。

### Q: 如何撤销邀请？

A: 通过 API 删除邀请：

```bash
curl -X DELETE /api/v1/invitations/{invitation_id} \
  -H "Authorization: Bearer <token>"
```

### Q: 如何启用 MFA？

A: 
1. 在租户设置中启用 MFA
2. 用户在个人设置中绑定 MFA 设备
3. 支持 TOTP、SMS 和 Email 方式

### Q: 忘记管理员密码怎么办？

A: 
1. 直接访问 Keycloak Admin Console（默认 http://localhost:8081/admin）
2. 使用 Keycloak 管理员账号重置
3. 或通过数据库直接修改

### Q: Token 过期了怎么办？

A: 使用 Refresh Token 刷新：

```bash
curl -X POST /api/v1/auth/refresh \
  -d '{"refresh_token": "your-refresh-token"}'
```

## 权限和角色

### Q: RBAC 和 ABAC 有什么区别？

A: Auth9 主要实现 RBAC（基于角色的访问控制）。未来版本可能会支持 ABAC（基于属性的访问控制）。

### Q: 角色可以继承吗？

A: 可以。Auth9 支持角色继承，子角色会自动继承父角色的所有权限。

### Q: 如何为用户分配角色？

A: 
```bash
curl -X POST /api/v1/rbac/assign \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "role_ids": ["role-uuid-1", "role-uuid-2"]
  }'
```

### Q: 权限变更多久生效？

A: 
- **新 Token**：立即生效
- **现有 Token**：需要重新交换 Token（默认 5 分钟缓存）

### Q: 如何查看用户的所有权限？

A: 
```bash
curl "/api/v1/rbac/user-roles?user_id=xxx&tenant_id=xxx" \
  -H "Authorization: Bearer <token>"
```

## 集成

### Q: 如何在我的应用中集成 Auth9？

A: 
1. 在 Auth9 中注册您的应用
2. 获取 client_id 和 client_secret
3. 实现 OIDC 认证流程
4. 使用 gRPC 进行 Token 交换

详见：[认证流程](认证流程.md)

### Q: 支持哪些编程语言？

A: 任何支持 HTTP 和 gRPC 的语言都可以集成，包括：
- Rust
- Go
- Node.js/TypeScript
- Python
- Java
- C#
- PHP

### Q: 有 SDK 吗？

A: 
- **官方**：计划中的 auth9-sdk
- **社区**：欢迎贡献

当前可以使用标准的 OIDC 库和 gRPC 客户端。

### Q: 可以和现有的认证系统集成吗？

A: 可以通过 Keycloak 的 Federation 功能集成：
- LDAP/Active Directory
- 其他 OIDC Provider
- SAML IdP

## 性能

### Q: Auth9 的性能如何？

A: 
- REST API：< 50ms P99
- Token Exchange：< 20ms P99
- 支持 1000+ QPS（单实例）

### Q: 如何优化性能？

A: 
1. 启用 Redis 缓存
2. 增加服务副本数
3. 优化数据库索引
4. 使用 CDN 缓存静态资源
5. 启用 gRPC 连接池

详见：[架构设计](架构设计.md#6-性能设计) 和 [请求流向说明](请求流向说明.md)

### Q: 支持水平扩展吗？

A: 完全支持。auth9-core 是无状态的，可以自由扩展副本数。

### Q: 数据库会成为瓶颈吗？

A: TiDB 原生支持分布式扩展，不会成为瓶颈。如使用 MySQL，建议配置主从复制。

## 安全

### Q: Auth9 安全吗？

A: 
- ✅ 基于成熟的 Keycloak 认证引擎
- ✅ 使用行业标准的 JWT 和 OIDC
- ✅ 支持 MFA
- ✅ 完整的审计日志
- ✅ 定期安全更新

### Q: 如何报告安全漏洞？

A: 请发送邮件到 security@auth9.dev（如果有）或在 GitHub 上私下报告。

### Q: Token 存储在哪里？

A: 
- **推荐**：HttpOnly Cookie（Web）或 Secure Storage（移动端）
- **不推荐**：LocalStorage（易受 XSS 攻击）

### Q: 如何防止暴力破解？

A: 
- 启用登录频率限制
- 配置账号锁定策略
- 启用 MFA
- 监控异常登录

### Q: 支持 HTTPS 吗？

A: 支持。生产环境强烈建议启用 HTTPS。

## 品牌定制和邮件

### Q: 如何自定义 Auth9 的外观？

A: Auth9 支持完整的品牌定制：

```bash
curl -X PUT /api/v1/system/branding \
  -H "Authorization: Bearer <token>" \
  -d '{
    "config": {
      "logo_url": "https://example.com/logo.png",
      "primary_color": "#007AFF",
      "company_name": "我的公司"
    }
  }'
```

配置会立即应用到：
- Auth9 管理界面
- Keycloak 登录页面（动态加载）
- 邀请邮件

详见 [Keycloak 主题定制](Keycloak主题定制.md)。

### Q: 如何配置邮件服务？

A: 通过 API 配置邮件设置：

```bash
curl -X PUT /api/v1/system/email-settings \
  -H "Authorization: Bearer <token>" \
  -d '{
    "config": {
      "provider": "smtp",
      "smtp_host": "smtp.gmail.com",
      "smtp_port": 587,
      "smtp_username": "your-email@gmail.com",
      "smtp_password": "app-password"
    }
  }'
```

支持 SMTP 和 AWS SES。详见 [配置说明](配置说明.md#19-邮件配置)。

### Q: 可以自定义邮件模板吗？

A: 可以。Auth9 支持自定义所有邮件模板（邀请、密码重置等）：

```bash
curl -X PUT /api/v1/email-templates/invitation \
  -H "Authorization: Bearer <token>" \
  -d '{
    "subject": "欢迎加入 {{tenant_name}}",
    "html_body": "<html>...</html>"
  }'
```

支持变量替换，如 `{{user_name}}`、`{{tenant_name}}` 等。

### Q: 如何测试邮件配置？

A: 使用测试邮件功能：

```bash
curl -X POST /api/v1/system/email-settings/test \
  -H "Authorization: Bearer <token>" \
  -d '{"to_email": "test@example.com"}'
```

## 监控和维护

### Q: 如何监控 Auth9？

A: 
- Prometheus metrics：`/metrics` 端点
- 健康检查：`/health` 端点
- 日志聚合：支持 ELK、Loki 等

详见：[分析与安全告警](分析与安全告警.md)

### Q: 如何备份数据？

A: 
```bash
# 备份数据库
mysqldump -h tidb -u root -p auth9 > backup.sql

# 备份 Redis（可选）
redis-cli BGSAVE
```

建议定期使用数据库的原生备份工具进行备份， 并将备份文件存储在安全的位置。 详见：[安装部署](安装部署.md)

### Q: 数据保留多久？

A: 
- 用户数据：永久（除非删除）
- 审计日志：可配置（默认 90 天）
- Session 数据：配置的过期时间

### Q: 如何清理旧数据？

A: 
```bash
# 清理过期的审计日志
curl -X POST /api/v1/admin/cleanup-audit-logs \
  -H "Authorization: Bearer <token>" \
  -d '{"older_than_days": 90}'
```

## 社区和支持

### Q: 如何贡献代码？

A: 
1. Fork 仓库
2. 创建功能分支
3. 提交 Pull Request
4. 等待 Review

详见：[贡献指南](贡献指南.md)

### Q: 在哪里提问？

A: 
- **GitHub Issues**：Bug 报告和功能请求
- **GitHub Discussions**：一般讨论和问题
- **Stack Overflow**：标签 `auth9`

### Q: 有商业支持吗？

A: 目前是社区支持。未来可能提供商业支持服务。

### Q: 可以商用吗？

A: 可以。Auth9 采用 MIT 许可证，允许商业使用。

### Q: 有付费版本吗？

A: 没有。Auth9 完全开源免费。

## 技术问题

### Q: 为什么选择 Rust？

A: 
- 内存安全
- 高性能
- 优秀的并发支持
- 丰富的生态系统

### Q: 为什么使用 TiDB？

A: 
- MySQL 兼容（易迁移）
- 水平扩展能力
- 分布式事务
- HTAP 能力

### Q: 前端为什么用 React Router 7？

A: 
- 优秀的 SSR 支持
- 现代化开发体验
- 类型安全的 loaders 和 actions
- 良好的性能
- 文件系统路由

### Q: 可以替换数据库吗？

A: 理论上可以，但需要修改 SQL 语句和迁移脚本。

### Q: 可以不用 Docker 吗？

A: 可以直接从源码编译运行，但 Docker 是推荐的部署方式。

## 路线图

### Q: 未来会支持哪些功能？

A: 
- [ ] 更多认证方式（WebAuthn、生物识别）
- [ ] ABAC 支持
- [ ] 更多语言的 SDK
- [ ] GraphQL API
- [ ] 更好的 UI/UX
- [ ] 插件系统
- [ ] 更多集成（Slack、Teams 等）

关注 [GitHub Issues](https://github.com/gpgkd906/auth9/issues) 和 [Discussions](https://github.com/gpgkd906/auth9/discussions) 获取最新规划。

### Q: 何时发布 1.0 版本？

A: 计划在 2024 年 Q2。

### Q: 会支持 Postgres 吗？

A: 在路线图中，预计 0.3.0 版本。

## 其他

### Q: Auth9 这个名字的由来？

A: Auth（认证）+ 9（"久"的谐音，寓意长久），寓意提供持久稳定的认证服务。

### Q: 文档在哪里？

A: 
- **Wiki**：https://github.com/gpgkd906/auth9/wiki
- **API 文档**：https://api.auth9.dev/docs（如果有）
- **本地**：`/docs` 目录

### Q: 如何获取最新消息？

A: 
- Watch GitHub 仓库
- 关注 Release Notes
- 加入社区讨论

## 没有找到答案？

如果您的问题未在此列出，请：

1. 搜索 [GitHub Issues](https://github.com/gpgkd906/auth9/issues)
2. 查看 [GitHub Discussions](https://github.com/gpgkd906/auth9/discussions)
3. 提交新的 Issue 或 Discussion

## 相关文档

- [快速开始](快速开始.md)
- [架构设计](架构设计.md)
- [REST API](REST-API.md)
- [故障排查](故障排查.md)
- [最佳实践](最佳实践.md)
