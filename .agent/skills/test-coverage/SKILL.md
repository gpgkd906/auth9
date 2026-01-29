---
name: test-coverage
description: Run tests and check coverage for Auth9 project. Use when running unit tests, integration tests, coverage analysis, or writing new repository/service tests.
---

# Test Coverage Skill

## Backend (Auth9 Core)

### Run Unit Tests
```bash
cd auth9-core
cargo test --lib
```

### Run Integration Tests
```bash
cd auth9-core
cargo test --test '*'
```
*Requires Docker for testcontainers. Set `DATABASE_URL` to use external database.*

### Run Coverage Analysis
```bash
cd auth9-core
cargo tarpaulin --ignore-config --run-types Tests --out Json --output-dir target/tarpaulin
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

## Repository Integration Tests

### Test Infrastructure

Located in `tests/common/mod.rs`, provides:

| Function | Purpose |
|----------|---------|
| `get_test_pool()` | Get MySQL connection (testcontainers or `DATABASE_URL`) |
| `setup_database(&pool)` | Run migrations |
| `cleanup_database(&pool)` | Clear test data in FK-safe order |

### Test Pattern

```rust
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::TenantRepository;

mod common;

#[tokio::test]
async fn test_example() {
    // 1. Get pool (skip if unavailable)
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    // 2. Setup and cleanup
    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    // 3. Create repository
    let repo = TenantRepositoryImpl::new(pool.clone());

    // 4. Test logic here...

    // 5. Cleanup after test
    common::cleanup_database(&pool).await.unwrap();
}
```

### Test File Structure

```
auth9-core/tests/
├── common/mod.rs      # Test utilities
├── tenant_test.rs     # TenantRepository tests
├── user_test.rs       # UserRepository tests
├── rbac_test.rs       # RbacRepository tests
├── service_test.rs    # ServiceRepository tests
└── audit_test.rs      # AuditRepository tests
```

### Environment Modes

| Mode | Configuration | Use Case |
|------|--------------|----------|
| **Testcontainers** | No env vars | CI/CD (auto-start MySQL) |
| **External DB** | `DATABASE_URL=mysql://...` | Local dev (faster) |

Local development example:
```bash
export DATABASE_URL="mysql://root@localhost:4000/auth9_test"
cargo test --test '*'
```

### Writing Repository Tests

1. **Import the repository trait and impl**:
   ```rust
   use auth9_core::repository::tenant::TenantRepositoryImpl;
   use auth9_core::repository::TenantRepository;
   ```

2. **Import domain types for inputs**:
   ```rust
   use auth9_core::domain::{CreateTenantInput, UpdateTenantInput, TenantStatus};
   ```

3. **Use `*entity.id` to extract UUID from StringUuid**:
   ```rust
   let input = CreatePermissionInput {
       service_id: *service.id,  // Dereference StringUuid
       // ...
   };
   ```

4. **Test both success and edge cases**:
   - Create → find → verify fields
   - Update → verify changes
   - Delete → verify not found
   - Query with filters and pagination

### Cross-Entity Test Helper

For tests needing related entities (e.g., RBAC needs tenant + user + service):

```rust
async fn setup_test_entities(pool: &MySqlPool) 
    -> (Tenant, User, Service) 
{
    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo.create(&CreateTenantInput {
        name: "Test Tenant".to_string(),
        slug: format!("test-{}", Uuid::new_v4()),
        logo_url: None,
        settings: None,
    }).await.unwrap();

    // ... create user and service similarly

    (tenant, user, service)
}
```

---

## Troubleshooting

- **Network errors in tarpaulin**: Add `required_permissions: ["network"]`
- **Testcontainers startup fails**: Ensure Docker is running
- **Compilation errors**: Run `cargo clean` first
