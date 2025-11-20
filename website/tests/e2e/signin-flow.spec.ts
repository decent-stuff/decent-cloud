import { test, expect } from '@playwright/test';
import {
	registerNewAccount,
	signOut,
	type AuthCredentials,
} from './fixtures/auth-helpers';

/**
 * E2E Tests for Sign-In Flow
 *
 * Prerequisites:
 * - API server running at http://localhost:8080
 * - Dev server running at http://localhost:5173
 * - Clean test database
 */

test.describe('Sign-In Flow', () => {
	let testCredentials: AuthCredentials;

	test.beforeAll(async ({ browser }) => {
		// Create a test account once for all sign-in tests
		const page = await browser.newPage();
		testCredentials = await registerNewAccount(page);
		await signOut(page);
		await page.close();
	});

	test.beforeEach(async ({ page }) => {
		// Start from home page
		await page.goto('/');
		await expect(page.locator('text=Connect Wallet')).toBeVisible();
	});

	test('should sign in successfully with valid credentials', async ({
		page,
	}) => {
		// Step 1: Click "Connect Wallet"
		await page.click('text=Connect Wallet');
		await expect(page.locator('text=Sign In')).toBeVisible();

		// Step 2: Click "Sign In"
		await page.click('text=Sign In');

		// Step 3: Select "Seed Phrase" method
		await expect(page.locator('text=Seed Phrase')).toBeVisible();
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		// Step 4: Enter seed phrase
		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await expect(seedInput).toBeVisible();
		await seedInput.fill(testCredentials.seedPhrase);

		// Continue button should be enabled
		const continueBtn = page.locator('button:has-text("Continue")');
		await expect(continueBtn).toBeEnabled();
		await continueBtn.click();

		// Step 5: Enter username
		await expect(
			page.locator('input[placeholder="alice"]'),
		).toBeVisible();
		await page.fill(
			'input[placeholder="alice"]',
			testCredentials.username,
		);

		// Wait for account lookup
		await page.waitForTimeout(500);

		// Step 6: Complete sign-in
		await page.click('button:has-text("Continue"), button:has-text("Sign In")');

		// Step 7: Success screen
		await expect(
			page.locator('text=Welcome').or(page.locator('text=Success')),
		).toBeVisible({ timeout: 10000 });

		// Should show username
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();

		// Step 8: Go to dashboard
		await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

		// Step 9: Verify dashboard access
		await expect(page).toHaveURL(/\/dashboard/);

		// Username should appear in header
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();
	});

	test('should reject invalid seed phrase', async ({ page }) => {
		await page.click('text=Connect Wallet');
		await page.click('text=Sign In');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await expect(seedInput).toBeVisible();

		// Enter invalid seed phrase
		await seedInput.fill('invalid seed phrase that is not valid at all');
		await page.waitForTimeout(300);

		// Should show validation error or disabled continue button
		const continueBtn = page.locator('button:has-text("Continue")');
		const isDisabled = await continueBtn.isDisabled();
		const hasError = await page
			.locator('text=Invalid, text=valid seed phrase')
			.isVisible()
			.catch(() => false);

		expect(isDisabled || hasError).toBeTruthy();
	});

	test('should reject sign-in with non-existent username', async ({
		page,
	}) => {
		await page.click('text=Connect Wallet');
		await page.click('text=Sign In');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		// Use valid seed phrase but different username
		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await seedInput.fill(testCredentials.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Enter non-existent username
		await page.fill(
			'input[placeholder="alice"]',
			'nonexistentuser123456',
		);
		await page.waitForTimeout(500);

		// Try to continue
		await page.click('button:has-text("Continue"), button:has-text("Sign In")');

		// Should show error
		await expect(
			page.locator('text=not found, text=does not exist').or(page.locator('text=Account not found')),
		).toBeVisible({ timeout: 5000 });
	});

	test('should reject sign-in when public key not in account', async ({
		page,
	}) => {
		// Create a new account
		const newPage = await page.context().newPage();
		const newCredentials = await registerNewAccount(newPage);
		await signOut(newPage);
		await newPage.close();

		// Try to sign in with old seed phrase but new username
		await page.click('text=Connect Wallet');
		await page.click('text=Sign In');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await seedInput.fill(testCredentials.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Enter the NEW username (which doesn't have this public key)
		await page.fill(
			'input[placeholder="alice"]',
			newCredentials.username,
		);
		await page.click('button:has-text("Continue"), button:has-text("Sign In")');

		// Should show error about key not matching
		await expect(
			page.locator('text=not authorized, text=key does not match').or(page.locator('text=Public key not found')),
		).toBeVisible({ timeout: 5000 });
	});

	test('should maintain session after page refresh', async ({ page }) => {
		// Sign in
		await page.click('text=Connect Wallet');
		await page.click('text=Sign In');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await seedInput.fill(testCredentials.seedPhrase);
		await page.click('button:has-text("Continue")');
		await page.fill(
			'input[placeholder="alice"]',
			testCredentials.username,
		);
		await page.click('button:has-text("Continue"), button:has-text("Sign In")');

		await expect(
			page.locator('text=Welcome').or(page.locator('text=Success')),
		).toBeVisible({ timeout: 10000 });
		await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

		// Verify signed in
		await expect(page).toHaveURL(/\/dashboard/);
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();

		// Refresh page
		await page.reload();

		// Should still be signed in
		await expect(page).toHaveURL(/\/dashboard/);
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).toBeVisible();
	});

	test('should sign out successfully', async ({ page }) => {
		// Sign in first
		await page.click('text=Connect Wallet');
		await page.click('text=Sign In');
		await page.click('text=Seed Phrase');
		await page.click('button:has-text("Continue")');

		const seedInput = page.locator(
			'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
		);
		await seedInput.fill(testCredentials.seedPhrase);
		await page.click('button:has-text("Continue")');
		await page.fill(
			'input[placeholder="alice"]',
			testCredentials.username,
		);
		await page.click('button:has-text("Continue"), button:has-text("Sign In")');

		await expect(
			page.locator('text=Welcome').or(page.locator('text=Success')),
		).toBeVisible({ timeout: 10000 });
		await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

		// Click logout
		await page.click('button:has-text("Logout")');

		// Should redirect to home page
		await expect(page).toHaveURL('/');
		await expect(page.locator('text=Connect Wallet')).toBeVisible();

		// Username should not be visible
		await expect(
			page.locator(`text=@${testCredentials.username}`),
		).not.toBeVisible();
	});
});
