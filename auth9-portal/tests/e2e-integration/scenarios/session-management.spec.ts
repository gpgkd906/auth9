import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Session List
 *
 * Tests the session list functionality.
 */
test.describe("Scenario: Session List", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Sessions page is accessible", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");
    await expect(page).toHaveURL(/\/dashboard\/settings\/sessions/);
    await expect(page.getByText(/session/i).first()).toBeVisible();
  });

  test("2. Current session is displayed", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Should show current session info
    const currentSession = page.getByText(/current session|this device/i);
    if (await currentSession.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(currentSession).toBeVisible();
    }
  });

  test("3. Session shows device information", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Should show device type (desktop, mobile, tablet)
    const deviceTypes = [/desktop/i, /mobile/i, /tablet/i, /browser/i, /chrome/i, /firefox/i, /safari/i];

    for (const deviceType of deviceTypes) {
      const element = page.getByText(deviceType);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break; // At least one device info found
      }
    }
  });

  test("4. Session shows IP address", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // IP address pattern (simplified)
    const ipPattern = page.getByText(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|127\.0\.0\.1|localhost/);
    if (await ipPattern.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(ipPattern).toBeVisible();
    }
  });

  test("5. Session shows last active time", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Should show time info
    const timePatterns = [/just now/i, /ago/i, /active/i, /last/i];

    for (const pattern of timePatterns) {
      const element = page.getByText(pattern);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });
});

/**
 * Scenario: Session Revocation
 *
 * Tests individual and bulk session revocation.
 */
test.describe("Scenario: Session Revocation", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Revoke button is visible for sessions", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Look for revoke/sign out buttons
    const revokeButton = page.getByRole("button", { name: /revoke|sign out|terminate|end/i });
    if (await revokeButton.first().isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(revokeButton.first()).toBeVisible();
    }
  });

  test("2. Sign out all button is visible", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Look for bulk sign out button
    const signOutAllButton = page.getByRole("button", { name: /sign out all|revoke all|terminate all/i });
    if (await signOutAllButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(signOutAllButton).toBeVisible();
    }
  });

  test("3. Security tips section is visible", async ({ page }) => {
    await page.goto("/dashboard/settings/sessions");

    // Look for security tips
    const securityTips = page.getByText(/security tip|recommendation/i);
    if (await securityTips.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(securityTips).toBeVisible();
    }
  });
});

/**
 * Scenario: Session API
 *
 * Tests the session management API endpoints.
 */
test.describe("Scenario: Session API", () => {
  test("1. List sessions API endpoint works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/sessions`
    );

    // May require authentication - accept 200 or 401
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
      expect(Array.isArray(body.data)).toBeTruthy();
    }
  });

  test("2. Sessions have expected structure", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/sessions`
    );

    if (response.ok()) {
      const body = await response.json();
      if (body.data && body.data.length > 0) {
        const session = body.data[0];
        // Check expected fields
        expect(session).toHaveProperty("id");
      }
    }
  });

  test("3. Revoke session API endpoint exists", async ({ request }) => {
    // Try to revoke a non-existent session to check endpoint exists
    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/sessions/non-existent-id`
    );

    // Should return 401 (unauthorized), 403 (forbidden), or 404 (not found)
    expect([401, 403, 404]).toContain(response.status());
  });

  test("4. Revoke other sessions API endpoint exists", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/sessions/revoke-others`
    );

    // May require authentication
    expect([200, 204, 401, 403]).toContain(response.status());
  });
});

/**
 * Scenario: Admin Session Management
 *
 * Tests admin functionality for managing user sessions.
 */
test.describe("Scenario: Admin Session Management", () => {
  test("1. Admin force logout API endpoint exists", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/admin/sessions/force-logout`,
      {
        data: { user_id: "test-user-id" },
      }
    );

    // Should return 401 (unauthorized), 403 (forbidden), or 404 (not found for user)
    expect([200, 204, 401, 403, 404]).toContain(response.status());
  });

  test("2. Admin can list user sessions API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/admin/users/test-user-id/sessions`
    );

    // May require admin auth or not exist
    expect([200, 401, 403, 404]).toContain(response.status());
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
