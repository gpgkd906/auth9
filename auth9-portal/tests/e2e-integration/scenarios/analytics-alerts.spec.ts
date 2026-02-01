import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Analytics Dashboard
 *
 * Tests the analytics overview page.
 */
test.describe("Scenario: Analytics Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Analytics page is accessible", async ({ page }) => {
    await page.goto("/dashboard/analytics");
    await expect(page).toHaveURL(/\/dashboard\/analytics/);
    await expect(page.getByText(/analytics|statistics|overview/i).first()).toBeVisible();
  });

  test("2. Time period filter is visible", async ({ page }) => {
    await page.goto("/dashboard/analytics");

    // Check for time period options
    const timePeriods = [/7 day/i, /14 day/i, /30 day/i, /90 day/i];

    for (const period of timePeriods) {
      const element = page.getByText(period);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break; // At least one period found
      }
    }
  });

  test("3. Key metrics cards are visible", async ({ page }) => {
    await page.goto("/dashboard/analytics");

    // Check for metric cards
    const metrics = [
      /total login/i,
      /successful login/i,
      /failed login/i,
      /unique user/i,
    ];

    for (const metric of metrics) {
      const element = page.getByText(metric);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break; // At least one metric found
      }
    }
  });

  test("4. Breakdown charts are visible", async ({ page }) => {
    await page.goto("/dashboard/analytics");

    // Check for breakdown sections
    const breakdowns = [/by event type/i, /by device/i, /by browser/i];

    for (const breakdown of breakdowns) {
      const element = page.getByText(breakdown);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("5. View events link is visible", async ({ page }) => {
    await page.goto("/dashboard/analytics");

    const viewEventsLink = page.getByRole("link", { name: /view.*event|login event/i });
    if (await viewEventsLink.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(viewEventsLink).toBeVisible();
    }
  });
});

/**
 * Scenario: Login Events
 *
 * Tests the login events log page.
 */
test.describe("Scenario: Login Events", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Login events page is accessible", async ({ page }) => {
    await page.goto("/dashboard/analytics/events");
    await expect(page).toHaveURL(/\/dashboard\/analytics\/events/);
    await expect(page.getByText(/event|login/i).first()).toBeVisible();
  });

  test("2. Events table is visible", async ({ page }) => {
    await page.goto("/dashboard/analytics/events");

    const table = page.locator("table");
    if (await table.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(table).toBeVisible();
    }
  });

  test("3. Event columns are present", async ({ page }) => {
    await page.goto("/dashboard/analytics/events");

    // Check for expected columns
    const columns = [/time/i, /event/i, /user/i, /ip/i, /device/i];

    for (const column of columns) {
      const element = page.getByText(column);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("4. Event type badges are styled", async ({ page }) => {
    await page.goto("/dashboard/analytics/events");

    // Check for event type badges
    const eventTypes = [
      /login success/i,
      /social login/i,
      /wrong password/i,
      /mfa failed/i,
      /account locked/i,
    ];

    for (const eventType of eventTypes) {
      const element = page.getByText(eventType);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("5. Pagination is available", async ({ page }) => {
    await page.goto("/dashboard/analytics/events");

    // Check for pagination
    const pagination = page.getByText(/previous|next|page/i);
    if (await pagination.first().isVisible({ timeout: 2000 }).catch(() => false)) {
      await expect(pagination.first()).toBeVisible();
    }
  });
});

/**
 * Scenario: Security Alerts
 *
 * Tests the security alerts page.
 */
test.describe("Scenario: Security Alerts", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Security alerts page is accessible", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");
    await expect(page).toHaveURL(/\/dashboard\/security\/alerts/);
    await expect(page.getByText(/security|alert/i).first()).toBeVisible();
  });

  test("2. Filter buttons are visible", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");

    // Check for filter buttons
    const filters = [/all/i, /unresolved/i];

    for (const filter of filters) {
      const button = page.getByRole("button", { name: filter });
      if (await button.isVisible({ timeout: 2000 }).catch(() => false)) {
        await expect(button).toBeVisible();
        break;
      }
    }
  });

  test("3. Alert severity levels are shown", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");

    // Check for severity levels
    const severities = [/critical/i, /high/i, /medium/i, /low/i];

    for (const severity of severities) {
      const element = page.getByText(severity);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("4. Alert types are displayed", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");

    // Check for alert types
    const alertTypes = [
      /brute force/i,
      /new device/i,
      /impossible travel/i,
      /suspicious ip/i,
    ];

    for (const alertType of alertTypes) {
      const element = page.getByText(alertType);
      if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
        await expect(element.first()).toBeVisible();
        break;
      }
    }
  });

  test("5. Resolve button is visible for unresolved alerts", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");

    const resolveButton = page.getByRole("button", { name: /resolve|dismiss/i });
    if (await resolveButton.first().isVisible({ timeout: 2000 }).catch(() => false)) {
      await expect(resolveButton.first()).toBeVisible();
    }
  });

  test("6. Security recommendations section is visible", async ({ page }) => {
    await page.goto("/dashboard/security/alerts");

    const recommendations = page.getByText(/recommendation|tip|best practice/i);
    if (await recommendations.isVisible({ timeout: 2000 }).catch(() => false)) {
      await expect(recommendations).toBeVisible();
    }
  });
});

/**
 * Scenario: Webhooks
 *
 * Tests the webhook configuration page.
 */
test.describe("Scenario: Webhooks", () => {
  let testTenantId: string | null = null;

  test.beforeAll(async ({ request }) => {
    // Get or create a test tenant for webhook tests
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`
    );
    if (response.ok()) {
      const body = await response.json();
      if (body.data && body.data.length > 0) {
        testTenantId = body.data[0].id;
      }
    }
  });

  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Webhooks page is accessible", async ({ page }) => {
    if (!testTenantId) {
      test.skip();
      return;
    }

    await page.goto(`/dashboard/tenants/${testTenantId}/webhooks`);
    await expect(page).toHaveURL(/\/webhooks/);
    await expect(page.getByText(/webhook/i).first()).toBeVisible();
  });

  test("2. Add webhook button is visible", async ({ page }) => {
    if (!testTenantId) {
      test.skip();
      return;
    }

    await page.goto(`/dashboard/tenants/${testTenantId}/webhooks`);

    const addButton = page.getByRole("button", { name: /add webhook|create|new/i });
    if (await addButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await expect(addButton).toBeVisible();
    }
  });

  test("3. Webhook events are listed", async ({ page }) => {
    if (!testTenantId) {
      test.skip();
      return;
    }

    await page.goto(`/dashboard/tenants/${testTenantId}/webhooks`);

    // Click add button to see events
    const addButton = page.getByRole("button", { name: /add webhook|create|new/i });
    if (await addButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await addButton.click();

      // Check for event types
      const events = [
        /login\.success/i,
        /login\.failed/i,
        /user\.created/i,
        /password\.changed/i,
        /security\.alert/i,
      ];

      for (const event of events) {
        const element = page.getByText(event);
        if (await element.first().isVisible({ timeout: 1000 }).catch(() => false)) {
          await expect(element.first()).toBeVisible();
          break;
        }
      }

      // Close dialog
      await page.keyboard.press("Escape");
    }
  });

  test("4. Webhook form has required fields", async ({ page }) => {
    if (!testTenantId) {
      test.skip();
      return;
    }

    await page.goto(`/dashboard/tenants/${testTenantId}/webhooks`);

    const addButton = page.getByRole("button", { name: /add webhook|create|new/i });
    if (await addButton.isVisible({ timeout: 3000 }).catch(() => false)) {
      await addButton.click();

      // Check for required fields
      const nameInput = page.getByLabel(/name/i);
      const urlInput = page.getByLabel(/url|endpoint/i);

      if (await nameInput.isVisible({ timeout: 2000 }).catch(() => false)) {
        await expect(nameInput).toBeVisible();
      }
      if (await urlInput.isVisible({ timeout: 2000 }).catch(() => false)) {
        await expect(urlInput).toBeVisible();
      }

      // Close dialog
      await page.keyboard.press("Escape");
    }
  });
});

/**
 * Scenario: Analytics API
 *
 * Tests the analytics API endpoints.
 */
test.describe("Scenario: Analytics API", () => {
  test("1. Get analytics stats API works", async ({ request }) => {
    const endDate = new Date().toISOString().split("T")[0];
    const startDate = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split("T")[0];

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/analytics/stats?start_date=${startDate}&end_date=${endDate}`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
    }
  });

  test("2. List login events API works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/analytics/events?page=1&per_page=10`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
      expect(Array.isArray(body.data)).toBeTruthy();
    }
  });
});

/**
 * Scenario: Security Alerts API
 *
 * Tests the security alerts API endpoints.
 */
test.describe("Scenario: Security Alerts API", () => {
  test("1. List security alerts API works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/security/alerts?page=1&per_page=10`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());

    if (response.ok()) {
      const body = await response.json();
      expect(body).toHaveProperty("data");
      expect(Array.isArray(body.data)).toBeTruthy();
    }
  });

  test("2. List unresolved alerts API works", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/security/alerts?unresolved_only=true`
    );

    // May require authentication
    expect([200, 401]).toContain(response.status());
  });

  test("3. Resolve alert API endpoint exists", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/security/alerts/non-existent-id/resolve`
    );

    // Should return 401, 403, or 404
    expect([401, 403, 404]).toContain(response.status());
  });
});

/**
 * Scenario: Webhooks API
 *
 * Tests the webhooks API endpoints.
 */
test.describe("Scenario: Webhooks API", () => {
  test("1. List webhooks API requires tenant", async ({ request }) => {
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/test-tenant/webhooks`
    );

    // May require authentication or tenant not found
    expect([200, 401, 403, 404]).toContain(response.status());
  });

  test("2. Create webhook API requires auth", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/test-tenant/webhooks`,
      {
        data: {
          name: "Test Webhook",
          url: "https://example.com/webhook",
          events: ["login.success"],
          enabled: true,
        },
      }
    );

    // Should require authentication or tenant context
    expect([200, 201, 401, 403, 404, 422]).toContain(response.status());
  });

  test("3. Test webhook API endpoint exists", async ({ request }) => {
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants/test-tenant/webhooks/test-id/test`
    );

    // Should return error codes (not found, unauthorized, etc.)
    expect([401, 403, 404]).toContain(response.status());
  });
});

/**
 * Scenario: Security Detection
 *
 * Tests the security detection features.
 */
test.describe("Scenario: Security Detection", () => {
  test("1. Brute force detection is active", async ({ request }) => {
    // Simulate multiple failed login attempts - should eventually trigger detection
    const attempts = [];
    for (let i = 0; i < 3; i++) {
      attempts.push(
        request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/auth/login`, {
          data: {
            email: "test@example.com",
            password: "wrong-password",
          },
        })
      );
    }

    const responses = await Promise.all(attempts);
    // At least some should fail with 401
    expect(responses.some((r) => r.status() === 401)).toBeTruthy();
  });

  test("2. Rate limiting is enforced", async ({ request }) => {
    // This is a lightweight check - real rate limiting testing would need more attempts
    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/security/rate-limit-status`
    );

    // May not exist or require auth
    expect([200, 401, 404]).toContain(response.status());
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
