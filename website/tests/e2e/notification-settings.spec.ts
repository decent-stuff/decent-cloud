import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Provider Support Page - Notification Settings
 *
 * Tests notification configuration and rate limiting display on /dashboard/provider/support
 *
 * Note: Notifications live in step 2 of the provider setup wizard. Tests land there
 * by pre-setting the persisted wizard step in localStorage before navigating.
 *
 * Consolidated: snapshot tests on /dashboard/provider/support are merged into one
 * test to reduce redundant page.goto() calls. Behavioral toggle flows stay separate.
 */
test.describe('Provider Support - Notification Settings', () => {
	test.beforeEach(async ({ page }) => {
		// Provider setup wizard persists its step in localStorage and defaults to step 1
		// (Support Portal). The Notifications section is rendered only on step 2
		// ("Contacts & Notifications"), so land there before each test navigates.
		await page.evaluate(() =>
			localStorage.setItem('provider-setup-wizard-step', '2'),
		);
	});

	test('notification settings: section, channels, save button, tier limits, and usage grid render correctly', async ({
		page,
	}) => {
		// Single navigation covers what was previously six snapshot tests:
		// "display notifications section", "display channel selection options",
		// "have save notifications button", "display free tier limit badges",
		// "display usage statistics grid", and "display usage counts with limits".
		//
		// The first op after goto is an auto-retrying expect(...).toBeVisible(),
		// so the prior networkidle wait would have been redundant; each subsequent
		// assertion also auto-retries, covering the async-loaded usage grid.
		await page.goto('/dashboard/provider/support');

		// Notifications heading + description
		await expect(page.locator('h2:has-text("Notifications")')).toBeVisible();
		await expect(page.locator('text=Get alerted when customers need support')).toBeVisible();

		// Each channel is wrapped in its own <label>; use that to avoid matching the
		// email-verification banner and the usage grid column labels.
		const notifications = page.locator('#notifications');
		await expect(notifications.locator('label:has-text("Email")')).toBeVisible();
		await expect(notifications.locator('label:has-text("Telegram")')).toBeVisible();
		await expect(notifications.locator('label:has-text("SMS")')).toBeVisible();

		// Save button
		await expect(page.locator('button:has-text("Save Notifications")')).toBeVisible();

		// Free-tier limit badges
		await expect(page.locator('text=Free').first()).toBeVisible();
		await expect(page.locator('text=Free (50/day)')).toBeVisible();
		await expect(page.locator('text=5 free/day')).toBeVisible();

		// Usage grid (3 columns: Email, Telegram, SMS)
		const usageGrid = page.locator('.grid.grid-cols-3');
		await expect(usageGrid).toBeVisible({ timeout: 10000 });
		await expect(usageGrid.locator('text=Email')).toBeVisible();
		await expect(usageGrid.locator('text=Telegram')).toBeVisible();
		await expect(usageGrid.locator('text=SMS')).toBeVisible();

		// Telegram count/limit (e.g. "0/50"); SMS anchored (e.g. "0/5") so it does
		// not also match the Telegram cell.
		const telegramUsage = usageGrid.locator('div:has-text("Telegram")').locator('..');
		await expect(telegramUsage.locator('text=/\\d+\\/50/')).toBeVisible();

		const smsUsage = usageGrid.locator('div:has-text("SMS")').locator('..');
		await expect(smsUsage.locator('text=/^\\d+\\/5$/')).toBeVisible();
	});

	test('notification settings: telegram checkbox reveals chat ID input', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		// networkidle is load-bearing here: the Telegram label is SSR'd before
		// its onclick handler binds, so clicking pre-hydration is a silent no-op.
		await page.waitForLoadState('networkidle');
		// Click the label containing "Telegram" to toggle the checkbox
		await page.click('label:has-text("Telegram")');
		await expect(page.locator('input[placeholder="Chat ID"]')).toBeVisible();
		// Should show link to Telegram bot
		await expect(page.locator('a[href^="https://t.me/"]')).toBeVisible();
	});

	test('notification settings: sms checkbox reveals phone input', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await page.waitForLoadState('networkidle');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('notification settings: multiple channels can be selected simultaneously', async ({
		page,
	}) => {
		await page.goto('/dashboard/provider/support');
		await page.waitForLoadState('networkidle');
		await page.click('label:has-text("Telegram")');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="Chat ID"]')).toBeVisible();
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('notification settings: test notification button appears with valid input', async ({
		page,
	}) => {
		await page.goto('/dashboard/provider/support');
		await page.waitForLoadState('networkidle');

		// Enable Telegram and enter a chat ID
		await page.click('label:has-text("Telegram")');
		await page.fill('input[placeholder="Chat ID"]', '123456789');

		// Test button should appear
		await expect(page.locator('button:has-text("Send Test")')).toBeVisible();
	});
});
