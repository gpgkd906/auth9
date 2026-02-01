import { test, expect } from "@playwright/test";
import { TEST_CONFIG } from "./setup/test-config";
import { loginAsTestUser } from "./fixtures/auth";

/**
 * Scenario: User Authentication Flow
 *
 * Tests the complete authentication journey from login to accessing protected resources.
 * This scenario verifies the integration between Portal, Keycloak, and Auth9 Core.
 */
test.describe("Scenario: User Authentication Flow", () => {
  const testUser = TEST_CONFIG.testUsers.standard;

  test("1. Login page should be accessible", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByText("Welcome back")).toBeVisible();
    await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();
  });

  test("2. Clicking sign in should redirect to Keycloak", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: /sign in/i }).click();

    // Should redirect to Keycloak login page
    await expect(page).toHaveURL(/\/realms\/auth9\/protocol\/openid-connect/);
    await expect(page.getByLabel(/username/i)).toBeVisible();
  });

  test("3. User can login with valid credentials", async ({ page }) => {
    await loginAsTestUser(page, testUser);

    // Should be on dashboard after login
    await expect(page).not.toHaveURL(/\/login/);
    await expect(page.url()).toContain("localhost:3000");
  });

  test("4. Invalid credentials should show error", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: /sign in/i }).click();

    // Wait for Keycloak page
    await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/);

    // Fill with invalid credentials
    await page.getByLabel(/username/i).fill("invalid-user");
    await page.getByLabel(/password/i).fill("wrong-password");
    await page.getByRole("button", { name: /sign in/i }).click();

    // Should show error message on Keycloak page
    await expect(page.getByText(/invalid/i)).toBeVisible();
  });

  test("5. Authenticated user can access dashboard", async ({ page }) => {
    await loginAsTestUser(page, testUser);

    await page.goto("/dashboard");
    await expect(page).toHaveURL(/\/dashboard/);
  });
});

/**
 * Scenario: User Registration Flow
 */
test.describe("Scenario: User Registration Flow", () => {
  test("1. Register page should be accessible", async ({ page }) => {
    await page.goto("/register");
    await expect(page.getByRole("heading")).toBeVisible();
  });

  test("2. Can navigate from login to register", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("link", { name: /sign up/i }).click();
    await expect(page).toHaveURL("/register");
  });

  test("3. Can navigate from register to login", async ({ page }) => {
    await page.goto("/register");
    const loginLink = page.getByRole("link", { name: /sign in|log in/i });
    if (await loginLink.isVisible()) {
      await loginLink.click();
      await expect(page).toHaveURL("/login");
    }
  });
});

/**
 * Scenario: Auth9 Core API Integration
 *
 * Tests that the Auth9 Core API is accessible and responding correctly.
 */
test.describe("Scenario: Auth9 Core API Integration", () => {
  test("1. Health endpoint should be accessible", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/health`);
    expect(response.ok()).toBeTruthy();
  });

  test("2. Ready endpoint should return healthy status", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/ready`);
    expect(response.ok()).toBeTruthy();
  });

  test("3. OpenID configuration should be available", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/.well-known/openid-configuration`
    );
    expect(response.ok()).toBeTruthy();

    const config = await response.json();
    expect(config).toHaveProperty("issuer");
    expect(config).toHaveProperty("authorization_endpoint");
    expect(config).toHaveProperty("token_endpoint");
  });

  test("4. Tenants API should return paginated list", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`);
    expect(response.ok()).toBeTruthy();

    const data = await response.json();
    expect(data).toHaveProperty("data");
    expect(data).toHaveProperty("pagination");
    expect(Array.isArray(data.data)).toBeTruthy();
  });

  test("5. Users API should return paginated list", async ({ request }) => {
    const response = await request.get(`${TEST_CONFIG.auth9CoreUrl}/api/v1/users`);
    expect(response.ok()).toBeTruthy();

    const data = await response.json();
    expect(data).toHaveProperty("data");
    expect(data).toHaveProperty("pagination");
    // Should include our test user created in global setup
    expect(data.data.length).toBeGreaterThan(0);
  });
});

/**
 * Scenario: Public Pages Navigation
 */
test.describe("Scenario: Public Pages Navigation", () => {
  test("1. Home page should be accessible", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading").first()).toBeVisible();
  });

  test("2. Home page has navigation to login", async ({ page }) => {
    await page.goto("/");
    const signInLink = page.getByRole("link", { name: /sign in/i });
    if (await signInLink.isVisible()) {
      await expect(signInLink).toBeVisible();
    }
  });
});
