---
name: e2e-testing
description: Run E2E tests for Auth9 portal using Playwright with hybrid testing strategy.
---

# E2E Testing for Auth9 Portal

## Test Strategy

| Type | Directory | Target | Requirements | Purpose |
|------|-----------|--------|--------------|---------|
| Frontend isolation | `tests/e2e/` | localhost:5173 | Vite dev only | UI rendering |
| Full-stack integration | `tests/e2e-integration/` | localhost:3000 | Docker + all services | Login, API |

## Commands

```bash
cd auth9-portal

# Full-stack tests (ALWAYS use reset)
npm run test:e2e:full:reset    # Reset env + run tests (recommended)
npm run test:e2e:full          # Run only (requires services)
npm run test:e2e:full -- --ui  # With UI mode
npm run test:e2e:full -- --headed  # See browser

# Frontend isolation tests (fast, no Docker)
npm run test:e2e
npm run test:e2e:ui
```

## Critical Rule

**ALWAYS** use `test:e2e:full:reset` for full-stack tests:
- Resets Docker environment (clean TiDB, Redis, Keycloak)
- Creates fresh test users
- Prevents flaky tests from dirty data

## Test File Structure

```
auth9-portal/tests/
├── e2e/                     # Frontend isolation
│   └── login.spec.ts
└── e2e-integration/         # Full-stack
    ├── auth-flow.spec.ts
    ├── global-setup.ts      # Creates test users
    └── setup/
        ├── test-config.ts   # URLs, credentials
        └── keycloak-admin.ts
```

## Test Configuration

```typescript
// tests/e2e-integration/setup/test-config.ts
export const TEST_CONFIG = {
  portalUrl: "http://localhost:3000",
  auth9CoreUrl: "http://localhost:8080",
  keycloakUrl: "http://localhost:8081",
  testUsers: {
    standard: { username: "e2e-test-user", password: "TestPass1234!" },
    admin: { username: "e2e-admin-user", password: "SecurePass123!" },
  },
};
```

## Writing Tests

### Scenario-Based Pattern

```typescript
import { test, expect } from "@playwright/test";
import { TEST_CONFIG } from "./setup/test-config";

test.describe("Scenario: User Login", () => {
  const testUser = TEST_CONFIG.testUsers.standard;

  test("1. User can login", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL(/\/realms\/auth9/);
    await page.getByLabel(/username/i).fill(testUser.username);
    await page.getByLabel(/password/i).fill(testUser.password);
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL(/localhost:3000/);
  });
});
```

### API Test Pattern

```typescript
test("Health endpoint accessible", async ({ request }) => {
  const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/health`);
  expect(response.ok()).toBeTruthy();
});
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Services not ready | `docker-compose logs`, `./scripts/reset-docker.sh` |
| Dirty data failures | Always use `test:e2e:full:reset` |
| Login failures | Check Keycloak theme is `auth9`, verify test user exists |

## View Reports

```bash
npx playwright show-report playwright-report       # Frontend tests
npx playwright show-report playwright-report-full  # Full-stack tests
```
