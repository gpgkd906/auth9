/**
 * Playwright Global Setup
 * Runs once before all tests to prepare test data
 */

import { KeycloakAdminClient } from "./setup/keycloak-admin";
import { TEST_CONFIG } from "./setup/test-config";

async function waitForServices(): Promise<void> {
  const services = [
    { name: "Portal", url: `${TEST_CONFIG.portalUrl}/login` },
    { name: "Auth9 Core", url: `${TEST_CONFIG.auth9CoreUrl}/health` },
    { name: "Keycloak", url: `${TEST_CONFIG.keycloakUrl}/health/ready` },
  ];

  console.log("Waiting for services to be ready...");

  for (const service of services) {
    let ready = false;
    let attempts = 0;
    const maxAttempts = 30;

    while (!ready && attempts < maxAttempts) {
      try {
        const response = await fetch(service.url);
        if (response.ok || response.status === 200 || response.status === 302) {
          ready = true;
          console.log(`✓ ${service.name} is ready`);
        }
      } catch {
        // Service not ready yet
      }

      if (!ready) {
        attempts++;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    }

    if (!ready) {
      throw new Error(
        `${service.name} is not ready after ${maxAttempts} seconds. URL: ${service.url}`
      );
    }
  }
}

async function globalSetup(): Promise<void> {
  console.log("\n========== E2E Global Setup ==========\n");

  // Wait for all services to be ready
  await waitForServices();

  // Setup test data in Keycloak
  const keycloak = new KeycloakAdminClient();
  await keycloak.authenticate();
  await keycloak.setupTestUsers();

  console.log("\n========== Setup Complete ==========\n");
}

export default globalSetup;
