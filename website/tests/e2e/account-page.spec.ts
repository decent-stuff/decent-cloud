import { test, expect } from '@playwright/test';
import { registerNewAccount, signIn } from './fixtures/auth-helpers';
import type { AuthCredentials } from './fixtures/auth-helpers';

/**
 * E2E Tests for Account Settings Page
 *
 * Prerequisites:
 * - API server running at http://localhost:8080
 * - Dev server running at http://localhost:5173
 * - Clean test database
 */

test.describe('Account Settings Page', () => {
	let testCredentials: AuthCredentials;

	test.beforeAll(async ({ browser }) => {
		// Create a test account once
		const page = await browser.newPage();
		testCredentials = await registerNewAccount(page);
		await page.close();
	});

	test.beforeEach(async ({ page }) => {
		// Sign in before each test
		await signIn(page, testCredentials);
	});

	test('should display account overview correctly', async ({ page }) => {
		// Navigate to account page
		await page.goto('/dashboard/account');

		// Verify page title
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Verify account overview section
		await expect(page.locator('text=Account Overview')).toBeVisible();

		// Verify username is displayed
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();

		// Verify account ID is displayed (truncated hex)
		await expect(page.locator('text=Account ID')).toBeVisible();

		// Verify created date is displayed
		await expect(page.locator('text=Created')).toBeVisible();

		// Verify active keys count is displayed
		await expect(page.locator('text=Active Keys')).toBeVisible();
		await expect(page.locator('text=1 key')).toBeVisible(); // New account has 1 key
	});

	test('should copy username to clipboard', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Grant clipboard permissions
		await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);

		// Find and click the copy button next to username
		const usernameSection = page.locator('text=Username').locator('..');
		const copyButton = usernameSection.locator('button').first();

		await copyButton.click();

		// Verify copied (look for checkmark or success indicator)
		await expect(copyButton.locator('text=✓')).toBeVisible();

		// Verify clipboard contents
		const clipboardText = await page.evaluate(() =>
			navigator.clipboard.readText(),
		);
		expect(clipboardText).toBe(testCredentials.username);
	});

	test('should copy account ID to clipboard', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Grant clipboard permissions
		await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);

		// Find and click the copy button next to account ID
		const accountIdSection = page.locator('text=Account ID').locator('..');
		const copyButton = accountIdSection.locator('button').first();

		await copyButton.click();

		// Verify copied (look for checkmark)
		await expect(copyButton.locator('text=✓')).toBeVisible();

		// Clipboard should contain full account ID (hex string)
		const clipboardText = await page.evaluate(() =>
			navigator.clipboard.readText(),
		);
		expect(clipboardText).toMatch(/^[0-9a-f]+$/i);
		expect(clipboardText.length).toBeGreaterThan(20);
	});

	test('should show account link in sidebar', async ({ page }) => {
		await page.goto('/dashboard');

		// Verify "Account" link exists in sidebar
		const accountLink = page.locator('a:has-text("Account")');
		await expect(accountLink).toBeVisible();

		// Click it and verify navigation
		await accountLink.click();
		await expect(page).toHaveURL('/dashboard/account');
	});

	test('should show username in header', async ({ page }) => {
		await page.goto('/dashboard');

		// Username should appear in header
		await expect(
			page.locator(`text=@${testCredentials.username}`).first(),
		).toBeVisible();

		// Clicking username should navigate to account page
		const usernameLink = page.locator(`a:has-text("@${testCredentials.username}")`).first();
		await usernameLink.click();

		await expect(page).toHaveURL('/dashboard/account');
	});

	test('should show warning for users without accounts', async ({
		page,
	}) => {
		// For this test, we'd need to sign in with an identity that has no account
		// This would require mocking or using Internet Identity without account registration
		// Skip for now - this is an edge case for legacy users

		test.skip();
	});

	test('should handle navigation between account and profile pages', async ({
		page,
	}) => {
		// Start at account page
		await page.goto('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Navigate to profile page
		await page.click('a:has-text("Profile")');
		await expect(page).toHaveURL('/dashboard/profile');
		await expect(
			page.locator('h1:has-text("Profile Settings")'),
		).toBeVisible();

		// Navigate back to account page
		await page.click('a:has-text("Account")');
		await expect(page).toHaveURL('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();
	});

	test('should format created date as human-readable', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Find created date element
		const createdSection = page.locator('text=Created').locator('..');
		const dateText = await createdSection.textContent();

		// Should contain a month name (e.g., "January", "February")
		const hasMonthName =
			/(January|February|March|April|May|June|July|August|September|October|November|December)/.test(
				dateText || '',
			);
		expect(hasMonthName).toBeTruthy();
	});

	test('should be accessible via direct URL', async ({ page }) => {
		// Navigate directly to account page
		await page.goto('/dashboard/account');

		// Should load without errors
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();
	});
});
