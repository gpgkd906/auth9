import { chromium } from 'playwright';

(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  try {
    // Step 1: Navigate to demo login page
    console.log('Step 1: Navigate to demo login...');
    await page.goto('http://localhost:3002/login');
    await page.waitForLoadState('networkidle');

    // Step 2: Should redirect to Keycloak
    console.log('Step 2: Current URL:', page.url());
    console.log('Waiting for Keycloak login page...');

    // Wait for Keycloak login form
    await page.waitForSelector('input[name="username"]', { timeout: 10000 });

    // Step 3: Fill in credentials
    console.log('Step 3: Filling credentials...');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'SecurePass123!');
    await page.click('button[type="submit"]');

    // Wait for redirect to dashboard
    console.log('Waiting for dashboard...');
    await page.waitForURL('**/dashboard**', { timeout: 15000 });

    console.log('Step 4: Dashboard URL:', page.url());

    // Check dashboard content
    const title = await page.title();
    console.log('Page title:', title);

    // Get page content
    const pageContent = await page.content();
    console.log('Has Identity Token:', pageContent.includes('Identity Token'));
    console.log('Has Dashboard:', pageContent.includes('Dashboard'));
    console.log('Has admin@auth9.local:', pageContent.includes('admin@auth9.local'));
    console.log('Has Auth9-signed:', pageContent.includes('Auth9-signed'));

    console.log('\n=== Scenario 3 Result ===');
    console.log('PASS: Successfully logged in and redirected to dashboard');

  } catch (error) {
    console.error('ERROR:', error.message);
    console.log('\n=== Scenario 3 Result ===');
    console.log('FAIL:', error.message);
  } finally {
    await browser.close();
  }
})();
