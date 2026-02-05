---
name: rust-conventions
description: Rust coding conventions and patterns for auth9-core development.
---

# Rust Conventions (auth9-core)

## Tech Stack

| Component | Library |
|-----------|---------|
| Web | axum + Tower middleware |
| gRPC | tonic |
| Database | sqlx (compile-time SQL, TiDB) |
| Async | tokio |
| Logging | tracing |
| Cache | redis-rs, NoOpCacheManager (tests) |
| Testing | mockall, wiremock |

## Code Organization

```
auth9-core/src/
├── domain/      # Pure models with validation
├── service/     # Business logic (depends on repo traits)
├── repository/  # Data access (mockable traits)
├── api/         # HTTP handlers (thin)
├── grpc/        # gRPC handlers (thin)
└── cache/       # CacheManager + NoOpCacheManager
```

## Error Handling

```rust
// ❌ BAD
let result = db.query().await.ok();

// ✅ GOOD - use Result with context
let result = db.query()
    .await
    .context("Failed to query tenant")?;

// ✅ GOOD - custom error types
#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(Uuid),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

## Testing - NO EXTERNAL DEPENDENCIES

All tests run fast (~1-2s) with **no Docker**:

| Component | Approach |
|-----------|----------|
| Repository | Mock traits with `mockall` |
| Service | Unit tests with mock repos |
| gRPC | `NoOpCacheManager` + mocks |
| Keycloak | `wiremock` HTTP mocking |

### Prohibited

- No testcontainers
- No real database connections
- No real Redis connections
- No faker library

### Repository Mock Pattern

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, input: &CreateTenantInput) -> Result<Tenant>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Tenant>>;
}
```

### Service Layer Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::tenant::MockTenantRepository;

    #[tokio::test]
    async fn test_create_tenant() {
        let mut mock = MockTenantRepository::new();
        mock.expect_find_by_slug()
            .returning(|_| Ok(None));
        mock.expect_create()
            .returning(|input| Ok(Tenant { name: input.name.clone(), ..Default::default() }));

        let service = TenantService::new(Arc::new(mock), None);
        let result = service.create(input).await;
        assert!(result.is_ok());
    }
}
```

### gRPC Tests

```rust
use auth9_core::cache::NoOpCacheManager;

#[tokio::test]
async fn test_exchange_token() {
    let cache = NoOpCacheManager::new();  // No Redis
    let service = TokenExchangeService::new(jwt_manager, cache, repos...);
    // ...
}
```

### Keycloak Tests

```rust
use wiremock::{Mock, ResponseTemplate, MockServer};

#[tokio::test]
async fn test_keycloak() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token"
        })))
        .mount(&mock_server).await;
    // Use mock_server.uri()
}
```

## Commands

```bash
cargo test              # All tests (fast)
cargo llvm-cov --html   # Coverage report
cargo clippy            # Lint
cargo fmt               # Format
```
