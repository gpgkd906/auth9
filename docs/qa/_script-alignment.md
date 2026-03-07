# 自动化脚本对齐指南

> **用途**：本文档列出 `scripts/qa/auto/` 中尚未通过的自动化测试脚本，按根因分类。  
> **处理方式**：由 QA 测试 Agent 在实际测试执行时，根据当前环境状态对齐脚本（修正断言、补充前置数据、调整端点路径等）。  
> **更新时间**：2026-03-07

---

## 一、数据依赖（11 个脚本）

这类脚本需要 Agent **先完成前置数据准备**（创建用户、服务、Token、租户关系等），再运行脚本进行验证。

| 脚本 | 对应 QA 文档 | 通过/总数 | 需 Agent 准备的前置数据 |
|------|-------------|-----------|------------------------|
| `action-12-api-sdk-advanced.sh` | [action/12](./action/12-api-sdk-advanced.md) | 2/5 | 需确保 Action 有执行记录产生 `success_rate` 统计字段；并发创建需 Action 引擎就绪 |
| `auth-03-password.sh` | [auth/03](./auth/03-password.md) | 4/5 | 密码修改端点 `PUT /api/v1/users/me/password` 需确认实际路径；可能需浏览器流程代替 API 直调 |
| `auth-06-client-credentials.sh` | [auth/06](./auth/06-client-credentials.md) | 4/5 | 需预先创建 Service Client 并获取 `client_id` + `client_secret`，配置 Keycloak client_credentials grant |
| `integration-01-concurrent-operations.sh` | [integration/01](./integration/01-concurrent-operations.md) | 0/4 | 并发创建用户需要唯一邮箱 + 有效 tenant 关联；Agent 需确保 DB 中无同名用户残留 |
| `invitation-01-create-send.sh` | [invitation/01](./invitation/01-create-send.md) | 3/5 | 邀请已有成员应返回 409，当前返回 500；需确认被邀请用户是否已在 tenant 中 |
| `invitation-02-accept.sh` | [invitation/02](./invitation/02-accept.md) | 4/5 | 接受邀请需有效 invitation token；Agent 需先创建邀请再提取 token |
| `invitation-03-manage.sh` | [invitation/03](./invitation/03-manage.md) | 1/5 | 撤销端点 `POST /invitations/{id}/revoke` 返回 404；需确认实际撤销端点路径（可能为 PATCH 或 DELETE） |
| `provisioning-01-scim-token-management.sh` | [provisioning/01](./provisioning/01-scim-token-management.md) | 2/5 | 需先创建企业 SSO Connector，再通过 Connector 端点管理 SCIM Token |
| `provisioning-02-scim-user-crud.sh` | [provisioning/02](./provisioning/02-scim-user-crud.md) | 0/5 | 需先创建 SSO Connector + SCIM Token；SCIM 端点鉴权使用 Bearer Token 而非 Admin Token |
| `session-03-alerts.sh` | [session/03](./session/03-alerts.md) | 2/5 | `GET /api/v1/security/alerts` 返回 403；需确认该端点的授权要求（可能需要特定权限或 System Token） |
| `tenant-04-b2b-org-creation.sh` | [tenant/04](./tenant/04-b2b-org-creation.md) | 4/5 | 重复 slug 创建应返回 409 但实际返回 201；需确认 slug 唯一约束是否已实现 |

### Agent 操作指引

1. **通用前置**：使用 `gen_default_admin_token` 或设置 `QA_TOKEN` 环境变量
2. **SCIM 类**：需依次创建 Identity Provider → Enterprise SSO Connector → SCIM Token
3. **邀请类**：需创建测试用户 + 发送邀请 → 提取 invitation ID/token
4. **Client Credentials**：需在 Service 下创建 Client → 获取 client_id/secret → 配置 Keycloak

---

## 二、深度逻辑问题（18 个脚本）

这类脚本的 API 端点路径、请求格式或断言逻辑需要调整，与当前 API 实现存在偏差。

| 脚本 | 对应 QA 文档 | 通过/总数 | 问题描述 |
|------|-------------|-----------|----------|
| `action-04-security.sh` | [action/04](./action/04-security.md) | 1/4 | Action 创建返回 400：沙箱 fs/OOM 类 action 的 `script` 字段格式不符合 API 校验规则 |
| `action-06-async-fetch.sh` | [action/06](./action/06-async-fetch.md) | 0/5 | 同上，async/fetch 类 action 的 script 内容被 API 校验拒绝 |
| `action-08-execution-advanced.sh` | [action/08](./action/08-execution-advanced.md) | 1/4 | 同上，context validation 类 action 脚本格式问题 |
| `action-10-security-boundary.sh` | [action/10](./action/10-security-boundary.md) | 0/4 | 同上，内存/隔离/注入类 action 脚本格式问题 |
| `action-11-security-attack-defense.sh` | [action/11](./action/11-security-attack-defense.md) | 1/4 | 同上，命令注入/提权类 action 脚本格式问题 |
| `identity-provider-03-tenant-enterprise-sso-connectors.sh` | [identity-provider/03](./identity-provider/03-tenant-enterprise-sso-connectors.md) | 2/5 | SSO Connector CRUD 需先创建 Identity Provider；端点路径和请求体格式需与实际 API 对齐 |
| `integration-02-password-policy.sh` | [integration/02](./integration/02-password-policy.md) | 4/5 | admin 设置临时密码端点 `PUT /api/v1/admin/users/{id}/password` 返回 404；需确认实际路径 |
| `integration-03-rate-limiting.sh` | [integration/03](./integration/03-rate-limiting.md) | 4/5 | 429 响应缺少 `Retry-After` header；可能是当前限流实现未包含此 header |
| `integration-09-security-hardening-config.sh` | [integration/09](./integration/09-security-hardening-config.md) | 2/5 | 测试期望错误信息包含 "gRPC" 和 "production" 关键字，但实际错误信息格式不同 |
| `integration-10-security-hardening-p2.sh` | [integration/10](./integration/10-security-hardening-p2.md) | 0/4 | 级联删除测试：创建用户返回 422（字段校验）；需对齐用户创建的必填字段 |
| `integration-11-keycloak26-event-stream.sh` | [integration/11](./integration/11-keycloak26-event-stream.md) | 3/4 | Keycloak OIDC discovery URL 未返回有效内容；需确认 Keycloak 实例是否运行且 realm 正确 |
| `rbac-03-assignment.sh` | [rbac/03](./rbac/03-assignment.md) | 3/5 | 角色分配后查询角色详情未返回权限列表；权限 ID 硬编码需改为动态获取 |
| `rbac-04-advanced.sh` | [rbac/04](./rbac/04-advanced.md) | 1/3 | 角色层次（parent_role_id）创建后查询返回 null；需确认角色继承 API 的实际响应格式 |
| `session-05-auth-security-regression.sh` | [session/05](./session/05-auth-security-regression.md) | 3/5 | 限流触发阈值可能 > 30 次请求；限流 key 折叠行为与预期不符 |
| `session-07-oauth-state-csrf.sh` | [session/07](./session/07-oauth-state-csrf.md) | 3/5 | OAuth login 端点无响应（可能需浏览器环境）；state cookie 检测需 SSO 配置就绪 |
| `session-08-identity-token-whitelist.sh` | [session/08](./session/08-identity-token-whitelist-tenant-token-enforcement.md) | 3/4 | Identity Token 访问 `/tenants` 返回 500 而非 200；需确认白名单策略实现 |
| `tenant-03-status-lifecycle.sh` | [tenant/03](./tenant/03-status-lifecycle.md) | 0/5 | 获取租户详情返回 400；可能 tenant ID 格式或查询参数有误 |
| `user-02-advanced.sh` | [user/02](./user/02-advanced.md) | 3/5 | MFA 启用端点 `POST /api/v1/users/{id}/mfa` 返回 403；需确认 MFA 管理端点的权限要求 |

### Agent 操作指引

1. **Action 类**：需调查 Action API 对 `script` 字段的校验规则（语法、长度、禁止关键字），调整测试脚本内容
2. **端点路径类**：需查阅 `auth9-core/src/domains/*/routes.rs` 确认实际路由定义，更新脚本中的 URL
3. **响应格式类**：需对实际 API 响应做 `curl` 调用采样，更新脚本中的 `jq` 路径和断言值
4. **权限类**：需确认哪些端点需要 System Token vs Tenant Token，调整 Token 生成方式

---

## 三、安全检查差异（27 个脚本）

这类脚本测试安全相关行为，脚本预期的安全响应与当前 API 实际行为存在差异。

| 脚本 | 通过/总数 | 差异描述 |
|------|-----------|----------|
| `security-advanced-attacks-04-oidc-advanced.sh` | 2/3 | 无效 `client_secret` 返回 415（Content-Type 不匹配）而非 400/401 |
| `security-api-security-01-rest-api.sh` | 4/5 | Tenant User 可访问 `system/email` 配置（返回 200），预期应为 401/403 |
| `security-api-security-03-rate-limiting.sh` | 3/4 | 审计日志端点返回 403；需确认 audit logs 的授权要求 |
| `security-api-security-06-api-boundary-pagination.sh` | 2/3 | 批量服务创建数量为 0；可能受限流或创建逻辑影响 |
| `security-authentication-01-oidc-security.sh` | 2/5 | auth code 回放攻击测试返回 415 而非 400/401；需使用正确的 Content-Type |
| `security-authentication-02-token-security.sh` | 1/3 | `alg:none` / HS256 混淆 token 返回 429（被限流）而非 401 |
| `security-authentication-03-mfa-security.sh` | 4/5 | 未认证请求返回 429 而非 401；高频测试触发限流 |
| `security-authentication-04-password-security.sh` | 2/4 | `forgot-password` 端点返回 429；密码策略响应不含 `hashAlgorithm` 字段 |
| `security-authentication-05-idp-security.sh` | 3/4 | 模板表达式 `${7*7}` 在响应中被计算为 49；可能是 SSTI 风险或误报 |
| `security-authorization-01-tenant-isolation.sh` | 1/4 | 跨租户 PATCH 返回 405（方法不允许）而非 403/404；角色列表端点返回 405 |
| `security-authorization-02-rbac-bypass.sh` | 3/5 | viewer 创建用户返回 409（而非 403）；viewer 删除用户返回 500（而非 403） |
| `security-authorization-03-privilege-escalation.sh` | 4/5 | 邀请创建返回 429；高频测试触发限流 |
| `security-authorization-05-system-config-authz.sh` | 7/8 | Service Client 更新系统品牌返回 422（校验失败）而非 401/403 |
| `security-data-security-01-sensitive-data.sh` | 4/5 | 用户响应中包含 `password` 字段名（可能是字段名匹配误报，非明文密码） |
| `security-data-security-02-encryption.sh` | 4/5 | RSA modulus 长度字段有前导空格，导致数字正则匹配失败 |
| `security-data-security-03-secrets-management.sh` | 1/4 | Portal `.git/config` 返回 500 而非 403/404；client secret 轮换返回 403 |
| `security-data-security-04-encryption-impl.sh` | 2/3 | 源码中有 3 处 `SETTINGS_ENCRYPTION_KEY` 硬编码引用（预期 0） |
| `security-file-security-01-file-upload.sh` | 1/3 | `ftp:` scheme 和 AWS metadata URL 在 branding 字段中返回 404 而非 400/422 |
| `security-file-security-02-theme-resource-url-security.sh` | 2/3 | 主题资源 URL 安全检查部分失败 |
| `security-infrastructure-02-security-headers.sh` | 2/5 | 安全 header（CSP、X-Frame-Options 等）缺失或不符合预期 |
| `security-infrastructure-03-dependency-audit.sh` | 2/4 | Dockerfile 使用 `:latest` tag；trivy 扫描发现 CRITICAL 漏洞 |
| `security-input-validation-01-injection.sh` | 4/5 | 邮箱命令注入测试返回 429；高频测试触发限流 |
| `security-input-validation-02-xss.sh` | 2/5 | XSS payload 在 `display_name` 中返回 429；service name XSS 返回 403 |
| `security-input-validation-04-parameter-tampering.sh` | 3/4 | 字符串类型的 numeric 字段返回 403 而非 400/422 |
| `security-input-validation-06-deserialization.sh` | 1/3 | 超大 JSON body 未返回 413；可能未配置 body size limit |
| `security-logging-monitoring-01-log-security.sh` | 2/5 | CRLF/换行注入测试返回 429；高频测试触发限流 |
| `security-session-management-01-session-security.sh` | 3/4 | 列出会话返回 401；可能需要不同类型的 Token |

### Agent 操作指引

1. **限流触发（429）**：安全测试高频调用容易触发限流。Agent 需在测试间添加适当延迟（`sleep 2-5s`），或临时调高限流阈值
2. **Content-Type 差异（415）**：OIDC token 端点需要 `application/x-www-form-urlencoded` 而非 JSON
3. **权限模型差异**：部分安全断言假设的权限模型（如 viewer 不能创建用户）需与实际 RBAC 策略对齐
4. **安全 Header**：需确认 auth9-core 和反向代理层各自负责哪些安全 header
5. **敏感数据**：`password` 字段匹配可能是 JSON key 名（如 `password_hash`）而非明文泄露，需精确化断言

---

## 四、已通过脚本（38 个）

以下脚本已通过，无需对齐：

- `action-01-crud.sh` · `action-02-execution.sh` · `action-03-logs.sh`
- `action-05-api-sdk.sh` · `action-07-crud-advanced.sh` · `action-09-logs-detail.sh`
- `analytics-01-overview.sh` · `analytics-02-events.sh`
- `audit-01-audit-logs.sh`
- `auth-01-oidc-login.sh` · `auth-04-social.sh` · `auth-05-boundary.sh` · `auth-07-public-endpoints.sh`
- `identity-provider-01-crud.sh` · `identity-provider-02-toggle-validation.sh`
- `integration-04-health-check.sh` · `integration-05-keycloak-events.sh`
- `rbac-01-permission.sh` · `rbac-02-role.sh` · `rbac-05-abac-policy-management.sh`
- `service-01-service-crud.sh` · `service-02-client.sh` · `service-04-tenant-service-toggle.sh`
- `session-01-session.sh` · `session-02-login-events.sh` · `session-04-boundary.sh` · `session-06-token-blacklist-failsafe.sh`
- `settings-01-branding.sh` · `settings-02-email-provider.sh` · `settings-03-email-templates.sh`
- `tenant-01-crud.sh` · `tenant-02-list-settings.sh`
- `user-01-crud.sh` · `user-03-validation.sh`
- `webhook-01-crud.sh` · `webhook-02-trigger.sh` · `webhook-03-reliability.sh` · `webhook-04-boundary.sh`

---

## 五、已删除脚本（11 个）

以下脚本因端点不存在、需 Docker 网络或依赖外部工具，已从 `scripts/qa/auto/` 中删除：

| 类别 | 脚本 | 删除原因 |
|------|------|----------|
| SDK（vitest） | `sdk-01` ~ `sdk-06` (6 个) | 依赖 vitest 测试框架，不适合 bash 脚本测试 |
| gRPC（Docker 网络） | `auth-02-token-exchange.sh` | 需要 Docker 内部网络访问 gRPC 端口 |
| gRPC（Docker 网络） | `security-advanced-attacks-02-grpc-security.sh` | 同上 |
| gRPC（Docker 网络） | `security-api-security-02-grpc-api.sh` | 同上 |
| 端点不存在 | `service-03-oidc.sh` | OIDC 配置端点在 routes.rs 中不存在 |
| 外部工具 | `security-advanced-attacks-01-supply-chain-security.sh` | 执行 npm audit / cargo audit，非 API 测试 |

---

## 六、环境变量参考

QA 测试 Agent 可通过以下环境变量为脚本注入前置数据：

| 变量 | 说明 | 示例 |
|------|------|------|
| `QA_TENANT_ID` | 目标租户 ID | `a1b2c3d4-...` |
| `QA_SERVICE_ID` | 目标服务 ID | `e5f6a7b8-...` |
| `QA_ADMIN_ID` | 管理员用户 ID | `11223344-...` |
| `QA_TOKEN` | 预生成的 Tenant Token | `eyJhbG...` |

脚本内部通过 `scripts/qa/lib/setup.sh` 中的 `qa_get_tenant_id()`、`qa_get_service_id()`、`qa_get_admin_id()`、`gen_default_admin_token()` 函数使用这些变量。优先使用环境变量，若未设置则回退到数据库查询。
