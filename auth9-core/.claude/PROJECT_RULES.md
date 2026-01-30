# Auth9 Project Rules for Claude Code

## Project Overview

Auth9 is a self-hosted identity and access management service designed to replace Auth0.

### Architecture

| Component | Tech Stack | Purpose |
|-----------|------------|---------|
| **auth9-core** | Rust (axum, tonic, sqlx) | Backend API & gRPC services |
| **auth9-portal** | Remix + TypeScript + Vite | Admin dashboard UI |
| **Database** | TiDB (MySQL compatible) | Tenant, user, RBAC data |
| **Cache** | Redis | Session, token caching |
| **Auth Engine** | Keycloak | OIDC provider |

### Core Concepts

- **Headless Keycloak**: Keycloak handles OIDC/MFA only; business logic in auth9-core
- **Token Exchange**: Identity Token → Tenant Access Token with roles/permissions
- **Multi-tenant**: Isolated tenants with custom settings and RBAC

### Key Directories

```
auth9-core/src/
├── api/          # REST API handlers (axum)
├── grpc/         # gRPC services (tonic)
├── domain/       # Domain models
├── repository/   # Data access layer (sqlx)
├── service/      # Business logic
├── keycloak/     # Keycloak Admin API client
├── jwt/          # JWT signing & validation
└── cache/        # Redis caching

auth9-portal/app/
├── routes/       # Remix file-system routes
├── components/   # UI components
├── services/     # API client layer
└── lib/          # Utilities
```

### Performance Requirements

- Token Exchange latency: < 20ms (use Redis cache)
- Auth QPS: > 1000 requests/second
- Availability: 99.9%

---

## Rust Conventions (auth9-core)

### Tech Stack

- **Web**: axum + Tower middleware
- **gRPC**: tonic
- **Database**: sqlx (compile-time SQL checking, MySQL/TiDB)
- **Async**: tokio runtime
- **Logging**: tracing (structured logs + distributed tracing)
- **Serialization**: serde
- **JWT**: jsonwebtoken
- **Cache**: redis-rs

### Code Organization

```rust
// Module structure follows domain-driven design:
// domain/   → Pure domain models with validation
// service/  → Business logic (depends on repository traits)
// repository/ → Data access (implements traits)
// api/      → HTTP handlers (thin layer)
// grpc/     → gRPC handlers (thin layer)
```

### Error Handling

```rust
// ❌ BAD - swallowing errors
let result = db.query().await.ok();

// ✅ GOOD - use Result with context
let result = db.query()
    .await
    .context("Failed to query tenant")?;

// ✅ GOOD - use custom error types
#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### Async Patterns

```rust
// ✅ Use tokio::spawn for background tasks
tokio::spawn(async move {
    cache.invalidate(&key).await;
});

// ✅ Use ? for error propagation in async
async fn get_user(&self, id: Uuid) -> Result<User> {
    let user = self.repo.find_by_id(id).await?;
    Ok(user)
}
```

### Testing

```rust
// Unit tests: mock dependencies with mockall
#[cfg(test)]
mod tests {
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_tenant() {
        let mut mock_repo = MockTenantRepository::new();
        mock_repo.expect_create()
            .returning(|t| Ok(t));
        // ...
    }
}

// Integration tests: use testcontainers-rs
```

### SQL with sqlx

```rust
// ✅ Use query_as! for compile-time checking
let tenant = sqlx::query_as!(
    Tenant,
    "SELECT * FROM tenants WHERE id = ?",
    id
)
.fetch_one(&pool)
.await?;
```

---

## Remix/TypeScript Conventions (auth9-portal)

### Tech Stack

- **Framework**: Remix + Vite
- **UI**: Radix UI + Tailwind CSS
- **Validation**: Zod + Conform
- **Testing**: Vitest (unit), Playwright (E2E)
- **State**: Remix loader/action (server-first)

### Apple-Style UI Design

```tsx
// Design principles:
// - Minimalism: generous whitespace, clean lines
// - Glassmorphism: backdrop-filter: blur(), semi-transparent bg
// - Large radius: rounded-2xl for cards, rounded-xl for buttons
// - Subtle animations: hover/focus transitions (200-300ms)
// - System fonts: font-sans (Inter/SF Pro Display)
// - Restrained colors: neutral grays + single accent color

// ✅ Example card
<div className="rounded-2xl bg-white/80 backdrop-blur-xl
               shadow-sm border border-gray-100 p-6">
  ...
</div>
```

### Route Structure

```
app/routes/
├── _index.tsx          # Landing page
├── login.tsx           # Login page
├── dashboard.tsx       # Dashboard layout
├── dashboard._index.tsx # Dashboard home
├── tenants._index.tsx  # Tenant list
└── tenants.$id.tsx     # Tenant detail
```

### Data Loading Pattern

```tsx
// ✅ Use loader for data fetching
export async function loader({ request }: LoaderFunctionArgs) {
  const tenants = await api.getTenants();
  return json({ tenants });
}

// ✅ Use action for mutations
export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const result = await api.createTenant(formData);
  return redirect(`/tenants/${result.id}`);
}
```

### Component Guidelines

```tsx
// ✅ Functional components only
export function TenantCard({ tenant }: { tenant: Tenant }) {
  return (
    <Card className="hover:shadow-md transition-shadow duration-200">
      <CardHeader>
        <h3 className="text-lg font-semibold">{tenant.name}</h3>
      </CardHeader>
    </Card>
  );
}

// ✅ Extract hooks for reusable logic
function useTenantForm() {
  const [isPending, startTransition] = useTransition();
  // ...
}
```

### API Service Layer

```typescript
// app/services/api.ts
// Centralize API calls with proper typing
export const api = {
  async getTenants(): Promise<Tenant[]> {
    const res = await fetch(`${API_URL}/api/v1/tenants`);
    if (!res.ok) throw new Error('Failed to fetch tenants');
    return res.json();
  },
};
```

---

## Testing Conventions

### TDD Development Workflow

When implementing features or fixing bugs, follow the Test-Driven Development (TDD) cycle:

```
1. RED    → Write a failing test first
2. GREEN  → Write minimal code to pass the test
3. REFACTOR → Improve code while keeping tests green
```

### TDD Rules

- **Write tests BEFORE implementation code**
- **One test at a time**: Don't write multiple failing tests
- **Minimal implementation**: Only write enough code to pass the current test
- **Refactor continuously**: Clean up after each green phase

### Coverage Requirement

**Minimum test coverage: 90%**

```bash
# Rust - check coverage with tarpaulin
cargo tarpaulin --out Html --output-dir target/coverage

# TypeScript - check coverage with vitest
npm run test -- --coverage
```

Coverage targets by layer:
| Layer | Target |
|-------|--------|
| Domain/Business logic | 95%+ |
| Service layer | 90%+ |
| Repository layer | 85%+ |
| API handlers | 80%+ |

### Test Pyramid

```
        E2E Tests (few, critical flows)
              /\
             /  \
    Integration Tests (API, gRPC, DB)
           /      \
          /        \
     Unit Tests (business logic core)
```

### auth9-core (Rust)

#### Unit Tests

```rust
// Location: src/*/tests.rs or inline #[cfg(test)]
// Use mockall for mocking dependencies

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tenant_validation() {
        let tenant = Tenant::new("Test", "test-slug");
        assert!(tenant.validate().is_ok());
    }
}
```

#### Integration Tests

```rust
// Location: tests/api/, tests/grpc/
// Use testcontainers-rs for MySQL + Redis

#[tokio::test]
async fn test_create_tenant_api() {
    let app = TestApp::spawn().await;

    let response = app.client
        .post("/api/v1/tenants")
        .json(&json!({"name": "Test", "slug": "test"}))
        .send()
        .await;

    assert_eq!(response.status(), 201);
}
```

#### Commands

```bash
cargo test --lib        # Unit tests (fast)
cargo test --test '*'   # Integration tests (requires Docker)
cargo tarpaulin --out Html  # Coverage report
```

### auth9-portal (TypeScript)

#### Unit Tests (Vitest)

```typescript
// Location: tests/unit/*.test.ts
import { describe, it, expect } from 'vitest';

describe('utils', () => {
  it('should format date correctly', () => {
    expect(formatDate(new Date('2024-01-01'))).toBe('Jan 1, 2024');
  });
});
```

#### E2E Tests (Playwright)

```typescript
// Location: tests/e2e/*.spec.ts
import { test, expect } from '@playwright/test';

test('login flow', async ({ page }) => {
  await page.goto('/login');
  await page.fill('[name="email"]', 'admin@example.com');
  await page.click('button[type="submit"]');
  await expect(page).toHaveURL('/dashboard');
});
```

#### Commands

```bash
npm run test           # Vitest unit tests
npm run lint           # ESLint
npm run typecheck      # TypeScript check
npx playwright test    # E2E tests
```

### Test Data Conventions

- Use descriptive names: `test_create_tenant_with_valid_data`
- Keep test data minimal and focused
- Clean up test data after integration tests
