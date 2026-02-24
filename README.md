# Auth9

**[中文版](README-zh.md)** | English

**An experiment: can AI-native software development lifecycle "polish" software like humans do?**

Auth9 is two things at once: a self-hosted identity and access management platform (an alternative to Auth0), and a living experiment proving that AI agents can drive the full software development lifecycle — from planning and implementation to testing, bug fixing, and deployment. Almost all code in this project was AI-generated, and almost every step was driven by skills — from the backend Rust services to frontend React components, from test cases to deployment scripts.

> For a detailed writeup of the methodology, see the **[blog post](docs/blog-ai-native-sdlc.md)**.

---

## The Experiment

I didn't set out to build an identity platform. I wanted to answer a more fundamental question: **can an AI-native development process actually produce a polished application?**

I chose IAM as the test subject deliberately. This isn't simple CRUD: multi-tenant data isolation, OIDC/OAuth2 flows, Token Exchange, hierarchical RBAC permissions, webhook signature verification, audit logging — interconnected complexity where one wrong decision cascades into a dozen subtle bugs. Security isn't a nice-to-have; it's the very reason the system exists.

If AI-native SDLC can produce a polished IAM platform, it can work for most applications.

## The Real Challenge: Verifiability

AI coding tools make you write code faster. But writing code was never the real difficulty. The difficulty is **knowing whether the code is correct** — and knowing it fast enough, automated enough, that verification doesn't become the bottleneck.

The AI-native development process doesn't eliminate verification work. It makes verification **systematic and automated enough** to keep pace with AI-speed code generation. If AI writes code 10x faster but verification stays manual, you've just created a 10x larger QA backlog.

## Testing Shifted Left

**Testing didn't disappear. It became more important.** What changed is the form.

Traditional automated tests still exist in the codebase — `cargo test`, Playwright, Vitest. All AI-generated, all essential. What we added is a layer *before* code-level tests: **QA test documents**. Structured specifications that describe what to test, how to test it, and how to verify correctness at the data layer. AI generates them; humans review and approve them. Then AI executes them — including browser automation, API calls, database queries, and gRPC validation.

The human's role: review every generated test document for completeness, edge cases, and security considerations the AI might miss; observe the agent's automated testing to check if its behavior meets expectations. The AI's role: generate the documents, execute them, report failures, and fix what it can.

## The Closed-Loop Pipeline

The pipeline chains 16 Agent Skills together, where the output of each phase feeds the next:

```
Human + AI ──► Plan feature
                  │
                  ▼
          ┌─ Generate QA / Security / UIUX test docs
          │   (qa-doc-gen)
          ▼
          ┌─ Execute tests automatically
          │   Browser automation, API testing,
          │   DB validation, gRPC regression,
          │   performance benchmarks
          ▼
          ┌─ Failures? Create structured tickets
          │   (docs/ticket/)
          ▼
          ┌─ AI reads ticket → verifies issue →
          │   fixes code → resets environment →
          │   re-runs tests → closes ticket
          │   (ticket-fix)
          ▼
          ┌─ Periodically audit doc quality
          │   (qa-doc-governance)
          ▼
          ┌─ Align tests after refactors
          │   (align-tests, test-coverage)
          ▼
          ┌─ Deploy to Kubernetes
          │   (deploy-gh-k8s)
          └─────────────────────────
```

### Agent Skills

The `.agents/skills/` directory contains 16 skills covering every phase of the development lifecycle:

| Phase | Skills | What They Do |
|-------|--------|-------------|
| **Plan** | `project-bootstrap` | Scaffold a new project from scratch |
| **Code** | `rust-conventions`, `keycloak-theme` | Coding standards, theme development |
| **Test Docs** | `qa-doc-gen`, `qa-doc-governance` | Generate and govern test documentation |
| **Execute Tests** | `qa-testing`, `e2e-testing`, `performance-testing`, `auth9-grpc-regression` | Run QA, E2E, load, and gRPC tests |
| **Fix** | `ticket-fix`, `align-tests` | Auto-fix tickets, realign tests after refactors |
| **Coverage** | `test-coverage` | Enforce >=90% coverage across all layers |
| **Deploy** | `deploy-gh-k8s` | GitHub Actions gate → K8s deploy → health check |
| **Operate** | `ops`, `reset-local-env` | Logs, troubleshooting, environment reset |

### Documents as Executable Specifications

The `docs/` directory isn't passive documentation — it's a machine-readable test suite:

| Directory | Files | Purpose |
|-----------|-------|---------|
| `docs/qa/` | 96 | Functional test scenarios with step-by-step procedures, expected results, and SQL validation queries |
| `docs/security/` | 48 | Security test cases across 11 categories (API security, auth, injection, session, etc.) |
| `docs/uiux/` | 12 | UI/UX test cases with visibility-first navigation verification |
| `docs/ticket/` | — | Active defect tickets, created and consumed by AI |

### The Self-Healing Loop

The `ticket-fix` skill is the core mechanism of how AI "polishes" software. When a test fails, a structured ticket is created. AI reads the ticket, reproduces the issue, fixes it, resets the environment, re-runs the test, and closes the ticket.

Not every failed test is a bug. The skill explicitly handles **false positives** — when a failure is caused by flawed test procedures rather than code defects, it updates the QA document to prevent recurrence. Every failure makes the test suite better.

### What the Human Actually Does

This is human-AI collaboration, not replacement:

- **Planning**: Define what to build, acceptance criteria, architectural tradeoffs
- **Reviewing**: Test documents and first-version code. QA execution and ticket-fix run autonomously
- **Steering**: Root cause analysis for false positives, governance remediation decisions
- **Architecture**: Domain modeling, data flow design, security boundaries

After 20 rounds of iteration, AI-executed tests still produce tickets — but far fewer than the early rounds, and the application gets richer in detail with each pass. The polishing loop runs faster, and every round is documented.

**The human's core value lies in defining "what we want to do and what we don't want to do"** and providing good enough taste and judgment. The role of human experts actually becomes more important — we need true full-stack engineers who understand not only development but also infrastructure, DevOps, and security.

As a developer, I've always advocated for extreme programming. As a tech lead, I trust my team members, but I leverage agile development methodologies, including test-driven practices, for risk management as much as possible. So when it comes to AI, my perspective is quite open: I believe almost all risk management techniques used in software development, especially extreme programming practices, can be applied to managing Agents.

## By the Numbers

- **16** Agent Skills covering the full development lifecycle
- **156** test documents (96 QA + 48 security + 12 UI/UX)
- **9** tool scripts for token generation, API testing, gRPC smoke tests
- **~2,300** lines of skill definitions
- **1** human

---

## The IAM Platform

Auth9 is a fully functional identity platform — the product that this methodology builds and maintains.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                              │
├─────────────────┬─────────────────────┬─────────────────────────┤
│  auth9-portal   │  Business Services  │      auth9-sdk          │
│ (React Router 7)│                     │      (Optional)         │
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
    │ (MySQL) │          │ (Cache) │
    └─────────┘          └─────────┘
```

### Components

| Component | Technology | Description |
|-----------|------------|-------------|
| **auth9-core** | Rust (axum, tonic, sqlx) | Backend API & gRPC services |
| **auth9-portal** | React Router 7 + TypeScript + Vite | Admin dashboard UI |
| **Database** | TiDB (MySQL compatible) | Tenant, user, RBAC data |
| **Cache** | Redis | Session, token caching |
| **Auth Engine** | Keycloak | OIDC provider (optional) |

### Features

- **Multi-tenant**: Isolated tenants with custom settings
- **SSO**: Single Sign-On via OIDC
- **Dynamic RBAC**: Roles, permissions, inheritance
- **Token Exchange**: Service-to-service authentication
- **Audit Logs**: Track all administrative actions
- **Modern UI**: React Router 7-based design system
- **Action Engine**: Event-driven automation workflows with JavaScript/TypeScript
- **TypeScript SDK**: Official SDK for seamless integration
- **Invitation System**: Email-based user onboarding with automated workflows
- **Brand Customization**: Custom logos, colors, themes for tenant branding
- **Email Templates**: Flexible email template system with multi-language support
- **Password Management**: Password policies, reset, and change
- **Session Management**: View and revoke active sessions
- **WebAuthn/Passkey**: Passwordless authentication
- **Social Login**: Google, GitHub, OIDC, SAML support
- **Security Alerts**: Real-time threat detection
- **Login Analytics**: Detailed login statistics and events
- **Webhooks**: Real-time event notifications

## Quick Start

### Local Development

```bash
# Start dependencies (TiDB, Redis, Keycloak)
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Run auth9-core
cd auth9-core
cp .env.example .env
cargo run

# Run auth9-portal
cd auth9-portal
cp .env.example .env
npm install
npm run dev
```

### Full Stack with Docker

```bash
docker-compose up -d
```

- Portal: http://localhost:3000
- API: http://localhost:8080
- Keycloak: http://localhost:8081

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/tenants` | GET, POST | List/create tenants |
| `/api/v1/users` | GET, POST | List/create users |
| `/api/v1/services` | GET, POST | List/register services |
| `/api/v1/roles` | GET, POST | List/create roles |
| `/api/v1/rbac/assign` | POST | Assign roles to users |
| `/api/v1/audit-logs` | GET | Query audit logs |

## gRPC Services

```protobuf
service TokenExchange {
  rpc ExchangeToken(ExchangeTokenRequest) returns (ExchangeTokenResponse);
  rpc ValidateToken(ValidateTokenRequest) returns (ValidateTokenResponse);
  rpc GetUserRoles(GetUserRolesRequest) returns (GetUserRolesResponse);
}
```

## Development

### Running Tests

```bash
# auth9-core
cd auth9-core
cargo test --lib           # Unit tests
cargo test --test '*'      # Integration tests

# auth9-portal
cd auth9-portal
npm run test               # Unit tests
npm run lint               # Linting
npm run typecheck          # Type checking
```

### CI/CD

- **CI**: Runs on every PR to `main`
  - Rust: fmt, clippy, tests
  - Node: lint, typecheck, tests, build
  - Docker: build test

- **CD**: Runs on push to `main`
  - Builds and pushes Docker images to GHCR
  - Generates deployment summary with image tags

### Deployment

```bash
# Kubernetes
kubectl create secret generic auth9-secrets \
  --from-literal=DATABASE_URL='mysql://...' \
  --from-literal=JWT_SECRET='...' \
  -n auth9

./deploy/deploy.sh
```

Docker images are automatically built and pushed to GHCR on merge to main:

```
ghcr.io/gpgkd906/auth9-core:latest
ghcr.io/gpgkd906/auth9-portal:latest
```

## Documentation

- **[Blog: AI-Native SDLC](docs/blog-ai-native-sdlc.md)** — Detailed writeup of the methodology
- **[Architecture](docs/architecture.md)** — System design overview
- **[Design System](docs/design-system.md)** — Liquid Glass UI design language
- **[API Access Control](docs/api-access-control.md)** — Authorization model
- **[QA Test Cases](docs/qa/README.md)** — 96 functional test documents
- **[Security Test Cases](docs/security/README.md)** — 48 security test documents
- **[UI/UX Test Cases](docs/uiux/README.md)** — 12 UI/UX test documents
- **[Keycloak Theme](docs/keycloak-theme.md)** — Login page customization

## Authorization Model

Auth9 authorization is centralized in `auth9-core/src/policy/mod.rs`.

- Primary entry points:
  - `enforce(config, auth, input)` for stateless checks
  - `enforce_with_state(state, auth, input)` for DB-aware checks (platform admin fallback, tenant owner checks, shared-tenant checks)
- `PolicyInput` is composed of:
  - `PolicyAction`: what operation is being attempted
  - `ResourceScope`: what resource scope is being accessed (`Global`, `Tenant`, `User`)
- Tenant listing uses `resolve_tenant_list_mode_with_state(...)` to resolve visibility mode (`all`, membership-based, token-tenant only).

### Handler Rule

For new HTTP endpoints:

1. Map endpoint behavior to a `PolicyAction`.
2. Construct the correct `ResourceScope`.
3. Call `enforce(...)` or `enforce_with_state(...)` before business logic.
4. Keep handler-level `TokenType` branching out of authorization code.

Business constraints (for example password confirmation failure, disabled public registration) may still return domain errors in handlers, but token authorization must stay in Policy.

## License

MIT
