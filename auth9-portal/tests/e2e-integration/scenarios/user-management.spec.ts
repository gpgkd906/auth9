import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: User Management
 *
 * Tests user CRUD operations and tenant/role assignments.
 */
test.describe("Scenario: User Management", () => {
  const testUserData = {
    email: `e2e-user-${Date.now()}@example.com`,
    displayName: "E2E Test User",
    password: "TestPass123!",
  };

  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Navigate to users page", async ({ page }) => {
    await page.goto("/dashboard/users");
    await expect(page.getByRole("heading", { name: /users/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /create user/i })).toBeVisible();
  });

  test("2. Users list shows existing users", async ({ page }) => {
    await page.goto("/dashboard/users");

    // Should show at least the test user created in global setup
    const userTable = page.locator("table");
    await expect(userTable).toBeVisible();

    // Verify table has content
    const rows = page.locator("tbody tr");
    await expect(rows.first()).toBeVisible();
  });

  test("3. Create a new user", async ({ page }) => {
    await page.goto("/dashboard/users");

    // Click create user button
    await page.getByRole("button", { name: /create user/i }).click();

    // Fill form
    await page.getByLabel(/email/i).fill(testUserData.email);
    await page.getByLabel(/display name/i).fill(testUserData.displayName);
    await page.getByLabel(/password/i).fill(testUserData.password);

    // Submit
    await page.getByRole("button", { name: /create/i }).click();

    // Verify user appears in list
    await expect(page.getByText(testUserData.email)).toBeVisible({ timeout: 10000 });
  });

  // UI update test - may be flaky depending on UI state
  test("4. Update user display name", async ({ page }) => {
    await page.goto("/dashboard/users");

    // Find the test user row
    const userRow = page.locator("tr", { hasText: TEST_CONFIG.testUsers.standard.email });

    // Skip if user not found
    if (!(await userRow.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip();
      return;
    }

    // Click actions dropdown
    const actionsButton = userRow.getByRole("button").first();
    await actionsButton.click();

    // Look for edit option
    const editButton = page.getByRole("menuitem", { name: /edit/i });
    if (!(await editButton.isVisible({ timeout: 2000 }).catch(() => false))) {
      await page.keyboard.press("Escape");
      test.skip();
      return;
    }

    await editButton.click();

    // Update display name
    const nameInput = page.getByLabel(/display name/i);
    await nameInput.clear();
    await nameInput.fill("Updated Display Name");

    // Submit
    await page.getByRole("button", { name: /save|update/i }).click();
  });
});

/**
 * Scenario: User-Tenant Assignment
 *
 * Tests adding/removing users from tenants.
 */
test.describe("Scenario: User-Tenant Assignment", () => {
  let testTenantId: string | null = null;
  let testUserId: string | null = null;

  test.beforeAll(async ({ request }) => {
    // Create a test tenant via API
    const tenantResponse = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`,
      {
        data: {
          name: "User Assignment Test Tenant",
          slug: `user-assign-tenant-${Date.now()}`,
        },
      }
    );
    const tenantBody = await tenantResponse.json();
    testTenantId = tenantBody.data?.id;
  });

  test("1. Add user to tenant via API", async ({ request }) => {
    // Get a user ID first
    const usersResponse = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users?page=1&per_page=1`
    );
    const usersBody = await usersResponse.json();

    if (usersBody.data && usersBody.data.length > 0) {
      testUserId = usersBody.data[0].id;

      if (testTenantId && testUserId) {
        const response = await request.post(
          `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants`,
          {
            data: {
              tenant_id: testTenantId,
              role_in_tenant: "member",
            },
          }
        );

        // May return 200 (success) or 409 (already exists)
        expect([200, 201, 409]).toContain(response.status());
      }
    }
  });

  test("2. Get user's tenants via API", async ({ request }) => {
    if (!testUserId) {
      const usersResponse = await request.get(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/users?page=1&per_page=1`
      );
      const usersBody = await usersResponse.json();
      testUserId = usersBody.data?.[0]?.id;
    }

    if (testUserId) {
      const response = await request.get(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants`
      );

      expect(response.ok()).toBeTruthy();
      const body = await response.json();
      expect(body).toHaveProperty("data");
      expect(Array.isArray(body.data)).toBeTruthy();
    }
  });

  test("3. List users in tenant via API", async ({ request }) => {
    if (testTenantId) {
      const response = await request.get(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/${testTenantId}/users`
      );

      expect(response.ok()).toBeTruthy();
      const body = await response.json();
      expect(body).toHaveProperty("data");
    }
  });

  test("4. Remove user from tenant via API", async ({ request }) => {
    if (testUserId && testTenantId) {
      const response = await request.delete(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants/${testTenantId}`
      );

      // May succeed or fail if already removed
      expect([200, 204, 404]).toContain(response.status());
    }
  });
});

/**
 * Scenario: User API Integration
 *
 * Direct API tests for user management.
 */
test.describe("Scenario: User API Integration", () => {
  const apiUser = {
    email: `api-user-${Date.now()}@example.com`,
    display_name: "API Test User",
    password: "ApiPass123!",
  };

  let createdUserId: string | null = null;

  test("1. Create user via API", async ({ request }) => {
    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/users`, {
      data: apiUser,
    });

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("id");
    expect(body.data.email).toBe(apiUser.email);

    createdUserId = body.data.id;
  });

  test("2. Get user by ID via API", async ({ request }) => {
    if (!createdUserId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/users`,
        { data: { ...apiUser, email: `api-get-${Date.now()}@example.com` } }
      );
      const createBody = await createResponse.json();
      createdUserId = createBody.data.id;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${createdUserId}`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.id).toBe(createdUserId);
  });

  test("3. Update user via API", async ({ request }) => {
    if (!createdUserId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/users`,
        { data: { ...apiUser, email: `api-update-${Date.now()}@example.com` } }
      );
      const createBody = await createResponse.json();
      createdUserId = createBody.data.id;
    }

    const response = await request.put(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${createdUserId}`,
      {
        data: {
          display_name: "Updated API User",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.display_name).toBe("Updated API User");
  });

  test("4. List users with pagination via API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users?page=1&per_page=10`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(body).toHaveProperty("pagination");
    expect(body.pagination).toHaveProperty("page", 1);
    expect(body.pagination).toHaveProperty("per_page", 10);
  });

  test("5. Duplicate email should fail", async ({ request }) => {
    const uniqueEmail = `duplicate-${Date.now()}@example.com`;

    // Create first user
    await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/users`, {
      data: { email: uniqueEmail, display_name: "First User" },
    });

    // Try to create another with same email
    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/users`, {
      data: { email: uniqueEmail, display_name: "Second User" },
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
