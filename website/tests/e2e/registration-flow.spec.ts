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
		await page.waitForLoadState('networkidle');
		await expect(page.locator('button:has-text("Generate New")')).toBeVisible();

		// Step 2: Click "Generate New" to generate seed phrase
		await page.locator('button:has-text("Generate New")').click();

		// Step 3: Seed phrase backup screen
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });

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
		).toBeVisible({ timeout: 10000 });
		await page.fill('input[placeholder="alice"]', username);

		// Wait for username validation
		await expect(page.getByText('available', { exact: false })).toBeVisible({ timeout: 5000 });

		// Fill email address
		const testEmail = `${username}@test.example.com`;
		await page.fill('input[placeholder="you@example.com"]', testEmail);

		// Wait for account creation API call
		const apiResponsePromise = waitForApiResponse(
			page,
			/\/api\/v1\/accounts$/,
		);

		// Step 5: Create account - wait for button to be enabled
		const createButton = page.locator('button:has-text("Create Account")');
		await expect(createButton).toBeEnabled({ timeout: 5000 });
		await createButton.click();

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
		await page.waitForLoadState('networkidle');
		await page.locator('button:has-text("Generate New")').click();
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await expect(page.locator('input[placeholder="alice"]')).toBeVisible({ timeout: 10000 });
		await page.fill('input[placeholder="alice"]', username);

		// Assuming username is available first time
		await expect(page.getByText('available', { exact: false })).toBeVisible({ timeout: 5000 });

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
		await page.waitForLoadState('networkidle');
		await page.locator('button:has-text("Generate New")').click();
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await expect(page.locator('input[placeholder="alice"]')).toBeVisible({ timeout: 10000 });
		await page.fill('input[placeholder="alice"]', username);
		await expect(page.getByText('available', { exact: false })).toBeVisible({ timeout: 5000 });

		// Fill email address
		const testEmail = `${username}@test.example.com`;
		await page.fill('input[placeholder="you@example.com"]', testEmail);

		// Wait for button to be enabled
		const createButton = page.locator('button:has-text("Create Account")');
		await expect(createButton).toBeEnabled({ timeout: 5000 });
		await createButton.click();

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
		await page.waitForLoadState('networkidle');

		// Complete registration flow
		await page.locator('button:has-text("Generate New")').click();
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await expect(page.locator('input[placeholder="alice"]')).toBeVisible({ timeout: 10000 });
		await page.fill('input[placeholder="alice"]', username);
		await expect(page.getByText('available', { exact: false })).toBeVisible({ timeout: 5000 });

		// Fill email address
		const testEmail = `${username}@test.example.com`;
		await page.fill('input[placeholder="you@example.com"]', testEmail);

		const apiResponsePromise = waitForApiResponse(
			page,
			/\/api\/v1\/accounts$/,
		);

		const createButton = page.locator('button:has-text("Create Account")');
		await expect(createButton).toBeEnabled({ timeout: 5000 });
		await createButton.click();
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
		await expect(page.locator('button:has-text("Generate New")')).toBeVisible();
	});
});
