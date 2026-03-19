# Auth9 QA 测试用例文档

本目录包含 Auth9 系统的手动测试用例。文档正在向“每文档不超过 5 个场景”的规范收敛，便于多名 QA 工程师并行测试。

默认原则：

- QA 应优先验证 Auth9 Portal、Auth9 API 和用户可见的 Auth9 产品行为。
- 认证链路中的托管页面统一称为”Auth9 品牌认证页”，由 Auth9 内置 OIDC 引擎承载。
- 底层身份引擎相关配置通过 Auth9 API 或数据库直接验证。

## 测试用例索引

## 文档治理

- 规范文件: [docs/qa/_standards.md](./_standards.md)
- 清单真值: [docs/qa/_manifest.yaml](./_manifest.yaml)
- 校验脚本: `./scripts/qa-doc-lint.sh`
- 周期执行入口: `./scripts/run-weekly-qa-governance.sh`

推荐周期任务：

1. 每周执行一次 `./scripts/run-weekly-qa-governance.sh`
2. 版本发布前强制执行一次 `./scripts/run-weekly-qa-governance.sh`
3. 仅看审计不阻断时可用 `./scripts/run-weekly-qa-governance.sh --no-lint`

### 租户管理 (5 个文档, 25 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [tenant/01-crud.md](./tenant/01-crud.md) | 创建、更新、删除操作 | 5 |
| [tenant/02-list-settings.md](./tenant/02-list-settings.md) | 列表、搜索、设置 | 5 |
| [tenant/03-status-lifecycle.md](./tenant/03-status-lifecycle.md) | 租户状态生命周期（Active/Inactive/Suspended）及业务影响 | 5 |
| [tenant/04-b2b-org-creation.md](./tenant/04-b2b-org-creation.md) | B2B 组织自助创建、域名验证、Pending 状态、/users/me/tenants | 5 |
| [tenant/05-security-malicious-ip-blacklist.md](./tenant/05-security-malicious-ip-blacklist.md) | 租户级恶意 IP 黑名单配置、租户隔离与平台优先级 | 5 |

### 用户管理 (6 个文档, 28 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [user/01-crud.md](./user/01-crud.md) | 创建、更新、租户关联 | 5 |
| [user/02-advanced.md](./user/02-advanced.md) | 删除、MFA、列表 | 5 |
| [user/03-validation.md](./user/03-validation.md) | 边界测试、验证 | 3 |
| [user/04-account-profile.md](./user/04-account-profile.md) | 个人资料 API、Profile 页面、自更新权限 | 5 |
| [user/05-account-security.md](./user/05-account-security.md) | 修改密码、Passkeys、会话、关联身份 | 5 |
| [user/06-account-navigation.md](./user/06-account-navigation.md) | Account 导航布局、侧边栏、Settings 清理 | 5 |

### RBAC 角色权限 (5 个文档, 22 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [rbac/01-permission.md](./rbac/01-permission.md) | 权限 CRUD | 4 |
| [rbac/02-role.md](./rbac/02-role.md) | 角色 CRUD、继承 | 5 |
| [rbac/03-assignment.md](./rbac/03-assignment.md) | 权限分配、用户角色 | 5 |
| [rbac/04-advanced.md](./rbac/04-advanced.md) | 层次视图、循环检测 | 3 |
| [rbac/05-abac-policy-management.md](./rbac/05-abac-policy-management.md) | ABAC 策略草稿、发布、回滚、模拟 | 5 |

### 服务与客户端 (6 个文档, 30 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [service/01-service-crud.md](./service/01-service-crud.md) | 服务 CRUD（含 Actions/Branding 级联删除） | 5 |
| [service/02-client.md](./service/02-client.md) | 客户端管理、密钥 | 5 |
| [service/03-oidc.md](./service/03-oidc.md) | OIDC 配置、URI 验证 | 5 |
| [service/04-tenant-service-toggle.md](./service/04-tenant-service-toggle.md) | 租户服务启停 | 5 |
| [service/05-integration-info.md](./service/05-integration-info.md) | 集成信息 API 与 Portal 页面 | 5 |
| [service/06-service-branding.md](./service/06-service-branding.md) | Service 级品牌配置、公开端点 client_id、托管认证页品牌集成 | 5 |

### 邀请管理 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [invitation/01-create-send.md](./invitation/01-create-send.md) | 创建、发送邀请 | 5 |
| [invitation/02-accept.md](./invitation/02-accept.md) | 接受邀请流程 | 5 |
| [invitation/03-manage.md](./invitation/03-manage.md) | 撤销、删除、过滤 | 5 |

### 会话与安全 (8 个文档, 39 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [session/01-session.md](./session/01-session.md) | 会话管理、撤销 | 5 |
| [session/02-login-events.md](./session/02-login-events.md) | 登录事件记录 | 5 |
| [session/03-alerts.md](./session/03-alerts.md) | 安全告警检测 | 5 |
| [session/04-boundary.md](./session/04-boundary.md) | 边界测试 | 5 |
| [session/05-auth-security-regression.md](./session/05-auth-security-regression.md) | 鉴权与令牌安全回归（越权强退、refresh 撤销一致性、callback token 泄露、限流绕过） | 5 |
| [session/06-token-blacklist-failsafe.md](./session/06-token-blacklist-failsafe.md) | Token 黑名单 Fail-Closed 策略（Redis 故障 503、重试机制、向后兼容） | 4 |
| [session/07-oauth-state-csrf.md](./session/07-oauth-state-csrf.md) | OAuth State CSRF 校验（cookie 存储、回调校验、过期、安全属性） | 5 |
| [session/08-identity-token-whitelist-tenant-token-enforcement.md](./session/08-identity-token-whitelist-tenant-token-enforcement.md) | Identity Token 白名单、Tenant Token 强制校验、切租户 token 边界 | 5 |

### Webhook (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [webhook/01-crud.md](./webhook/01-crud.md) | Webhook CRUD | 5 |
| [webhook/02-trigger.md](./webhook/02-trigger.md) | 事件触发、签名 | 5 |
| [webhook/03-reliability.md](./webhook/03-reliability.md) | 重试、自动禁用 | 4 |
| [webhook/04-boundary.md](./webhook/04-boundary.md) | URL 验证、边界 | 3 |

### 认证流程 (23 个文档, 107 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [auth/01-oidc-login.md](./auth/01-oidc-login.md) | OIDC 登录流程（**Sign in with password** 路径） | 4 |
| [auth/02-token-exchange.md](./auth/02-token-exchange.md) | Token Exchange | 5 |
| [auth/03-password.md](./auth/03-password.md) | 密码管理（优先通过 Auth9 代理页验证） | 5 |
| [auth/04-social.md](./auth/04-social.md) | 社交登录、OIDC 端点（通过 Auth9 登录入口触发） | 5 |
| [auth/05-boundary.md](./auth/05-boundary.md) | 边界测试 | 3 |
| [auth/06-client-credentials.md](./auth/06-client-credentials.md) | Client Credentials、服务对服务授权 | 5 |
| [auth/07-public-endpoints.md](./auth/07-public-endpoints.md) | Public 端点访问控制与最小暴露 | 5 |
| [auth/08-demo-auth-flow.md](./auth/08-demo-auth-flow.md) | Auth9 Demo 完整认证流程回归（等价 **Sign in with password** 路径） | 5 |
| [auth/09-enterprise-sso-discovery.md](./auth/09-enterprise-sso-discovery.md) | 企业 SSO 域名发现与登录路由（API 主路径） | 5 |
| [auth/10-b2b-onboarding-flow.md](./auth/10-b2b-onboarding-flow.md) | B2B 首次入驻流程（三种登录方式均可触发） | 5 |
| [auth/11-tenant-selection-token-exchange.md](./auth/11-tenant-selection-token-exchange.md) | 登录后 tenant 选择、tenant token exchange、identity token 权限收敛、gRPC tenant token 使用 | 5 |
| [auth/12-enterprise-sso-ui-regression.md](./auth/12-enterprise-sso-ui-regression.md) | 企业 SSO UI 入口可见性与异常回归（Portal `/login`） | 2 |
| [auth/14-landing-public-pages.md](./auth/14-landing-public-pages.md) | Landing 公共页面（Privacy / Terms / Docs）入口、内容、三语翻译 | 5 |
| [auth/15-dark-mode-auth-contrast.md](./auth/15-dark-mode-auth-contrast.md) | 独立认证页与 Auth9 品牌认证页的 Dark Mode 对比度回归 | 5 |
| [auth/16-pkce-flow.md](./auth/16-pkce-flow.md) | PKCE (RFC 7636) 参数透传、Cookie 存储、Public Client 强制验证 | 5 |
| [auth/19-hosted-login-routes-and-branding.md](./auth/19-hosted-login-routes-and-branding.md) | Hosted Login 路由托管、Portal branding、`/mfa/verify` 占位 | 5 |
| [auth/17-email-otp-login.md](./auth/17-email-otp-login.md) | Email OTP 无密码登录（发送/验证端点、Portal UI 入口、租户级开关、防枚举） | 5 | 🆕
| [auth/18-oidc-login-mfa-advanced.md](./auth/18-oidc-login-mfa-advanced.md) | OIDC 登录进阶页回归（TOTP 注册、认证器选择、登出） | 3 |
| [auth/20-hosted-login-api.md](./auth/20-hosted-login-api.md) | Hosted Login API（密码登录、登出、密码重置、Backend Flag 切换） | 5 |
| [auth/21-hosted-login-rollout.md](./auth/21-hosted-login-rollout.md) | Hosted Login 灰度上线与回滚（LOGIN_MODE 开关、百分比分流、指标观测、回滚验证） | 5 |
| [auth/22-email-verification.md](./auth/22-email-verification.md) | 邮箱验证流程（发送验证邮件、Token 消费、Replay 防护、防枚举） | 5 |
| [auth/23-required-actions.md](./auth/23-required-actions.md) | Required Actions 与登录后跳转（Pending Actions API、Force Password、Complete Profile） | 5 |
| [auth/24-mfa-totp-recovery.md](./auth/24-mfa-totp-recovery.md) | MFA 本地化（TOTP 注册/验证/重放防护、Recovery Code 生成/消费、MFA 登录挑战、Session 过期） | 5 |
| [auth/25-auth9-oidc-local-token-issuance.md](./auth/25-auth9-oidc-local-token-issuance.md) | Auth9 本地 OIDC Token 签发（授权码流程、Code Replay、PKCE 验证、Refresh 轮转、ID Token Claims） | 5 |
| [auth/26-enterprise-oidc-broker.md](./auth/26-enterprise-oidc-broker.md) | 企业 OIDC Broker 原生登录（OIDC 连接器 CRUD、userInfoUrl 必填、Auth9 broker 路由、claim mapping） | 5 |
| [auth/27-enterprise-saml-broker.md](./auth/27-enterprise-saml-broker.md) | 企业 SAML Broker 原生登录（SAML 连接器 CRUD、SP Metadata 生成、证书校验、Auth9 broker 路由） | 5 |
| [auth/28-federated-identity-linking.md](./auth/28-federated-identity-linking.md) | Federated Identity Linking（社交登录身份关联、Unlink/Re-link、first_login_policy 策略控制、confirm-link 过期） | 5 |
| [auth/29-portal-auth-ui-completion.md](./auth/29-portal-auth-ui-completion.md) | Portal 认证 UI 补全（密码登录表单、MFA 验证页面、TOTP 注册页面、OTP 组件） | 5 |

### 系统设置 (4 个文档, 20 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [settings/01-branding.md](./settings/01-branding.md) | 登录页品牌设置 | 5 |
| [settings/02-email-provider.md](./settings/02-email-provider.md) | 邮件服务商配置 | 5 |
| [settings/03-email-templates.md](./settings/03-email-templates.md) | 邮件模板管理 | 5 |
| [settings/04-security-malicious-ip-blacklist.md](./settings/04-security-malicious-ip-blacklist.md) | 平台级恶意 IP 黑名单配置、校验与 suspicious_ip 告警联动 | 5 |

### 身份提供商 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [identity-provider/01-crud.md](./identity-provider/01-crud.md) | 创建、更新、删除身份提供商 | 5 |
| [identity-provider/02-toggle-validation.md](./identity-provider/02-toggle-validation.md) | 启用/禁用、验证、登录集成 | 5 |
| [identity-provider/03-tenant-enterprise-sso-connectors.md](./identity-provider/03-tenant-enterprise-sso-connectors.md) | 租户级企业 SSO 连接器管理（SAML/OIDC） | 5 |

### Passkeys (3 个文档, 15 个场景) 🆕
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [passkeys/01-passkeys.md](./passkeys/01-passkeys.md) | 原生 WebAuthn 注册、列表、删除 | 5 |
| [passkeys/02-passkey-auth.md](./passkeys/02-passkey-auth.md) | Passkey 登录认证流程 | 5 |
| [passkeys/03-passkey-api.md](./passkeys/03-passkey-api.md) | WebAuthn API 端点测试 | 5 |

### 分析与统计 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [analytics/01-overview.md](./analytics/01-overview.md) | 统计概览、时间范围筛选 | 5 |
| [analytics/02-events.md](./analytics/02-events.md) | 登录事件列表、分页 | 5 |
| [analytics/03-federation-events.md](./analytics/03-federation-events.md) | 联邦审计与安全事件 (FR5) | 5 |

### 审计日志 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [audit/01-audit-logs.md](./audit/01-audit-logs.md) | 审计日志查看、验证 | 5 |

### Action (12 个文档, 49 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [action/01-crud.md](./action/01-crud.md) | CRUD 基础（入口、创建、列表） | 4 |
| [action/02-execution.md](./action/02-execution.md) | 执行基础（触发器、条件、失败、顺序） | 4 |
| [action/03-logs.md](./action/03-logs.md) | 日志查询基础（列表/筛选/范围/用户/全局） | 5 |
| [action/04-security.md](./action/04-security.md) | 安全基础（沙箱与无限循环） | 4 |
| [action/05-api-sdk.md](./action/05-api-sdk.md) | API/SDK 基础（CRUD、筛选、批量、测试） | 5 |
| [action/06-async-fetch.md](./action/06-async-fetch.md) | Async/Await、fetch()、setTimeout、安全限制 | 5 |
| [action/07-crud-advanced.md](./action/07-crud-advanced.md) | CRUD 进阶（详情、更新、启停、删除） | 4 |
| [action/08-execution-advanced.md](./action/08-execution-advanced.md) | 执行进阶（超时、禁用、上下文、Service 隔离） | 4 |
| [action/09-logs-detail.md](./action/09-logs-detail.md) | 日志详情查看 | 1 |
| [action/10-security-boundary.md](./action/10-security-boundary.md) | 安全边界（内存、隔离、注入） | 4 |
| [action/11-security-attack-defense.md](./action/11-security-attack-defense.md) | 攻击防护（命令注入、提权、伪造、脚本注入） | 4 |
| [action/12-api-sdk-advanced.md](./action/12-api-sdk-advanced.md) | API/SDK 进阶（日志、统计、错误、并发、AI Agent） | 5 |

### SDK (@auth9/core + @auth9/node) (10 个文档, 50 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [sdk/01-core-types-utils.md](./sdk/01-core-types-utils.md) | 类型导出、snake/camel 转换、错误体系、Claims 辨别 | 5 |
| [sdk/02-http-client.md](./sdk/02-http-client.md) | HTTP 客户端、自动转换、错误映射、Token Provider | 5 |
| [sdk/03-token-verification.md](./sdk/03-token-verification.md) | JWKS Token 验证、三种 Token 类型、Audience 验证 | 5 |
| [sdk/04-grpc-client-credentials.md](./sdk/04-grpc-client-credentials.md) | gRPC 4 方法、Client Credentials、Token 缓存 | 5 |
| [sdk/05-express-middleware.md](./sdk/05-express-middleware.md) | Express 中间件、权限控制、角色控制、AuthInfo | 5 |
| [sdk/06-middleware-testing.md](./sdk/06-middleware-testing.md) | Next.js/Fastify 中间件、Mock Token、构建输出 | 5 |
| [sdk/07-core-management-clients.md](./sdk/07-core-management-clients.md) | 核心管理 API 子客户端（Tenants/Users/Services/Roles/Invitations） | 5 |
| [sdk/08-security-enterprise-clients.md](./sdk/08-security-enterprise-clients.md) | 安全与企业功能子客户端（IdP/SSO/SAML/ABAC/Sessions/Webhooks/SCIM/TenantServices） | 5 |
| [sdk/09-auth-password-passkey-clients.md](./sdk/09-auth-password-passkey-clients.md) | 认证流程与凭证管理子客户端（Password/Passkeys/EmailOtp/Auth/Organizations） | 5 |
| [sdk/10-observability-config-clients.md](./sdk/10-observability-config-clients.md) | 可观测性与系统配置子客户端（AuditLogs/Analytics/SecurityAlerts/System/EmailTemplates/Branding） | 5 |

### 集成测试 (19 个文档, 87 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [integration/01-concurrent-operations.md](./integration/01-concurrent-operations.md) | 并发操作、竞态条件 | 4 |
| [integration/02-password-policy.md](./integration/02-password-policy.md) | 密码策略强制执行 | 5 |
| [integration/03-rate-limiting.md](./integration/03-rate-limiting.md) | 限流策略与异常窗口验证 | 5 |
| [integration/04-health-check.md](./integration/04-health-check.md) | 健康检查端点与依赖状态 | 5 |
| [integration/06-init-seed-data.md](./integration/06-init-seed-data.md) | Init 初始种子数据、幂等性、底层认证同步恢复 | 5 |
| [integration/07-observability-metrics.md](./integration/07-observability-metrics.md) | Prometheus /metrics 端点、HTTP 指标、X-Request-ID、路径折叠 | 5 |
| [integration/08-observability-stack.md](./integration/08-observability-stack.md) | 可观测性栈启动、Grafana 仪表盘、业务指标、限流指标 | 5 |
| [integration/09-security-hardening-config.md](./integration/09-security-hardening-config.md) | 生产环境安全启动校验、REST aud 严格校验、HSTS 条件下发、gRPC audience 必填 | 5 |
| [integration/10-security-hardening-p2.md](./integration/10-security-hardening-p2.md) | 事务性级联删除原子性、事件源安全校验、外部系统同步 | 5 |
| [integration/12-otp-service-layer.md](./integration/12-otp-service-layer.md) | OTP 通用服务层基础设施（OtpManager、OtpChannel、速率限制、CacheOperations 扩展） | 5 |
| [integration/13-identity-engine-state-injection.md](./integration/13-identity-engine-state-injection.md) | IdentityEngine 抽象注入、Session/Identity Provider/Realm Sync 回归 | 3 |
| [integration/15-neutral-identity-schema-migration.md](./integration/15-neutral-identity-schema-migration.md) | 中性身份字段迁移回归（identity_subject / provider_session_id / provider_alias） | 4 |
| [integration/16-auth9-oidc-skeleton-and-backend-flag.md](./integration/16-auth9-oidc-skeleton-and-backend-flag.md) | `auth9-oidc` 服务骨架、`IDENTITY_BACKEND` 开关与双 backend smoke test | 4 |
| [integration/17-identity-engine-capabilities-state-cleanup.md](./integration/17-identity-engine-capabilities-state-cleanup.md) | Identity Engine 最小能力面补齐、`state` 去 `keycloak_client()` 出口、adapter contract 回归 | 3 |
| [integration/18-business-layer-keycloak-decoupling.md](./integration/18-business-layer-keycloak-decoupling.md) | 业务层去 `KeycloakClient` 直接依赖、handler 中性 DTO、Password/WebAuthn/SCIM/SAML 抽象回归 | 5 |
| [integration/19-phase1-identity-abstraction-closure.md](./integration/19-phase1-identity-abstraction-closure.md) | Phase 1 身份抽象层 closure 验收（默认 `keycloak` backend、`auth9_oidc` stub、adapter contract、中性字段主路径） | 4 |
| [integration/20-local-credential-store.md](./integration/20-local-credential-store.md) | Phase 3 FR1 本地 Credential Store（中性模型、migration、repository 契约） | 5 |
| [integration/21-email-verification-required-actions.md](./integration/21-email-verification-required-actions.md) | Phase 3 FR3 邮箱验证与 Required Actions（schema 完整性、migration 幂等性、adapter 契约、Identity Token 白名单、Backend fallback） | 5 |
| [integration/22-config-keycloak-retirement.md](./integration/22-config-keycloak-retirement.md) | Phase 5 FR4 Config 重构 — KeycloakConfig 退役、字段提升、SAML 方法补全 | 5 |

### SAML Application (4 个文档, 20 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [saml-application/01-crud.md](./saml-application/01-crud.md) | SAML Application CRUD（创建、列表、获取、更新、删除） | 5 |
| [saml-application/02-metadata-validation.md](./saml-application/02-metadata-validation.md) | IdP Metadata XML 获取、输入校验、属性映射、跨租户隔离 | 5 |
| [saml-application/03-portal-ui.md](./saml-application/03-portal-ui.md) | Portal UI 入口可见性、创建表单、列表、启停、删除、Metadata URL 复制 | 5 |
| [saml-application/04-certificate-encryption.md](./saml-application/04-certificate-encryption.md) | IdP 签名证书下载、证书过期信息、加密校验、SLO POST Binding | 5 |

### SCIM Provisioning (5 个文档, 25 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [provisioning/01-scim-token-management.md](./provisioning/01-scim-token-management.md) | SCIM Bearer Token 创建、列表、吊销（管理 API） | 5 |
| [provisioning/02-scim-user-crud.md](./provisioning/02-scim-user-crud.md) | SCIM 用户创建、查询、列表、替换、增量更新、停用 | 5 |
| [provisioning/03-scim-group-crud.md](./provisioning/03-scim-group-crud.md) | SCIM 组 CRUD、Group-Role 映射管理 | 5 |
| [provisioning/04-scim-bulk-discovery.md](./provisioning/04-scim-bulk-discovery.md) | Bulk 批量操作、ServiceProviderConfig/Schemas/ResourceTypes 发现 | 5 |
| [provisioning/05-scim-auth-logs.md](./provisioning/05-scim-auth-logs.md) | SCIM 鉴权安全（无效/过期/吊销 Token）、审计日志查询 | 5 |

### Identity Engine (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [identity_engine/decouple_keycloak_types.md](./identity_engine/decouple_keycloak_types.md) | 解耦 Keycloak 类型重构验证（编译、Clippy、trait 中性化、SmtpServerConfig 迁移） | 5 |

---

## 统计概览

| 模块 | 文档数 | 场景数 |
|------|--------|--------|
| 租户管理 | 5 | 25 |
| 用户管理 | 6 | 28 |
| RBAC 角色权限 | 5 | 22 |
| 服务与客户端 | 6 | 30 |
| 邀请管理 | 3 | 15 |
| 会话与安全 | 8 | 39 |
| Webhook | 4 | 17 |
| 认证流程 | 23 | 107 |
| 系统设置 | 4 | 20 |
| 身份提供商 | 3 | 15 |
| Passkeys | 3 | 15 |
| 分析与统计 | 3 | 15 |
| 审计日志 | 1 | 5 |
| Action | 12 | 49 |
| SDK | 10 | 50 |
| 集成测试 | 19 | 87 |
| SAML Application | 4 | 20 |
| SCIM Provisioning | 5 | 25 |
| Identity Engine | 1 | 5 |
| **总计** | **119** | **564** |

### 文档对齐记录

| 日期 | 版本 | 变更 |
|------|------|------|
| 2026-03-19 | 5.26.0 | **Phase 5 FR5 基础设施清理**：新增 `integration/qa-infrastructure-keycloak-cleanup.md`（5 场景），覆盖 Docker Compose 无 Keycloak 依赖、Portal 登录模式简化、API 健康检查、K8s 配置清理、脚本清理验证；集成测试 23 文档 106 场景；共 125 文档 588 场景 |
| 2026-03-19 | 5.25.0 | **Phase 5 FR2 解耦 Keycloak 类型**：新增 `identity_engine/decouple_keycloak_types.md`（5 场景），覆盖编译通过、Clippy 无新增警告、IdentityEngine trait 无 Keycloak 类型引用、中性类型定义验证、SmtpServerConfig 迁移验证；纯重构验证，无 API/UI 变更；共 124 文档 583 场景 |
| 2026-03-18 | 5.24.0 | **Phase 4 FR4 Federated Identity Linking**：新增 `auth/28-federated-identity-linking.md`（5 场景），覆盖社交登录 linked identity 写入、Unlink/Re-link 完整流程、`prompt_confirm` 策略阻止静默 takeover、`create_new` 策略创建独立账号、confirm-link token 过期错误；认证 24 文档 112 场景；总计 123 文档 578 场景 |
| 2026-03-18 | 5.23.0 | **Phase 4 FR2 Enterprise OIDC Connector**：新增 `auth/26-enterprise-oidc-broker.md`（5 场景），覆盖 OIDC 连接器 CRUD 不触发 Keycloak IDP、userInfoUrl 必填验证、Auth9 原生 broker 路由、endpoint 可达性测试、Discovery 返回 Auth9 broker URL；更新 `auth/09-enterprise-sso-discovery.md` 的场景 1/5 预期（OIDC 使用 Auth9 broker 而非 Keycloak kc_idp_hint）；认证 23 文档 107 场景；总计 123 文档 578 场景 |
| 2026-03-18 | 5.22.0 | **Phase 3 总控 FR 文档治理与闭环**：跨文档修复 `security/authentication/03-mfa-security.md` 背景说明（Keycloak→Auth9 本地 MFA）及 Recovery Code 检查清单；修复 `security/authentication/04-password-security.md` 背景说明（Keycloak→Auth9 本地密码管理）；修复 `integration/20-local-credential-store.md` migration 文件数（3→4）和测试数（32→39）；修复 `auth/20-hosted-login-api.md` 场景 5 auth9_oidc 预期（501→200 正常响应）；总计 122 文档 573 场景 |
| 2026-03-18 | 5.21.0 | **Phase 3 FR5 Token Issuance 与 FR4 MFA 本地化**：新增 `auth/24-mfa-totp-recovery.md`（5 场景）、`auth/25-auth9-oidc-local-token-issuance.md`（5 场景），覆盖 TOTP 注册/验证/重放防护、Recovery Code 生成/消费、MFA 登录挑战、授权码完整流程、Code Replay、PKCE 验证、Refresh 轮转、ID Token Claims；认证 22 文档 102 场景；总计 122 文档 573 场景 |
| 2026-03-18 | 5.20.0 | **Phase 3 FR3 邮箱验证与 Required Actions 本地化**：新增 `auth/22-email-verification.md`（5 场景）、`auth/23-required-actions.md`（5 场景）、`integration/21-email-verification-required-actions.md`（5 场景）；跨文档更新 `auth/20-hosted-login-api.md` 端点列表；修复 Identity Token 白名单缺失 pending-actions/complete-action 端点 bug；修复 auth9-oidc 启动时自动建表（Dockerfile + db.rs）；认证 22 文档 102 场景、集成 20 文档 91 场景；总计 122 文档 573 场景 |
| 2026-03-17 | 5.19.0 | **Keycloak Phase 1 FR3 Closure**：新增 `integration/19-phase1-identity-abstraction-closure.md`（4 场景），收束默认 `keycloak` backend、`auth9_oidc` stub、adapter contract 与中性字段主路径总验收；同步更新 `session/03-alerts.md`、`user/02-advanced.md`、`user/04-account-profile.md`、`identity-provider/02-toggle-validation.md` 的主断言字段说明；集成测试统计更新为 19 个文档 86 个场景；总计 118 个文档 553 个场景 |
| 2026-03-17 | 5.18.0 | **Keycloak Phase 1 FR2**：新增 `integration/18-business-layer-keycloak-decoupling.md`（5 场景），覆盖业务层去 `KeycloakClient` 直接依赖、handler 中性 DTO、Password/WebAuthn/SCIM/SAML 抽象回归；集成测试统计更新为 18 个文档 82 个场景；总计 111 个文档 517 个场景 |
| 2026-03-17 | 5.17.0 | **Keycloak Phase 1 FR4**：新增 `integration/16-auth9-oidc-skeleton-and-backend-flag.md`（4 场景），覆盖 `auth9-oidc` 独立服务骨架、`IDENTITY_BACKEND` 默认/切换/非法值校验；同步更新 `integration/13`、`integration/14` 的分支覆盖边界说明；集成测试统计更新为 16 个文档 74 个场景；总计 109 个文档 509 个场景 |
| 2026-03-17 | 5.16.0 | **Keycloak Phase 1 FR3**：新增 `integration/15-neutral-identity-schema-migration.md`（4 场景），覆盖 `identity_subject` / `provider_session_id` / `provider_alias` 的 migration、主路径读写与兼容性；同步更新用户、会话、Enterprise SSO 相关 QA 文档字段说明；集成测试统计更新为 15 个文档 70 个场景；总计 108 个文档 505 个场景 |
| 2026-03-17 | 5.15.0 | **QA 治理收敛**：拆分 `auth/01-oidc-login.md`，新增 `auth/18-oidc-login-mfa-advanced.md`（3 场景），移出进阶 MFA/TOTP/认证器选择场景以满足单文档场景数限制；为 `sdk/07`、`sdk/08` 补检查清单，并为多份 UI 文档补 `入口可见性` 说明；文档总数更新为 107，场景总数保持 501 |
| 2026-03-17 | 5.14.0 | **Keycloak Phase 1 FR2**：新增 `integration/14-keycloak-adapter-layer.md`（4 场景），覆盖 Keycloak adapter 注入链、Session revoke、Identity Provider CRUD、linked identity 回归；同步更新 `integration/13-identity-engine-state-injection.md` 背景说明；集成测试统计更新为 14 个文档 66 个场景；总计 106 个文档 501 个场景 |
| 2026-03-17 | 5.13.0 | **Keycloak Phase 1 FR1**：新增 `integration/13-identity-engine-state-injection.md`（3 场景），覆盖 `IdentityEngine` 抽象注入、Session API 回归、Identity Provider API 回归；集成测试统计更新为 13 个文档 62 个场景；总计 105 个文档 497 个场景 |
| 2026-03-17 | 5.12.0 | **SDK Phase 3 认证流程与凭证管理**：新增 `sdk/09-auth-password-passkey-clients.md`（5 场景），覆盖 Password/Passkeys/EmailOtp/Auth/Organizations 5 个子客户端 21 个方法；同步修正 SDK 统计（8→9 文档、40→45 场景）；共 104 个文档 494 个场景 |
| 2026-03-14 | 4.3.0 | 新增平台级恶意 IP 黑名单 QA 文档（`settings/04`），并同步修正 `session/04` 对可疑 IP 告警来源的说明，覆盖黑名单配置、输入校验、`suspicious_ip`/`critical` 告警联动 |
| 2026-03-16 | 5.11.0 | **SAML IdP 出站 Phase 3（证书/加密/SLO）**：新增 `saml-application/04-certificate-encryption.md`（5 场景），覆盖 IdP 签名证书下载公开端点、证书过期信息受保护端点、Assertion 加密缺少 SP 证书校验、SLO POST Binding 验证、Portal 证书下载链接与过期告警 badge；跨文档更新 `01-crud.md` 端点列表、`02-metadata-validation.md` 公开端点列表、`03-portal-ui.md` 列表展示项；共 103 个文档 489 个场景 |
| 2026-03-16 | 5.10.0 | **SAML IdP 出站 Phase 2 Portal UI**：新增 `saml-application/03-portal-ui.md`（5 场景），覆盖 Tenant 详情页入口可见性、创建表单完整提交、列表展示与 Metadata URL 复制、启停切换、删除；更新 `01-crud.md` Phase 说明；跨文档更新 `uiux/21-tenant-detail-pages.md` Quick Links 与导航路径补充 SAML Applications；共 102 个文档 484 个场景 |
| 2026-03-16 | 5.9.0 | **SAML IdP 出站 Phase 1**：新增 `saml-application/01-crud.md`（5 场景）、`saml-application/02-metadata-validation.md`（5 场景），覆盖 SAML Application CRUD、IdP Metadata XML 公开端点、输入校验、属性映射验证、跨租户隔离、默认值；跨文档无影响（纯新增 API，无修改现有端点）；共 101 个文档 479 个场景 |
| 2026-03-16 | 5.8.0 | 新增 `auth/17-email-otp-login.md`（5 个场景）；更新 `auth/01-oidc-login.md` 认证方式表增加 Email OTP 行；更新 `settings/01-branding.md` BrandingConfig 字段表增加 `email_otp_enabled` |

---

## 测试分配建议

每位 QA 工程师可以领取 1-2 个文档进行测试。文档之间相对独立，可以并行执行。

**建议的执行顺序**（如有依赖）：
1. 认证流程 (auth/*) - 先确保登录功能正常
2. 用户账户 (user/04~06) - 测试个人资料、Account 页面、导航布局
3. 系统设置 (settings/*) - 配置品牌和邮件
4. 租户管理 (tenant/*) - 创建测试租户
5. 用户管理 (user/01~03) - 创建测试用户
6. 身份提供商 (identity-provider/*) - 配置社交登录
7. Passkeys (passkeys/*) - 测试无密码登录
8. 服务与客户端 (service/*) - 配置测试服务
9. RBAC (rbac/*) - 配置角色和权限
10. 邀请管理 (invitation/*) - 测试邀请流程
11. 会话与安全 (session/*) - 测试安全功能
12. Webhook (webhook/*) - 测试事件通知
13. 分析与统计 (analytics/*) - 验证登录统计
14. 审计日志 (audit/*) - 验证操作记录
15. SCIM Provisioning (provisioning/*) - 需先配置企业 SSO Connector

---

## 测试环境准备

### 启动服务

```bash
# 启动依赖服务
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# 启动后端
cd auth9-core && cargo run

# 启动前端
cd auth9-portal && npm run dev
```

### 数据库连接

```bash
mysql -h 127.0.0.1 -P 4000 -u root -D auth9
```

## 测试用例结构

每个测试场景包含：

1. **初始状态** - 测试前置条件
2. **目的** - 验证的功能点
3. **测试操作流程** - 详细步骤
4. **预期结果** - 界面预期表现
5. **预期数据状态** - 数据库验证 SQL

---

## 常用验证查询

```sql
-- 查看最近审计日志
SELECT action, resource_type, resource_id, created_at
FROM audit_logs ORDER BY created_at DESC LIMIT 10;

-- 查看用户的所有租户
SELECT t.name, tu.role_in_tenant
FROM tenant_users tu JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}';

-- 查看用户的有效权限
SELECT DISTINCT p.code
FROM user_tenant_roles utr
JOIN role_permissions rp ON rp.role_id = utr.role_id
JOIN permissions p ON p.id = rp.permission_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';

-- 查看未解决的安全告警
SELECT alert_type, severity, user_id, created_at
FROM security_alerts WHERE resolved_at IS NULL;

-- 查看登录事件统计
SELECT event_type, COUNT(*) as count
FROM login_events
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
GROUP BY event_type;

-- 查看系统设置
SELECT category, setting_key, JSON_EXTRACT(value, '$.type') as type
FROM system_settings;

-- SCIM: 查看 Token 状态
SELECT id, token_prefix, description, expires_at, last_used_at, revoked_at
FROM scim_tokens WHERE connector_id = '{connector_id}';

-- SCIM: 查看用户 SCIM 追踪字段
SELECT id, email, scim_external_id, scim_provisioned_by
FROM users WHERE scim_external_id IS NOT NULL;

-- SCIM: 查看 Group-Role 映射
SELECT scim_group_id, scim_group_display_name, role_id
FROM scim_group_role_mappings WHERE connector_id = '{connector_id}';

-- SCIM: 查看最近操作日志
SELECT operation, resource_type, status, created_at
FROM scim_provisioning_logs WHERE connector_id = '{connector_id}'
ORDER BY created_at DESC LIMIT 10;
```

---

## 问题报告格式

```markdown
## Bug: [简短描述]

**测试文档**: [文档路径]
**场景**: #X
**复现步骤**:
1. ...
2. ...

**预期结果**: ...
**实际结果**: ...
**数据库状态**: [相关 SQL 查询结果]
```

---

## 测试数据准备

### 自动化种子数据

为了快速搭建测试环境，Auth9 提供了专用的测试数据种子（Seed Data）：

```bash
# 加载基础 QA 测试数据
cd auth9-core
cargo run --bin seed-data -- --dataset=qa-basic --reset

# 或使用 YAML 配置
# 参考 scripts/seed-data/qa-basic.yaml
```

详细的种子数据设计和使用方法，请参考 [测试数据种子设计文档](../testing/seed-data-design.md)。

---

## 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|----------|
| 2026-03-19 | 5.26.0 | **Phase 5 FR5 基础设施清理**：新增 integration/qa-infrastructure-keycloak-cleanup（Docker Compose/K8s/Portal/脚本 Keycloak 残留移除）共 5 场景；集成测试 23 文档 106 场景；共 125 文档 588 场景 |
| 2026-03-19 | 5.25.0 | **Phase 5 FR2 解耦 Keycloak 类型**：新增 identity_engine/decouple_keycloak_types（编译、Clippy、trait 中性化、中性类型定义、SmtpServerConfig 迁移）共 5 场景；纯重构验证；共 124 文档 583 场景 |
| 2026-03-18 | 5.24.0 | **Phase 4 FR4 Federated Identity Linking**：新增 auth/28（社交登录身份关联、Unlink/Re-link、first_login_policy 策略、confirm-link 过期）共 5 场景；共 123 文档 578 场景 |
| 2026-03-18 | 5.22.0 | **Phase 3 总控 FR 文档治理与闭环**：跨文档修复安全文档背景说明、集成测试预期值、Hosted Login API 预期；共 122 文档 573 场景 |
| 2026-03-18 | 5.21.0 | **Phase 3 FR5 Token Issuance 与 FR4 MFA**：新增 auth/24（MFA TOTP/Recovery）、auth/25（本地 OIDC Token 签发）共 10 场景；共 122 文档 573 场景 |
| 2026-03-18 | 5.20.0 | **Phase 3 FR3 邮箱验证与 Required Actions**：新增 3 个文档（auth/22、auth/23、integration/21）共 15 场景；修复 Identity Token 白名单和 auth9-oidc 自动建表；共 122 文档 573 场景 |
| 2026-03-16 | 5.11.0 | **SAML IdP 出站 Phase 3 测试文档**：新增 `saml-application/04-certificate-encryption.md`（5 场景），覆盖证书端点、加密校验、SLO POST Binding；跨文档更新 `01-crud.md`、`02-metadata-validation.md`、`03-portal-ui.md`；共 103 个文档 489 个场景 |
| 2026-03-16 | 5.10.0 | **SAML IdP 出站 Phase 2 Portal UI 测试文档**：新增 `saml-application/03-portal-ui.md`（5 场景），覆盖 Portal 入口可见性、表单创建、列表、启停、删除；更新 `01-crud.md`、`uiux/21-tenant-detail-pages.md`；共 102 个文档 484 个场景 |
| 2026-03-16 | 5.9.0 | **SAML IdP 出站 Phase 1 测试文档**：新增 `saml-application/01-crud.md`（5 场景）、`saml-application/02-metadata-validation.md`（5 场景）；共 101 个文档 479 个场景 |
| 2026-03-16 | 5.7.0 | **OTP 通用服务层基础设施测试**：新增 `integration/12-otp-service-layer.md`（5 场景），覆盖 OtpManager 单元测试验证、模块代码结构完整性、CacheOperations 扩展完整性、速率限制默认配置、编译和 Lint 检查；跨文档无影响（纯后端基础设施层，无 API/UI 变更）；共 99 个文档 469 个场景 |
| 2026-03-15 | 5.6.0 | **PKCE (RFC 7636) 安全增强文档**：新增 `auth/16-pkce-flow.md`（5 场景），覆盖 Portal 密码/SSO 登录 PKCE 参数透传、cookie 存储生命周期、authorize 端点透传验证、向后兼容、public client 强制 PKCE；跨文档影响：更新 `session/07-oauth-state-csrf.md` cookie 格式说明（`state` → `{ state, codeVerifier }`）、`auth/08-demo-auth-flow.md` 补充 PKCE 强制说明、`security/authentication/01-oidc-security.md` 标记 PKCE 已实现；共 98 个文档 464 个场景 |
| 2026-03-14 | 5.5.2 | **Portal Selector 组件替换文档同步**：更新 `rbac/02-role.md` 与 `identity-provider/03-tenant-enterprise-sso-connectors.md`，补充 Roles 父角色选择器与 Tenant SSO `Provider Type` 已切换为项目统一 Select 组件的验证要点；同步更新 UI/UX 文档 `18-roles-abac-pages.md`、`21-tenant-detail-pages.md` 的下拉控件样式与条件字段说明；QA 文档总数与场景数不变（97/459） |
| 2026-03-14 | 5.5.1 | **租户级恶意 IP 黑名单文档同步**：新增 `tenant/05-security-malicious-ip-blacklist.md`，覆盖租户详情页入口可见性、租户级黑名单配置、非法 IP 拒绝、跨租户隔离、平台级优先；同步更新 `session/04-boundary.md` 对可疑 IP 告警来源的说明，并为 `security/authorization/01-tenant-isolation.md` 补充租户级黑名单隔离场景；共 97 个文档 459 个场景 |
| 2026-03-12 | 5.5.0 | **Landing 公共页面 + 语言切换器 DropdownMenu 重构**：新增 `auth/14-landing-public-pages.md`（5 场景），覆盖 `/privacy`、`/terms`、`/docs` 页面入口可见性、内容完整性、三语翻译；跨文档影响：更新 UIUX `12-i18n-localization.md`（语言切换器从 `<select>` 改为 DropdownMenu 描述）、`13-landing-page-interactions.md`（补充 Footer 链接说明）、`14-global-controls-placement.md`（更新 LanguageSwitcher 实现描述）；新增 UIUX `23-public-pages-layout.md`（5 场景）覆盖 PublicPageLayout 布局、prose-glass 排版、Docs 卡片网格；共 96 个文档 454 个场景 |
| 2026-03-07 | 5.4.1 | **i18n 三语扩展 cross-doc 同步**：更新 `auth/01-oidc-login.md`、`passkeys/02-passkey-auth.md`、`auth/12-enterprise-sso-ui-regression.md` 的语言说明，从双语扩展为三语（追加 `ja`） |
| 2026-02-22 | 5.3.0 | **新增 SCIM 2.0 Provisioning 测试文档**：覆盖 SCIM Bearer Token 管理（`provisioning/01`）、用户 CRUD（`provisioning/02`）、组 CRUD 与 Group-Role 映射（`provisioning/03`）、Bulk 批量操作与 Discovery 端点（`provisioning/04`）、鉴权安全与审计日志（`provisioning/05`）；跨文档影响：更新 `webhook/02-trigger.md` 新增 6 个 SCIM 事件类型、`identity-provider/03` 补充 SCIM Token 管理端点引用；共 94 个文档 444 个场景 |
| 2026-02-22 | 5.2.1 | 新增仓库级周期治理入口脚本 `scripts/run-weekly-qa-governance.sh`（扩展审计 + 严格 lint + 日志落盘），并在 README 文档治理章节补充定期执行建议 |
| 2026-02-21 | 5.2.0 | 第二阶段文档治理完成：将超长文档拆分为 `action/07~12` 与 `auth/12`，使既有超限文档全部收敛到每文档 ≤5 场景；`action/01~05`、`auth/09` 改为基础/进阶分层；索引同步为 91 个文档、429 个场景 |
| 2026-02-21 | 5.1.0 | 新增 QA 文档治理基线：增加 `_standards.md`、`_manifest.yaml` 与 `scripts/qa-doc-lint.sh`；补齐 README 漏索引文档（`auth/06`、`auth/07`、`integration/03~05`、`provisioning/01~02`）；统一通用认证场景为“无痕/清 Cookie/Sign out”可执行流程；补充 `action/01`、`integration/01`、`integration/02` 检查清单，并增强 `tenant/01`、`service/01`、`settings/02`、`user/04`、`rbac/02` 的 UI 入口可见性说明；总计 84 个文档 429 个场景 |
| 2026-03-07 | 5.0.1 | Portal 国际化文档对齐：新增 UI/UX 国际化专项文档（见 `docs/uiux/12-i18n-localization.md`），并为 `auth/01`、`auth/12`、`passkeys/02` 增加默认语言与切换说明，避免测试步骤继续假设登录页默认英文；QA 文档总数与场景数不变（95/449） |
| 2026-02-21 | 5.0.0 | **Action 迁移到 Service 级别 + Service Branding**：Action 从 Tenant 级别迁移到 Service 级别（API 路径 `/tenants/{id}/actions` → `/services/{id}/actions`，DB 字段 `tenant_id` → `service_id`，Portal 入口从 Tenant 详情页 Quick Links 迁移到 Service 详情页 Actions Tab）；新增 Service 级品牌配置（`service/06-service-branding.md`）覆盖 API CRUD、公开端点 client_id 查询、Keycloak 主题集成；更新 `action/01-06` 全部 6 个文档、`service/01`（级联删除）、`settings/01`（两级品牌架构说明）；共 76 个文档 374 个场景 |
| 2026-02-18 | 4.4.2 | 补充多 tenant 登录后 `/tenant/select` 分流说明，统一 6 份既有文档执行步骤（`auth/01`、`session/07`、`integration/06`、`passkeys/02`、`user/06`、`service/05`），避免 QA 对登录后页面路径理解不一致；文档总数与场景数不变（74/364） |
| 2026-02-18 | 4.4.1 | 新增会话与安全文档 `session/08`：覆盖 Identity Token 最小白名单、tenant 接口强制 Tenant Token、tenant/service 不匹配拒绝、切租户后旧 token 隔离；共 74 个文档 364 个场景 |
| 2026-02-18 | 4.4.0 | 新增 tenant 选择与 token exchange 测试文档（`auth/11`），并更新 B2B 入驻路由说明（`auth/10`）：覆盖登录后 `/tenant/select` 分流、切换 tenant 强制 exchange、identity token 最小白名单、gRPC 使用 tenant token；共 73 个文档 359 个场景 |
| 2026-02-18 | 4.3.0 | 新增 B2B 入驻流程与 OAuth State CSRF 修复测试：OAuth State CSRF 校验（`session/07`）、B2B 组织自助创建 API（`tenant/04`）、B2B 首次入驻与租户路由（`auth/10`），覆盖 state cookie 生命周期、域名验证、Pending 状态、Onboarding 向导、组织切换器；共 72 个文档 354 个场景 |
| 2026-02-18 | 4.2.0 | 新增安全加固第二轮测试：Token 黑名单 Fail-Closed 策略（`session/06`）、事务性级联删除原子性 & Webhook Secret 生产强制校验（`integration/10`），覆盖 P0-1/P0-2/P0-3 安全改进；共 68 个文档 334 个场景 |
| 2026-02-17 | 4.1.1 | 对齐企业 SSO 测试执行路径：`auth/09-enterprise-sso-discovery.md`、`identity-provider/03-tenant-enterprise-sso-connectors.md` 新增 `auth9-demo`（`/enterprise/login` 与 `/demo/enterprise/*`）操作步骤；文档总数与场景数不变（66/325） |
| 2026-02-17 | 4.1.0 | 新增企业 SSO 测试文档：`auth/09-enterprise-sso-discovery.md` 与 `identity-provider/03-tenant-enterprise-sso-connectors.md`，覆盖域名发现、`kc_idp_hint` 路由、租户级连接器 CRUD 与冲突校验；共 66 个文档 325 个场景 |
| 2026-02-14 | 4.0.0 | 新增 Service Integration Info（API 端点 + Portal Integration 标签页），共 64 个文档 315 个场景 |
| 2026-02-14 | 3.9.0 | 新增 Auth9 Demo 完整认证流程回归测试（OAuth 登录、public client token exchange、gRPC tenant slug 支持、登出），共 62 个文档 305 个场景 |
| 2026-02-13 | 3.8.0 | 新增 Action 模块（CRUD、执行、日志、安全沙箱、API/SDK 集成、Async/Await fetch 支持），共 61 个文档 300 个场景 |
| 2026-02-11 | 3.7.0 | 新增会话与安全回归测试（管理员端点越权、refresh 撤销一致性、OIDC callback token 泄露、限流 header 绕过与高基数 key），共 55 个文档 265 个场景 |
| 2026-02-11 | 3.6.0 | 新增安全加固集成测试（production 启动 fail-fast、gRPC 鉴权配置校验、REST tenant token aud 严格校验、HSTS 条件下发、gRPC validate_token audience 必填），共 54 个文档 260 个场景 |
| 2026-02-11 | 3.5.0 | 新增全栈可观测性集成测试（Prometheus /metrics 端点、HTTP 指标、X-Request-ID、路径折叠、可观测性栈启动、Grafana 仪表盘、业务指标、限流指标），共 53 个文档 255 个场景 |
| 2026-02-10 | 3.4.0 | 新增 Init 种子数据集成测试（首次创建、幂等性、自定义邮箱、Keycloak 重置恢复、Portal 登录验证），共 45 个文档 215 个场景 |
| 2026-02-08 | 3.3.0 | Passkeys 模块重写：原生 WebAuthn 注册、Passkey 登录认证、API 端点测试，共 44 个文档 210 个场景 |
| 2026-02-08 | 3.2.0 | 新增用户账户模块（个人资料 API、Account 页面、导航布局），共 42 个文档 200 个场景 |
| 2026-02-05 | 3.1.0 | 新增集成测试模块（并发操作、密码策略），共 39 个文档 185 个场景；新增测试数据种子基础设施 |
| 2026-02-02 | 3.0.0 | 新增系统设置、身份提供商、Passkeys、分析统计、审计日志模块，共 37 个文档 175 个场景 |
| 2024-02-02 | 2.0.0 | 细分文档，每个不超过 5 个场景，共 28 个文档 |
| 2024-02-02 | 1.0.0 | 初始版本 |
