import { test, expect } from '@playwright/test';

test('homepage loads and shows IP address', async ({ page }) => {
  await page.goto('/');

  // Site title is visible
  await expect(page.locator('.site-title')).toBeVisible();

  // IP address is displayed in the hero area
  await expect(page.locator('.ip-display')).toBeVisible();
  const ipText = await page.locator('.ip-display').textContent();
  expect(ipText).toMatch(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/);

  // Version badge is visible
  await expect(page.locator('.version-badge')).toContainText(/IPv[46]/);
});

test('theme toggle works', async ({ page }) => {
  await page.goto('/');
  const toggle = page.locator('.theme-toggle');
  await expect(toggle).toBeVisible();

  // Click cycles through themes
  await toggle.click();
  const theme = await page.locator('html').getAttribute('data-theme');
  expect(['dark', 'light']).toContain(theme);
});

test('info cards are rendered', async ({ page }) => {
  await page.goto('/');

  // Wait for data to load
  await expect(page.locator('.ip-display')).toBeVisible();

  // Cards section should be present with at least one card
  await expect(page.locator('.cards .card').first()).toBeVisible();
});
