# Test Coverage Skill

Check and run tests with coverage analysis for Auth9 project.

## When to Use

Use this when:
- Running unit tests
- Running integration tests
- Checking test coverage
- Writing new repository/service tests
- Verifying test health before commits

## Backend (auth9-core)

### Run Unit Tests

```bash
cd auth9-core
cargo test --lib
```

Fast tests with no external dependencies.

### Run Integration Tests

```bash
cd auth9-core
cargo test --test '*'
```

Requires Docker for testcontainers. Set `DATABASE_URL` to use external database.

### Run Coverage Analysis

```bash
cd auth9-core
cargo tarpaulin --ignore-config --run-types Tests --out Json --output-dir target/tarpaulin
```

Also supports HTML output:

```bash
cd auth9-core
cargo tarpaulin --out Html --output-dir target/coverage
```

## Frontend (auth9-portal)

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
| `get_test_pool()` | Get MySQL connection (testcontainers with unique DB per test) |
| `setup_database(&pool)` | Run migrations on test database |
| `cleanup_database(&pool)` | Clear test data (optional, each test gets unique DB) |

**Important**: testcontainers MySQL root user has **NO PASSWORD** by default:
```rust
// Correct: no password
let root_url = format!("mysql://root@127.0.0.1:{}/mysql", port);

// Wrong: will fail authentication
let root_url = format!("mysql://root:password@127.0.0.1:{}/mysql", port);
```

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

## Coverage Health Check

Expected coverage targets:
- **Domain/Business logic**: 95%+
- **Service layer**: 90%+
- **Repository layer**: 85%+
- **API handlers**: 80%+

Use tarpaulin JSON output to check:

```bash
cd auth9-core
cargo tarpaulin --ignore-config --run-types Tests --out Json --output-dir target/tarpaulin
cat target/tarpaulin/tarpaulin-report.json | jq '.files | to_entries[] | {file: .key, coverage: .value.coverage}'
```

---

## API Integration Tests

### Test Infrastructure

API tests use `TestApp` from `tests/common/mod.rs` which provides:
- Full HTTP server with Axum router
- Isolated database per test (testcontainers)
- WireMock server for external service mocking
- Helper methods for HTTP requests

### Test Pattern

```rust
use crate::common::TestApp;
use wiremock::{Mock, ResponseTemplate, matchers::{method, path_regex}};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_api_endpoint() {
    let app = TestApp::spawn().await;
    let client = app.http_client();

    // Setup Keycloak mocks if needed (see below)

    // Make API request
    let response = client
        .post(&app.api_url("/api/v1/tenants"))
        .json(&json!({"name": "Test", "slug": "test"}))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
}
```

### Keycloak Mocking Pattern

**All Service/User operations require Keycloak mocks**. Use this complete pattern:

```rust
// 1. Admin Token Mock (required for all Keycloak operations)
Mock::given(method("POST"))
    .and(path_regex("/realms/master/protocol/openid-connect/token.*"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "access_token": "mock-admin-token",
        "expires_in": 36000,  // Long expiry to avoid token refresh during test
        "refresh_token": "mock-refresh-token",
        "token_type": "bearer"
    })))
    .mount(&app.mock_server)
    .await;

// 2. Create OIDC Client Mock (for Service creation)
let mock_client_uuid = "mock-client-uuid-123";
Mock::given(method("POST"))
    .and(path_regex("/admin/realms/.*/clients"))
    .respond_with(ResponseTemplate::new(201).insert_header(
        "Location",
        format!("{}/admin/realms/test/clients/{}", app.mock_server.uri(), mock_client_uuid)
    ))
    .mount(&app.mock_server)
    .await;

// 3. Get Client Secret Mock (for Service creation)
Mock::given(method("GET"))
    .and(path_regex("/admin/realms/.*/clients/.*/client-secret"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "type": "secret",
        "value": "mock-client-secret"
    })))
    .mount(&app.mock_server)
    .await;

// 4. Create User Mock (for User operations)
let mock_user_id = "mock-keycloak-user-uuid";
Mock::given(method("POST"))
    .and(path("/admin/realms/test/users"))
    .respond_with(ResponseTemplate::new(201).insert_header(
        "Location",
        format!("{}/admin/realms/test/users/{}", app.mock_server.uri(), mock_user_id)
    ))
    .mount(&app.mock_server)
    .await;

// 5. Update/Delete User Mocks (if needed)
Mock::given(method("PUT"))
    .and(path(format!("/admin/realms/test/users/{}", mock_user_id)))
    .respond_with(ResponseTemplate::new(204))
    .mount(&app.mock_server)
    .await;

Mock::given(method("DELETE"))
    .and(path(format!("/admin/realms/test/users/{}", mock_user_id)))
    .respond_with(ResponseTemplate::new(204))
    .mount(&app.mock_server)
    .await;
```

**Key Points**:
- Always use `path_regex` instead of exact `path` for dynamic paths (realms, client IDs)
- Set long token expiry (36000s = 10 hours) to avoid mid-test refresh
- Mock responses must match Keycloak's exact format (e.g., secret response has `type` and `value` fields)
- Use `.named("mock_name")` for easier debugging

### API Test File Structure

```
auth9-core/tests/
├── common/mod.rs           # TestApp and helpers
├── health_api_test.rs      # Health & readiness endpoints
├── tenant_api_test.rs      # Tenant CRUD + validation
├── user_api_test.rs        # User CRUD + tenant association + MFA
├── service_api_test.rs     # Service/Client CRUD + secret management
├── role_api_test.rs        # Role/Permission CRUD + assignment
├── audit_api_test.rs       # Audit log querying + pagination
└── auth_api_test.rs        # OIDC discovery + authorization
```

### Writing API Tests Best Practices

1. **Test Complete Flows**: Create → Read → Update → Delete → Verify
2. **Test Error Cases**: 404 not found, 409 conflict, validation errors
3. **Test Pagination**: Create multiple items, verify page/per_page work
4. **Use Unique IDs**: Always use `uuid::Uuid::new_v4()` for slugs/identifiers
5. **Assert Response Structure**: Check both status code and response body fields
6. **Clean Test Data**: Each test gets isolated database, no manual cleanup needed

---

## Troubleshooting

- **Network errors in tarpaulin**: Add `required_permissions: ["network"]` in tarpaulin.toml
- **Testcontainers startup fails**: Ensure Docker is running
- **Compilation errors**: Run `cargo clean` first
- **Database connection timeout**: Increase timeout in test or check if TiDB is running
- **Keycloak mock not matching**: Use `path_regex` instead of exact `path`, check response format matches Keycloak exactly
- **502 Bad Gateway in Service tests**: Missing Keycloak admin token or OIDC client creation mock
- **Authentication denied (MySQL)**: testcontainers uses root with NO password, remove `:password` from connection string
