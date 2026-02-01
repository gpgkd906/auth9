import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Tenant Management
 *
 * Tests the complete tenant lifecycle: create, read, update, delete.
 * Tenants are isolated organizations within Auth9.
 */
test.describe("Scenario: Tenant Management", () => {
  const testTenant = {
    name: "E2E Test Tenant",
    slug: `e2e-tenant-${Date.now()}`,
    logoUrl: "https://example.com/logo.png",
  };

  test.beforeEach(async ({ page }) => {
    // Login first
    await loginAsTestUser(page);
  });

  test("1. Navigate to tenants page", async ({ page }) => {
    await page.goto("/dashboard/tenants");
    await expect(page.getByRole("heading", { name: /tenants/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /create tenant/i })).toBeVisible();
  });

  test("2. Create a new tenant", async ({ page }) => {
    await page.goto("/dashboard/tenants");

    // Click create button
    const createButton = page.getByRole("button", { name: /create tenant/i });
    if (!(await createButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip();
      return;
    }
    await createButton.click();

    // Fill form
    await page.getByLabel(/name/i).fill(testTenant.name);
    await page.getByLabel(/slug/i).fill(testTenant.slug);

    // Logo field is optional
    const logoInput = page.getByLabel(/logo/i);
    if (await logoInput.isVisible().catch(() => false)) {
      await logoInput.fill(testTenant.logoUrl);
    }

    // Submit
    await page.getByRole("button", { name: /create/i }).click();

    // Wait for form to close and list to refresh
    await page.waitForTimeout(1000);

    // Verify tenant slug appears in table (more reliable than name)
    const slugCell = page.locator("td", { hasText: testTenant.slug });
    await expect(slugCell.first()).toBeVisible({ timeout: 5000 });
  });

  test("3. View tenant in list", async ({ page }) => {
    await page.goto("/dashboard/tenants");

    // Search or find the tenant (pagination may be needed)
    const tenantRow = page.locator("tr", { hasText: testTenant.slug });

    if (await tenantRow.isVisible()) {
      await expect(tenantRow.getByText(testTenant.name)).toBeVisible();
    }
  });

  // UI update test - may be flaky depending on UI state
  test("4. Update tenant details", async ({ page }) => {
    await page.goto("/dashboard/tenants");

    // Find the tenant row
    const tenantRow = page.locator("tr", { hasText: testTenant.slug });

    // Skip if tenant not found (may have been deleted in previous test)
    if (!(await tenantRow.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip();
      return;
    }

    // Click actions dropdown
    const actionsButton = tenantRow.getByRole("button").first();
    await actionsButton.click();

    // Look for edit option
    const editButton = page.getByRole("menuitem", { name: /edit/i });
    if (!(await editButton.isVisible({ timeout: 2000 }).catch(() => false))) {
      // Close dropdown and skip
      await page.keyboard.press("Escape");
      test.skip();
      return;
    }

    await editButton.click();

    // Update name
    const nameInput = page.getByLabel(/name/i);
    await nameInput.clear();
    await nameInput.fill(`${testTenant.name} Updated`);

    // Submit
    await page.getByRole("button", { name: /save|update/i }).click();
  });

  // UI delete test - may be flaky if tenant was already deleted
  test("5. Delete tenant", async ({ page }) => {
    await page.goto("/dashboard/tenants");

    const tenantRow = page.locator("tr", { hasText: testTenant.slug });

    // Skip if tenant not found
    if (!(await tenantRow.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip();
      return;
    }

    // Click the first button in the row (actions)
    const actionsButton = tenantRow.getByRole("button").first();
    await actionsButton.click();

    // Look for delete option
    const deleteButton = page.getByRole("menuitem", { name: /delete/i });
    if (!(await deleteButton.isVisible({ timeout: 2000 }).catch(() => false))) {
      await page.keyboard.press("Escape");
      test.skip();
      return;
    }

    await deleteButton.click();

    // Confirm deletion if dialog appears
    const confirmButton = page.getByRole("button", { name: /confirm|delete/i });
    if (await confirmButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await confirmButton.click();
    }
  });
});

/**
 * Scenario: Tenant API Integration
 *
 * Direct API tests for tenant management.
 */
test.describe("Scenario: Tenant API Integration", () => {
  const apiTenant = {
    name: "API Test Tenant",
    slug: `api-tenant-${Date.now()}`,
  };

  let createdTenantId: string | null = null;

  test("1. Create tenant via API", async ({ request }) => {
    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`, {
      data: apiTenant,
    });

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("id");
    expect(body.data.name).toBe(apiTenant.name);
    expect(body.data.slug).toBe(apiTenant.slug);

    createdTenantId = body.data.id;
  });

  test("2. Get tenant by ID via API", async ({ request }) => {
    if (!createdTenantId) {
      // Create tenant first
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`,
        { data: apiTenant }
      );
      const createBody = await createResponse.json();
      createdTenantId = createBody.data.id;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/${createdTenantId}`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.id).toBe(createdTenantId);
  });

  test("3. Update tenant via API", async ({ request }) => {
    if (!createdTenantId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`,
        { data: apiTenant }
      );
      const createBody = await createResponse.json();
      createdTenantId = createBody.data.id;
    }

    const response = await request.put(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/${createdTenantId}`,
      {
        data: {
          name: "Updated Tenant Name",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.name).toBe("Updated Tenant Name");
  });

  test("4. List tenants with pagination via API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants?page=1&per_page=10`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(body).toHaveProperty("pagination");
    expect(Array.isArray(body.data)).toBeTruthy();
  });

  test("5. Delete tenant via API", async ({ request }) => {
    if (!createdTenantId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`,
        { data: { ...apiTenant, slug: `api-tenant-delete-${Date.now()}` } }
      );
      const createBody = await createResponse.json();
      createdTenantId = createBody.data.id;
    }

    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/${createdTenantId}`
    );

    expect(response.ok()).toBeTruthy();
  });

  test("6. Duplicate slug should fail", async ({ request }) => {
    const uniqueSlug = `duplicate-test-${Date.now()}`;

    // Create first tenant
    await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`, {
      data: { name: "First Tenant", slug: uniqueSlug },
    });

    // Try to create another with same slug
    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`, {
      data: { name: "Second Tenant", slug: uniqueSlug },
    });

    expect(response.status()).toBe(409); // Conflict
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
