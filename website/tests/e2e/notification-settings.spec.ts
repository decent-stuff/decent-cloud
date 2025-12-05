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

	test('should show telegram input when telegram checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		// Click the label containing "Telegram" to toggle the checkbox
		await page.click('label:has-text("Telegram")');
		await expect(page.locator('input[placeholder="e.g. 123456789"]')).toBeVisible();
		await expect(page.locator('a[href^="https://t.me/"]')).toBeVisible();
	});

	test('should show phone input when sms checkbox checked', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await page.click('label:has-text("SMS")');
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('should show account email when email option visible', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		// Email option shows account email or prompt to add one
		await expect(
			page.locator('text=Notifications will be sent to').or(page.locator('text=Add an email'))
		).toBeVisible();
	});

	test('should allow multiple channels to be selected', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		// Select telegram and sms (email uses account email, no input needed)
		await page.click('label:has-text("Telegram")');
		await page.click('label:has-text("SMS")');
		// Telegram and SMS input fields should be visible
		await expect(page.locator('input[placeholder="e.g. 123456789"]')).toBeVisible();
		await expect(page.locator('input[placeholder="+1 555-123-4567"]')).toBeVisible();
	});

	test('should have save button', async ({ page }) => {
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('button:has-text("Save Settings")')).toBeVisible();
	});
});
