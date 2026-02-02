# Auth9 QA 测试用例文档

本目录包含 Auth9 系统的手动测试用例，每个文档不超过 5 个场景，便于多名 QA 工程师并行测试。

## 测试用例索引

### 租户管理 (2 个文档, 10 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [tenant/01-crud.md](./tenant/01-crud.md) | 创建、更新、删除操作 | 5 |
| [tenant/02-list-settings.md](./tenant/02-list-settings.md) | 列表、搜索、设置 | 5 |

### 用户管理 (3 个文档, 13 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [user/01-crud.md](./user/01-crud.md) | 创建、更新、租户关联 | 5 |
| [user/02-advanced.md](./user/02-advanced.md) | 删除、MFA、列表 | 5 |
| [user/03-validation.md](./user/03-validation.md) | 边界测试、验证 | 3 |

### RBAC 角色权限 (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [rbac/01-permission.md](./rbac/01-permission.md) | 权限 CRUD | 4 |
| [rbac/02-role.md](./rbac/02-role.md) | 角色 CRUD、继承 | 5 |
| [rbac/03-assignment.md](./rbac/03-assignment.md) | 权限分配、用户角色 | 5 |
| [rbac/04-advanced.md](./rbac/04-advanced.md) | 层次视图、循环检测 | 3 |

### 服务与客户端 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [service/01-service-crud.md](./service/01-service-crud.md) | 服务 CRUD | 5 |
| [service/02-client.md](./service/02-client.md) | 客户端管理、密钥 | 5 |
| [service/03-oidc.md](./service/03-oidc.md) | OIDC 配置、URI 验证 | 5 |

### 邀请管理 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [invitation/01-create-send.md](./invitation/01-create-send.md) | 创建、发送邀请 | 5 |
| [invitation/02-accept.md](./invitation/02-accept.md) | 接受邀请流程 | 5 |
| [invitation/03-manage.md](./invitation/03-manage.md) | 撤销、删除、过滤 | 5 |

### 会话与安全 (4 个文档, 20 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [session/01-session.md](./session/01-session.md) | 会话管理、撤销 | 5 |
| [session/02-login-events.md](./session/02-login-events.md) | 登录事件记录 | 5 |
| [session/03-alerts.md](./session/03-alerts.md) | 安全告警检测 | 5 |
| [session/04-boundary.md](./session/04-boundary.md) | 边界测试 | 5 |

### Webhook (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [webhook/01-crud.md](./webhook/01-crud.md) | Webhook CRUD | 5 |
| [webhook/02-trigger.md](./webhook/02-trigger.md) | 事件触发、签名 | 5 |
| [webhook/03-reliability.md](./webhook/03-reliability.md) | 重试、自动禁用 | 4 |
| [webhook/04-boundary.md](./webhook/04-boundary.md) | URL 验证、边界 | 3 |

### 认证流程 (5 个文档, 23 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [auth/01-oidc-login.md](./auth/01-oidc-login.md) | OIDC 登录流程 | 5 |
| [auth/02-token-exchange.md](./auth/02-token-exchange.md) | Token Exchange | 5 |
| [auth/03-password.md](./auth/03-password.md) | 密码管理 | 5 |
| [auth/04-social.md](./auth/04-social.md) | 社交登录、OIDC 端点 | 5 |
| [auth/05-boundary.md](./auth/05-boundary.md) | 边界测试 | 3 |

### 系统设置 (3 个文档, 15 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [settings/01-branding.md](./settings/01-branding.md) | 登录页品牌设置 | 5 |
| [settings/02-email-provider.md](./settings/02-email-provider.md) | 邮件服务商配置 | 5 |
| [settings/03-email-templates.md](./settings/03-email-templates.md) | 邮件模板管理 | 5 |

### 身份提供商 (2 个文档, 10 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [identity-provider/01-crud.md](./identity-provider/01-crud.md) | 创建、更新、删除身份提供商 | 5 |
| [identity-provider/02-toggle-validation.md](./identity-provider/02-toggle-validation.md) | 启用/禁用、验证、登录集成 | 5 |

### Passkeys (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [passkeys/01-passkeys.md](./passkeys/01-passkeys.md) | Passkey 注册、列表、删除、登录 | 5 |

### 分析与统计 (2 个文档, 10 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [analytics/01-overview.md](./analytics/01-overview.md) | 统计概览、时间范围筛选 | 5 |
| [analytics/02-events.md](./analytics/02-events.md) | 登录事件列表、分页 | 5 |

### 审计日志 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [audit/01-audit-logs.md](./audit/01-audit-logs.md) | 审计日志查看、验证 | 5 |

---

## 统计概览

| 模块 | 文档数 | 场景数 |
|------|--------|--------|
| 租户管理 | 2 | 10 |
| 用户管理 | 3 | 13 |
| RBAC 角色权限 | 4 | 17 |
| 服务与客户端 | 3 | 15 |
| 邀请管理 | 3 | 15 |
| 会话与安全 | 4 | 20 |
| Webhook | 4 | 17 |
| 认证流程 | 5 | 23 |
| 系统设置 | 3 | 15 |
| 身份提供商 | 2 | 10 |
| Passkeys | 1 | 5 |
| 分析与统计 | 2 | 10 |
| 审计日志 | 1 | 5 |
| **总计** | **37** | **175** |

---

## 测试分配建议

每位 QA 工程师可以领取 1-2 个文档进行测试。文档之间相对独立，可以并行执行。

**建议的执行顺序**（如有依赖）：
1. 认证流程 (auth/*) - 先确保登录功能正常
2. 系统设置 (settings/*) - 配置品牌和邮件
3. 租户管理 (tenant/*) - 创建测试租户
4. 用户管理 (user/*) - 创建测试用户
5. 身份提供商 (identity-provider/*) - 配置社交登录
6. Passkeys (passkeys/*) - 测试无密码登录
7. 服务与客户端 (service/*) - 配置测试服务
8. RBAC (rbac/*) - 配置角色和权限
9. 邀请管理 (invitation/*) - 测试邀请流程
10. 会话与安全 (session/*) - 测试安全功能
11. Webhook (webhook/*) - 测试事件通知
12. 分析与统计 (analytics/*) - 验证登录统计
13. 审计日志 (audit/*) - 验证操作记录

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

## 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|----------|
| 2026-02-02 | 3.0.0 | 新增系统设置、身份提供商、Passkeys、分析统计、审计日志模块，共 37 个文档 175 个场景 |
| 2024-02-02 | 2.0.0 | 细分文档，每个不超过 5 个场景，共 28 个文档 |
| 2024-02-02 | 1.0.0 | 初始版本 |
