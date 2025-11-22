import { test, expect } from '@playwright/test';
import {
	registerNewAccount,
	signIn,
	setupConsoleLogging,
} from './fixtures/auth-helpers';
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
		setupConsoleLogging(page);
		testCredentials = await registerNewAccount(page);
		await page.close();
	});

	test.beforeEach(async ({ page }) => {
		// Set up console logging to capture browser console output
		setupConsoleLogging(page);

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

	test('should handle navigation between account sections', async ({
		page,
	}) => {
		// Start at account page
		await page.goto('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Navigate to security section
		await page.click('a:has-text("Security")');
		await expect(page).toHaveURL('/dashboard/account/security');
		await expect(
			page.locator('h1:has-text("Security")'),
		).toBeVisible();

		// Navigate back to account overview
		await page.click('a:has-text("Account")');
		await expect(page).toHaveURL('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Navigate to profile section
		await page.click('a:has-text("Public Profile")');
		await expect(page).toHaveURL('/dashboard/account/profile');
		await expect(
			page.locator('h1:has-text("Public Profile")'),
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

	test('should edit device name', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Find the Devices section
		await expect(page.locator('text=Devices')).toBeVisible();

		// Click on device name to start editing (default is "Unnamed Device")
		const deviceNameBtn = page.locator('button:has-text("Unnamed Device")').first();
		await deviceNameBtn.click();

		// Should show edit input
		const editInput = page.locator('input[placeholder="Device name"]');
		await expect(editInput).toBeVisible();

		// Enter new device name
		const newName = 'My Test Device';
		await editInput.fill(newName);

		// Click Save
		await page.click('button:has-text("Save")');

		// Wait for save to complete - button should disappear
		await expect(editInput).not.toBeVisible({ timeout: 5000 });

		// New device name should be displayed
		await expect(page.locator(`button:has-text("${newName}")`)).toBeVisible();
	});

	test('should cancel device name edit', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Click on device name to start editing
		const deviceNameBtn = page.locator('button').filter({ hasText: /Device|Unnamed/ }).first();
		await deviceNameBtn.click();

		// Should show edit input
		const editInput = page.locator('input[placeholder="Device name"]');
		await expect(editInput).toBeVisible();

		// Click Cancel
		await page.click('button:has-text("Cancel")');

		// Edit input should disappear
		await expect(editInput).not.toBeVisible();
	});

	test('should not show Remove button for single key account', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Find the Devices section
		await expect(page.locator('text=Devices')).toBeVisible();

		// Should show "1 key" in Active Keys
		await expect(page.locator('text=1 key')).toBeVisible();

		// Remove button should NOT be visible (can't remove last key)
		await expect(page.locator('button:has-text("Remove")')).not.toBeVisible();
	});

	test('should display device key info correctly', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Should show device with Active status
		await expect(page.locator('text=Active').first()).toBeVisible();

		// Should show key icon for active key
		await expect(page.locator('text=🔑')).toBeVisible();

		// Should show truncated public key (hex format)
		const keyDisplay = page.locator('.font-mono').filter({ hasText: /[0-9a-f]+\.\.\.[0-9a-f]+/i });
		await expect(keyDisplay.first()).toBeVisible();
	});

	test('should show Add Device button', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Should show Add Device button
		const addDeviceBtn = page.locator('button:has-text("+ Add Device")');
		await expect(addDeviceBtn).toBeVisible();
	});

	test('should open Add Device modal', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Click Add Device button
		await page.click('button:has-text("+ Add Device")');

		// Modal should appear with seed phrase
		await expect(page.locator('text=Add New Device')).toBeVisible();
		await expect(page.locator('text=Generate a new seed phrase')).toBeVisible();

		// Should show 12 words in grid
		const wordElements = page.locator('.font-mono.font-semibold');
		await expect(wordElements).toHaveCount(12);
	});

	test('should add new device with seed phrase', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Initial key count
		await expect(page.locator('text=1 key')).toBeVisible();

		// Click Add Device button
		await page.click('button:has-text("+ Add Device")');

		// Wait for modal
		await expect(page.locator('text=Add New Device')).toBeVisible();

		// Check the confirmation checkbox
		await page.check('input[type="checkbox"]');

		// Click Add Device
		await page.click('button:has-text("Add Device")');

		// Wait for success
		await expect(page.locator('text=Device Added!')).toBeVisible({ timeout: 10000 });

		// Close modal
		await page.click('button:has-text("Done")');

		// Should now show 2 keys
		await expect(page.locator('text=2 keys')).toBeVisible({ timeout: 5000 });

		// Remove button should now be visible (since there are 2 keys)
		await expect(page.locator('button:has-text("Remove")').first()).toBeVisible();
	});

	test('should cancel Add Device modal', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Click Add Device button
		await page.click('button:has-text("+ Add Device")');

		// Wait for modal
		await expect(page.locator('text=Add New Device')).toBeVisible();

		// Click Cancel
		await page.click('button:has-text("Cancel")');

		// Modal should close
		await expect(page.locator('text=Add New Device')).not.toBeVisible();

		// Still should have 1 key
		await expect(page.locator('text=1 key')).toBeVisible();
	});
});
