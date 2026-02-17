# Auth9 QA 测试用例文档

本目录包含 Auth9 系统的手动测试用例，每个文档不超过 5 个场景，便于多名 QA 工程师并行测试。

## 测试用例索引

### 租户管理 (4 个文档, 20 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [tenant/01-crud.md](./tenant/01-crud.md) | 创建、更新、删除操作 | 5 |
| [tenant/02-list-settings.md](./tenant/02-list-settings.md) | 列表、搜索、设置 | 5 |
| [tenant/03-status-lifecycle.md](./tenant/03-status-lifecycle.md) | 租户状态生命周期（Active/Inactive/Suspended）及业务影响 | 5 |
| [tenant/04-b2b-org-creation.md](./tenant/04-b2b-org-creation.md) | B2B 组织自助创建、域名验证、Pending 状态、/users/me/tenants | 5 |

### 用户管理 (6 个文档, 28 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [user/01-crud.md](./user/01-crud.md) | 创建、更新、租户关联 | 5 |
| [user/02-advanced.md](./user/02-advanced.md) | 删除、MFA、列表 | 5 |
| [user/03-validation.md](./user/03-validation.md) | 边界测试、验证 | 3 |
| [user/04-account-profile.md](./user/04-account-profile.md) | 个人资料 API、Profile 页面、自更新权限 | 5 |
| [user/05-account-security.md](./user/05-account-security.md) | 修改密码、Passkeys、会话、关联身份 | 5 |
| [user/06-account-navigation.md](./user/06-account-navigation.md) | Account 导航布局、侧边栏、Settings 清理 | 5 |

### RBAC 角色权限 (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [rbac/01-permission.md](./rbac/01-permission.md) | 权限 CRUD | 4 |
| [rbac/02-role.md](./rbac/02-role.md) | 角色 CRUD、继承 | 5 |
| [rbac/03-assignment.md](./rbac/03-assignment.md) | 权限分配、用户角色 | 5 |
| [rbac/04-advanced.md](./rbac/04-advanced.md) | 层次视图、循环检测 | 3 |

### 服务与客户端 (5 个文档, 25 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [service/01-service-crud.md](./service/01-service-crud.md) | 服务 CRUD | 5 |
| [service/02-client.md](./service/02-client.md) | 客户端管理、密钥 | 5 |
| [service/03-oidc.md](./service/03-oidc.md) | OIDC 配置、URI 验证 | 5 |
| [service/04-tenant-service-toggle.md](./service/04-tenant-service-toggle.md) | 租户服务启停 | 5 |
| [service/05-integration-info.md](./service/05-integration-info.md) | 集成信息 API 与 Portal 页面 | 5 |

### 邀请管理 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [invitation/01-create-send.md](./invitation/01-create-send.md) | 创建、发送邀请 | 5 |
| [invitation/02-accept.md](./invitation/02-accept.md) | 接受邀请流程 | 5 |
| [invitation/03-manage.md](./invitation/03-manage.md) | 撤销、删除、过滤 | 5 |

### 会话与安全 (7 个文档, 34 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [session/01-session.md](./session/01-session.md) | 会话管理、撤销 | 5 |
| [session/02-login-events.md](./session/02-login-events.md) | 登录事件记录 | 5 |
| [session/03-alerts.md](./session/03-alerts.md) | 安全告警检测 | 5 |
| [session/04-boundary.md](./session/04-boundary.md) | 边界测试 | 5 |
| [session/05-auth-security-regression.md](./session/05-auth-security-regression.md) | 鉴权与令牌安全回归（越权强退、refresh 撤销一致性、callback token 泄露、限流绕过） | 5 |
| [session/06-token-blacklist-failsafe.md](./session/06-token-blacklist-failsafe.md) | Token 黑名单 Fail-Closed 策略（Redis 故障 503、重试机制、向后兼容） | 4 |
| [session/07-oauth-state-csrf.md](./session/07-oauth-state-csrf.md) | OAuth State CSRF 校验（cookie 存储、回调校验、过期、安全属性） | 5 |

### Webhook (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [webhook/01-crud.md](./webhook/01-crud.md) | Webhook CRUD | 5 |
| [webhook/02-trigger.md](./webhook/02-trigger.md) | 事件触发、签名 | 5 |
| [webhook/03-reliability.md](./webhook/03-reliability.md) | 重试、自动禁用 | 4 |
| [webhook/04-boundary.md](./webhook/04-boundary.md) | URL 验证、边界 | 3 |

### 认证流程 (8 个文档, 38 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [auth/01-oidc-login.md](./auth/01-oidc-login.md) | OIDC 登录流程 | 5 |
| [auth/02-token-exchange.md](./auth/02-token-exchange.md) | Token Exchange | 5 |
| [auth/03-password.md](./auth/03-password.md) | 密码管理 | 5 |
| [auth/04-social.md](./auth/04-social.md) | 社交登录、OIDC 端点 | 5 |
| [auth/05-boundary.md](./auth/05-boundary.md) | 边界测试 | 3 |
| [auth/08-demo-auth-flow.md](./auth/08-demo-auth-flow.md) | Auth9 Demo 完整认证流程回归（OAuth 登录、Token Exchange、登出） | 5 |
| [auth/09-enterprise-sso-discovery.md](./auth/09-enterprise-sso-discovery.md) | 企业 SSO 域名发现与 `kc_idp_hint` 登录路由 | 5 |
| [auth/10-b2b-onboarding-flow.md](./auth/10-b2b-onboarding-flow.md) | B2B 首次入驻流程（Onboarding 向导、Dashboard 租户路由、组织切换器） | 5 |

### 系统设置 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [settings/01-branding.md](./settings/01-branding.md) | 登录页品牌设置 | 5 |
| [settings/02-email-provider.md](./settings/02-email-provider.md) | 邮件服务商配置 | 5 |
| [settings/03-email-templates.md](./settings/03-email-templates.md) | 邮件模板管理 | 5 |

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

### 分析与统计 (2 个文档, 10 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [analytics/01-overview.md](./analytics/01-overview.md) | 统计概览、时间范围筛选 | 5 |
| [analytics/02-events.md](./analytics/02-events.md) | 登录事件列表、分页 | 5 |

### 审计日志 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [audit/01-audit-logs.md](./audit/01-audit-logs.md) | 审计日志查看、验证 | 5 |

### Action (6 个文档, 35 个场景) 🆕
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [action/01-crud.md](./action/01-crud.md) | 创建、更新、删除、启用/禁用 | 5 |
| [action/02-execution.md](./action/02-execution.md) | 脚本执行、V8 沙箱、上下文注入 | 5 |
| [action/03-logs.md](./action/03-logs.md) | 执行日志、统计、查询 | 5 |
| [action/04-security.md](./action/04-security.md) | 安全沙箱、超时、资源限制 | 5 |
| [action/05-api-sdk.md](./action/05-api-sdk.md) | REST API、TypeScript SDK 集成 | 10 |
| [action/06-async-fetch.md](./action/06-async-fetch.md) | Async/Await、fetch()、setTimeout、安全限制 | 5 |

### SDK (@auth9/core + @auth9/node) (6 个文档, 30 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [sdk/01-core-types-utils.md](./sdk/01-core-types-utils.md) | 类型导出、snake/camel 转换、错误体系、Claims 辨别 | 5 |
| [sdk/02-http-client.md](./sdk/02-http-client.md) | HTTP 客户端、自动转换、错误映射、Token Provider | 5 |
| [sdk/03-token-verification.md](./sdk/03-token-verification.md) | JWKS Token 验证、三种 Token 类型、Audience 验证 | 5 |
| [sdk/04-grpc-client-credentials.md](./sdk/04-grpc-client-credentials.md) | gRPC 4 方法、Client Credentials、Token 缓存 | 5 |
| [sdk/05-express-middleware.md](./sdk/05-express-middleware.md) | Express 中间件、权限控制、角色控制、AuthInfo | 5 |
| [sdk/06-middleware-testing.md](./sdk/06-middleware-testing.md) | Next.js/Fastify 中间件、Mock Token、构建输出 | 5 |

### 集成测试 (7 个文档, 35 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [integration/01-concurrent-operations.md](./integration/01-concurrent-operations.md) | 并发操作、竞态条件 | 5 |
| [integration/02-password-policy.md](./integration/02-password-policy.md) | 密码策略强制执行 | 5 |
| [integration/06-init-seed-data.md](./integration/06-init-seed-data.md) | Init 初始种子数据、幂等性、Keycloak 重置恢复 | 5 |
| [integration/07-observability-metrics.md](./integration/07-observability-metrics.md) | Prometheus /metrics 端点、HTTP 指标、X-Request-ID、路径折叠 | 5 |
| [integration/08-observability-stack.md](./integration/08-observability-stack.md) | 可观测性栈启动、Grafana 仪表盘、业务指标、限流指标 | 5 |
| [integration/09-security-hardening-config.md](./integration/09-security-hardening-config.md) | 生产环境安全启动校验、REST aud 严格校验、HSTS 条件下发、gRPC audience 必填 | 5 |
| [integration/10-security-hardening-p2.md](./integration/10-security-hardening-p2.md) | 事务性级联删除原子性、Webhook Secret 生产强制校验、外部系统同步 | 5 |

---

## 统计概览

| 模块 | 文档数 | 场景数 |
|------|--------|--------|
| 租户管理 | 4 | 20 |
| 用户管理 | 6 | 28 |
| RBAC 角色权限 | 4 | 17 |
| 服务与客户端 | 5 | 25 |
| 邀请管理 | 3 | 15 |
| 会话与安全 | 7 | 34 |
| Webhook | 4 | 17 |
| 认证流程 | 8 | 38 |
| 系统设置 | 3 | 15 |
| 身份提供商 | 3 | 15 |
| Passkeys | 3 | 15 |
| 分析与统计 | 2 | 10 |
| 审计日志 | 1 | 5 |
| Action | 6 | 35 |
| SDK | 6 | 30 |
| 集成测试 | 7 | 35 |
| **总计** | **72** | **354** |

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

### Keycloak 管理

- 地址：http://localhost:8081/admin
- 凭证：admin / admin

---

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
