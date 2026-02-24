# Auth9 Project Deep Investigation Report (2026-02-24)

> Scope: Repository-based static investigation (code, docs, scripts, test assets, and baseline local command execution).
>
> Language: This report provides both **English** and **Chinese** versions.

---

## English Version

## 1) Executive Summary

Auth9 is already a **high-maturity open-source IAM platform** with strong architectural decisions (Rust core + policy-first authorization + headless Keycloak model), rich test/documentation assets, and a disciplined AI-native engineering workflow.

At the same time, current baseline verification shows several engineering gaps in environment/toolchain readiness and integration consistency (build/lint/test blockers), which now represent the highest-leverage improvement area.

### 1.1 Overall Evaluation (Six Dimensions)

| Dimension | Score (10) | Assessment |
|---|---:|---|
| Functional completeness | **8.8** | Core IAM capabilities are broad and practical; a few feature chains are still incomplete (for example: Action trigger coverage continuity in docs/status). |
| Business process rationality | **9.1** | End-to-end closed loop (plan → docs → test → ticket → fix → deploy) is unusually systematic and strong. |
| System security | **9.0** | Security posture is strong: policy-first model, sandbox controls, dedicated security test docs; needs tighter doc/code synchronization for security validation artifacts. |
| Architectural advancement | **9.3** | Domainized Rust backend, clear layer boundaries, scalable deployment model, API+gRPC coexistence. |
| Performance optimization | **8.6** | Good performance engineering direction and benchmark tooling exist; needs regular benchmark baselines and CI observability linkage. |
| Technical debt management | **8.4** | Debt awareness/process exists, but practical debt registry is under-representing active issues seen in baseline checks. |

**Composite score: 8.87 / 10 (Excellent).**

---

## 2) Investigation Method & Evidence Baseline

### 2.1 Repository Evidence Highlights

- Backend codebase: `auth9-core/src` + `auth9-core/tests`
- Frontend codebase: `auth9-portal/app` + `auth9-portal/tests`
- Security/QA docs: `docs/security`, `docs/qa`, `docs/uiux`
- Architecture and implementation docs: `README.md`, `docs/architecture.md`, `docs/actions-implementation-status.md`, `docs/action-engine-security.md`
- Performance tooling: `scripts/benchmark.sh`

### 2.2 Quantitative Snapshot (current repository)

- Rust files (`auth9-core/src` + tests): **220**, approx **103,340 LOC**
- Portal TS/TSX files (`auth9-portal/app` + tests): **158**, approx **50,664 LOC**
- Portal route files: **50**
- DB migrations: **32**
- QA docs: **96**
- Security docs: **48**
- UI/UX docs: **12**

### 2.3 Baseline Command Outcomes (before any changes)

- `cd auth9-core && cargo test`
  - **Failed** in environment because `protoc` is missing (build prerequisite issue).
- `cd auth9-portal && npm run lint`
  - **Failed** due ESLint/AJV runtime error (`defaultMeta` undefined).
- `cd auth9-portal && npm run build`
  - **Failed** due unresolved `@auth9/core` package entry in current workspace runtime.
- `cd auth9-portal && npm run test -- --run`
  - **Partially failed** (6 files failed), including unresolved `@auth9/core` and some selector/assertion instability in tenant route tests.

These failures are not catastrophic architecture flaws, but they are high-priority engineering quality gates.

---

## 3) Six-Dimension Deep Analysis

## 3.1 Functional Completeness

### Strengths

1. **Core IAM feature set is extensive**
   - Multi-tenant isolation, RBAC, token exchange, audit logs, invitation workflow, branding, session management, passkeys, social login, action engine.
2. **Dual interface model (REST + gRPC)**
   - Supports both human admin workflows and service-to-service integrations.
3. **Strong implementation depth in Action subsystem**
   - CRUD, batch operations, testing endpoint, logs/stats, and SDK integration are all documented as implemented.

### Gaps / Risks

1. **Feature-chain closure consistency is not fully unified across docs/code/tests**
   - Some docs indicate incomplete trigger integration paths (e.g., PostEmailVerification path continuity still pending in status docs).
2. **SDK/workspace linkage fragility**
   - Frontend build currently blocked by package entry resolution for `@auth9/core` in this environment.

### Recommendations

- **P0**: Stabilize monorepo package resolution contracts (workspace + exports + build artifacts).
- **P1**: Introduce “feature closure checklist” per feature: API + policy + docs + tests + SDK integration all green.

---

## 3.2 Business Process Rationality

### Strengths

1. **AI-native SDLC is process-complete**
   - Explicit loop across planning, QA/security docs generation, automated execution, ticketing, auto-fix, test alignment, deployment.
2. **Documentation as executable governance asset**
   - Large QA/security documentation corpus is treated as active verification assets, not passive documents.
3. **Human-AI responsibility boundary is clear**
   - Human focuses on intent, architecture, acceptance; AI handles repetitive implementation/testing loops.

### Gaps / Risks

1. **Process quality depends on doc freshness**
   - If code and docs drift, “automation confidence” can become overestimated.
2. **Debt register under-captures active operational quality issues**
   - Current debt tracker does not yet reflect all baseline blockers seen in build/lint/test.

### Recommendations

- **P0**: Add mandatory “doc-code consistency review gate” in PR workflow for critical modules.
- **P1**: Expand `docs/debt` records to include active CI/toolchain and workspace integration risks.

---

## 3.3 System Security

### Strengths

1. **Policy-first centralized authorization**
   - `auth9-core/src/policy/mod.rs` defines centralized `PolicyAction` and scoped enforcement logic.
2. **Action Engine sandbox design is security-aware**
   - Timeout, heap limits, SSRF controls, request quotas, domain allowlist, process/filesystem restriction model documented.
3. **Security testing culture is visible at documentation layer**
   - 48 dedicated security docs indicate broad scenario coverage intent.

### Gaps / Risks

1. **Security doc references can drift from actual test artifacts**
   - Some referenced test files/paths in docs appear inconsistent with current test tree, reducing audit confidence.
2. **Toolchain blockers can delay security regression cycles**
   - Build instability (e.g., protoc/env gaps) impacts frequency of reliable security verification.

### Recommendations

- **P0**: Introduce automated “security-doc path existence check” in CI.
- **P1**: Ensure every critical security claim has a traceable test case reference that resolves in current repo.

---

## 3.4 Architectural Advancement

### Strengths

1. **Clear layered architecture with domain decomposition**
   - `domains/` segmentation is mature (authorization, identity, integration, platform, provisioning, security_observability, tenant_access).
2. **Modern backend technology and type-safe stack**
   - Rust + axum + tonic + sqlx + tracing; suitable for high-concurrency, security-sensitive IAM workloads.
3. **Policy-centric access control aligns with long-term maintainability**
   - Authorization logic is extracted from handlers into centralized policy semantics.

### Gaps / Risks

1. **Documentation heterogeneity across iterations**
   - Some docs reflect old assumptions/paths and need convergence.
2. **Cross-package integration complexity rising**
   - As SDK/portal/core evolve, contract governance becomes a first-class architectural concern.

### Recommendations

- **P0**: Define and enforce versioned internal interface contracts (core ↔ SDK ↔ portal).
- **P1**: Add architecture decision records (ADRs) for key domain and integration boundaries.

---

## 3.5 Performance Optimization

### Strengths

1. **Performance engineering is explicitly operationalized**
   - `scripts/benchmark.sh` provides repeatable benchmark entry points and latency/QPS summary output.
2. **Rust runtime choice gives strong potential ceiling**
   - Suitable for low-latency API and high concurrency in identity workloads.

### Gaps / Risks

1. **No continuously published benchmark baseline in CI artifacts**
   - Performance trend visibility across commits is limited.
2. **Benchmark workflow still mostly manual**
   - Needs stronger automation and SLO coupling.

### Recommendations

- **P1**: Add scheduled performance runs (e.g., nightly) with history retention.
- **P1**: Track P95/P99 latency SLOs for key auth endpoints (`/health`, `/ready`, token-related APIs).

---

## 3.6 Technical Debt

### Strengths

1. **Debt governance structure exists**
   - Dedicated `docs/debt` framework with status, priority, and review cadence.
2. **Project has explicit quality mindset**
   - Strong testing/documentation infrastructure and correction loops reduce uncontrolled debt growth.

### Current Debt Signals

1. **Environment and dependency/tooling debt**
   - `protoc` prerequisite not self-guarded early enough for contributor experience.
2. **Frontend toolchain debt**
   - ESLint runtime error and package entry resolution issues block quality gates.
3. **Test robustness debt (frontend integration)**
   - Several tests are brittle (duplicate-text selectors, unstable assertions).

### Recommendations

- **P0**: Treat current baseline blockers as tracked debt items with owners and due dates.
- **P1**: Improve test selectors toward role/aria/data-testid strategies to reduce flaky/ambiguous assertions.

---

## 4) Horizontal Industry Benchmark (Auth9 vs Similar IAM Products)

> Note: This section is a strategic engineering benchmark based on repository evidence + common industry positioning patterns, focusing on architecture and delivery capability rather than marketing claims.

## 4.1 Comparison Matrix

| Dimension | Auth9 | Keycloak | Auth0 | ZITADEL | Ory (Kratos/Hydra stack) | SuperTokens |
|---|---|---|---|---|---|---|
| Deployment model | Self-hosted first | Self-hosted first | Managed SaaS first | SaaS + self-host options | Cloud + self-host components | OSS/self-host + managed options |
| Core IAM breadth | High (RBAC, token exchange, audit, action engine, passkeys) | Very high | Very high | High | High (modular) | Medium-High |
| Extensibility model | Action engine + Rust service extensibility | SPI/extensions | Actions/hooks/extensibility | Event/webhook based extensibility | Composable services/APIs | Recipe/plugin-like extension model |
| Architecture modernity | Rust core + policy-first + domainized modules | Mature Java monolith-ish platform core | Mature SaaS platform | Modern cloud-native posture | Strongly composable cloud-native identity stack | Developer-centric pragmatic architecture |
| Ops complexity | Medium-High | High | Low (for customer) | Medium | High (composition burden) | Low-Medium |
| Cost control potential | Very high (self-host) | Very high (self-host) | Lower at scale (SaaS MAU pricing) | Medium-High | Medium-High | Medium-High |
| Custom business logic fit | High | Medium-High | High (platform-limited by SaaS boundaries) | Medium-High | High | Medium |

## 4.2 Strategic Positioning of Auth9

Auth9’s best-fit zone is:

1. **Teams needing Auth0-like experience with self-host cost control**.
2. **Security-sensitive B2B SaaS requiring tenant-scoped authorization and custom policy logic**.
3. **Engineering-led organizations willing to trade moderate ops complexity for high customization and sovereignty**.

Not the best fit (yet):

- Teams that require “zero-ops managed identity” with minimal engineering investment.
- Organizations needing out-of-the-box global compliance/audit certifications bundled as a managed service from day one.

---

## 5) Priority Improvement Roadmap

## Next 30 days (P0)

1. Fix contributor and CI quality gates:
   - add toolchain preflight (protoc check + clear setup message)
   - resolve frontend lint runtime dependency conflict
   - fix SDK package entry/exports resolution for portal build path
2. Convert current blockers into formal debt tickets in `docs/debt`.

## Next 60–90 days (P1)

1. Establish cross-module contract tests (core ↔ SDK ↔ portal).
2. Add doc-to-code integrity checks for security and architecture references.
3. Stabilize flaky frontend integration tests.

## Next 90–180 days (P2)

1. CI-integrated performance trend dashboards and SLO enforcement.
2. Expand benchmark publication (repeatable comparative reports per release).
3. Continue reducing architecture/documentation divergence via ADRs and governance automation.

---

## 6) Final Conclusion

Auth9 is not a prototype-level IAM project; it is already an **engineering-serious platform** with strong architecture and process discipline. The main gap is no longer “can it do IAM,” but “how quickly can quality gates and integration contracts be hardened to enterprise reliability standards.”

If the current P0 integration/toolchain issues are resolved rapidly, Auth9 has a clear path to becoming a leading self-hosted IAM option in the “high-control, high-customization, cost-sensitive” segment.

---

## 中文版本

## 1）执行摘要

Auth9 已经是一个**成熟度较高的开源 IAM 平台**：在架构（Rust Core + Policy-First + Headless Keycloak）、测试/文档资产、以及 AI 原生研发流程方面都具备明显优势。

同时，本次基线检查也显示：当前最需要优先补强的是工程质量闸门（构建/静态检查/集成链路一致性），这些问题已成为影响交付确定性的主要短板。

### 1.1 六维综合评分

| 维度 | 评分（10分） | 结论 |
|---|---:|---|
| 功能完整性 | **8.8** | IAM 主干能力覆盖较全，少数功能链路仍有闭环缺口（如 Action 触发器链路在文档状态上仍有待统一）。 |
| 业务流程合理性 | **9.1** | “计划→文档→测试→票据→修复→部署”闭环完整，流程设计领先多数同类开源项目。 |
| 系统安全性 | **9.0** | 策略中心化、沙箱隔离、专项安全文档覆盖较好；需进一步提升文档与代码证据链一致性。 |
| 架构先进性 | **9.3** | 领域化拆分、层次清晰、REST+gRPC 并行、可扩展性强。 |
| 性能优化 | **8.6** | 已有性能基准脚本和优化意识；需把性能基线纳入持续集成与趋势观测。 |
| 技术负债 | **8.4** | 负债治理机制存在，但当前活跃问题（工具链/集成）尚未充分体现在负债台账中。 |

**综合得分：8.87 / 10（优秀）。**

---

## 2）调研方法与证据基线

### 2.1 证据来源

- 后端：`auth9-core/src`、`auth9-core/tests`
- 前端：`auth9-portal/app`、`auth9-portal/tests`
- 测试文档：`docs/qa`、`docs/security`、`docs/uiux`
- 架构与实现文档：`README.md`、`docs/architecture.md`、`docs/actions-implementation-status.md`、`docs/action-engine-security.md`
- 性能脚本：`scripts/benchmark.sh`

### 2.2 当前仓库量化快照

- Rust 文件（含测试）：**220**，约 **103,340 行**
- Portal TS/TSX 文件（含测试）：**158**，约 **50,664 行**
- Portal 路由文件：**50**
- 数据库迁移：**32**
- QA 文档：**96**
- 安全文档：**48**
- UI/UX 文档：**12**

### 2.3 基线命令结果（改动前）

- `cd auth9-core && cargo test`
  - **失败**：环境缺少 `protoc`（构建前置依赖问题）。
- `cd auth9-portal && npm run lint`
  - **失败**：ESLint/AJV 运行时异常（`defaultMeta` undefined）。
- `cd auth9-portal && npm run build`
  - **失败**：`@auth9/core` 入口解析失败（工作区包链路问题）。
- `cd auth9-portal && npm run test -- --run`
  - **部分失败**（6 个文件），包含 `@auth9/core` 解析问题和部分测试选择器不稳定问题。

这些问题不是“架构不可行”，但已经是当前最优先的工程质量闸门。

---

## 3）六维深度评估

## 3.1 功能完整性

### 优势

1. **IAM 核心能力覆盖广**
   - 多租户、RBAC、Token Exchange、审计日志、邀请流程、品牌配置、会话管理、Passkey、社交登录、Action 引擎等。
2. **接口形态完善（REST + gRPC）**
   - 同时支持管理端与服务间集成场景。
3. **Action 子系统实现深度较高**
   - CRUD、批量、测试执行、日志统计、SDK 集成均有明确实现痕迹。

### 缺口/风险

1. **功能闭环在“文档-代码-测试”间尚未完全统一**
   - 文档仍显示个别触发器链路未完全闭环。
2. **SDK/Portal 集成韧性不足**
   - 当前前端构建受 `@auth9/core` 入口解析阻塞。

### 建议

- **P0**：优先修复 monorepo 包解析契约（workspace/exports/build 产物一致性）。
- **P1**：为每个功能建立闭环清单：API + Policy + 文档 + 测试 + SDK 集成必须同时通过。

---

## 3.2 业务流程合理性

### 优势

1. **AI 原生研发闭环完整**
   - 从计划、文档生成、自动执行、票据化、自动修复到部署形成闭环。
2. **文档即治理资产**
   - QA/安全文档规模大，且承担执行与回归价值。
3. **人机分工边界清晰**
   - 人负责目标与架构判断，AI 执行高频实现与验证循环。

### 缺口/风险

1. **流程质量依赖文档新鲜度**
   - 文档漂移会削弱自动化结论可信度。
2. **负债台账对活跃工程问题覆盖不足**
   - 当前 build/lint/test 阻塞尚未充分沉淀为标准化负债项。

### 建议

- **P0**：关键模块 PR 增加“文档-代码一致性”强制检查点。
- **P1**：将当前工具链/集成问题纳入 `docs/debt` 并指定 owner 和截止时间。

---

## 3.3 系统安全性

### 优势

1. **Policy-First 授权中心化**
   - `auth9-core/src/policy/mod.rs` 统一定义 `PolicyAction` 与作用域校验。
2. **Action 引擎安全边界清晰**
   - 超时、堆限制、SSRF 防护、请求配额、域名白名单、进程/文件系统隔离策略完备。
3. **安全专项测试文化明显**
   - 48 份安全文档覆盖面较广。

### 缺口/风险

1. **安全文档引用与实际测试工件存在漂移风险**
   - 部分文档中的测试路径与当前目录结构不完全一致，影响可审计性。
2. **工具链不稳定会降低安全回归频率**
   - 构建依赖问题会拖慢安全验证节奏。

### 建议

- **P0**：在 CI 增加“安全文档引用路径有效性检查”。
- **P1**：关键安全声明必须绑定可解析到仓库现状的测试用例路径。

---

## 3.4 架构先进性

### 优势

1. **分层清晰 + 领域化拆分成熟**
   - `domains/` 已覆盖 authorization / identity / integration / platform / provisioning / security_observability / tenant_access。
2. **技术栈现代且契合 IAM 场景**
   - Rust + axum + tonic + sqlx + tracing 对高并发、强安全场景友好。
3. **授权逻辑从 handler 抽离到 policy**
   - 有利于长期可维护性与一致性。

### 缺口/风险

1. **文档迭代历史带来的异构性**
   - 存在旧假设与新结构并存现象。
2. **跨模块契约复杂度上升**
   - core/SDK/portal 并行演进后，契约治理必须工程化。

### 建议

- **P0**：建立并强制执行 core ↔ SDK ↔ portal 的版本化内部契约。
- **P1**：为关键边界补充 ADR（架构决策记录）。

---

## 3.5 性能优化

### 优势

1. **已有可执行基准脚本**
   - `scripts/benchmark.sh` 对 QPS/P99 具有直接观测价值。
2. **Rust 技术路线具备高性能上限**
   - 对认证链路的低延迟和高并发目标友好。

### 缺口/风险

1. **缺少 CI 连续性能基线发布**
   - 难以持续观察回归趋势。
2. **基准流程偏手工**
   - 与 SLO 的自动联动不足。

### 建议

- **P1**：引入定时性能回归（如 nightly）并保留历史结果。
- **P1**：对核心端点建立 P95/P99 的明确 SLO 与告警阈值。

---

## 3.6 技术负债

### 优势

1. **负债治理框架已存在**
   - `docs/debt` 具备状态、优先级、审查周期等机制。
2. **质量导向明确**
   - 测试与文档体系完善，有利于抑制无序负债扩张。

### 当前负债信号

1. **环境/工具链负债**
   - `protoc` 前置依赖缺少更前置的 fail-fast 提示与自动化检查。
2. **前端工程链路负债**
   - lint 运行时错误与包入口解析阻塞质量闸门。
3. **测试鲁棒性负债**
   - 部分前端集成测试选择器脆弱（重复文本匹配导致歧义）。

### 建议

- **P0**：将上述阻塞项转化为可追踪负债条目，明确责任人与完成时间。
- **P1**：测试选择器逐步迁移到 role/aria/data-testid 组合策略。

---

## 4）横向行业对标（Auth9 vs 同类 IAM）

> 说明：本节基于仓库证据与行业常见定位进行工程视角对标，重点比较“架构能力与交付模型”，非营销宣传对比。

## 4.1 对标矩阵

| 维度 | Auth9 | Keycloak | Auth0 | ZITADEL | Ory（Kratos/Hydra） | SuperTokens |
|---|---|---|---|---|---|---|
| 部署模型 | 自托管优先 | 自托管优先 | SaaS 优先 | SaaS+自托管 | 云+可组合自托管 | OSS/自托管+托管 |
| IAM 能力广度 | 高 | 很高 | 很高 | 高 | 高（模块化） | 中高 |
| 可扩展性模型 | Action 引擎 + Rust 服务扩展 | SPI 扩展 | Actions/Hook 扩展 | 事件/Webhook 扩展 | 组合式 API 扩展 | Recipe/插件式扩展 |
| 架构现代性 | Rust + Policy-First + 领域化 | 成熟稳健（Java） | 成熟 SaaS 平台 | 云原生现代化 | 组合式云原生 | 开发者友好务实 |
| 运维复杂度 | 中高 | 高 | 低（客户侧） | 中 | 高（组合成本） | 低中 |
| 成本可控性 | 很高（自托管） | 很高（自托管） | 中低（规模后成本高） | 中高 | 中高 | 中高 |
| 自定义业务逻辑匹配 | 高 | 中高 | 高（受 SaaS 边界约束） | 中高 | 高 | 中 |

## 4.2 Auth9 战略定位

Auth9 最有优势的目标区间：

1. 需要“接近 Auth0 体验 + 自托管成本可控”的团队。
2. 对租户级授权、定制策略、安全可控有较高要求的 B2B SaaS。
3. 愿意用适度运维复杂度换取高定制与数据主权的工程型组织。

当前不占优区间：

- 希望“零运维托管身份服务”的小团队。
- 需要开箱即用托管合规认证（并由供应商背书）的组织。

---

## 5）优先改进路线图

## 未来 30 天（P0）

1. 修复质量闸门阻塞：
   - 增加 toolchain preflight（`protoc` 检查 + 明确提示）
   - 解决 frontend lint 运行时依赖冲突
   - 修复 `@auth9/core` 入口/导出解析链路
2. 将当前阻塞项正式登记到 `docs/debt`。

## 未来 60–90 天（P1）

1. 建立 core ↔ SDK ↔ portal 契约测试。
2. 增加安全/架构文档引用完整性自动校验。
3. 提升前端集成测试稳定性（选择器与断言策略升级）。

## 未来 90–180 天（P2）

1. 将性能趋势纳入 CI 仪表盘与 SLO 体系。
2. 建立版本化行业对标输出（每个 release 一次）。
3. 通过 ADR 和治理自动化持续收敛架构/文档漂移。

---

## 6）最终结论

Auth9 已经跨过“能不能做 IAM”的阶段，进入“如何达到企业级交付确定性”的阶段。当前关键不在能力缺失，而在工程质量闸门与跨模块契约的稳定性建设。

只要尽快完成 P0 问题收敛，Auth9 在“高可控、高定制、成本敏感”的自托管 IAM 赛道具备明显竞争力。
