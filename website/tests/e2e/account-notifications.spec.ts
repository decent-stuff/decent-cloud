import { test, expect } from './fixtures/test-account';

test.describe('Account Notification Settings', () => {
	test('should display notification channels with descriptions', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('h1:has-text("Notifications")')).toBeVisible();
		await expect(page.locator('h2:has-text("Notification Channels")')).toBeVisible();
	});

	test('should show email channel with usage info', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("Email")')).toBeVisible();
		// Static descriptions were removed; per-channel usage info now identifies each channel
		await expect(page.locator('text=/^\\d+ sent today$/')).toBeVisible();
	});

	test('should show telegram channel with usage info', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("Telegram")')).toBeVisible();
		await expect(page.locator('text=/^\\d+\\/50 sent today$/')).toBeVisible();
	});

	test('should show sms channel with usage info', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("SMS")')).toBeVisible();
		await expect(page.locator('text=/^\\d+\\/5 sent today$/')).toBeVisible();
	});

	test('should show email input when email checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.waitForLoadState('networkidle');
		// Check the channel checkbox directly by ID; the dependent input appears
		// client-side once config state updates.
		await page.check('#notify-email');
		await expect(page.locator('input[placeholder="your@email.com"]')).toBeVisible();
	});

	test('should show telegram input when telegram checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.waitForLoadState('networkidle');
		await page.check('#notify-telegram');
		await expect(page.locator('input[placeholder="Telegram Chat ID"]')).toBeVisible();
	});

	test('should show sms input when sms checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.waitForLoadState('networkidle');
		await page.check('#notify-sms');
		await expect(page.locator('input[placeholder="+1234567890"]')).toBeVisible();
	});
});
