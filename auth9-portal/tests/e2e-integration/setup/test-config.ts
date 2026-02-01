/**
 * E2E Integration Test Configuration
 * All environment-specific values for full-stack testing
 */

export const TEST_CONFIG = {
  // Service URLs
  portalUrl: "http://localhost:3000",
  auth9CoreUrl: "http://localhost:8080",
  keycloakUrl: "http://localhost:8081",
  keycloakRealm: "auth9",

  // Keycloak Admin (master realm)
  keycloakAdmin: {
    username: "admin",
    password: "admin",
  },

  // Test Users (created during setup)
  testUsers: {
    standard: {
      username: "e2e-test-user",
      email: "e2e-test@example.com",
      password: "Test123!",
      firstName: "E2E",
      lastName: "TestUser",
    },
    admin: {
      username: "e2e-admin-user",
      email: "e2e-admin@example.com",
      password: "Admin123!",
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
