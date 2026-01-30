# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

### auth9-core (Rust)
```bash
cd auth9-core
cargo build                    # Build
cargo test                     # Run all tests (fast, no external dependencies)
cargo test --lib               # Unit tests only
cargo test --test '*'          # Integration tests only
cargo test test_name           # Run single test by name
cargo test -- --nocapture      # Run with output
cargo clippy                   # Lint
cargo fmt                      # Format
cargo tarpaulin --out Html     # Coverage report
```

### auth9-portal (TypeScript/Remix)
```bash
cd auth9-portal
npm install                    # Install dependencies
npm run dev                    # Dev server
npm run build                  # Build
npm run test                   # Unit tests (Vitest)
npm run lint                   # ESLint
npm run typecheck              # TypeScript check
```

### Local Development with Docker
```bash
# Start dependencies (TiDB, Redis, Keycloak)
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Run backend
cd auth9-core && cargo run

# Run frontend
cd auth9-portal && npm run dev
```

## Architecture

Auth9 is a self-hosted identity and access management service (Auth0 alternative).

**Core Concept**: Headless Keycloak architecture - Keycloak handles OIDC/MFA only; all business logic lives in auth9-core. Token Exchange flow: Identity Token → Tenant Access Token with roles/permissions.

| Component | Stack | Purpose |
|-----------|-------|---------|
| auth9-core | Rust (axum, tonic, sqlx) | Backend API & gRPC |
| auth9-portal | Remix + TypeScript + Vite | Admin dashboard |
| Database | TiDB (MySQL compatible) | Tenant, user, RBAC data |
| Cache | Redis | Session, token caching |
| Auth Engine | Keycloak | OIDC provider |

### Code Organization (auth9-core)
```
auth9-core/src/
├── api/          # REST API handlers (axum) - thin layer
├── grpc/         # gRPC handlers (tonic) - thin layer
├── domain/       # Pure domain models with validation
├── service/      # Business logic (depends on repository traits)
├── repository/   # Data access layer (implements traits, mockall support)
├── keycloak/     # Keycloak Admin API client
├── jwt/          # JWT signing & validation
├── cache/        # Redis caching (CacheManager, NoOpCacheManager)
├── config/       # Configuration types
└── error/        # Error types
```

## Skills

Project skills are in `.claude/skills/`. Read the relevant skill file before executing related tasks:
- `ops.md` - Running tests, Docker/K8s logs, troubleshooting
- `test-coverage.md` - Coverage analysis, writing tests with mocks
- `reset-local-env.md` - Resetting local development environment

## Testing Strategy

### No External Dependencies
All tests run fast (~1-2 seconds) with **no Docker or external services**:
- Repository layer: Mock traits with `mockall`
- Service layer: Unit tests with mock repositories
- gRPC services: `NoOpCacheManager` + mock repositories
- Keycloak: `wiremock` HTTP mocking

### Prohibited
- No testcontainers - tests must not start Docker containers
- No real database connections - use mock repositories
- No real Redis connections - use `NoOpCacheManager`
- No faker library - construct test data directly

### Test File Locations
- **Service layer tests**: `src/service/*.rs` in `#[cfg(test)]` modules
- **Repository trait mocks**: `#[cfg_attr(test, mockall::automock)]` on trait definitions
- **gRPC integration tests**: `tests/grpc_*.rs`
- **Keycloak tests**: `tests/keycloak_unit_test.rs` (uses wiremock)

### Mock Patterns

Repository layer:
```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
}
```

Service layer tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::MockTenantRepository;

    #[tokio::test]
    async fn test_create_tenant_success() {
        let mut mock = MockTenantRepository::new();
        mock.expect_find_by_slug().returning(|_| Ok(None));
        mock.expect_create().returning(|input| Ok(Tenant { ... }));

        let service = TenantService::new(Arc::new(mock), None);
        let result = service.create(input).await;
        assert!(result.is_ok());
    }
}
```

gRPC tests (use NoOpCacheManager):
```rust
fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}
```
