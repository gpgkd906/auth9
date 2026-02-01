import { defineConfig, devices } from "@playwright/test";

/**
 * Full-stack E2E test configuration
 * Requires Docker environment to be running (docker-compose up -d)
 *
 * Usage:
 *   npm run test:e2e:full       # Run tests (requires services running)
 *   npm run test:e2e:full:reset # Reset environment and run tests
 */
export default defineConfig({
  testDir: "./tests/e2e-integration",
  fullyParallel: false, // Run tests sequentially for scenario-based tests
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Single worker for predictable test order
  reporter: [
    ["list"],
    ["html", { outputFolder: "playwright-report-full", open: "never" }],
  ],
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
  ],
  // Global setup - prepare test data via Keycloak Admin API
  globalSetup: "./tests/e2e-integration/global-setup.ts",
  globalTeardown: "./tests/e2e-integration/global-teardown.ts",
  // Timeout settings
  timeout: 30000,
  expect: {
    timeout: 10000,
  },
});
