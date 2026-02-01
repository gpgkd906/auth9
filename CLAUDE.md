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
make coverage                  # Coverage report (excludes repository/migration layers)
make coverage-html             # Coverage HTML report
```

### auth9-portal (TypeScript/React Router 7)
```bash
cd auth9-portal
npm install                    # Install dependencies
npm run dev                    # Dev server
npm run build                  # Build
npm run test                   # Unit tests (Vitest)
npm run lint                   # ESLint (flat config)
npm run typecheck              # TypeScript check
npm run test:e2e               # E2E tests - frontend isolation (fast)
npm run test:e2e:full          # E2E tests - full-stack (requires Docker)
npm run test:e2e:full:reset    # Reset env + full-stack E2E tests
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
| auth9-portal | React Router 7 + TypeScript + Vite | Admin dashboard |
| Database | TiDB (MySQL compatible) | Tenant, user, RBAC data |
| Cache | Redis | Session, token caching |
| Auth Engine | Keycloak | OIDC provider |

### Data Modeling Rules

**No Foreign Keys**: TiDB 是分布式数据库，外键约束会导致跨节点协调开销。所有引用完整性在应用层管理：

- 迁移文件中不使用 `FOREIGN KEY` 约束
- 保留 `INDEX` 用于查询性能
- 级联删除在 Service 层实现
- 孤儿记录清理通过定期任务或删除操作时处理

**删除操作的级联处理**:
| 删除对象 | 需清理的关联表 |
|---------|---------------|
| Tenant | tenant_users, services, webhooks, invitations |
| User | tenant_users, sessions, password_reset_tokens, linked_identities |
| Service | permissions, roles, clients |
| Role | role_permissions, user_tenant_roles, parent_role_id references |
| TenantUser | user_tenant_roles |

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

## Documentation

Project documentation is in `docs/`. Read the relevant doc before related tasks:
- `design-system.md` - Auth9 Portal UI design system (Liquid Glass style, colors, components)
- `architecture.md` - System architecture overview
- `api-access-control.md` - API access control design
- `keycloak-theme.md` - Keycloak theme customization

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
- **HTTP handler tests**: `tests/api/http/*_http_test.rs` (uses `HasServices` DI pattern)
- **gRPC integration tests**: `tests/grpc_*.rs`
- **Keycloak tests**: `tests/keycloak_unit_test.rs` (uses wiremock)

### HTTP Handler DI Pattern
All API handlers use `<S: HasServices>` generic instead of concrete `AppState`. This enables testing production handler code with `TestAppState` + mock repositories. See `test-coverage.md` skill for details.

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

### auth9-portal Testing

**Test Environment**: Use `happy-dom` (not `jsdom`) for better React Router 7 compatibility.

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    environment: "happy-dom",  // Required for React Router 7 form actions
    // ...
  },
});
```

**Route Testing Pattern**: Use `createRoutesStub` from `react-router`:
```typescript
import { createRoutesStub } from "react-router";

it("renders page with loader data", async () => {
  const RoutesStub = createRoutesStub([
    {
      path: "/dashboard/users",
      Component: UsersPage,
      loader,
    },
  ]);

  render(<RoutesStub initialEntries={["/dashboard/users"]} />);
  await waitFor(() => {
    expect(screen.getByText("Users")).toBeInTheDocument();
  });
});
```

**Action Testing Pattern**: Use `FormData` for form submissions:
```typescript
// Helper to create form requests
function createFormRequest(url: string, data: Record<string, string>): Request {
  const formData = new FormData();
  for (const [key, value] of Object.entries(data)) {
    formData.append(key, value);
  }
  return new Request(url, { method: "POST", body: formData });
}

it("action validates input", async () => {
  const request = createFormRequest("http://localhost/register", {
    email: "test@example.com",
    password: "password123",
  });
  const response = await action({ request, params: {}, context: {} });
  expect(response.status).toBe(302);
});
```

### E2E Testing (Playwright)

**Hybrid Strategy**: Frontend isolation tests (fast, no Docker) + Full-stack integration tests (requires Docker).

```
tests/
├── e2e/                    # Frontend isolation tests (快速, 无 Docker)
│   └── login.spec.ts       # UI rendering, navigation
└── e2e-integration/        # Full-stack integration tests (需要 Docker)
    ├── setup/
    │   ├── test-config.ts      # Test configuration (URLs, credentials)
    │   └── keycloak-admin.ts   # Keycloak Admin API client
    ├── global-setup.ts         # Wait for services, create test users
    └── auth-flow.spec.ts       # Scenario-based tests
```

| Type | Directory | Target URL | Requirements | Purpose |
|------|-----------|------------|--------------|---------|
| Frontend isolation | `tests/e2e/` | localhost:5173 | Vite dev only | UI/navigation |
| Full-stack integration | `tests/e2e-integration/` | localhost:3000 | Docker + all services | Login/API |

**Commands**:
```bash
npm run test:e2e              # Frontend isolation tests (fast)
npm run test:e2e:full         # Full-stack tests (requires Docker)
npm run test:e2e:full:reset   # Reset environment + full-stack tests
```

**Test Data Preparation**: Use Keycloak Admin API in `global-setup.ts`:
```typescript
// tests/e2e-integration/setup/keycloak-admin.ts
const keycloak = new KeycloakAdminClient();
await keycloak.authenticate();  // Master realm admin
await keycloak.createUser({
  username: "e2e-test-user",
  email: "e2e-test@example.com",
  password: "Test123!",
  firstName: "E2E",
  lastName: "TestUser",
});
```

**Scenario-based Test Pattern**:
```typescript
test.describe("Scenario: User Authentication Flow", () => {
  const testUser = TEST_CONFIG.testUsers.standard;

  test("1. Login page should be accessible", async ({ page }) => { ... });
  test("2. Clicking sign in should redirect to Keycloak", async ({ page }) => { ... });
  test("3. User can login with valid credentials", async ({ page }) => { ... });
});

test.describe("Scenario: Auth9 Core API Integration", () => {
  test("1. Health endpoint should be accessible", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/health`);
    expect(response.ok()).toBeTruthy();
  });
});
```

**Test Configuration** (`tests/e2e-integration/setup/test-config.ts`):
- Fixed test user credentials (environment is reset before each test run)
- Service URLs: Portal (3000), Auth9 Core (8080), Keycloak (8081)
- Keycloak Admin credentials for test data setup
