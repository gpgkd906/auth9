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
cargo tarpaulin --ignore-config --run-types Tests --out Json --output-dir target/tarpaulin
```

HTML output:
```bash
cargo tarpaulin --out Html --output-dir target/coverage
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

        // Setup expectations
        mock.expect_find_by_slug()
            .with(eq("test-tenant"))
            .returning(|_| Ok(None));

        mock.expect_create()
            .returning(|input| Ok(Tenant {
                name: input.name.clone(),
                slug: input.slug.clone(),
                ..Default::default()
            }));

        // Create service with mock
        let service = TenantService::new(Arc::new(mock), None);

        // Test
        let input = CreateTenantInput {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            logo_url: None,
            settings: None,
        };

        let result = service.create(input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Tenant");
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = TenantService::new(Arc::new(mock), None);
        let result = service.get(id).await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
```

---

## Writing gRPC Tests

gRPC tests use `NoOpCacheManager` instead of real Redis:

```rust
use auth9_core::cache::NoOpCacheManager;
use auth9_core::grpc::TokenExchangeService;

fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

#[tokio::test]
async fn test_exchange_token_success() {
    let cache_manager = create_test_cache();

    // Setup mock repositories
    let user_repo = Arc::new(TestUserRepository::new());
    let service_repo = Arc::new(TestServiceRepository::new());
    let rbac_repo = Arc::new(TestRbacRepository::new());

    // Populate test data
    user_repo.add_user(create_test_user(user_id)).await;
    // ...

    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,  // NoOpCacheManager, no Redis needed
        user_repo,
        service_repo,
        rbac_repo,
    );

    let response = grpc_service.exchange_token(request).await;
    assert!(response.is_ok());
}
```

---

## Writing Keycloak Tests

Use `wiremock` to mock Keycloak HTTP endpoints:

```rust
use wiremock::{Mock, ResponseTemplate, matchers::{method, path}};

#[tokio::test]
async fn test_keycloak_create_user() {
    let mock_server = MockServer::start().await;

    // Mock admin token
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300,
            "token_type": "bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock user creation
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let client = KeycloakClient::new(config_with_mock_url(&mock_server.uri()));
    let result = client.create_user(&input).await;
    assert!(result.is_ok());
}
```

---

## Repository Mock Pattern

Repository traits use `#[cfg_attr(test, mockall::automock)]`:

```rust
// In src/repository/tenant.rs
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Tenant>>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<Tenant>>;
    async fn count(&self) -> Result<i64>;
    async fn update(&self, id: StringUuid, input: &UpdateTenantInput) -> Result<Tenant>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
}
```

This generates `MockTenantRepository` for use in tests.

---

## Coverage Targets

| Layer | Target |
|-------|--------|
| Domain/Business logic | 95%+ |
| Service layer | 90%+ |
| Repository traits | N/A (implementation tested via service) |
| API handlers | 80%+ |
| gRPC handlers | 85%+ |

---

## Troubleshooting

- **Compilation errors**: Run `cargo clean` first
- **Mock expectations not met**: Check predicate conditions with `mockall::predicate::*`
- **Network errors in tarpaulin**: Add `required_permissions: ["network"]` in tarpaulin.toml
