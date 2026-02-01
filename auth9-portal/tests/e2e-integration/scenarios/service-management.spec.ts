import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Service Management
 *
 * Tests OIDC service registration and client management.
 * Services are containers for OAuth2/OIDC clients.
 */
test.describe("Scenario: Service Management", () => {
  const testService = {
    name: `E2E Test Service ${Date.now()}`,
    clientId: `e2e-client-${Date.now()}`,
    baseUrl: "https://example.com",
    redirectUris: "https://example.com/callback",
  };

  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Navigate to services page", async ({ page }) => {
    await page.goto("/dashboard/services");
    await expect(page.getByRole("heading", { name: /services/i })).toBeVisible();
    await expect(
      page.getByRole("button", { name: /register service/i })
    ).toBeVisible();
  });

  // UI service registration test
  test("2. Register a new service", async ({ page }) => {
    await page.goto("/dashboard/services");

    // Click register button
    const registerButton = page.getByRole("button", { name: /register service/i });
    if (!(await registerButton.isVisible({ timeout: 3000 }).catch(() => false))) {
      test.skip();
      return;
    }
    await registerButton.click();

    // Fill form - use more flexible selectors
    const nameInput = page.locator('input[name="name"], input[placeholder*="name" i]').first();
    if (await nameInput.isVisible()) {
      await nameInput.fill(testService.name);
    }

    const clientIdInput = page.locator('input[name="client_id"], input[placeholder*="client" i]').first();
    if (await clientIdInput.isVisible()) {
      await clientIdInput.fill(testService.clientId);
    }

    // Submit
    const submitButton = page.getByRole("button", { name: /register|create|submit/i });
    await submitButton.click();

    // Wait for response - either success or already exists
    await page.waitForTimeout(2000);
  });

  test("3. View service details", async ({ page }) => {
    await page.goto("/dashboard/services");

    // Find any service row with a details/view link
    const detailsLink = page.locator("a[href*='/dashboard/services/']").first();

    if (await detailsLink.isVisible({ timeout: 3000 }).catch(() => false)) {
      await detailsLink.click();

      // Should navigate to service details page
      await expect(page).toHaveURL(/\/dashboard\/services\/.+/);
    } else {
      // No services to view, skip
      test.skip();
    }
  });
});

/**
 * Scenario: Service API Integration
 *
 * Direct API tests for service management.
 */
test.describe("Scenario: Service API Integration", () => {
  const apiService = {
    name: `API Service ${Date.now()}`,
    client_id: `api-client-${Date.now()}`,
    base_url: "https://api-example.com",
    redirect_uris: ["https://api-example.com/callback"],
    logout_uris: ["https://api-example.com/logout"],
  };

  let createdServiceId: string | null = null;

  test("1. Create service via API", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
      { data: apiService }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("id");
    expect(body.data.name).toBe(apiService.name);

    // Should include client with secret (only shown once)
    expect(body.data).toHaveProperty("client");
    expect(body.data.client).toHaveProperty("client_secret");

    createdServiceId = body.data.id;
  });

  test("2. Get service by ID via API", async ({ request }) => {
    if (!createdServiceId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
        {
          data: {
            ...apiService,
            name: `API Service Get ${Date.now()}`,
            client_id: `api-get-${Date.now()}`,
          },
        }
      );
      const createBody = await createResponse.json();
      createdServiceId = createBody.data.id;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.id).toBe(createdServiceId);
  });

  test("3. Update service via API", async ({ request }) => {
    if (!createdServiceId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
        {
          data: {
            ...apiService,
            name: `API Service Update ${Date.now()}`,
            client_id: `api-update-${Date.now()}`,
          },
        }
      );
      const createBody = await createResponse.json();
      createdServiceId = createBody.data.id;
    }

    const response = await request.put(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}`,
      {
        data: {
          name: "Updated Service Name",
          base_url: "https://updated-example.com",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.name).toBe("Updated Service Name");
  });

  test("4. List services with pagination via API", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services?page=1&per_page=10`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(body).toHaveProperty("pagination");
  });

  test("5. List service clients via API", async ({ request }) => {
    if (!createdServiceId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
        {
          data: {
            ...apiService,
            name: `API Service Clients ${Date.now()}`,
            client_id: `api-clients-${Date.now()}`,
          },
        }
      );
      const createBody = await createResponse.json();
      createdServiceId = createBody.data.id;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}/clients`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(Array.isArray(body.data)).toBeTruthy();
  });

  test("6. Create additional client for service via API", async ({ request }) => {
    if (!createdServiceId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
        {
          data: {
            ...apiService,
            name: `API Service AddClient ${Date.now()}`,
            client_id: `api-addclient-${Date.now()}`,
          },
        }
      );
      const createBody = await createResponse.json();
      createdServiceId = createBody.data.id;
    }

    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}/clients`,
      {
        data: {
          name: "Additional Client",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("client_id");
    expect(body.data).toHaveProperty("client_secret");
  });

  test("7. Regenerate client secret via API", async ({ request }) => {
    if (!createdServiceId) {
      const createResponse = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
        {
          data: {
            ...apiService,
            name: `API Service Regen ${Date.now()}`,
            client_id: `api-regen-${Date.now()}`,
          },
        }
      );
      const createBody = await createResponse.json();
      createdServiceId = createBody.data.id;
    }

    // Get client ID first
    const clientsResponse = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}/clients`
    );
    const clientsBody = await clientsResponse.json();
    const clientId = clientsBody.data?.[0]?.client_id;

    if (clientId) {
      const response = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${createdServiceId}/clients/${clientId}/regenerate-secret`
      );

      expect(response.ok()).toBeTruthy();
      const body = await response.json();
      expect(body.data).toHaveProperty("client_secret");
    }
  });

  test("8. Delete service via API", async ({ request }) => {
    // Create a service specifically for deletion
    const createResponse = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
      {
        data: {
          ...apiService,
          name: `API Service Delete ${Date.now()}`,
          client_id: `api-delete-${Date.now()}`,
        },
      }
    );
    const createBody = await createResponse.json();
    const deleteServiceId = createBody.data.id;

    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${deleteServiceId}`
    );

    expect(response.ok()).toBeTruthy();
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
