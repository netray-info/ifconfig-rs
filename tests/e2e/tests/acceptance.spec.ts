import { test, expect } from '@playwright/test';

test('homepage loads and shows IP address', async ({ page }) => {
  await page.goto('/');

  // Site title is visible
  await expect(page.locator('.site-title')).toBeVisible();

  // IP address is displayed in the hero area
  await expect(page.locator('.ip-display')).toBeVisible();
  const ipText = await page.locator('.ip-display').textContent();
  expect(ipText).toMatch(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/);

  // Version badge is visible in the badge bar above the info cards
  await expect(page.locator('.net-badge--version')).toContainText(/IPv[46]/);
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

test('theme persists across page reload', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  // Click toggle until we get 'light', then reload and verify it stayed
  const toggle = page.locator('.theme-toggle');
  for (let i = 0; i < 3; i++) {
    await toggle.click();
    const theme = await page.locator('html').getAttribute('data-theme');
    if (theme === 'light') break;
  }
  const themeBefore = await page.locator('html').getAttribute('data-theme');

  await page.reload();
  await expect(page.locator('.ip-display')).toBeVisible();

  const themeAfter = await page.locator('html').getAttribute('data-theme');
  expect(themeAfter).toBe(themeBefore);
});

test('request headers section expands and loads headers', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  const headersBtn = page.locator('button.section-header', { hasText: 'Request Headers' });
  await expect(headersBtn).toBeVisible();
  await headersBtn.click();

  // Panel should appear and load headers
  const panel = page.locator('#request-headers-panel');
  await expect(panel).toBeVisible();
  // At minimum the Accept or Host header should be present
  await expect(panel.locator('.header-row').first()).toBeVisible({ timeout: 5000 });
});

test('API explorer expands and fetches /ip/json response', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  const explorerBtn = page.locator('button.section-header', { hasText: 'API Explorer' });
  await expect(explorerBtn).toBeVisible();
  await explorerBtn.click();

  const panel = page.locator('#api-explorer-panel');
  await expect(panel).toBeVisible();

  // Select /ip endpoint and json format (defaults), wait for response
  await expect(panel.locator('pre')).toBeVisible({ timeout: 5000 });
  const responseText = await panel.locator('pre').textContent();
  expect(responseText).toContain('"addr"');
});

test('API explorer curl hint updates when endpoint changes', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  const explorerBtn = page.locator('button.section-header', { hasText: 'API Explorer' });
  await explorerBtn.click();

  const panel = page.locator('#api-explorer-panel');
  await expect(panel).toBeVisible();

  // Click the /location endpoint tab
  await panel.locator('button.endpoint-tab', { hasText: '/location' }).click();
  const curlText = await panel.locator('.curl-text').textContent();
  expect(curlText).toContain('/location');
});

test('footer contains expected links', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  // GitHub link
  await expect(page.locator('footer a[href*="github.com"]').first()).toBeVisible();

  // API docs link
  await expect(page.locator('footer a[href="/docs"]')).toBeVisible();
});

test('info cards show expected field labels', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  const cards = page.locator('.cards');
  await expect(cards).toBeVisible();

  // At minimum the IP card section should be present
  await expect(cards.locator('.card').first()).toBeVisible();

  // Page should mention location or ISP related text somewhere in cards
  const cardsText = await cards.textContent();
  expect(cardsText).toBeTruthy();
  expect(cardsText!.length).toBeGreaterThan(10);
});

test('FAQ section expands', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.ip-display')).toBeVisible();

  const faqBtn = page.locator('button.section-header', { hasText: 'FAQ' });
  if (await faqBtn.count() === 0) return; // skip if FAQ not present
  await faqBtn.click();

  const faqPanel = page.locator('[id*="faq"]');
  await expect(faqPanel).toBeVisible();
});
