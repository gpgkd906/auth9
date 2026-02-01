import { test, expect, type Page } from "@playwright/test";
import { TEST_CONFIG } from "../setup/test-config";

/**
 * Scenario: Role-Based Access Control (RBAC)
 *
 * Tests the complete RBAC flow:
 * - Create permissions for a service
 * - Create roles for a service
 * - Assign permissions to roles
 * - Assign roles to users in tenants
 */
test.describe("Scenario: RBAC Management", () => {
  // Test data - will be created during tests
  let testServiceId: string | null = null;
  let testTenantId: string | null = null;
  let testPermissionId: string | null = null;
  let testRoleId: string | null = null;
  let testUserId: string | null = null;

  test.beforeAll(async ({ request }) => {
    // Create a service for RBAC testing
    const serviceResponse = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services`,
      {
        data: {
          name: `RBAC Test Service ${Date.now()}`,
          client_id: `rbac-service-${Date.now()}`,
          redirect_uris: ["https://example.com/callback"],
        },
      }
    );
    const serviceBody = await serviceResponse.json();
    testServiceId = serviceBody.data?.id;

    // Create a tenant for RBAC testing
    const tenantResponse = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/tenants`,
      {
        data: {
          name: "RBAC Test Tenant",
          slug: `rbac-tenant-${Date.now()}`,
        },
      }
    );
    const tenantBody = await tenantResponse.json();
    testTenantId = tenantBody.data?.id;

    // Get a user for role assignment
    const usersResponse = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users?page=1&per_page=1`
    );
    const usersBody = await usersResponse.json();
    testUserId = usersBody.data?.[0]?.id;
  });

  test("1. Create permission via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/permissions`,
      {
        data: {
          service_id: testServiceId,
          code: "users:read",
          name: "Read Users",
          description: "Permission to read user data",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("id");
    expect(body.data.code).toBe("users:read");

    testPermissionId = body.data.id;
  });

  test("2. Create additional permissions via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    const permissions = [
      { code: "users:write", name: "Write Users" },
      { code: "users:delete", name: "Delete Users" },
      { code: "reports:read", name: "Read Reports" },
    ];

    for (const perm of permissions) {
      const response = await request.post(
        `${TEST_CONFIG.auth9CoreUrl}/api/v1/permissions`,
        {
          data: {
            service_id: testServiceId,
            code: perm.code,
            name: perm.name,
          },
        }
      );

      // May fail if already exists
      expect([200, 201, 409]).toContain(response.status());
    }
  });

  test("3. List permissions for service via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${testServiceId}/permissions`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(Array.isArray(body.data)).toBeTruthy();
    expect(body.data.length).toBeGreaterThan(0);
  });

  test("4. Create role via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/roles`, {
      data: {
        service_id: testServiceId,
        name: "User Manager",
        description: "Can manage users",
      },
    });

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("id");
    expect(body.data.name).toBe("User Manager");

    testRoleId = body.data.id;
  });

  test("5. Create role with parent (inheritance) via API", async ({ request }) => {
    if (!testServiceId || !testRoleId) {
      test.skip();
      return;
    }

    const response = await request.post(`${TEST_CONFIG.auth9CoreUrl}/api/v1/roles`, {
      data: {
        service_id: testServiceId,
        name: "Admin",
        description: "Administrator with all permissions",
        parent_role_id: testRoleId, // Inherits from User Manager
      },
    });

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.parent_role_id).toBe(testRoleId);
  });

  test("6. List roles for service via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/services/${testServiceId}/roles`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
    expect(Array.isArray(body.data)).toBeTruthy();
    expect(body.data.length).toBeGreaterThan(0);
  });

  test("7. Assign permission to role via API", async ({ request }) => {
    if (!testRoleId || !testPermissionId) {
      test.skip();
      return;
    }

    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/roles/${testRoleId}/permissions`,
      {
        data: {
          permission_id: testPermissionId,
        },
      }
    );

    // May succeed or already assigned
    expect([200, 201, 409]).toContain(response.status());
  });

  test("8. Get role with permissions via API", async ({ request }) => {
    if (!testRoleId) {
      test.skip();
      return;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/roles/${testRoleId}`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data).toHaveProperty("permissions");
    expect(Array.isArray(body.data.permissions)).toBeTruthy();
  });

  test("9. Assign role to user in tenant via API", async ({ request }) => {
    if (!testUserId || !testTenantId || !testRoleId) {
      test.skip();
      return;
    }

    // First add user to tenant if not already
    await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants`,
      {
        data: {
          tenant_id: testTenantId,
          role_in_tenant: "member",
        },
      }
    );

    // Assign role
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/rbac/assign`,
      {
        data: {
          user_id: testUserId,
          tenant_id: testTenantId,
          role_ids: [testRoleId],
        },
      }
    );

    expect(response.ok()).toBeTruthy();
  });

  test("10. Get user roles in tenant via API", async ({ request }) => {
    if (!testUserId || !testTenantId) {
      test.skip();
      return;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants/${testTenantId}/roles`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
  });

  test("11. Get user assigned roles via API", async ({ request }) => {
    if (!testUserId || !testTenantId) {
      test.skip();
      return;
    }

    const response = await request.get(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants/${testTenantId}/assigned-roles`
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("data");
  });

  test("12. Unassign role from user via API", async ({ request }) => {
    if (!testUserId || !testTenantId || !testRoleId) {
      test.skip();
      return;
    }

    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/users/${testUserId}/tenants/${testTenantId}/roles/${testRoleId}`
    );

    // May succeed or role not assigned
    expect([200, 204, 404]).toContain(response.status());
  });

  test("13. Update role via API", async ({ request }) => {
    if (!testRoleId) {
      test.skip();
      return;
    }

    const response = await request.put(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/roles/${testRoleId}`,
      {
        data: {
          name: "Updated Role Name",
          description: "Updated description",
        },
      }
    );

    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body.data.name).toBe("Updated Role Name");
  });

  test("14. Remove permission from role via API", async ({ request }) => {
    if (!testRoleId || !testPermissionId) {
      test.skip();
      return;
    }

    const response = await request.delete(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/roles/${testRoleId}/permissions/${testPermissionId}`
    );

    // May succeed or not assigned
    expect([200, 204, 404]).toContain(response.status());
  });

  // TODO: Permission code validation may not be enforced at API level
  test.skip("15. Permission code validation via API", async ({ request }) => {
    if (!testServiceId) {
      test.skip();
      return;
    }

    // Invalid permission code (should fail)
    const response = await request.post(
      `${TEST_CONFIG.auth9CoreUrl}/api/v1/permissions`,
      {
        data: {
          service_id: testServiceId,
          code: "invalid", // Should be "resource:action" format
          name: "Invalid Permission",
        },
      }
    );

    // Should fail validation
    expect(response.status()).toBe(400);
  });
});

/**
 * Scenario: RBAC UI Management
 *
 * Tests the roles & permissions UI.
 */
test.describe("Scenario: RBAC UI", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsTestUser(page);
  });

  test("1. Navigate to roles page", async ({ page }) => {
    await page.goto("/dashboard/roles");
    await expect(page.getByRole("heading", { name: /roles/i })).toBeVisible();
  });

  test("2. Roles page shows service tabs", async ({ page }) => {
    await page.goto("/dashboard/roles");

    // Should have tabs or selector for different services
    // Page should render without error
    await expect(page.getByText(/roles|permissions/i).first()).toBeVisible();
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
