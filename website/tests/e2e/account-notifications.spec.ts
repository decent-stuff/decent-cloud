import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for /dashboard/account/notifications.
 *
 * Consolidated: snapshot assertions (channels + descriptions + usage) are in
 * one test; interaction assertions (checkbox toggles reveal inputs) are in a
 * second test. Two navigations total instead of seven.
 */

test.describe('Account Notification Settings', () => {
	test('notification channels render with descriptions and usage info', async ({ page }) => {
		// Single navigation covers what was previously four snapshot tests:
		// "display notification channels", "show email/telegram/sms channel".
		await page.goto('/dashboard/account/notifications');
		await expect(page.locator('h1:has-text("Notifications")')).toBeVisible();
		await expect(page.locator('h2:has-text("Notification Channels")')).toBeVisible();

		// Each channel label and its daily-usage counter
		await expect(page.locator('label:has-text("Email")')).toBeVisible();
		await expect(page.locator('text=/^\\d+ sent today$/')).toBeVisible();

		await expect(page.locator('label:has-text("Telegram")')).toBeVisible();
		await expect(page.locator('text=/^\\d+\\/50 sent today$/')).toBeVisible();

		await expect(page.locator('label:has-text("SMS")')).toBeVisible();
		await expect(page.locator('text=/^\\d+\\/5 sent today$/')).toBeVisible();
	});

	test('checking each channel checkbox reveals its input field', async ({ page }) => {
		// Single navigation covers what was previously three interaction tests.
		// The checkboxes are independent — toggling one does not affect the others.
		await page.goto('/dashboard/account/notifications');

		// Wait for the checkboxes to be interactive (Svelte hydrates async).
		await page.locator('#notify-email').waitFor({ state: 'visible' });

		// Email
		await page.locator('#notify-email').click();
		await expect(page.locator('input[placeholder="your@email.com"]')).toBeVisible();

		// Telegram
		await page.locator('#notify-telegram').waitFor({ state: 'visible' });
		await page.locator('#notify-telegram').click();
		await expect(page.locator('input[placeholder="Telegram Chat ID"]')).toBeVisible();

		// SMS
		await page.locator('#notify-sms').waitFor({ state: 'visible' });
		await page.locator('#notify-sms').click();
		await expect(page.locator('input[placeholder="+1234567890"]')).toBeVisible();
	});
});
