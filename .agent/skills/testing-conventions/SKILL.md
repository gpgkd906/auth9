---
name: testing-conventions
description: Testing conventions for Auth9 project
globs: "**/*test*.{rs,ts,tsx}"
alwaysApply: false
---

# Testing Conventions

## Test Pyramid

```
        E2E Tests (few, critical flows)
              /\
             /  \
    Integration Tests (API, gRPC, DB)
           /      \
          /        \
     Unit Tests (business logic core)
```

## auth9-core (Rust)

### Unit Tests

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

### Integration Tests

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

### Commands

```bash
cargo test --lib        # Unit tests (fast)
cargo test --test '*'   # Integration tests (requires Docker)
cargo tarpaulin --out Html  # Coverage report
```

## auth9-portal (TypeScript)

### Unit Tests (Vitest)

```typescript
// Location: tests/unit/*.test.ts
import { describe, it, expect } from 'vitest';

describe('utils', () => {
  it('should format date correctly', () => {
    expect(formatDate(new Date('2024-01-01'))).toBe('Jan 1, 2024');
  });
});
```

### E2E Tests (Playwright)

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

### Commands

```bash
npm run test           # Vitest unit tests
npm run lint           # ESLint
npm run typecheck      # TypeScript check
npx playwright test    # E2E tests
```

## Test Data Conventions

- Use descriptive names: `test_create_tenant_with_valid_data`
- Keep test data minimal and focused
- Clean up test data after integration tests
