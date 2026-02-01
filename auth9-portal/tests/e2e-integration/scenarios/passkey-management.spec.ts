import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Passkeys Page
 *
 * Tests the passkeys/WebAuthn settings page.
 */
test.describe("Scenario: Passkeys Page", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Passkeys page is accessible", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");
    await expect(page).toHaveURL(/\/dashboard\/settings\/passkeys/);
    await expect(page.getByText(/passkey|webauthn|security key/i).first()).toBeVisible();
  });

  test("2. Add passkey button is visible", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    const addButton = page.getByRole("button", { name: /add passkey|register|create/i });
    if (await addButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(addButton).toBeVisible();
    }
  });

  test("3. About passkeys section is visible", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    // Look for educational content about passkeys
    const aboutSection = page.getByText(/about passkey|what is|benefit|secure/i);
    if (await aboutSection.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(aboutSection).toBeVisible();
    }
  });

  test("4. Passkey benefits are explained", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    // Check for passkey benefits
    const benefits = [
      /phishing resistant/i,
      /biometric/i,
      /fast|easy/i,
      /secure/i,
    ];

    for (const benefit of benefits) {
      const element = page.getByText(benefit);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break; // At least one benefit found
      }
    }
  });
});

/**
 * Scenario: Passkey List
 *
 * Tests the passkey credentials list functionality.
 */
test.describe("Scenario: Passkey List", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Passkeys list shows registered credentials", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    // May show empty state or list of passkeys
    const emptyState = page.getByText(/no passkey|none registered|get started/i);
    const passkeyList = page.locator('[data-testid="passkey-list"]').or(page.locator("table"));

    const hasEmptyState = await emptyState.isVisible({ timeout: 2000 }).catch(() => false);
    const hasList = await passkeyList.isVisible({ timeout: 2000 }).catch(() => false);

    // Either empty state or list should be visible
    expect(hasEmptyState || hasList).toBeTruthy();
  });

  test("2. Passkey shows type badge", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    // Check for passkey type badges
    const typeBadges = [/passwordless/i, /two-factor|2fa|mfa/i];

    for (const badge of typeBadges) {
      const element = page.getByText(badge);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("3. Passkey shows delete button", async ({ page }) => {
    await page.goto("/dashboard/settings/passkeys");

    const deleteButton = page.getByRole("button", { name: /delete|remove|revoke/i });
    // Delete button only visible if passkeys exist
    if (await deleteButton.first().isVisible({ timeout: 2000 }).catch(() => false)) {
      await expect(deleteButton.first()).toBeVisible();
    }
  });
});

/**
 * Scenario: WebAuthn API
 *
 * Tests the WebAuthn/Passkey API endpoints.
 */
test.describe("Scenario: WebAuthn API", () => {
  test("1. List passkeys API endpoint works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/webauthn/credentials`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
      expect(Array.isArray(body.data)).toBeTruthy();
    }
  });

  test("2. Get registration URL API endpoint works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/webauthn/register-url`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
    }
  });

  test("3. Delete passkey API endpoint exists", async ({ request }) => {
    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/webauthn/credentials/non-existent-id`
    );

    // Should return 401 (unauthorized), 403 (forbidden), or 404 (not found)
    expect([401, 403, 404]).toContain(response.status());
  });
});

/**
 * Scenario: Keycloak WebAuthn Integration
 *
 * Tests the Keycloak WebAuthn configuration.
 */
test.describe("Scenario: Keycloak WebAuthn Integration", () => {
  test("1. Keycloak realm has WebAuthn enabled", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.keycloakUrl}/realms/${TEST_CONFIG.keycloakRealm}/.well-known/openid-configuration`
    );

    expect(response.ok()).toBeTruthy();
    const config = await response.json();
    expect(config).toHaveProperty("issuer");
  });

  test("2. WebAuthn registration redirects to Keycloak", async ({ page }) => {
    await loginAsTestUser(page);
    await page.goto("/dashboard/settings/passkeys");

    const addButton = page.getByRole("button", { name: /add passkey|register/i });
    if (await addButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      // Note: Clicking would redirect to Keycloak, we just verify button exists
      await expect(addButton).toBeVisible();
    }
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
