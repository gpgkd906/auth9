/**
 * Playwright Global Teardown
 * Runs once after all tests to cleanup test data
 *
 * Note: Since we require environment reset before tests,
 * cleanup is optional but good for debugging scenarios
 */

async function globalTeardown(): Promise<void> {
  console.log("\n========== E2E Global Teardown ==========\n");

  // Cleanup is optional since we reset environment before each test run
  // Uncomment below if you want to clean up test users after tests
  //
  // const keycloak = new KeycloakAdminClient();
  // await keycloak.authenticate();
  // await keycloak.cleanupTestUsers();

  console.log("Teardown complete (no cleanup - environment will be reset)");

  console.log("\n========== Teardown Complete ==========\n");
}

export default globalTeardown;
