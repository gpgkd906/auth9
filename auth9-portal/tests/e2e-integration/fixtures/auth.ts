import { type Page } from "@playwright/test";
import { TEST_CONFIG, type TestUser } from "../setup/test-config";

/**
 * Login as a test user via Keycloak
 */
export async function loginAsTestUser(
  page: Page,
  user: TestUser = TEST_CONFIG.testUsers.standard
): Promise<void> {
  await page.goto("/login");
  await page.getByRole("button", { name: /sign in/i }).click();
  await page.waitForURL(/\/realms\/auth9\/protocol\/openid-connect/);

  await page.getByLabel(/username/i).fill(user.username);
  await page.getByLabel(/password/i).fill(user.password);
  await page.getByRole("button", { name: /sign in/i }).click();

  await page.waitForURL(/localhost:3000/, { timeout: 15000 });
}

/**
 * Logout from the application
 */
export async function logout(page: Page): Promise<void> {
  // Navigate to logout or click logout button
  await page.goto("/api/v1/auth/logout");
}

/**
 * Check if user is logged in
 */
export async function isLoggedIn(page: Page): Promise<boolean> {
  await page.goto("/dashboard");
  return page.url().includes("/dashboard");
}
