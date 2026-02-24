# Auth9

中文 | **[English](README.md)**

**一次实验：AI 原生的软件开发生命周期，能否像人类一样"打磨"软件？**

Auth9 同时代表了两样东西：一个自托管的身份认证和访问管理平台（一个 Auth0 的替代方案），以及一个真实的实验——证明 AI Agent 可以驱动完整的软件开发生命周期，从规划和实现到测试、修复和部署。这个项目中几乎所有的代码都由 AI 生成，几乎所有步骤都由 skills 驱动——从后端 Rust 服务到前端 React 组件，从测试用例到部署脚本。

> 方法论的详细阐述请见 **[博客文章](docs/blog-ai-native-sdlc-zh.md)**。

---

## 实验初衷

最初的目标并不是要构建一个身份认证平台。我想探究一个更根本的问题：**用 AI 原生的开发流程，真的能够打磨出合格的应用吗？**

我选择了 IAM（身份认证与访问管理）作为实验对象。这不是简单的增删改查：多租户数据隔离、OIDC/OAuth2 流程、Token Exchange、层级化 RBAC 权限、Webhook 签名验证、审计日志——这些环环相扣的复杂性意味着，一个错误决策会引发十几个隐蔽的 bug。安全不是锦上添花，而是系统存在的根本意义。

如果 AI 原生的开发流程能打磨出一个合格的 IAM 平台，那它对大多数应用都适用。

## 真正的挑战：可验证性

AI 编码工具确实让写代码更快了。但写代码从来不是真正的难点。难点在于**如何知道代码是否正确**——而且要足够快、足够自动化，不能让验证本身成为瓶颈。

AI 原生的开发流程不是要消除验证工作，而是要让验证**足够系统化、足够自动化**，跟上 AI 生成代码的速度。如果 AI 写代码快了 10 倍，但验证还是手动操作，那你只是制造了一个 10 倍大的 QA 积压。

## 测试左移

**测试没有消失，反而变得更加重要了。** 变化的是形式。

传统的自动化测试在代码库中依然存在——`cargo test`、Playwright、Vitest。全部由 AI 生成，全部必要。我们在代码级测试*之前*增加了一层：**QA 测试文档**。结构化的规格说明，描述要测什么、怎么测、如何在数据层验证正确性。AI 生成这些文档，人类审查和批准。然后 AI 执行测试——包括浏览器自动化、API 调用、数据库查询和 gRPC 验证。

人类的角色：审查每份生成的测试文档，检查完整性、边界情况，以及 AI 可能遗漏的安全考量；观察 agent 的自动测试，检查它的测试行为是否符合预期。AI 的角色：生成文档、执行测试、报告失败、修复力所能及的问题。

## 闭环流水线

整个流水线将 16 个 Agent Skills 串联起来，每个阶段的输出都是下一个阶段的输入：

```
人类 + AI ──► 规划功能
                  │
                  ▼
          ┌─ 生成 QA / 安全 / UIUX 测试文档
          │   (qa-doc-gen)
          ▼
          ┌─ 自动执行测试
          │   浏览器自动化、API 测试、
          │   数据库验证、gRPC 回归测试、
          │   性能基准测试
          ▼
          ┌─ 失败？创建结构化工单
          │   (docs/ticket/)
          ▼
          ┌─ AI 读取工单 → 验证问题 →
          │   修复代码 → 重置环境 →
          │   重新运行测试 → 关闭工单
          │   (ticket-fix)
          ▼
          ┌─ 定期审计文档质量
          │   (qa-doc-governance)
          ▼
          ┌─ 重构后对齐测试
          │   (align-tests, test-coverage)
          ▼
          ┌─ 部署到 Kubernetes
          │   (deploy-gh-k8s)
          └─────────────────────────
```

### Agent Skills

`.agents/skills/` 目录包含 16 个技能，覆盖开发生命周期的每个阶段：

| 阶段 | Skills | 职责 |
|------|--------|------|
| **规划** | `project-bootstrap` | 从零开始搭建新项目脚手架 |
| **编码** | `rust-conventions`, `keycloak-theme` | 编码规范、主题开发 |
| **测试文档** | `qa-doc-gen`, `qa-doc-governance` | 生成和管理测试文档 |
| **执行测试** | `qa-testing`, `e2e-testing`, `performance-testing`, `auth9-grpc-regression` | QA、端到端、压力测试、gRPC 测试 |
| **修复** | `ticket-fix`, `align-tests` | 自动修复工单、重构后对齐测试 |
| **覆盖率** | `test-coverage` | 确保所有层 >=90% 覆盖率 |
| **部署** | `deploy-gh-k8s` | GitHub Actions 门禁 → K8s 部署 → 健康检查 |
| **运维** | `ops`, `reset-local-env` | 日志、故障排查、环境重置 |

### 文档即可执行规格

`docs/` 目录不是被动的文档——它是机器可读的测试套件：

| 目录 | 文件数 | 用途 |
|------|--------|------|
| `docs/qa/` | 96 | 功能测试场景，包含逐步操作、预期结果和 SQL 验证查询 |
| `docs/security/` | 48 | 11 个类别的安全测试用例（API 安全、认证、注入、会话等） |
| `docs/uiux/` | 12 | UI/UX 测试用例，可见性优先的导航验证 |
| `docs/ticket/` | — | 活跃的缺陷工单，由 AI 创建和消费 |

### 自愈循环

`ticket-fix` skill 是 AI "打磨"软件的核心机制。测试失败后，系统创建结构化工单。AI 读取工单、复现问题、修复代码、重置环境、重新运行测试、关闭工单。

不是每个失败的测试都是 bug。该技能显式处理**误报**——当失败由测试流程缺陷而非代码缺陷导致时，它会更新 QA 文档以防止再次发生。每次失败都会让测试套件变得更好。

### 人类到底在做什么

这是人机协作，不是替代：

- **规划**：决定做什么、定义验收标准、选择架构权衡
- **审查**：审查测试文档和第一版代码。QA 执行和工单修复自主运行
- **纠偏**：误报的根因分析、治理修复决策
- **架构**：领域建模、数据流设计、安全边界

经过 20 轮迭代，AI 执行的测试仍然会产生工单——但比早期少得多，应用的细节也在每一轮中变得更加丰富。打磨循环跑得更快了，而且每一轮都有据可查。

**人类的核心价值在于定义"我们想做什么，不想做什么"**，并提供足够好的品味和判断。实际上人类专家的角色变得更加关键——我们需要真正的全栈工程师，不仅懂开发，还得熟悉基础设施、DevOps 和安全。

作为开发者，我一向推崇极限开发。作为techlead，我信任我的组员们，但我会尽可能的用包括测试驱动在内的敏捷开发方法论来做风险管理。所以，对于 AI，我的看法相当开放，我认为几乎所有用于软件开发的风险管理手段，尤其是极限编程实践，都可以用于对 Agent 进行管理。

## 数字说话

- **16** 个 Agent Skills，覆盖完整开发生命周期
- **156** 份测试文档（96 QA + 48 安全 + 12 UI/UX）
- **9** 个工具脚本，用于 token 生成、API 测试、gRPC 冒烟测试
- **~2,300** 行 skill 定义
- **1** 个人类

---

## IAM 平台

Auth9 是一个完整的身份认证平台——这套方法论构建和维护的产品。

### 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        客户端层                                    │
├─────────────────┬─────────────────────┬─────────────────────────┤
│  auth9-portal   │    业务服务          │      auth9-sdk          │
│ (React Router 7)│                     │      (可选)              │
└────────┬────────┴──────────┬──────────┴────────────┬────────────┘
         │ REST API          │ gRPC                   │ gRPC
         ▼                   ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       auth9-core (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │  REST API    │  │ gRPC Server  │  │  JWT Engine  │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└────────┬────────────────────┬────────────────────────────────────┘
         │                    │
    ┌────┴────┐          ┌────┴────┐
    │  TiDB   │          │  Redis  │
    │ (MySQL) │          │ (缓存)  │
    └─────────┘          └─────────┘
```

### 组件

| 组件 | 技术栈 | 说明 |
|------|--------|------|
| **auth9-core** | Rust (axum, tonic, sqlx) | 后端 API 和 gRPC 服务 |
| **auth9-portal** | React Router 7 + TypeScript + Vite | 管理后台 UI |
| **数据库** | TiDB（MySQL 兼容） | 租户、用户、RBAC 数据 |
| **缓存** | Redis | 会话、Token 缓存 |
| **认证引擎** | Keycloak | OIDC 提供者（可选） |

### 功能

- **多租户**：隔离的租户与自定义设置
- **SSO**：通过 OIDC 的单点登录
- **动态 RBAC**：角色、权限、继承
- **Token Exchange**：服务间认证
- **审计日志**：追踪所有管理操作
- **现代 UI**：基于 React Router 7 的设计系统
- **Action Engine**：事件驱动的自动化工作流（JavaScript/TypeScript）
- **TypeScript SDK**：官方 SDK，无缝集成
- **邀请系统**：基于邮件的用户入职和自动化工作流
- **品牌定制**：自定义 Logo、颜色、主题
- **邮件模板**：灵活的邮件模板系统，支持多语言
- **密码管理**：密码策略、重置、修改
- **会话管理**：查看和吊销活跃会话
- **WebAuthn/Passkey**：无密码认证
- **社交登录**：Google、GitHub、OIDC、SAML 支持
- **安全告警**：实时威胁检测
- **登录分析**：详细的登录统计和事件
- **Webhooks**：实时事件通知

## 快速开始

### 本地开发

```bash
# 启动依赖（TiDB, Redis, Keycloak）
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# 运行 auth9-core
cd auth9-core
cp .env.example .env
cargo run

# 运行 auth9-portal
cd auth9-portal
cp .env.example .env
npm install
npm run dev
```

### Docker 全栈

```bash
docker-compose up -d
```

- Portal: http://localhost:3000
- API: http://localhost:8080
- Keycloak: http://localhost:8081

## API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/tenants` | GET, POST | 列出/创建租户 |
| `/api/v1/users` | GET, POST | 列出/创建用户 |
| `/api/v1/services` | GET, POST | 列出/注册服务 |
| `/api/v1/roles` | GET, POST | 列出/创建角色 |
| `/api/v1/rbac/assign` | POST | 分配角色给用户 |
| `/api/v1/audit-logs` | GET | 查询审计日志 |

## gRPC 服务

```protobuf
service TokenExchange {
  rpc ExchangeToken(ExchangeTokenRequest) returns (ExchangeTokenResponse);
  rpc ValidateToken(ValidateTokenRequest) returns (ValidateTokenResponse);
  rpc GetUserRoles(GetUserRolesRequest) returns (GetUserRolesResponse);
}
```

## 开发

### 运行测试

```bash
# auth9-core
cd auth9-core
cargo test --lib           # 单元测试
cargo test --test '*'      # 集成测试

# auth9-portal
cd auth9-portal
npm run test               # 单元测试
npm run lint               # Linting
npm run typecheck          # 类型检查
```

### CI/CD

- **CI**：每个 PR 到 `main` 时运行
  - Rust: fmt, clippy, tests
  - Node: lint, typecheck, tests, build
  - Docker: build test

- **CD**：推送到 `main` 时运行
  - 构建并推送 Docker 镜像到 GHCR
  - 生成带镜像标签的部署摘要

### 部署

```bash
# Kubernetes
kubectl create secret generic auth9-secrets \
  --from-literal=DATABASE_URL='mysql://...' \
  --from-literal=JWT_SECRET='...' \
  -n auth9

./deploy/deploy.sh
```

Docker 镜像在合并到 main 时自动构建并推送到 GHCR：

```
ghcr.io/gpgkd906/auth9-core:latest
ghcr.io/gpgkd906/auth9-portal:latest
```

## 文档

- **[博客：AI 原生 SDLC](docs/blog-ai-native-sdlc-zh.md)** — 方法论详细阐述
- **[架构](docs/architecture.md)** — 系统设计概览
- **[设计系统](docs/design-system.md)** — Liquid Glass UI 设计语言
- **[API 访问控制](docs/api-access-control.md)** — 授权模型
- **[QA 测试用例](docs/qa/README.md)** — 96 份功能测试文档
- **[安全测试用例](docs/security/README.md)** — 48 份安全测试文档
- **[UI/UX 测试用例](docs/uiux/README.md)** — 12 份 UI/UX 测试文档
- **[Keycloak 主题](docs/keycloak-theme.md)** — 登录页定制

## 授权模型

Auth9 的授权逻辑集中在 `auth9-core/src/policy/mod.rs`。

- 主要入口：
  - `enforce(config, auth, input)` 用于无状态检查
  - `enforce_with_state(state, auth, input)` 用于数据库感知检查（平台管理员回退、租户所有者检查、共享租户检查）
- `PolicyInput` 由以下组成：
  - `PolicyAction`：正在尝试的操作
  - `ResourceScope`：正在访问的资源范围（`Global`、`Tenant`、`User`）
- 租户列表使用 `resolve_tenant_list_mode_with_state(...)` 解析可见性模式（`all`、基于成员关系、仅 token 租户）

### Handler 规则

新增 HTTP 端点时：

1. 将端点行为映射到 `PolicyAction`
2. 构造正确的 `ResourceScope`
3. 在业务逻辑之前调用 `enforce(...)` 或 `enforce_with_state(...)`
4. 将 handler 层的 `TokenType` 分支逻辑排除在授权代码之外

业务约束（如密码确认失败、禁用公开注册）仍可在 handler 中返回领域错误，但 token 授权必须留在 Policy 中。

## 许可证

MIT
