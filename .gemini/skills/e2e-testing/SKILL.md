---
name: e2e-testing
description: Run E2E tests for Auth9 portal using Playwright. Use when testing authentication flow, frontend-backend integration, black-box testing, or when the user asks to run E2E tests, integration tests, or end-to-end tests. Always use test:e2e:full:reset to ensure clean environment.
---

# E2E Testing for Auth9 Portal

## Test Strategy Overview

Auth9 uses a **hybrid E2E testing approach**:

| Type | Directory | Target | Requirements | Purpose |
|------|-----------|--------|--------------|---------|
| **Frontend isolation** | `tests/e2e/` | localhost:5173 (Vite dev) | Vite dev only | UI rendering, navigation |
| **Full-stack integration** | `tests/e2e-integration/` | localhost:3000 (Production) | Docker + all services | Login flow, API integration |

## Critical Rule: Always Reset Before Full-Stack Tests

**MANDATORY**: Always use `test:e2e:full:reset` when running full-stack integration tests.

```bash
cd auth9-portal
npm run test:e2e:full:reset
```

**Why this is required:**
- E2E tests use fixed test users (e2e-test-user, e2e-admin-user)
- Docker environment may have dirty data from previous runs
- Reset ensures clean TiDB, Redis, Keycloak state
- Prevents flaky tests due to leftover sessions or users

**What it does:**
1. Stops all Docker containers
2. Removes project images and volumes
3. Rebuilds Keycloak theme
4. Rebuilds all images
5. Starts fresh services
6. Waits for services to be healthy
7. Runs full-stack E2E tests

**Expected runtime**: 3-5 minutes (including Docker rebuild)

## Test Commands

### Full-Stack Integration Tests (Recommended)

```bash
cd auth9-portal

# Reset environment + run full-stack tests (USE THIS)
npm run test:e2e:full:reset

# Run full-stack tests only (requires services already running)
npm run test:e2e:full

# Run with UI mode for debugging
npm run test:e2e:full -- --ui

# Run specific test file
npm run test:e2e:full tests/e2e-integration/auth-flow.spec.ts

# Run tests in headed mode (see browser)
npm run test:e2e:full -- --headed

# Debug mode (pause before each action)
npm run test:e2e:full -- --debug
```

### Frontend Isolation Tests (Fast)

```bash
cd auth9-portal

# Run frontend-only tests (no Docker needed)
npm run test:e2e

# Run with UI mode
npm run test:e2e:ui

# Run in headed mode
npm run test:e2e:headed
```

## Test File Locations

```
auth9-portal/tests/
├── e2e/                        # Frontend isolation tests
│   └── login.spec.ts          # UI rendering, navigation
│
└── e2e-integration/            # Full-stack integration tests
    ├── auth-flow.spec.ts      # Scenario-based tests
    ├── global-setup.ts        # Setup test users in Keycloak
    ├── global-teardown.ts     # Cleanup after tests
    └── setup/
        ├── test-config.ts     # Test configuration
        └── keycloak-admin.ts  # Keycloak Admin API client
```

## Test Configuration

All test configurations are in `tests/e2e-integration/setup/test-config.ts`:

```typescript
export const TEST_CONFIG = {
  // Service URLs
  portalUrl: "http://localhost:3000",
  auth9CoreUrl: "http://localhost:8080",
  keycloakUrl: "http://localhost:8081",
  keycloakRealm: "auth9",

  // Test Users (created during global setup)
  testUsers: {
    standard: {
      username: "e2e-test-user",
      email: "e2e-test@example.com",
      password: "Test123!",
    },
    admin: {
      username: "e2e-admin-user",
      email: "e2e-admin@example.com",
      password: "Admin123!",
    },
  },
};
```

## Writing New E2E Tests

### Scenario-Based Test Pattern

Organize tests as user scenarios with numbered steps:

```typescript
import { test, expect } from "@playwright/test";
import { TEST_CONFIG } from "./setup/test-config";

test.describe("Scenario: Feature Name", () => {
  const testUser = TEST_CONFIG.testUsers.standard;

  test("1. First step description", async ({ page }) => {
    await page.goto("/some-page");
    await expect(page.getByText("Expected content")).toBeVisible();
  });

  test("2. Second step description", async ({ page }) => {
    // Continue the scenario
  });
});
```

### API Integration Test Pattern

Test backend API endpoints:

```typescript
test.describe("Scenario: API Integration", () => {
  test("1. Health endpoint should be accessible", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/health`);
    expect(response.ok()).toBeTruthy();
  });

  test("2. API should return paginated data", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`);
    expect(response.ok()).toBeTruthy();
    
    const data = await response.json();
    expect(data).toHaveProperty("data");
    expect(data).toHaveProperty("pagination");
  });
});
```

### Authentication Flow Pattern

Test login/logout flows:

```typescript
test.describe("Scenario: User Login", () => {
  const testUser = TEST_CONFIG.testUsers.standard;

  test("1. User can login with valid credentials", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: /sign in/i }).click();

    // Wait for Keycloak redirect
    await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/);

    // Fill login form
    await page.getByLabel(/username/i).fill(testUser.username);
    await page.getByLabel(/password/i).fill(testUser.password);
    await page.getByRole("button", { name: /sign in/i }).click();

    // Should redirect back to app
    await page.waitForURL(/localhost:3000/, { timeout: 15000 });
    await expect(page).not.toHaveURL(/\/login/);
  });
});
```

## Test Data Preparation

Global setup (`tests/e2e-integration/global-setup.ts`) prepares test data:

```typescript
// 1. Wait for all services to be ready
await waitForServices();

// 2. Create test users via Keycloak Admin API
const keycloak = new KeycloakAdminClient();
await keycloak.authenticate();
await keycloak.setupTestUsers();
```

**Test users are created fresh on every reset:**
- Environment is reset before tests via `test:e2e:full:reset`
- Global setup creates users in clean Keycloak instance
- No need to check if users exist - they are always created fresh

## Troubleshooting

### Services Not Ready

If tests fail with "Service not ready" errors:

```bash
# Check service logs
docker-compose logs auth9-core
docker-compose logs auth9-portal
docker-compose logs keycloak

# Check service status
docker-compose ps

# Manually reset environment
./scripts/reset-docker.sh
```

### Test Failures Due to Dirty Data

If tests pass after reset but fail on subsequent runs:

```bash
# Always use reset command
npm run test:e2e:full:reset

# Check for leftover test data
docker exec -it auth9-tidb mysql -u root -p -e "USE auth9; SELECT * FROM users WHERE email LIKE 'e2e-%';"
```

### Keycloak Authentication Issues

If login tests fail:

```bash
# Verify Keycloak is using auth9 theme
# Go to: http://localhost:8081/admin/master/console
# Realm Settings > Themes > Login Theme: auth9

# Check Keycloak realm configuration
curl http://localhost:8081/realms/auth9/.well-known/openid-configuration

# Verify test user exists in Keycloak
# Keycloak Admin > Users > Search for "e2e-test-user"
```

### Browser Debugging

Run tests with UI mode to see what's happening:

```bash
# Full-stack tests with UI
npm run test:e2e:full -- --ui

# Run in headed mode (see actual browser)
npm run test:e2e:full -- --headed

# Debug mode (pause before each action)
npm run test:e2e:full -- --debug
```

## Test Reports

After running tests:

```bash
# View HTML report (frontend isolation tests)
npx playwright show-report playwright-report

# View HTML report (full-stack tests)
npx playwright show-report playwright-report-full
```

Reports include:
- Test results summary
- Screenshots on failure
- Videos on failure
- Trace files for debugging

## CI/CD Integration

In CI environment, tests automatically:
- Run with 2 retries on failure
- Use single worker (sequential execution)
- Skip `only` tests
- Generate HTML report
- Capture traces on first retry

Environment detection via `process.env.CI`:

```typescript
export default defineConfig({
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
});
```
