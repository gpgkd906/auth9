# Auth9 安全测试文档

本目录包含 Auth9 系统的安全测试用例，供安全测试工程师进行渗透测试和安全评估。

## 项目安全概述

Auth9 是一个自托管的身份认证服务，核心安全组件包括：
- **Keycloak**: OIDC 协议处理、MFA 认证
- **Auth9 Core**: 业务逻辑、Token Exchange、RBAC
- **Auth9 Portal**: 管理界面 (React Router 7)

## 测试用例索引

### 认证安全 (5 个文档, 24 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [authentication/01-oidc-security.md](./authentication/01-oidc-security.md) | OIDC 流程安全测试 | 5 | 高 |
| [authentication/02-token-security.md](./authentication/02-token-security.md) | JWT Token 安全测试 | 5 | 极高 |
| [authentication/03-mfa-security.md](./authentication/03-mfa-security.md) | 多因素认证安全测试 | 5 | 高 |
| [authentication/04-password-security.md](./authentication/04-password-security.md) | 密码安全测试 | 5 | 高 |
| [authentication/05-idp-security.md](./authentication/05-idp-security.md) | 身份提供商与邮件安全测试 | 4 | 高 | 🆕

### 授权安全 (4 个文档, 20 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [authorization/01-tenant-isolation.md](./authorization/01-tenant-isolation.md) | 租户隔离测试 | 5 | 极高 |
| [authorization/02-rbac-bypass.md](./authorization/02-rbac-bypass.md) | RBAC 权限绕过测试 | 5 | 极高 |
| [authorization/03-privilege-escalation.md](./authorization/03-privilege-escalation.md) | 权限提升测试 | 5 | 极高 |
| [authorization/04-resource-access.md](./authorization/04-resource-access.md) | 资源访问控制测试 | 5 | 高 |

### 输入验证 (6 个文档, 27 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [input-validation/01-injection.md](./input-validation/01-injection.md) | 注入攻击测试 (SQL/NoSQL) | 5 | 极高 |
| [input-validation/02-xss.md](./input-validation/02-xss.md) | 跨站脚本攻击测试 | 5 | 高 |
| [input-validation/03-csrf.md](./input-validation/03-csrf.md) | CSRF 攻击测试 | 5 | 高 |
| [input-validation/04-parameter-tampering.md](./input-validation/04-parameter-tampering.md) | 参数篡改测试 | 4 | 中 |
| [input-validation/05-ssrf.md](./input-validation/05-ssrf.md) | 服务器端请求伪造 (SSRF) 测试 | 5 | 极高 | 🆕
| [input-validation/06-deserialization.md](./input-validation/06-deserialization.md) | 反序列化安全测试 | 3 | 高 | 🆕

### API 安全 (4 个文档, 19 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [api-security/01-rest-api.md](./api-security/01-rest-api.md) | REST API 安全测试 | 5 | 高 |
| [api-security/02-grpc-api.md](./api-security/02-grpc-api.md) | gRPC API 安全测试 | 5 | 极高 |
| [api-security/03-rate-limiting.md](./api-security/03-rate-limiting.md) | 限流与 DoS 防护测试 | 5 | 高 |
| [api-security/04-cors-headers.md](./api-security/04-cors-headers.md) | CORS 与安全头测试 | 4 | 中 |

### 数据安全 (4 个文档, 17 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [data-security/01-sensitive-data.md](./data-security/01-sensitive-data.md) | 敏感数据暴露测试 | 5 | 极高 |
| [data-security/02-encryption.md](./data-security/02-encryption.md) | 加密安全测试 | 5 | 高 |
| [data-security/03-secrets-management.md](./data-security/03-secrets-management.md) | 密钥管理安全测试 | 4 | 极高 |
| [data-security/04-encryption-impl.md](./data-security/04-encryption-impl.md) | 加密实现安全测试 (AES-GCM) | 3 | 高 | 🆕

### 会话管理 (3 个文档, 14 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [session-management/01-session-security.md](./session-management/01-session-security.md) | 会话安全测试 | 5 | 高 |
| [session-management/02-token-lifecycle.md](./session-management/02-token-lifecycle.md) | Token 生命周期测试 | 5 | 高 |
| [session-management/03-logout-security.md](./session-management/03-logout-security.md) | 登出安全测试 | 4 | 中 |

### 基础设施安全 (3 个文档, 14 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [infrastructure/01-tls-config.md](./infrastructure/01-tls-config.md) | TLS 配置安全测试 | 5 | 高 |
| [infrastructure/02-security-headers.md](./infrastructure/02-security-headers.md) | HTTP 安全头测试 | 5 | 中 |
| [infrastructure/03-dependency-audit.md](./infrastructure/03-dependency-audit.md) | 依赖漏洞审计 | 4 | 高 |

### 业务逻辑安全 (2 个文档, 9 个场景) 🆕
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [business-logic/01-workflow-abuse.md](./business-logic/01-workflow-abuse.md) | 工作流滥用测试 | 5 | 极高 | 🆕
| [business-logic/02-race-conditions.md](./business-logic/02-race-conditions.md) | 竞态条件测试 | 4 | 极高 | 🆕

### 日志与监控安全 (1 个文档, 5 个场景) 🆕
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [logging-monitoring/01-log-security.md](./logging-monitoring/01-log-security.md) | 日志安全与检测系统测试 | 5 | 高 | 🆕

### 文件安全 (1 个文档, 4 个场景) 🆕
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [file-security/01-file-upload.md](./file-security/01-file-upload.md) | 文件上传安全测试 | 4 | 高 | 🆕

### 高级攻击 (6 个文档, 24 个场景)
| 文档 | 描述 | 场景数 | 风险等级 |
|------|------|--------|----------|
| [advanced-attacks/01-supply-chain-security.md](./advanced-attacks/01-supply-chain-security.md) | 供应链与依赖安全测试 | 5 | 极高 |
| [advanced-attacks/02-grpc-security.md](./advanced-attacks/02-grpc-security.md) | gRPC 安全测试 | 5 | 极高 |
| [advanced-attacks/03-detection-evasion.md](./advanced-attacks/03-detection-evasion.md) | 安全检测规避测试 | 4 | 高 | 🆕
| [advanced-attacks/04-oidc-advanced.md](./advanced-attacks/04-oidc-advanced.md) | OIDC 高级攻击测试 | 3 | 高 | 🆕
| [advanced-attacks/05-webhook-forgery.md](./advanced-attacks/05-webhook-forgery.md) | Webhook 伪造测试 | 3 | 高 | 🆕
| [advanced-attacks/06-http-smuggling.md](./advanced-attacks/06-http-smuggling.md) | HTTP 请求走私测试 | 2 | 高 | 🆕

---

## 统计概览

| 模块 | 文档数 | 场景数 |
|------|--------|--------|
| 认证安全 | 5 | 24 |
| 授权安全 | 4 | 20 |
| 输入验证 | 6 | 27 |
| API 安全 | 4 | 19 |
| 数据安全 | 4 | 17 |
| 会话管理 | 3 | 14 |
| 基础设施安全 | 3 | 14 |
| 业务逻辑安全 | 2 | 9 |
| 日志与监控安全 | 1 | 5 |
| 文件安全 | 1 | 4 |
| 高级攻击 | 6 | 24 |
| **总计** | **39** | **177** |

---

## OWASP ASVS 覆盖矩阵

| ASVS 章节 | 覆盖文档 | 覆盖程度 |
|-----------|---------|---------|
| V2 认证 | authentication/*, advanced-attacks/04 | 🟩 90% |
| V3 会话管理 | session-management/* | 🟩 80% |
| V4 访问控制 | authorization/* | 🟩 90% |
| V5 输入验证 | input-validation/* | 🟩 85% |
| V6 存储加密 | data-security/02, data-security/04 | 🟩 75% |
| V7 错误处理与日志 | logging-monitoring/01 | 🟧 60% |
| V8 数据保护 | data-security/01, data-security/03 | 🟩 70% |
| V9 通信安全 | infrastructure/01 | 🟩 75% |
| V11 业务逻辑 | business-logic/* | 🟩 70% |
| V12 文件与资源 | file-security/01, input-validation/05 | 🟩 70% |
| V13 API 安全 | api-security/*, advanced-attacks/05-06 | 🟩 85% |
| V14 配置 | infrastructure/02-03, advanced-attacks/01 | 🟩 75% |

---

## 风险等级定义

| 等级 | 标记 | 描述 |
|------|------|------|
| 极高 | 🔴 | 可能导致系统完全失控、数据大规模泄露或权限完全绕过 |
| 高 | 🟠 | 可能导致部分数据泄露、权限绕过或服务中断 |
| 中 | 🟡 | 可能导致信息泄露或对个别用户造成影响 |
| 低 | 🟢 | 潜在安全隐患，但利用难度较高或影响有限 |

---

## 测试分配建议

每位安全测试工程师可以领取 1-2 个文档进行测试。建议的执行顺序：

### 第一阶段：核心认证/授权 (P0)
1. authentication/02-token-security.md - Token 是系统核心
2. authorization/01-tenant-isolation.md - 多租户隔离是关键
3. authorization/02-rbac-bypass.md - RBAC 权限模型安全
4. api-security/02-grpc-api.md - gRPC 目前无认证保护
5. input-validation/05-ssrf.md - Webhook SSRF 风险 🆕
6. business-logic/01-workflow-abuse.md - Token Exchange 流程安全 🆕
7. business-logic/02-race-conditions.md - 竞态条件 🆕

### 第二阶段：输入/数据安全 (P1)
8. input-validation/01-injection.md - 注入攻击
9. data-security/01-sensitive-data.md - 敏感数据暴露
10. data-security/03-secrets-management.md - 密钥管理
11. data-security/04-encryption-impl.md - AES-GCM 加密实现 🆕
12. file-security/01-file-upload.md - 文件上传安全 🆕
13. input-validation/06-deserialization.md - 反序列化攻击 🆕
14. logging-monitoring/01-log-security.md - 日志与检测系统 🆕

### 第三阶段：会话/API/高级攻击 (P1)
15. session-management/01-session-security.md - 会话安全
16. api-security/01-rest-api.md - REST API 安全
17. api-security/03-rate-limiting.md - DoS 防护
18. advanced-attacks/03-detection-evasion.md - 检测规避 🆕
19. advanced-attacks/05-webhook-forgery.md - Webhook 伪造 🆕

### 第四阶段：其他安全测试 (P2)
20. authentication/05-idp-security.md - IdP 与邮件安全 🆕
21. advanced-attacks/04-oidc-advanced.md - OIDC 高级攻击 🆕
22. advanced-attacks/06-http-smuggling.md - HTTP 请求走私 🆕
23. 其余文档按需测试

---

## 测试环境准备

### 本地环境
```bash
# 启动依赖服务
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# 启动后端
cd auth9-core && cargo run

# 启动前端
cd auth9-portal && npm run dev
```

### 服务端点
| 服务 | 端口 | 用途 |
|------|------|------|
| Auth9 Portal | 3000 | 管理界面 |
| Auth9 Core (HTTP) | 8080 | REST API |
| Auth9 Core (gRPC) | 50051 | gRPC API |
| Keycloak | 8081 | OIDC 认证 |
| TiDB | 4000 | 数据库 |
| Redis | 6379 | 缓存 |

### 测试账户
| 角色 | 用户名 | 密码 | 用途 |
|------|--------|------|------|
| Platform Admin | admin@auth9.local | TestAdmin123! | 平台管理员 |
| Tenant Admin | tenant-admin@test.com | TestTenant123! | 租户管理员 |
| Normal User | user@test.com | TestUser123! | 普通用户 |

### 常用工具
- **Burp Suite**: HTTP/HTTPS 代理与渗透测试
- **grpcurl**: gRPC API 测试
- **sqlmap**: SQL 注入自动化测试
- **jwt.io**: JWT Token 解析
- **nikto**: Web 服务器扫描
- **smuggler**: HTTP 请求走私检测 🆕
- **h2spec**: HTTP/2 合规性测试 🆕
- **race-the-web**: 竞态条件测试 🆕

---

## 测试用例结构

每个测试场景包含：

1. **前置条件** - 测试环境和数据准备
2. **攻击目标** - 测试要验证的安全风险
3. **攻击步骤** - 详细的测试操作流程
4. **预期安全行为** - 系统应有的安全响应
5. **验证方法** - 如何确认安全措施生效
6. **修复建议** - 如发现漏洞的修复方向

---

## 漏洞报告格式

```markdown
## 漏洞: [简短描述]

**测试文档**: [文档路径]
**场景**: #X
**风险等级**: [极高/高/中/低]
**CVSS 评分**: X.X

### 漏洞描述
[详细描述漏洞本质]

### 复现步骤
1. ...
2. ...

### 影响范围
- 受影响的端点/功能
- 潜在的数据泄露范围
- 可能的攻击场景

### 证据
[请求/响应截图、日志等]

### 修复建议
[具体的修复方案]

### 参考资料
- [相关 OWASP 条目]
- [相关 CWE 编号]
```

---

## 参考文档

### 项目文档
- [架构设计](../architecture.md) - 系统架构概述
- [API 访问控制](../api-access-control.md) - 端点分类与权限设计

### 安全标准
- [OWASP Top 10 2021](https://owasp.org/Top10/)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [OWASP Testing Guide](https://owasp.org/www-project-web-security-testing-guide/)
- [OWASP ASVS 4.0](https://owasp.org/www-project-application-security-verification-standard/)
- [CWE Top 25](https://cwe.mitre.org/top25/archive/2023/2023_top25_list.html)

### 认证相关标准
- [RFC 6749 - OAuth 2.0](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 7519 - JWT](https://datatracker.ietf.org/doc/html/rfc7519)
- [RFC 8414 - OAuth 2.0 Discovery](https://datatracker.ietf.org/doc/html/rfc8414)
- [RFC 8693 - OAuth 2.0 Token Exchange](https://datatracker.ietf.org/doc/html/rfc8693)
- [OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html)

---

## 测试数据准备

### 安全测试专用数据

为了进行全面的安全测试，Auth9 提供了包含已知弱配置的测试数据：

```bash
# ⚠️ 警告：此数据集包含故意设置的安全漏洞，仅用于安全测试
cd auth9-core
cargo run --bin seed-data -- --dataset=security-vulnerable --reset

# 或使用 YAML 配置
# 参考 scripts/seed-data/security-vulnerable.yaml
```

此数据集包含：
- 弱密码策略租户
- SQL/XSS 注入测试用户
- 配置错误的客户端（redirect_uri 通配符）
- SSRF 测试 Webhook
- 循环角色继承
- 明文密码配置

详细说明请参考 [测试数据种子设计文档](../testing/seed-data-design.md)。

---

## 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|----------|
| 2026-02-07 | 2.0.0 | 基于 OWASP ASVS 差距分析，新增 12 个文档 47 个场景：SSRF、业务逻辑、竞态条件、日志安全、文件上传、反序列化、加密实现、检测规避、OIDC 高级攻击、Webhook 伪造、HTTP 走私、IdP 安全；共 39 个文档 177 个场景 |
| 2026-02-05 | 1.1.0 | 新增高级攻击模块（供应链安全、gRPC 安全），共 27 个文档 130 个场景；新增安全测试专用种子数据 |
| 2026-02-03 | 1.0.0 | 初始版本，25 个文档 120 个场景 |
