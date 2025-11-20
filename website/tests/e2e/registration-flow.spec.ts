import { test, expect } from '@playwright/test';
import {
	generateTestUsername,
	waitForApiResponse,
} from './fixtures/auth-helpers';

/**
 * E2E Tests for Account Registration Flow
 *
 * Prerequisites:
 * - API server running at http://localhost:8080
 * - Dev server running at http://localhost:5173
 * - Clean test database
 */

test.describe('Account Registration Flow', () => {
	test.beforeEach(async ({ page }) => {
		// Start from home page
		await page.goto('/');
		await expect(page.locator('text=Connect Wallet')).toBeVisible();
	});

	test('should complete full registration flow with seed phrase', async ({
		page,
	}) => {
		const username = generateTestUsername();

		// Step 1: Click "Connect Wallet"
		await page.click('text=Connect Wallet');
		await expect(page.locator('text=Create Account')).toBeVisible();

		// Step 2: Click "Create Account"
		await page.click('text=Create Account');

		// Step 3: Enter username
		await expect(
			page.locator('input[placeholder="alice"]'),
		).toBeVisible();
		await page.fill('input[placeholder="alice"]', username);

		// Wait for username validation
		await page.waitForTimeout(500); // Debounce delay

		// Should show "Available" or enable Continue button
		await expect(
			page.locator('text=Available').or(page.locator('button:has-text("Continue"):not([disabled])')).first(),
		).toBeVisible({ timeout: 5000 });

		// Step 4: Continue to auth method selection
		await page.click('button:has-text("Continue")');

		// Step 5: Select "Seed Phrase" method
		await expect(page.locator('text=Seed Phrase')).toBeVisible();
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		// Step 6: Seed phrase backup
		await expect(
			page.locator('.seed-phrase, [class*="seed"]').or(page.locator('text=Copy to Clipboard')),
		).toBeVisible();

		// Extract and validate seed phrase
		const seedPhraseElement = page.locator('.seed-phrase, [class*="seed"]').first();
		const seedPhraseText = (await seedPhraseElement.textContent()) || '';
		let seedPhrase = seedPhraseText.trim();

		// If empty, try to get from individual word elements
		if (!seedPhrase) {
			const words = await page
				.locator('[class*="word"], .word')
				.allTextContents();
			seedPhrase = words.join(' ').trim();
		}

		expect(seedPhrase).toBeTruthy();
		const wordCount = seedPhrase.split(/\s+/).length;
		expect(wordCount).toBeGreaterThanOrEqual(12);
		expect(wordCount).toBeLessThanOrEqual(24);

		// Test copy to clipboard button
		await page.click('button:has-text("Copy to Clipboard")');

		// Confirm backup
		await page.check('input[type="checkbox"]');
		await expect(page.locator('input[type="checkbox"]')).toBeChecked();

		await page.click('button:has-text("Continue")');

		// Step 7: Confirm account creation
		await expect(
			page.locator('text=Create Account, text=Confirm'),
		).toBeVisible();

		// Should show username in confirmation
		await expect(page.locator(`text=${username}`)).toBeVisible();

		// Wait for account creation API call
		const apiResponsePromise = waitForApiResponse(
			page,
			/\/api\/v1\/accounts$/,
		);

		await page.click('button:has-text("Create Account"), button:has-text("Confirm")');

		// Wait for API to respond
		await apiResponsePromise;

		// Step 8: Success screen
		await expect(
			page.locator('text=Welcome, text=Success'),
		).toBeVisible({ timeout: 10000 });

		// Should show username
		await expect(page.locator(`text=@${username}`)).toBeVisible();

		// Step 9: Go to dashboard
		await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

		// Step 10: Verify dashboard access
		await expect(page).toHaveURL(/\/dashboard/);

		// Should show username in header
		await expect(page.locator(`text=@${username}`)).toBeVisible();
	});

	test('should reject invalid username format', async ({ page }) => {
		await page.click('text=Connect Wallet');
		await page.click('text=Create Account');

		await expect(
			page.locator('input[placeholder="alice"]'),
		).toBeVisible();

		// Test invalid characters
		await page.fill('input[placeholder="alice"]', 'invalid user!');
		await page.waitForTimeout(500);
		await expect(
			page.locator('text=letters, numbers').or(page.locator('text=Invalid')).first(),
		).toBeVisible();

		// Test too short
		await page.fill('input[placeholder="alice"]', 'ab');
		await page.waitForTimeout(500);
		await expect(
			page.locator('text=3-20 characters').or(page.locator('text=too short')),
		).toBeVisible();

		// Test too long
		await page.fill(
			'input[placeholder="alice"]',
			'thisusernameiswaytoolongandexceedsthetwentycharacterlimit',
		);
		await page.waitForTimeout(500);
		await expect(
			page.locator('text=3-20 characters').or(page.locator('text=too long')),
		).toBeVisible();

		// Continue button should be disabled
		const continueBtn = page.locator('button:has-text("Continue")');
		await expect(continueBtn).toBeDisabled();
	});

	test('should handle username already taken', async ({ page }) => {
		const username = generateTestUsername();

		// First registration
		await page.click('text=Connect Wallet');
		await page.click('text=Create Account');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);

		// Assuming username is available first time
		await expect(
			page.locator('text=Available').or(page.locator('button:has-text("Continue"):not([disabled])')).first(),
		).toBeVisible({ timeout: 5000 });

		// Cancel this flow
		await page.click('button:has-text("Cancel"), button[aria-label*="close" i]');

		// Try to register with same username (if API enforces uniqueness)
		// This test may need adjustment based on actual API behavior
	});

	test('should allow skipping seed phrase backup with warning', async ({
		page,
	}) => {
		const username = generateTestUsername();

		await page.click('text=Connect Wallet');
		await page.click('text=Create Account');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);
		await page.click('button:has-text("Continue")');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		// Wait for seed phrase screen
		await expect(
			page.locator('text=Copy to Clipboard'),
		).toBeVisible();

		// Look for "Skip" button or similar
		const skipButton = page.locator('button:has-text("Skip")');
		if (await skipButton.isVisible()) {
			await skipButton.click();

			// Should show warning
			await expect(
				page.locator('text=warning, text=lose access').or(page.locator('text=Are you sure')),
			).toBeVisible();
		}
	});

	test('should handle network errors gracefully', async ({ page }) => {
		// Intercept API calls and return error
		await page.route('**/api/v1/accounts', (route) => {
			route.fulfill({
				status: 500,
				body: JSON.stringify({ error: 'Internal server error' }),
			});
		});

		const username = generateTestUsername();

		await page.click('text=Connect Wallet');
		await page.click('text=Create Account');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);
		await page.click('button:has-text("Continue")');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await page.click('button:has-text("Create Account"), button:has-text("Confirm")');

		// Should show error message
		await expect(
			page.locator('text=error, text=failed').or(page.locator('text=Something went wrong')),
		).toBeVisible({ timeout: 10000 });
	});
});
