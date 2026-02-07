# Repository Guidelines

## Project Structure & Module Organization
- `auth9-core/`: Rust backend (axum + tonic). Key code lives in `auth9-core/src/` with `api/`, `grpc/`, `service/`, `repository/`, `domain/`, `jwt/`, `cache/`, and `config/`.
- `auth9-portal/`: React Router 7 + TypeScript + Vite admin UI.
- `auth9-keycloak-events/` and `auth9-keycloak-theme/`: Keycloak integrations and theming.
- `docs/`: Architecture and design references.
- `deploy/` and `docker-compose*.yml`: Deployment and local infra.
- `scripts/`: Utility scripts for dev and ops.

## Skills

Project skills are in `.agents/skills/`. Read the relevant skill file before executing related tasks:
- `.agents/skills/ops/SKILL.md` - Running tests, Docker/K8s logs, troubleshooting
- `.agents/skills/test-coverage/SKILL.md` - Coverage analysis, writing tests with mocks
- `.agents/skills/reset-local-env/SKILL.md` - Resetting local development environment
- `.agents/skills/qs-testing/SKILL.md` - QA testing, manual testing, verify feature functionality, or test specific modules.
- `.agents/skills/auth9-grpc-regression/SKILL.md` - gRPC regression testing

## Build, Test, and Development Commands
Backend:
- `cd auth9-core && cargo build` — build backend.
- `cd auth9-core && cargo test` — run all Rust tests (fast, no external deps).
- `cd auth9-core && cargo clippy` — lint.
- `cd auth9-core && cargo fmt` — format.

Frontend:
- `cd auth9-portal && npm install`
- `cd auth9-portal && npm run dev` — dev server.
- `cd auth9-portal && npm run build` — production build.
- `cd auth9-portal && npm run test` — unit tests (Vitest).
- `cd auth9-portal && npm run lint` — ESLint.
- `cd auth9-portal && npm run typecheck` — TypeScript check.

Local infra:
- `docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d` — TiDB, Redis, Keycloak.

## Coding Style & Naming Conventions
- Rust: `cargo fmt` for formatting, `cargo clippy` for linting.
- TypeScript: ESLint (flat config) via `npm run lint`.
- Tests: prefer clear, explicit names; keep handlers thin (API/gRPC), push logic into `service/`.

## Testing Guidelines
- No external dependencies for Rust tests: no Docker, no real DB/Redis.
- Use `mockall` for repository traits and `wiremock` for Keycloak HTTP.
Rust test locations:
- Service tests: `auth9-core/src/service/*.rs` with `#[cfg(test)]`.
- HTTP handler tests: `auth9-core/tests/api/http/*_http_test.rs`.
- gRPC tests: `auth9-core/tests/grpc_*.rs`.
- Frontend tests use `happy-dom` (required for React Router 7).
- E2E: `npm run test:e2e` (frontend-only) and `npm run test:e2e:full` (full stack).

## Commit & Pull Request Guidelines
- Recent commit history uses short, descriptive messages (often in Chinese) without strict prefixes. Keep it concise and action-focused.
- PRs should include a clear summary and rationale, linked issues or context when available, and screenshots or recordings for UI changes.

## Security & Configuration Notes
- `auth9-core` requires `DATABASE_URL` and `JWT_SECRET`. Use `.env.example` files as starting points.
- Keycloak is optional but used for OIDC/MFA; ensure local Keycloak is running when testing those flows.
