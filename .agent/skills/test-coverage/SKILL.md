---
name: test-coverage
description: Run tests and check coverage for Auth9 project. Use when running unit tests, coverage analysis, or writing new service tests with mocks.
---

# Test Coverage Skill

## Backend (Auth9 Core)

### Run All Tests
```bash
cd auth9-core
cargo test
```
All tests run fast (~1-2 seconds) with **no Docker or external services required**.

### Run Coverage Analysis
```bash
cd auth9-core
cargo llvm-cov --ignore-config --run-types Tests --out Json --output-dir target/llvm-cov
```

## Frontend (Auth9 Portal)

### Run Tests
```bash
cd auth9-portal
npx vitest --run
```

### Run Coverage
```bash
cd auth9-portal
npm run test:coverage
```

---

## Testing Strategy

### No External Dependencies

Auth9-core tests use **mocks instead of real services**:

| Component | Testing Approach |
|-----------|-----------------|
| Repository layer | Mock traits with `mockall` |
| Service layer | Unit tests with mock repositories |
| gRPC services | `NoOpCacheManager` + mock repositories |
| Keycloak | `wiremock` HTTP mocking |

### Prohibited

- **No testcontainers** - tests must not start Docker containers
- **No real database connections** - use mock repositories
- **No real Redis connections** - use `NoOpCacheManager`
- **No faker library** - construct test data directly

---

## Test File Structure

```
auth9-core/
├── src/
│   ├── service/
│   │   ├── tenant.rs      # Service + unit tests (#[cfg(test)])
│   │   ├── user.rs        # Service + unit tests
│   │   ├── client.rs      # Service + unit tests
│   │   └── rbac.rs        # Service + unit tests
│   └── repository/
│       └── *.rs           # Traits with #[mockall::automock]
└── tests/
    ├── common/mod.rs              # Test config helpers
    ├── grpc_token_exchange_test.rs # gRPC tests with NoOpCacheManager
    └── keycloak_unit_test.rs      # Keycloak with wiremock
```

---

## Writing Service Layer Tests

Service tests live in `src/service/*.rs` inside `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::MockTenantRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_tenant_success() {
        let mut mock = MockTenantRepository::new();

        mock.expect_find_by_slug()
            .with(eq("test-tenant"))
            .returning(|_| Ok(None));

        mock.expect_create()
            .returning(|input| Ok(Tenant {
                name: input.name.clone(),
                slug: input.slug.clone(),
                ..Default::default()
            }));

        let service = TenantService::new(Arc::new(mock), None);

        let result = service.create(CreateTenantInput {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            logo_url: None,
            settings: None,
        }).await;

        assert!(result.is_ok());
    }
}
```

---

## Writing gRPC Tests

gRPC tests use `NoOpCacheManager` instead of real Redis:

```rust
use auth9_core::cache::NoOpCacheManager;

fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

#[tokio::test]
async fn test_exchange_token() {
    let cache_manager = create_test_cache();
    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,  // No Redis needed
        user_repo,
        service_repo,
        rbac_repo,
    );
    // ...
}
```

---

## Repository Mock Pattern

Repository traits use `#[cfg_attr(test, mockall::automock)]`:

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Tenant>>;
    // ...
}
```

This generates `MockTenantRepository` for use in tests.

---

## Coverage Targets

| Layer | Target |
|-------|--------|
| Domain/Business logic | 95%+ |
| Service layer | 90%+ |
| API handlers | 80%+ |
| gRPC handlers | 85%+ |

---

## Troubleshooting

- **Compilation errors**: Run `cargo clean` first
- **Mock expectations not met**: Check predicate conditions
- **Network errors in llvm-cov**: Add `required_permissions: ["network"]`
