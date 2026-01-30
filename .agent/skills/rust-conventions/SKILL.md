---
name: rust-conventions
description: Rust coding conventions for auth9-core
globs: auth9-core/**/*.rs
alwaysApply: false
---

# Rust Conventions (auth9-core)

## Tech Stack

- **Web**: axum + Tower middleware
- **gRPC**: tonic
- **Database**: sqlx (compile-time SQL checking, MySQL/TiDB)
- **Async**: tokio runtime
- **Logging**: tracing (structured logs + distributed tracing)
- **Serialization**: serde
- **JWT**: jsonwebtoken
- **Cache**: redis-rs (with NoOpCacheManager for testing)
- **Testing**: mockall, wiremock

## Code Organization

```rust
// Module structure follows domain-driven design:
// domain/   → Pure domain models with validation
// service/  → Business logic (depends on repository traits)
// repository/ → Data access (implements traits, mockable)
// api/      → HTTP handlers (thin layer)
// grpc/     → gRPC handlers (thin layer)
// cache/    → CacheManager + NoOpCacheManager for tests
```

## Error Handling

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

## Async Patterns

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

## Testing - NO EXTERNAL DEPENDENCIES

All tests run fast (~1-2 seconds) with **no Docker or external services**:

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

### Repository Mock Pattern

```rust
// Repository traits use mockall for auto-generated mocks
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Tenant>>;
    // ...
}
```

### Service Layer Tests

```rust
// Unit tests: mock dependencies with mockall
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::MockTenantRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_tenant() {
        let mut mock_repo = MockTenantRepository::new();

        mock_repo.expect_find_by_slug()
            .with(eq("test"))
            .returning(|_| Ok(None));

        mock_repo.expect_create()
            .returning(|input| Ok(Tenant {
                name: input.name.clone(),
                ..Default::default()
            }));

        let service = TenantService::new(Arc::new(mock_repo), None);
        let result = service.create(input).await;
        assert!(result.is_ok());
    }
}
```

### gRPC Tests with NoOpCacheManager

```rust
use auth9_core::cache::NoOpCacheManager;

fn create_test_cache() -> NoOpCacheManager {
    NoOpCacheManager::new()
}

#[tokio::test]
async fn test_exchange_token() {
    let cache = create_test_cache();  // No Redis needed
    let service = TokenExchangeService::new(
        jwt_manager,
        cache,
        user_repo,
        service_repo,
        rbac_repo,
    );
    // ...
}
```

### Keycloak Tests with WireMock

```rust
use wiremock::{Mock, ResponseTemplate, MockServer, matchers::{method, path}};

#[tokio::test]
async fn test_keycloak_operation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300,
            "token_type": "bearer"
        })))
        .mount(&mock_server)
        .await;

    let client = KeycloakClient::new(config_with_mock(&mock_server.uri()));
    // ...
}
```

## SQL with sqlx

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

## Commands

```bash
cargo test              # All tests (fast, no Docker)
cargo tarpaulin --out Html  # Coverage report
cargo clippy            # Linting
cargo fmt               # Format code
```
