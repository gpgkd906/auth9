import { test, expect } from "@playwright/test";

test.describe("Theme Toggle", () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test
    await page.goto("/");
    await page.evaluate(() => localStorage.clear());
  });

  test("should display theme toggle button on landing page", async ({ page }) => {
    await page.goto("/");
    const themeToggle = page.locator('[data-testid="theme-toggle"]');
    await expect(themeToggle).toBeVisible();
  });

  test("should display theme toggle button on login page", async ({ page }) => {
    await page.goto("/login");
    const themeToggle = page.locator('[data-testid="theme-toggle"]');
    await expect(themeToggle).toBeVisible();
  });

  test("should switch to dark mode when clicking moon icon", async ({ page }) => {
    await page.goto("/");

    // Initially should be in light mode (no data-theme attribute or light)
    const html = page.locator("html");

    // Click dark mode button
    const darkModeBtn = page.locator('[data-testid="theme-dark"]');
    await darkModeBtn.click();

    // Should have data-theme="dark" attribute
    await expect(html).toHaveAttribute("data-theme", "dark");
  });

  test("should switch back to light mode when clicking sun icon", async ({ page }) => {
    await page.goto("/");

    // Switch to dark mode first
    await page.locator('[data-testid="theme-dark"]').click();

    // Then switch back to light mode
    await page.locator('[data-testid="theme-light"]').click();

    // Should not have data-theme="dark"
    const html = page.locator("html");
    const theme = await html.getAttribute("data-theme");
    expect(theme).not.toBe("dark");
  });

  test("should persist theme preference in localStorage", async ({ page }) => {
    await page.goto("/");

    // Switch to dark mode
    await page.locator('[data-testid="theme-dark"]').click();

    // Check localStorage
    const storedTheme = await page.evaluate(() => localStorage.getItem("auth9-theme"));
    expect(storedTheme).toBe("dark");

    // Reload page
    await page.reload();

    // Should still be in dark mode
    const html = page.locator("html");
    await expect(html).toHaveAttribute("data-theme", "dark");
  });

  test("should persist light theme preference after reload", async ({ page }) => {
    await page.goto("/");

    // Switch to dark mode first
    await page.locator('[data-testid="theme-dark"]').click();

    // Then switch to light mode
    await page.locator('[data-testid="theme-light"]').click();

    // Check localStorage
    const storedTheme = await page.evaluate(() => localStorage.getItem("auth9-theme"));
    expect(storedTheme).toBe("light");

    // Reload page
    await page.reload();

    // Should still be in light mode
    const html = page.locator("html");
    const theme = await html.getAttribute("data-theme");
    expect(theme).not.toBe("dark");
  });
});
