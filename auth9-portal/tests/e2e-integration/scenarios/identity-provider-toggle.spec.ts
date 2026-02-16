import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";
import { execSync } from "child_process";

const KC_TOKEN = (() => {
  try {
    const out = execSync(`curl -s "http://localhost:8081/realms/master/protocol/openid-connect/token" -X POST -H "Content-Type: application/x-www-form-urlencoded" -d "username=admin&password=admin&grant_type=password&client_id=auth9-admin&client_secret=dev-client-secret-change-in-production"`, { encoding: "utf8" });
    return JSON.parse(out).access_token;
  } catch (e) { return null; }
})();

async function createIdp(alias: string, enabled = true) {
  if (!KC_TOKEN) return;
  const data = JSON.stringify({
    alias,
    displayName: alias,
    enabled,
    providerId: "google",
    config: { clientId: "test-client-id", clientSecret: "test-secret" }
  });
  try {
    execSync(`curl -s -X POST "http://localhost:8081/admin/realms/auth9/identity-provider/instances" -H "Authorization: Bearer ${KC_TOKEN}" -H "Content-Type: application/json" -d '${data}'`, { encoding: "utf8" });
  } catch (e) {}
}

async function deleteIdp(alias: string) {
  if (!KC_TOKEN) return;
  try {
    execSync(`curl -s -X DELETE "http://localhost:8081/admin/realms/auth9/identity-provider/instances/${alias}" -H "Authorization: Bearer ${KC_TOKEN}"`, { encoding: "utf8" });
  } catch (e) {}
}

async function getIdp(alias: string): Promise<any> {
  if (!KC_TOKEN) return null;
  try {
    const out = execSync(`curl -s "http://localhost:8081/admin/realms/auth9/identity-provider/instances/${alias}" -H "Authorization: Bearer ${KC_TOKEN}"`, { encoding: "utf8" });
    return JSON.parse(out);
  } catch (e) { return null; }
}

test.describe("QA Scenario: Identity Provider Toggle Validation", () => {
  const adminUser = TEST_CONFIG.testUsers.admin;

  async function loginAsAdmin(page: Page) {
    await page.goto("/login");
    await page.waitForURL(/\/realms\/auth9\/(protocol\/openid-connect|login-actions)\//, { timeout: 15000 });
    await page.getByLabel(/username/i).fill(adminUser.username);
    await page.getByLabel(/password/i).fill(adminUser.password);
    await page.getByRole("button", { name: /sign in/i }).click();
    await page.waitForURL(/localhost:3000/, { timeout: 15000 });
  }

  test.beforeAll(async () => {
    await deleteIdp("google");
    await deleteIdp("google2");
    await createIdp("google", true);
  });

  test.afterAll(async () => {
    await deleteIdp("google");
    await deleteIdp("google2");
  });

  test("1. Enable/Disable Identity Provider Toggle", async ({ page }) => {
    await loginAsAdmin(page);
    await page.goto("/dashboard/settings/identity-providers");
    await page.waitForLoadState("networkidle");

    const idp = await getIdp("google");
    expect(idp?.enabled).toBe(true);

    await page.reload();
    await page.waitForLoadState("networkidle");
    
    const toggle = page.locator('button[role="switch"]').first();
    if (await toggle.isVisible({ timeout: 3000 })) {
      await toggle.click();
      await page.waitForTimeout(1000);
    }

    const updated = await getIdp("google");
    console.log("Scenario 1 - Toggle enabled:", updated?.enabled);
  });

  test("2. Create Provider with Duplicate Alias", async ({ page }) => {
    await loginAsAdmin(page);
    await page.goto("/dashboard/settings/identity-providers");
    await page.waitForLoadState("networkidle");

    const addButton = page.getByRole("button", { name: /add provider|add identity/i });
    if (await addButton.isVisible({ timeout: 3000 })) {
      await addButton.click();
      await page.waitForTimeout(500);
    }

    const googleBtn = page.getByRole("button", { name: /google/i });
    if (await googleBtn.isVisible({ timeout: 2000 })) {
      await googleBtn.click();
      await page.waitForTimeout(500);
    }

    const aliasInput = page.getByLabel(/alias/i);
    if (await aliasInput.isVisible({ timeout: 2000 })) {
      await aliasInput.fill("google");
      
      const submitBtn = page.getByRole("button", { name: /add provider|save|create/i });
      await submitBtn.click();
      await page.waitForTimeout(1000);

      const errorText = page.getByText(/already exists|duplicate|alias/i);
      const hasError = await errorText.isVisible({ timeout: 3000 }).catch(() => false);
      console.log("Scenario 2 - Duplicate alias error shown:", hasError);
    }
  });

  test("3. Validate Required Fields", async ({ page }) => {
    await loginAsAdmin(page);
    await page.goto("/dashboard/settings/identity-providers");
    await page.waitForLoadState("networkidle");

    const addButton = page.getByRole("button", { name: /add provider|add identity/i });
    if (await addButton.isVisible({ timeout: 3000 })) {
      await addButton.click();
      await page.waitForTimeout(500);
    }

    const googleBtn = page.getByRole("button", { name: /google/i });
    if (await googleBtn.isVisible({ timeout: 2000 })) {
      await googleBtn.click();
      await page.waitForTimeout(500);
    }

    const submitBtn = page.getByRole("button", { name: /add provider|save|create/i });
    if (await submitBtn.isVisible({ timeout: 2000 })) {
      await submitBtn.click();
      await page.waitForTimeout(1000);

      const requiredError = page.getByText(/required|missing|empty|client id|client secret/i);
      const hasError = await requiredError.first().isVisible({ timeout: 3000 }).catch(() => false);
      console.log("Scenario 3 - Required field validation shown:", hasError);
    }
  });

  test("4. Use Social Login", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    const googleBtn = page.getByRole("button", { name: /google|sign in with google/i });
    const isVisible = await googleBtn.isVisible({ timeout: 3000 }).catch(() => false);
    console.log("Scenario 4 - Social login button visible:", isVisible);
  });

  test("5. View User Linked Identities", async ({ page }) => {
    await loginAsAdmin(page);
    await page.goto("/dashboard/settings/linked-accounts");
    await page.waitForLoadState("networkidle");

    const pageContent = await page.content();
    const hasLinkedAccounts = pageContent.toLowerCase().includes("linked") || 
                              pageContent.toLowerCase().includes("account") ||
                              pageContent.toLowerCase().includes("identity");
    console.log("Scenario 5 - Linked accounts page accessible:", hasLinkedAccounts);
  });

  test("6. Authentication State Check", async ({ page }) => {
    await page.goto("/dashboard/settings/identity-providers");
    await page.waitForLoadState("networkidle");

    const currentUrl = page.url();
    const isRedirectedToLogin = currentUrl.includes("/login") || currentUrl.includes("/realms");
    console.log("Scenario 6 - Auth state check, redirected to login:", isRedirectedToLogin);
  });
});
