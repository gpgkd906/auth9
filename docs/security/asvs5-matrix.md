# Auth9 ASVS 5.0 单文件矩阵（执行版）

**版本**: 2026-02-18  
**适用范围**: docs/security 全量可执行测试文档（46 份）  
**目标等级**: 整体 L2 + 高风险域 L3

---

## 1. 基线与覆盖目标

- ASVS 5.0 总控制项: 345
- 排除 V17(WebRTC) 后适用控制项: 333
- L1+L2 控制项: 246
- L3 控制项: 87

阶段目标：
1. Phase 1: L1+L2 >= 70%
2. Phase 2: L1+L2 >= 90% 且高风险域 L3 >= 60%
3. Phase 3: L1+L2 >= 95% 且高风险域 L3 >= 80%

高风险域：V4, V6, V7, V8, V9, V10, V13, V16。

---

## 2. 章节矩阵（ASVS 5.0）

| 章节 | 名称 | L1+L2 控制数 | 项目目标覆盖率 | 主要测试文档组 |
|---|---|---:|---:|---|
| V1 | Encoding and Sanitization | 27 | 85% | input-validation/* |
| V2 | Validation and Business Logic | 11 | 90% | business-logic/*, input-validation/* |
| V3 | Web Frontend Security | 19 | 90% | input-validation/02-03, infrastructure/02, auth9-portal, auth9-keycloak-theme |
| V4 | API and Web Service | 10 | 95% | api-security/*, authorization/* |
| V5 | File Handling | 9 | 90% | file-security/*, input-validation/05-06 |
| V6 | Authentication | 35 | 95% | authentication/* |
| V7 | Session Management | 18 | 95% | session-management/* |
| V8 | Authorization | 7 | 100% | authorization/*, business-logic/03 |
| V9 | Self-contained Tokens | 7 | 100% | authentication/02, session-management/02, advanced-attacks/04-07 |
| V10 | OAuth and OIDC | 29 | 95% | authentication/01, advanced-attacks/04-05 |
| V11 | Cryptography | 14 | 90% | data-security/02-04 |
| V12 | Secure Communication | 9 | 90% | infrastructure/01, input-validation/05 |
| V13 | Configuration | 13 | 95% | infrastructure/03, api-security/05-06, advanced-attacks/01 |
| V14 | Data Protection | 9 | 90% | data-security/* |
| V15 | Secure Coding and Architecture | 13 | 85% | advanced-attacks/01, advanced-attacks/07, infrastructure/03 |
| V16 | Security Logging and Error Handling | 16 | 90% | logging-monitoring/* |

---

## 3. 文档级矩阵映射

| 矩阵ID | 测试文档 | ASVS 5.0 控制映射 | 关联回归任务 |
|---|---|---|---|
| M-ADV-01 | `advanced-attacks/01-supply-chain-security.md` | V13.1,V13.2,V15.1,V15.2 | Backlog #14,20 |
| M-ADV-02 | `advanced-attacks/02-grpc-security.md` | V4.1,V4.2,V8.1,V13.2 | Backlog #3,20 |
| M-ADV-03 | `advanced-attacks/03-detection-evasion.md` | V16.1,V16.2,V16.3,V2.5 | Backlog #12,19 |
| M-ADV-04 | `advanced-attacks/04-oidc-advanced.md` | V10.1,V10.2,V10.4,V9.1 | Backlog #1,4 |
| M-ADV-05 | `advanced-attacks/05-webhook-forgery.md` | V10.5,V13.2,V16.2 | Backlog #5,20 |
| M-ADV-06 | `advanced-attacks/06-http-smuggling.md` | V4.3,V12.2,V13.3 | Backlog #20 |
| M-API-01 | `api-security/01-rest-api.md` | V4.1,V4.2,V4.3,V8.1 | Backlog #2,20 |
| M-API-02 | `api-security/02-grpc-api.md` | V4.1,V4.2,V13.2 | Backlog #3,20 |
| M-API-03 | `api-security/03-rate-limiting.md` | V4.4,V13.3,V2.5 | Backlog #20 |
| M-API-04 | `api-security/04-cors-headers.md` | V3.4,V12.1,V13.1 | Backlog #13,20 |
| M-API-05 | `api-security/05-rate-limit-bypass-hardening.md` | V4.4,V13.3,V16.2 | Backlog #3,19,20 |
| M-AUTH-01 | `authentication/01-oidc-security.md` | V10.1,V10.2,V10.3,V10.4 | Backlog #4,20 |
| M-AUTH-02 | `authentication/02-token-security.md` | V9.1,V9.2,V9.3,V6.2 | Backlog #1,20 |
| M-AUTH-03 | `authentication/03-mfa-security.md` | V6.7,V6.8,V7.3 | Backlog #20 |
| M-AUTH-04 | `authentication/04-password-security.md` | V6.1,V6.2,V6.3,V6.6 | Backlog #11,20 |
| M-AUTH-05 | `authentication/05-idp-security.md` | V10.5,V10.6,V6.4 | Backlog #4,20 |
| M-AUTHZ-01 | `authorization/01-tenant-isolation.md` | V8.1,V8.2,V4.2 | Backlog #2,20 |
| M-AUTHZ-02 | `authorization/02-rbac-bypass.md` | V8.1,V8.3,V8.4 | Backlog #2,9,20 |
| M-AUTHZ-03 | `authorization/03-privilege-escalation.md` | V8.2,V8.3,V8.4 | Backlog #2,9,20 |
| M-AUTHZ-04 | `authorization/04-resource-access.md` | V8.1,V8.2,V4.2 | Backlog #2,20 |
| M-AUTHZ-05 | `authorization/05-system-config-authz.md` | V8.2,V13.1,V13.2,V4.2 | Backlog #2,20 |
| M-BIZ-01 | `business-logic/01-workflow-abuse.md` | V2.1,V2.2,V2.5,V8.2 | Backlog #1,10,20 |
| M-BIZ-02 | `business-logic/02-race-conditions.md` | V2.5,V7.2,V8.2 | Backlog #9,10,11 |
| M-BIZ-03 | `business-logic/03-admin-operational-endpoint-abuse.md` | V8.2,V4.2,V16.2 | Backlog #2,12,20 |
| M-DATA-01 | `data-security/01-sensitive-data.md` | V14.1,V14.2,V16.4 | Backlog #20 |
| M-DATA-02 | `data-security/02-encryption.md` | V11.1,V11.2,V12.1,V14.3 | Backlog #20 |
| M-DATA-03 | `data-security/03-secrets-management.md` | V11.3,V13.4,V14.3,V15.3 | Backlog #14,20 |
| M-DATA-04 | `data-security/04-encryption-impl.md` | V11.1,V11.2,V11.4 | Backlog #20 |
| M-FILE-01 | `file-security/01-file-upload.md` | V5.1,V5.2,V5.3,V5.4 | Backlog #6,17,20 |
| M-INFRA-01 | `infrastructure/01-tls-config.md` | V12.1,V12.2,V13.1 | Backlog #3,13,20 |
| M-INFRA-02 | `infrastructure/02-security-headers.md` | V3.4,V12.1,V13.1 | Backlog #13,20 |
| M-INFRA-03 | `infrastructure/03-dependency-audit.md` | V13.1,V15.1,V15.2 | Backlog #14,20 |
| M-INPUT-01 | `input-validation/01-injection.md` | V1.2,V2.1,V4.2 | Backlog #20 |
| M-INPUT-02 | `input-validation/02-xss.md` | V1.2,V3.1,V3.2 | Backlog #13,20 |
| M-INPUT-03 | `input-validation/03-csrf.md` | V3.3,V7.1,V10.2 | Backlog #8,20 |
| M-INPUT-04 | `input-validation/04-parameter-tampering.md` | V2.1,V4.2,V8.2 | Backlog #2,20 |
| M-INPUT-05 | `input-validation/05-ssrf.md` | V5.4,V12.3,V13.2 | Backlog #6,20 |
| M-INPUT-06 | `input-validation/06-deserialization.md` | V5.5,V1.1,V2.1 | Backlog #17,20 |
| M-LOG-01 | `logging-monitoring/01-log-security.md` | V16.1,V16.2,V16.3,V16.4 | Backlog #5,12,19,20 |
| M-SESS-01 | `session-management/01-session-security.md` | V7.1,V7.2,V7.3 | Backlog #8,20 |
| M-SESS-02 | `session-management/02-token-lifecycle.md` | V7.2,V7.4,V9.1 | Backlog #1,4,11,20 |
| M-SESS-03 | `session-management/03-logout-security.md` | V7.5,V7.2,V16.2 | Backlog #12,20 |
| M-ADV-07 | `advanced-attacks/07-theme-css-injection.md` | V3.1,V14.1,V15.2 | Backlog #7,20 |
| M-API-06 | `api-security/06-api-boundary-pagination.md` | V4.4,V2.4,V13.3 | Backlog #16,20 |
| M-FILE-02 | `file-security/02-theme-resource-url-security.md` | V5.2,V3.4,V14.2 | Backlog #18,20 |
| M-LOG-02 | `logging-monitoring/02-error-response-leakage.md` | V16.4,V1.3,V4.3 | Backlog #15,20 |

---

## 4. Backlog 任务定义（#1-#20）

### P0
1. Token 类型混淆与降级链路回归
2. System/Admin 端点全量越权矩阵
3. gRPC 认证模式回归（none/api_key/mtls）
4. OIDC state/code/replay 专项
5. Keycloak 入站事件伪造与时序攻击
6. Webhook/Action SSRF 深度绕过
7. Theme custom_css 注入风控
8. Portal 会话安全与 CSRF 边界

### P1
9. RBAC 缓存一致性与竞态
10. 租户切换与 token exchange 竞态
11. 密码重置链路安全
12. 认证事件与审计完整性
13. 安全响应头与缓存策略
14. 依赖与构建产物供应链校验

### P2
15. 错误响应信息泄露差异测试
16. 批量 API 边界与分页滥用
17. 文件相关输入回归
18. Theme 外链资源与品牌 URL 安全
19. 登录检测规避与告警噪声对抗
20. 发布前安全回归最小集合

---

## 5. Backlog 到测试文档分解

| Backlog任务 | 对应矩阵测试文档 |
|---|---|
| #1 | `advanced-attacks/04-oidc-advanced.md`、`authentication/02-token-security.md`、`business-logic/01-workflow-abuse.md`、`session-management/02-token-lifecycle.md` |
| #2 | `api-security/01-rest-api.md`、`authorization/01-tenant-isolation.md`、`authorization/02-rbac-bypass.md`、`authorization/03-privilege-escalation.md`、`authorization/04-resource-access.md`、`authorization/05-system-config-authz.md`、`business-logic/03-admin-operational-endpoint-abuse.md`、`input-validation/04-parameter-tampering.md` |
| #3 | `advanced-attacks/02-grpc-security.md`、`api-security/02-grpc-api.md`、`api-security/05-rate-limit-bypass-hardening.md`、`infrastructure/01-tls-config.md` |
| #4 | `advanced-attacks/04-oidc-advanced.md`、`authentication/01-oidc-security.md`、`authentication/05-idp-security.md`、`session-management/02-token-lifecycle.md` |
| #5 | `advanced-attacks/05-webhook-forgery.md`、`logging-monitoring/01-log-security.md` |
| #6 | `file-security/01-file-upload.md`、`input-validation/05-ssrf.md` |
| #7 | `advanced-attacks/07-theme-css-injection.md` |
| #8 | `input-validation/03-csrf.md`、`session-management/01-session-security.md` |
| #9 | `authorization/02-rbac-bypass.md`、`authorization/03-privilege-escalation.md`、`business-logic/02-race-conditions.md` |
| #10 | `business-logic/01-workflow-abuse.md`、`business-logic/02-race-conditions.md` |
| #11 | `authentication/04-password-security.md`、`business-logic/02-race-conditions.md`、`session-management/02-token-lifecycle.md` |
| #12 | `advanced-attacks/03-detection-evasion.md`、`business-logic/03-admin-operational-endpoint-abuse.md`、`logging-monitoring/01-log-security.md`、`session-management/03-logout-security.md` |
| #13 | `api-security/04-cors-headers.md`、`infrastructure/01-tls-config.md`、`infrastructure/02-security-headers.md`、`input-validation/02-xss.md` |
| #14 | `advanced-attacks/01-supply-chain-security.md`、`data-security/03-secrets-management.md`、`infrastructure/03-dependency-audit.md` |
| #15 | `logging-monitoring/02-error-response-leakage.md` |
| #16 | `api-security/06-api-boundary-pagination.md` |
| #17 | `file-security/01-file-upload.md`、`input-validation/06-deserialization.md` |
| #18 | `file-security/02-theme-resource-url-security.md` |
| #19 | `advanced-attacks/03-detection-evasion.md`、`api-security/05-rate-limit-bypass-hardening.md`、`logging-monitoring/01-log-security.md` |
| #20 | `advanced-attacks/01-supply-chain-security.md`、`advanced-attacks/02-grpc-security.md`、`advanced-attacks/05-webhook-forgery.md`、`advanced-attacks/06-http-smuggling.md`、`api-security/01-rest-api.md`、`api-security/02-grpc-api.md`、`api-security/03-rate-limiting.md`、`api-security/04-cors-headers.md`、`api-security/05-rate-limit-bypass-hardening.md`、`authentication/01-oidc-security.md`、`authentication/02-token-security.md`、`authentication/03-mfa-security.md`、`authentication/04-password-security.md`、`authentication/05-idp-security.md`、`authorization/01-tenant-isolation.md`、`authorization/02-rbac-bypass.md`、`authorization/03-privilege-escalation.md`、`authorization/04-resource-access.md`、`authorization/05-system-config-authz.md`、`business-logic/01-workflow-abuse.md`、`business-logic/03-admin-operational-endpoint-abuse.md`、`data-security/01-sensitive-data.md`、`data-security/02-encryption.md`、`data-security/03-secrets-management.md`、`data-security/04-encryption-impl.md`、`file-security/01-file-upload.md`、`infrastructure/01-tls-config.md`、`infrastructure/02-security-headers.md`、`infrastructure/03-dependency-audit.md`、`input-validation/01-injection.md`、`input-validation/02-xss.md`、`input-validation/03-csrf.md`、`input-validation/04-parameter-tampering.md`、`input-validation/05-ssrf.md`、`input-validation/06-deserialization.md`、`logging-monitoring/01-log-security.md`、`session-management/01-session-security.md`、`session-management/02-token-lifecycle.md`、`session-management/03-logout-security.md`、`advanced-attacks/07-theme-css-injection.md`、`api-security/06-api-boundary-pagination.md`、`file-security/02-theme-resource-url-security.md`、`logging-monitoring/02-error-response-leakage.md` |

---

## 6. 回归执行规则

1. 每次版本发布前，必须执行所有包含 Backlog #1-#8 的文档。
2. 高风险域文档（V8/V9/V10/V13/V16）每周至少一次全量回归。
3. 安全缺陷修复后，必须重跑同矩阵ID及其关联 Backlog 任务。
4. 测试报告需记录矩阵ID、任务号、环境、证据链接、结果、风险等级。

---

## 7. 回归记录模板

### 基本信息
- 回归批次ID:
- 执行日期:
- 执行人:
- 环境: dev / staging / prod-like
- 版本/提交:

### 执行项
| 矩阵ID | 文档路径 | Backlog任务号 | 结果(pass/fail) | 证据链接 |
|---|---|---|---|---|
|  |  |  |  |  |

### 漏洞与异常
- 漏洞编号:
- 风险等级:
- 复现步骤摘要:
- 影响范围:
- 修复建议:

### 结论
- 本批次是否达到发布门槛: 是 / 否
- 阻断项:
- 后续动作:
