import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Notification Settings Page
 */
test.describe('Notification Settings Page', () => {
	test('should navigate to notifications from account page', async ({ page }) => {
		await page.goto('/dashboard/account');
		await page.click('a:has-text("Notifications")');
		await expect(page).toHaveURL('/dashboard/account/notifications');
		await expect(page.locator('h1:has-text("Notifications")')).toBeVisible();
	});

	test('should display channel selection options', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('text=Email')).toBeVisible();
		await expect(page.locator('text=Telegram')).toBeVisible();
		await expect(page.locator('text=SMS')).toBeVisible();
	});

	test('should show telegram input when telegram selected', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('input[value="telegram"]');
		await expect(page.locator('input[placeholder="e.g. 123456789"]')).toBeVisible();
		await expect(page.locator('text=@DecentCloudBot')).toBeVisible();
	});

	test('should show phone input when sms selected', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('input[value="sms"]');
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('should have save button', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('button:has-text("Save Settings")')).toBeVisible();
	});
});
