import { test, expect } from './fixtures/test-account';

test.describe('Account Notification Settings', () => {
	test('should display notification channels with descriptions', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('h1:has-text("Notifications")')).toBeVisible();
		await expect(page.locator('h2:has-text("Notification Channels")')).toBeVisible();
	});

	test('should show email channel with description', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("Email")')).toBeVisible();
		await expect(
			page.locator('text=Receive notifications via email for important updates')
		).toBeVisible();
	});

	test('should show telegram channel with description', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("Telegram")')).toBeVisible();
		await expect(
			page.locator('text=Get instant notifications through Telegram bot')
		).toBeVisible();
	});

	test('should show sms channel with description', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('label:has-text("SMS")')).toBeVisible();
		await expect(
			page.locator('text=Receive text message alerts on your phone')
		).toBeVisible();
	});

	test('should show email input when email checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('label:has-text("Email")');
		await expect(page.locator('input[placeholder="your@email.com"]')).toBeVisible();
	});

	test('should show telegram input when telegram checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('label:has-text("Telegram")');
		await expect(page.locator('input[placeholder="Telegram Chat ID"]')).toBeVisible();
	});

	test('should show sms input when sms checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="+1234567890"]')).toBeVisible();
	});
});
