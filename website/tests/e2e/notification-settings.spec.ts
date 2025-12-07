import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Provider Support Page - Notification Settings
 *
 * Tests notification configuration and rate limiting display on /dashboard/provider/support
 */
test.describe('Provider Support - Notification Settings', () => {
	test('should display notifications section', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await expect(page.locator('h2:has-text("Notifications")')).toBeVisible();
		await expect(page.locator('text=Configure how you receive support escalation alerts')).toBeVisible();
	});

	test('should display channel selection options', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await expect(page.locator('text=Email')).toBeVisible();
		await expect(page.locator('text=Telegram')).toBeVisible();
		await expect(page.locator('text=SMS')).toBeVisible();
	});

	test('should show telegram input when telegram checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		// Click the label containing "Telegram" to toggle the checkbox
		await page.click('label:has-text("Telegram")');
		await expect(page.locator('input[placeholder="Chat ID"]')).toBeVisible();
		// Should show link to Telegram bot
		await expect(page.locator('a[href^="https://t.me/"]')).toBeVisible();
	});

	test('should show phone input when sms checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('should allow multiple channels to be selected', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await page.click('label:has-text("Telegram")');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="Chat ID"]')).toBeVisible();
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('should have save notifications button', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await expect(page.locator('button:has-text("Save Notifications")')).toBeVisible();
	});

	test('should display free tier limit badges', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		// Email is free (unlimited)
		await expect(page.locator('text=Free').first()).toBeVisible();
		// Telegram has 50/day free limit
		await expect(page.locator('text=Free (50/day)')).toBeVisible();
		// SMS has 5/day free limit
		await expect(page.locator('text=5 free/day')).toBeVisible();
	});

	test('should display usage statistics grid', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		// Wait for usage data to load (displayed in a grid)
		await page.waitForLoadState('networkidle');

		// Usage grid should be visible with 3 columns: Email, Telegram, SMS
		const usageGrid = page.locator('.grid.grid-cols-3');
		await expect(usageGrid).toBeVisible({ timeout: 10000 });

		// Should show count labels
		await expect(usageGrid.locator('text=Email')).toBeVisible();
		await expect(usageGrid.locator('text=Telegram')).toBeVisible();
		await expect(usageGrid.locator('text=SMS')).toBeVisible();
	});

	test('should display usage counts with limits for Telegram and SMS', async ({ page }) => {
		await page.goto('/dashboard/provider/support');
		await page.waitForLoadState('networkidle');

		// Wait for usage grid
		const usageGrid = page.locator('.grid.grid-cols-3');
		await expect(usageGrid).toBeVisible({ timeout: 10000 });

		// Telegram should show count/limit format (e.g., "0/50")
		// SMS should show count/limit format (e.g., "0/5")
		// The format is: number/number where limits are 50 for Telegram and 5 for SMS
		const telegramUsage = usageGrid.locator('div:has-text("Telegram")').locator('..');
		await expect(telegramUsage.locator('text=/\\d+\\/50/')).toBeVisible();

		const smsUsage = usageGrid.locator('div:has-text("SMS")').locator('..');
		await expect(smsUsage.locator('text=/\\d+\\/5/')).toBeVisible();
	});

	test('should show test notification button when channel enabled with valid input', async ({
		page,
	}) => {
		await page.goto('/dashboard/provider/support');

		// Enable Telegram and enter a chat ID
		await page.click('label:has-text("Telegram")');
		await page.fill('input[placeholder="Chat ID"]', '123456789');

		// Test button should appear
		await expect(page.locator('button:has-text("Send Test")')).toBeVisible();
	});
});
