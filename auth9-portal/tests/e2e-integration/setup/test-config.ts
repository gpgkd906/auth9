/**
 * E2E Integration Test Configuration
 * All environment-specific values for full-stack testing
 */

export const TEST_CONFIG = {
  // Service URLs
  portalUrl: "http://localhost:3000",
  auth9CoreUrl: "http://localhost:8080",

  // Test Users (created during seed)
  testUsers: {
    standard: {
      username: "e2e-test-user",
      email: "e2e-test@example.com",
      password: "TestPass1234!",
      firstName: "E2E",
      lastName: "TestUser",
    },
    admin: {
      username: "e2e-admin-user",
      email: "e2e-admin@example.com",
      password: "SecurePass123!",
      firstName: "E2E",
      lastName: "AdminUser",
    },
  },

  // Test Tenant
  testTenant: {
    name: "E2E Test Tenant",
    slug: "e2e-test-tenant",
  },
} as const;

export type TestUser = (typeof TEST_CONFIG.testUsers)[keyof typeof TEST_CONFIG.testUsers];
