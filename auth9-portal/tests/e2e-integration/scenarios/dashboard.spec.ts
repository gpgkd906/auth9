import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Dashboard Overview
 *
 * Tests the main dashboard page and navigation.
 */
test.describe("Scenario: Dashboard Overview", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Dashboard shows stats cards", async ({ page }) => {
    await page.goto("/dashboard");

    // Should display stats
    await expect(page.getByText(/tenants/i).first()).toBeVisible();
    await expect(page.getByText(/users/i).first()).toBeVisible();
    await expect(page.getByText(/services/i).first()).toBeVisible();
  });

  test("2. Dashboard sidebar navigation works", async ({ page }) => {
    await page.goto("/dashboard");

    // Test navigation links
    const navItems = [
      { name: /tenants/i, url: "/dashboard/tenants" },
      { name: /users/i, url: "/dashboard/users" },
      { name: /services/i, url: "/dashboard/services" },
      { name: /roles/i, url: "/dashboard/roles" },
      { name: /audit/i, url: "/dashboard/audit-logs" },
      { name: /settings/i, url: "/dashboard/settings" },
    ];

    for (const item of navItems) {
      const link = page.getByRole("link", { name: item.name });
      if (await link.isVisible()) {
        await link.click();
        await expect(page).toHaveURL(new RegExp(item.url));
        await page.goto("/dashboard"); // Go back
      }
    }
  });

  test("3. Dashboard shows recent activity", async ({ page }) => {
    await page.goto("/dashboard");

    // Should have activity section
    const activityHeading = page.getByRole("heading", { name: "Recent Activity" });
    await expect(activityHeading).toBeVisible();
  });
});

/**
 * Scenario: Audit Logs
 *
 * Tests audit log viewing and filtering.
 */
test.describe("Scenario: Audit Logs", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Navigate to audit logs page", async ({ page }) => {
    await page.goto("/dashboard/audit-logs");
    await expect(page.getByRole("heading", { name: "Audit Logs" })).toBeVisible();
  });

  test("2. Audit logs table is visible", async ({ page }) => {
    await page.goto("/dashboard/audit-logs");

    // Should have a table with audit entries
    const table = page.locator("table");
    await expect(table).toBeVisible();

    // Should have expected columns
    await expect(page.getByText(/action/i).first()).toBeVisible();
  });

  test("3. Audit logs API returns data", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/audit-logs?limit=10`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(Array.isArray(body.data)).toBeTruthy();
  });

  test("4. Audit logs support pagination", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/audit-logs?limit=5&offset=0`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
  });
});

/**
 * Scenario: Settings Pages
 *
 * Tests settings navigation and rendering.
 */
test.describe("Scenario: Settings Pages", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Navigate to settings page", async ({ page }) => {
    await page.goto("/dashboard/settings");
    await expect(page).toHaveURL(/\/dashboard\/settings/);
  });

  test("2. Settings branding page accessible", async ({ page }) => {
    await page.goto("/dashboard/settings/branding");
    await expect(page.getByText(/branding/i).first()).toBeVisible();
  });

  test("3. Settings email page accessible", async ({ page }) => {
    await page.goto("/dashboard/settings/email");
    await expect(page.getByText(/email/i).first()).toBeVisible();
  });

  test("4. Settings email templates page accessible", async ({ page }) => {
    await page.goto("/dashboard/settings/email-templates");
    await expect(page.getByText(/template/i).first()).toBeVisible();
  });
});

/**
 * Scenario: Branding API
 */
test.describe("Scenario: Branding API", () => {
  test("1. Get branding settings via API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/system/branding`
    );

    // May return 200 or 404 if not configured
    expect([200, 404]).toContain(response.status());
  });

  test("2. Get public branding via API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/public/branding`
    );

    // May return 200 or 404
    expect([200, 404]).toContain(response.status());
  });
});

// Helper function
async function loginAsTestUser(page: Page): Promise<void> {
  const testUser = TEST_CONFIG.testUsers.standard;

  await page.goto("/login");
  await page.getByRole("button", { name: /sign in/i }).click();
  await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/);

  await page.getByLabel(/username/i).fill(testUser.username);
  await page.getByLabel(/password/i).fill(testUser.password);
  await page.getByRole("button", { name: /sign in/i }).click();

  await page.waitForURL(/localhost:3000/, { timeout: 15000 });
}
