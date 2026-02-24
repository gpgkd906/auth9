# Auth9

**A production-grade IAM platform built entirely through AI-native software development.**

Auth9 is two things at once: a self-hosted identity and access management service (Auth0 alternative), and a living proof-of-concept that AI agents can drive the full software development lifecycle — from planning and implementation to testing, bug fixing, and deployment.

## The Idea

Most AI coding tools help you write code faster. Auth9 asks a different question: **what if AI could own the entire development loop?**

This repository ships with 16 Claude Code skills and 9 automation scripts that form a closed-loop SDLC pipeline. A human developer plans features with AI, then the AI generates test documentation, executes QA scenarios via browser automation, creates tickets for failures, fixes them autonomously, and iterates until the software converges on correctness — all without manual intervention in the inner loop.

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

The inner loop (test → ticket → fix → re-test) runs autonomously. The human's role shifts from writing code and chasing bugs to **planning, reviewing, and steering**.

## AI-Native SDLC Pipeline

### Skills Overview

The `.claude/skills/` directory contains 16 skills that cover every phase of the development lifecycle:

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

Each QA document follows a strict template: initial state, objective, test steps, expected results, and expected data state — all parseable by AI agents for automated execution.

### Key Design Decisions

- **Documents are the source of truth for testing**, not code annotations or test frameworks alone. AI reads the docs and executes them.
- **Ticket-driven self-healing**: test failures produce structured tickets; AI fixes them and re-validates in a convergent loop.
- **Visibility-first UI testing**: every feature must be reachable through normal navigation, not just via direct URL.
- **Zero external dependencies for unit tests**: all tests run in ~1-2 seconds via mocks (mockall, wiremock, NoOpCacheManager), keeping the AI iteration loop fast.
- **Documentation governance prevents rot**: periodic audits classify issues by severity (P0/P1/P2) and auto-remediate.

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
