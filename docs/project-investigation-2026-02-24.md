# Auth9 Project Investigation Report (2026-02-24)

> Scope requested: in-depth analysis across six dimensions — functional completeness, business process rationality, system security, architectural advancement, performance optimization, and technical debt — plus horizontal industry benchmarking.

---

## A. English Version

## 1) Executive Summary

Auth9 is a strong self-hosted IAM platform with a clear architecture direction (headless Keycloak + Rust business core + React admin portal), broad feature coverage, and unusually mature testing/document governance for an early-stage product.

Current strengths are most evident in:
- Domain-oriented architecture and policy-first authorization design
- Wide security/QA documentation matrix and strong automation culture
- Multi-tenant + RBAC/ABAC + provisioning + action-engine capability breadth

Current primary risks/opportunities are:
- Build/toolchain portability (notably `protoc` requirement for `auth9-core`)
- Documentation consistency drift (cross-file numeric mismatches)
- Need for externally published performance benchmarks/SLO evidence

### Overall Assessment (this report)
- Functional completeness: **9.0/10**
- Business process rationality: **8.8/10**
- System security: **9.1/10**
- Architectural advancement: **9.3/10**
- Performance optimization: **8.5/10**
- Technical debt management: **8.6/10**
- **Composite score: 8.9/10 (Excellent)**

---

## 2) Evidence Baseline (Repository Reality Check)

### 2.1 Codebase scale snapshot (from local repository scan)
- `auth9-core/src`: **176 Rust files**, about **75,968 LOC**
- `auth9-core/src/domains`: **89 Rust files**, about **37,481 LOC**
- `auth9-portal/app + tests`: **158 TS/TSX files**, about **50,664 LOC**
- `auth9-portal/app/routes`: **50 route files**
- `auth9-core/migrations`: **32 SQL migrations**

### 2.2 Testing and quality assets (from docs and commands)
- Rust test annotations in `auth9-core/src + tests`: about **2,376** (`#[test]` / `#[tokio::test]` occurrences)
- `docs/security`: **48 markdown docs** (README total scenario count: **202**)
- `docs/qa`: **96 markdown docs** in directory; QA README aggregate table currently reports **94 docs / 444 scenarios** (consistency gap to be governed)

### 2.3 Baseline command execution results (before report change)
- `cd auth9-core && cargo test`: **failed** due to missing `protoc` in environment (`Could not find protoc`)
- `cd auth9-portal && npm run lint`: **passed**
- `cd auth9-portal && npm run test`: **fails in current baseline** (15 failures in tenant-related tests; unrelated to this report document change)

---

## 3) Six-Dimension Deep Evaluation

## 3.1 Functional Completeness — 9.0/10

### Findings
Auth9 already covers the core IAM capability set expected for modern B2B SaaS use cases:
- Multi-tenant tenant/user/service management
- OIDC + social/enterprise identity providers
- Dynamic RBAC and ABAC policy management
- Token exchange pattern
- WebAuthn/Passkeys
- SCIM provisioning suite
- Audit/security alerts, webhook integration, and action engine extension

### Evidence
- Feature set in `/README.md`
- Architecture and module mapping in `/docs/architecture.md`
- QA module coverage in `/docs/qa/README.md`
- Security-domain coverage in `/docs/security/README.md`

### Gaps
- Some strategic platform capabilities still rely on roadmap-level execution rather than fully closed operational loops (e.g., consistency across action/event integrations and documentation parity).

---

## 3.2 Business Process Rationality — 8.8/10

### Findings
Business process design is mostly rational and implementation-friendly:
- Clear split: protocol responsibilities delegated to Keycloak, business control in `auth9-core`
- Operations are documented through executable QA/security documents and governance scripts
- Strong fit for teams preferring self-hosting and cost control over pure SaaS convenience

### Strengths
- “Documents as executable specifications” workflow in root README
- Visible governance hooks (`scripts/qa-doc-lint.sh`, weekly governance script)

### Risks
- Process quality depends heavily on continued discipline in cross-document synchronization (observed stats drift between README-level aggregates)

---

## 3.3 System Security — 9.1/10

### Findings
Security posture is substantially above average for a self-hosted IAM project at this maturity:
- 48 security docs with broad ASVS-oriented categories
- Central policy enforcement architecture (`auth9-core/src/policy/mod.rs`)
- Security controls in action engine (allowlist, private IP blocking, timeout, request limits)
- Token/session/cache controls and security headers/middleware coverage

### Evidence
- `/docs/security/README.md`
- `auth9-core/src/policy/mod.rs`
- `auth9-core/src/domains/integration/service/action_engine.rs`
- `auth9-core/src/cache/mod.rs`

### Risks
- Security quality appears process-strong; production-grade confidence still benefits from continuously published real-world security test execution and regression trend evidence.

---

## 3.4 Architectural Advancement — 9.3/10

### Findings
Architecture is a major differentiator:
- Rust core + domain modularization + policy-first authorization
- Dual API style (REST + gRPC)
- Clear system boundary between identity protocol provider and business domain logic
- Strong testing architecture with mock-first design and no external dependencies for most unit tests

### Evidence
- `/docs/architecture.md` domain decomposition and deployment model
- Root README AI-native SDLC and skill system
- Policy/action/caching code structure in `auth9-core/src/*`

### Caveat
- Advanced architecture also increases integration surface area; operational excellence and observability discipline must keep pace.

---

## 3.5 Performance Optimization — 8.5/10

### Findings
Performance-oriented design choices are evident:
- Rust backend stack (axum/tonic/sqlx)
- Redis cache layer with typed operations and TTL strategy
- Token exchange model helps control token payload inflation
- Metrics instrumentation present in cache/action-engine paths

### Evidence
- `/docs/architecture.md`
- `auth9-core/src/cache/mod.rs`
- `auth9-core/src/domains/integration/service/action_engine.rs`

### Gaps
- Public, reproducible benchmark pack/SLO dashboard is not yet presented in core docs as a primary artifact.

---

## 3.6 Technical Debt — 8.6/10

### Findings
Debt management process exists and is explicit, but execution consistency can improve:
- Dedicated debt register (`docs/debt/README.md`) and status taxonomy
- Governance scripts in QA docs indicate preventive process thinking

### Observed debt signals
- Toolchain dependency friction (`protoc`) impacts out-of-box build reproducibility
- Documentation metric mismatches across top-level docs (e.g., QA totals)
- Current frontend baseline test failures in tenant-related suites indicate active stabilization work

### Recommendation priority
1. **P0**: Improve baseline reproducibility (tooling preflight checks for `protoc`, dev setup automation)
2. **P1**: Unify single source of truth for QA/Security counters
3. **P1**: Publish standard performance benchmark pack in docs

---

## 4) Horizontal Industry Benchmark (Peer Comparison)

Competitors compared: **Keycloak, Auth0, Ory (Kratos/Keto/Hydra stack), ZITADEL, SuperTokens**.

## 4.1 Positioning summary
- **Auth9 vs Keycloak**: More modern custom business layer and governance workflow; Keycloak remains stronger in ecosystem maturity/history.
- **Auth9 vs Auth0**: Better self-hosting control/cost profile; Auth0 stronger in turnkey global SaaS operations and commercial support.
- **Auth9 vs Ory/ZITADEL**: Auth9 is pragmatic and integrated; Ory/ZITADEL often stronger in specialized policy/identity-cloud product maturity and global product footprint.
- **Auth9 vs SuperTokens**: Auth9 is broader in enterprise IAM scope; SuperTokens is simpler/lighter for auth-only scenarios.

## 4.2 Capability matrix (qualitative)

| Dimension | Auth9 | Keycloak | Auth0 | Ory | ZITADEL | SuperTokens |
|---|---|---|---|---|---|---|
| Self-host friendly | High | High | Low | High | High | High |
| Multi-tenant IAM breadth | High | Medium-High | High | Medium-High | High | Medium |
| Built-in business extensibility | High (Action Engine + SDK) | Medium | High | Medium-High | Medium-High | Medium |
| Security engineering transparency | High (docs-heavy) | Medium | Medium | Medium-High | Medium-High | Medium |
| Out-of-box enterprise ecosystem maturity | Medium | High | Very High | High | High | Medium |
| Cost-control potential (self-host) | High | High | Low | High | High | High |

## 4.3 Strategic conclusion
Auth9 is best positioned for teams that need:
1. Self-hosted IAM with strong customization
2. Cost-efficient control over tenant-aware auth flows
3. Engineering teams comfortable owning operational excellence

For organizations prioritizing turnkey global SaaS operations and managed compliance evidence over customization depth, commercial platforms still keep an edge.

---

## 5) High-Impact Improvement Roadmap

### 0-30 days (Stabilization)
- Add environment preflight checks for `protoc` and critical dependencies
- Resolve/triage current frontend tenant-suite failures and publish status dashboard
- Normalize QA/Security counters into a machine-generated summary source

### 30-90 days (Competitive hardening)
- Publish repeatable benchmark suite (latency, throughput, p95/p99)
- Expand architecture decision records for key security/performance trade-offs
- Strengthen release quality gates linking doc-governance + test-governance + benchmark-governance

### 90+ days (Differentiation)
- Productize benchmark and security evidence as external trust artifacts
- Deepen ecosystem integrations and migration tooling against incumbent IAM stacks

---

## B. 中文版本

## 1）执行摘要

Auth9 已具备较强的企业级自托管 IAM 能力，架构方向清晰（Headless Keycloak + Rust 业务核心 + React 管理端），测试与文档治理体系在同阶段项目中表现突出。

当前优势主要在：
- 领域化架构与 policy-first 授权模型
- 覆盖面广的安全/QA 文档矩阵与自动化治理
- 多租户 + RBAC/ABAC + SCIM + Action Engine 的能力完整度

当前主要风险/机会：
- 工具链可移植性（`auth9-core` 对 `protoc` 的环境依赖）
- 文档统计数据存在跨文件漂移
- 性能指标缺少对外可复现实证（benchmark/SLO）

### 本报告综合评分
- 功能完整性：**9.0/10**
- 业务流程合理性：**8.8/10**
- 系统安全性：**9.1/10**
- 架构先进性：**9.3/10**
- 性能优化：**8.5/10**
- 技术负债：**8.6/10**
- **综合：8.9/10（优秀）**

---

## 2）证据基线（仓库实测）

### 2.1 代码规模快照
- `auth9-core/src`：**176 个 Rust 文件**，约 **75,968 行**
- `auth9-core/src/domains`：**89 个 Rust 文件**，约 **37,481 行**
- `auth9-portal/app + tests`：**158 个 TS/TSX 文件**，约 **50,664 行**
- `auth9-portal/app/routes`：**50 个路由文件**
- `auth9-core/migrations`：**32 个 SQL 迁移文件**

### 2.2 质量资产
- `auth9-core/src + tests` 中测试注解约 **2,376** 处
- `docs/security`：**48 个文档**（README 汇总 **202 场景**）
- `docs/qa`：目录中 **96 个文档**；但 QA README 当前汇总为 **94 文档 / 444 场景**（存在一致性治理空间）

### 2.3 基线命令结果（改动前）
- `cd auth9-core && cargo test`：**失败**（环境缺少 `protoc`）
- `cd auth9-portal && npm run lint`：**通过**
- `cd auth9-portal && npm run test`：**当前基线失败**（tenant 相关用例存在 15 个失败，非本次文档改动引入）

---

## 3）六维度深度评估

## 3.1 功能完整性（9.0/10）

Auth9 已覆盖现代 B2B IAM 的核心能力组合：多租户、OIDC/社交与企业身份源、RBAC/ABAC、Token Exchange、Passkeys、SCIM、审计与告警、Webhook、Action Engine。

证据来源：
- `/README.md`
- `/docs/architecture.md`
- `/docs/qa/README.md`
- `/docs/security/README.md`

改进点：
- 部分能力的“工程闭环成熟度”仍需持续收敛（能力实现与文档/运维证据的一致化）。

---

## 3.2 业务流程合理性（8.8/10）

流程设计总体合理：协议能力交给 Keycloak，业务控制由 `auth9-core` 统一承载；QA/Security 文档可执行化，且有治理脚本支持。

优势：
- 根 README 中“文档即可执行规范”设计
- `scripts/qa-doc-lint.sh` 与周期治理脚本形成制度化抓手

风险：
- 流程质量高度依赖文档同步纪律；目前已观察到统计口径漂移。

---

## 3.3 系统安全性（9.1/10）

安全建设在同类自托管项目中较强：
- 48 个安全文档覆盖广泛 ASVS 类别
- 中央策略引擎（`auth9-core/src/policy/mod.rs`）
- Action Engine 具备 allowlist / 私网拦截 / 超时 / 请求上限等防护
- 缓存层与中间件具备安全控制能力

证据来源：
- `/docs/security/README.md`
- `auth9-core/src/policy/mod.rs`
- `auth9-core/src/domains/integration/service/action_engine.rs`
- `auth9-core/src/cache/mod.rs`

改进点：
- 建议持续输出真实环境下安全回归执行趋势与结果证据，增强外部可信度。

---

## 3.4 架构先进性（9.3/10）

架构是 Auth9 核心竞争力之一：
- Rust 核心 + 领域分层 + policy-first 授权
- REST + gRPC 双接口风格
- 协议层与业务层边界清晰
- 单测零外部依赖导向，利于高频迭代

证据来源：
- `/docs/architecture.md`
- 根 README 的 AI-native SDLC 与技能体系
- `auth9-core/src/*` 结构实现

注意事项：
- 架构越先进，集成面越大，需要同等强度的可观测性与运维规范配套。

---

## 3.5 性能优化（8.5/10）

已体现明显性能导向：
- Rust（axum/tonic/sqlx）技术栈
- Redis 缓存抽象与 TTL 策略
- Token Exchange 减少 token 负担
- 缓存与 Action Engine 路径具备 metrics 埋点

证据来源：
- `/docs/architecture.md`
- `auth9-core/src/cache/mod.rs`
- `auth9-core/src/domains/integration/service/action_engine.rs`

改进点：
- 建议将可复现实测 benchmark/SLO 作为核心文档工件对外发布。

---

## 3.6 技术负债（8.6/10）

技术负债管理机制已建立，但执行一致性仍可增强：
- `docs/debt/README.md` 提供状态与优先级框架
- QA 治理脚本体现了“预防式流程”思路

当前债务信号：
- `protoc` 依赖影响新环境开箱成功率
- 文档统计口径跨文件不一致
- 前端 tenant 相关用例在当前基线存在失败，说明该区域仍在稳定化中

建议优先级：
1. **P0**：完善开发环境 preflight（含 `protoc`）
2. **P1**：建立 QA/Security 统计单一事实源
3. **P1**：补齐标准性能 benchmark 文档与流程

---

## 4）行业横向基准对比（Horizontal Benchmark）

对比对象：**Keycloak、Auth0、Ory、ZITADEL、SuperTokens**。

### 4.1 定位结论
- **对 Keycloak**：Auth9 在业务层现代化与治理流程上更激进；Keycloak 在生态成熟度与历史沉淀上更强。
- **对 Auth0**：Auth9 在自托管与成本可控性上占优；Auth0 在托管化运营与商业支持上更强。
- **对 Ory/ZITADEL**：Auth9 偏务实一体化；Ory/ZITADEL 在产品化深度和全球化商业成熟度上通常更领先。
- **对 SuperTokens**：Auth9 企业 IAM 覆盖面更广；SuperTokens 在轻量认证场景更简洁。

### 4.2 能力矩阵（定性）

| 维度 | Auth9 | Keycloak | Auth0 | Ory | ZITADEL | SuperTokens |
|---|---|---|---|---|---|---|
| 自托管友好度 | 高 | 高 | 低 | 高 | 高 | 高 |
| 多租户 IAM 覆盖 | 高 | 中高 | 高 | 中高 | 高 | 中 |
| 业务扩展能力 | 高（Action+SDK） | 中 | 高 | 中高 | 中高 | 中 |
| 安全工程透明度 | 高（文档体系强） | 中 | 中 | 中高 | 中高 | 中 |
| 企业生态成熟度 | 中 | 高 | 很高 | 高 | 高 | 中 |
| 成本可控潜力 | 高 | 高 | 低 | 高 | 高 | 高 |

### 4.3 战略判断
Auth9 的最佳适配客户是：
1. 需要自托管且可深度定制 IAM 的技术团队
2. 关注多租户业务控制和成本效率的 B2B SaaS 团队
3. 能够承担工程化运维责任、重视可持续治理的组织

若企业首要诉求是“全球托管化能力 + 商业级托底支持”，商业 SaaS 平台仍有优势。

---

## 5）高优先级改进路线图

### 0-30 天（稳定性）
- 增加 `protoc` 等关键依赖 preflight 检查
- 梳理并稳定前端 tenant 测试失败项，建立可视化状态追踪
- 将 QA/Security 统计改为机器生成汇总，避免手工漂移

### 30-90 天（竞争力加固）
- 发布标准化 benchmark 套件（延迟、吞吐、p95/p99）
- 对关键安全/性能取舍补齐 ADR
- 将文档治理、测试治理、性能治理纳入统一发布门禁

### 90 天+（差异化放大）
- 将 benchmark 与安全回归证据产品化为外部可信资产
- 完善与主流 IAM 的迁移工具链与集成生态

