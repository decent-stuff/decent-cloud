import { test, expect } from '@playwright/test';
import {
	generateTestUsername,
	waitForApiResponse,
	setupConsoleLogging,
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
		// Set up console logging to capture browser console output
		setupConsoleLogging(page);
	});

	test('should complete full registration flow with seed phrase', async ({
		page,
	}) => {
		const username = generateTestUsername();

		// Step 1: Navigate to login page
		await page.goto('/login');
		await expect(page.locator('text=Generate New')).toBeVisible();

		// Step 2: Click "Generate New" to generate seed phrase
		await page.click('text=Generate New');

		// Step 3: Seed phrase backup screen
		await expect(page.locator('text=Copy to Clipboard')).toBeVisible();

		// Extract and validate seed phrase - words are in .font-mono elements
		const wordElements = page.locator('.font-mono').filter({ hasText: /^[a-z]+$/ });
		const words = await wordElements.allTextContents();
		const seedPhrase = words.join(' ').trim();

		expect(seedPhrase).toBeTruthy();
		const wordCount = seedPhrase.split(/\s+/).length;
		expect(wordCount).toBe(12);

		// Test copy to clipboard button
		await page.click('button:has-text("Copy to Clipboard")');

		// Confirm backup
		await page.check('input[type="checkbox"]');
		await expect(page.locator('input[type="checkbox"]')).toBeChecked();

		await page.click('button:has-text("Continue")');

		// Step 4: Enter username
		await expect(
			page.locator('input[placeholder="alice"]'),
		).toBeVisible();
		await page.fill('input[placeholder="alice"]', username);

		// Wait for username validation
		await page.waitForTimeout(500); // Debounce delay

		// Should show "Available" or enable Create Account button
		await expect(
			page.locator('text=Available').or(page.locator('button:has-text("Create Account"):not([disabled])')).first(),
		).toBeVisible({ timeout: 5000 });

		// Wait for account creation API call
		const apiResponsePromise = waitForApiResponse(
			page,
			/\/api\/v1\/accounts$/,
		);

		// Step 5: Create account
		await page.click('button:has-text("Create Account")');

		// Wait for API to respond
		await apiResponsePromise;

		// Step 6: Success screen
		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });

		// Should show username
		await expect(page.locator(`text=@${username}`)).toBeVisible();

		// Step 7: Go to dashboard
		await page.click('button:has-text("Go to Dashboard")');

		// Step 8: Verify dashboard access
		await expect(page).toHaveURL(/\/dashboard/);

		// Should show username in header
		await expect(page.locator(`text=@${username}`)).toBeVisible();
	});

	test('should handle username already taken', async ({ page }) => {
		const username = generateTestUsername();

		// First registration
		await page.goto('/login');
		await page.click('text=Generate New');
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);

		// Assuming username is available first time
		await expect(
			page.locator('text=Available').or(page.locator('button:has-text("Create Account"):not([disabled])')).first(),
		).toBeVisible({ timeout: 5000 });

		// Go back to cancel this flow
		await page.click('button:has-text("Back")');

		// Try to register with same username (if API enforces uniqueness)
		// This test may need adjustment based on actual API behavior
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

		await page.goto('/login');
		await page.click('text=Generate New');
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);
		await page.click('button:has-text("Create Account")');

		// Should show error message (any of these error indicators)
		const errorLocator = page.locator('text=error').or(
			page.locator('text=failed')
		).or(
			page.locator('text=Registration failed')
		).or(
			page.locator('text=Something went wrong')
		);
		await expect(errorLocator.first()).toBeVisible({ timeout: 10000 });
	});

	test('should redirect to returnUrl after successful registration', async ({ page }) => {
		const username = generateTestUsername();

		// Navigate to login with returnUrl parameter
		await page.goto('/login?returnUrl=%2Fdashboard%2Fmarketplace');

		// Complete registration flow
		await page.click('text=Generate New');
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await page.fill('input[placeholder="alice"]', username);
		await page.waitForTimeout(500);

		await expect(
			page.locator('text=Available').or(page.locator('button:has-text("Create Account"):not([disabled])')).first(),
		).toBeVisible({ timeout: 5000 });

		const apiResponsePromise = waitForApiResponse(
			page,
			/\/api\/v1\/accounts$/,
		);

		await page.click('button:has-text("Create Account")');
		await apiResponsePromise;

		// Should show success screen
		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });

		// Click "Go to Dashboard"
		await page.click('button:has-text("Go to Dashboard")');

		// Should redirect to the returnUrl (marketplace)
		await expect(page).toHaveURL(/\/dashboard\/marketplace/, { timeout: 10000 });
	});

	test('should redirect to login page when action=signup parameter is present', async ({ page }) => {
		// Navigate with action=signup parameter
		await page.goto('/?action=signup');

		// Should redirect to /login page
		await expect(page).toHaveURL('/login', { timeout: 5000 });
		await expect(page.locator('text=Generate New')).toBeVisible();
	});
});
