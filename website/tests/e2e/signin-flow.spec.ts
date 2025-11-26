import { testLoggedOut as test, expect } from './fixtures/test-account';
import {
	registerNewAccount,
	signOut,
	setupConsoleLogging,
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
	test.beforeEach(async ({ page }) => {
		// Set up console logging to capture browser console output
		setupConsoleLogging(page);
	});

	test('should sign in successfully with valid credentials', async ({
		page,
		testAccountLoggedOut,
	}) => {
		// Step 1: Navigate to login page
		await page.goto('/login');
		await expect(page.locator('text=Import Existing')).toBeVisible();

		// Step 2: Click "Import Existing"
		await page.click('text=Import Existing');

		// Step 3: Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await expect(seedInput).toBeVisible();
		await seedInput.fill(testAccountLoggedOut.seedPhrase);

		// Continue button should be enabled
		const continueBtn = page.locator('button:has-text("Continue")');
		await expect(continueBtn).toBeEnabled();
		await continueBtn.click();

		// Step 4: Should auto-detect account and show success
		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });

		// Should show username
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();

		// Step 5: Go to dashboard
		await page.click('button:has-text("Go to Dashboard")');

		// Step 6: Verify dashboard access
		await expect(page).toHaveURL(/\/dashboard/);

		// Username should appear in header
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();
	});

	test('should reject invalid seed phrase', async ({ page }) => {
		await page.goto('/login');
		await page.click('text=Import Existing');

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await expect(seedInput).toBeVisible();

		// Enter invalid seed phrase
		await seedInput.fill('invalid seed phrase that is not valid at all');
		await page.waitForTimeout(300);

		// Click Continue to trigger validation
		await page.click('button:has-text("Continue")');

		// Should show validation error
		await expect(
			page.locator('text=Invalid seed phrase'),
		).toBeVisible({ timeout: 2000 });
	});

	test('should maintain session after page refresh', async ({ page, testAccountLoggedOut }) => {
		// Sign in
		await page.goto('/login');
		await page.click('text=Import Existing');

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });
		await page.click('button:has-text("Go to Dashboard")');

		// Verify signed in
		await expect(page).toHaveURL(/\/dashboard/);
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();

		// Refresh page
		await page.reload();

		// Should still be signed in
		await expect(page).toHaveURL(/\/dashboard/);
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();
	});

	test('should sign out successfully', async ({ page, testAccountLoggedOut }) => {
		// Sign in first
		await page.goto('/login');
		await page.click('text=Import Existing');

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });
		await page.click('button:has-text("Go to Dashboard")');

		// Wait for dashboard
		await expect(page).toHaveURL(/\/dashboard/);

		// Click logout
		await page.click('button:has-text("Logout")');

		// Should redirect to home page
		await expect(page).toHaveURL('/');
		await expect(page.locator('text=Sign In')).toBeVisible();

		// Username should not be visible
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).not.toBeVisible();
	});

	test('should auto-detect account from seed phrase', async ({ page, testAccountLoggedOut }) => {
		// Step 1: Navigate to login
		await page.goto('/login');

		// Step 2: Click "Import Existing"
		await page.click('text=Import Existing');

		// Step 3: Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Step 4: Should show "Detecting Account" briefly then auto-sign in
		// The account detection step may be very fast, so we wait for success
		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 15000 });

		// Should show the auto-detected username
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();

		// Step 5: Go to dashboard
		await page.click('button:has-text("Go to Dashboard")');
		await expect(page).toHaveURL(/\/dashboard/);
	});

	test('should redirect to returnUrl after successful sign-in', async ({ page, testAccountLoggedOut }) => {
		// Navigate to login with returnUrl parameter
		await page.goto('/login?returnUrl=%2Fdashboard%2Frentals');

		// Complete sign-in flow
		await page.click('text=Import Existing');

		// Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Should show success screen
		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });

		// Click "Go to Dashboard"
		await page.click('button:has-text("Go to Dashboard")');

		// Should redirect to the returnUrl (rentals)
		await expect(page).toHaveURL(/\/dashboard\/rentals/, { timeout: 10000 });
	});

	test('should redirect to returnUrl when accessing protected page directly', async ({ page, testAccountLoggedOut }) => {
		// Try to access protected page directly while logged out
		await page.goto('/dashboard/account');

		// Should stay on page with login prompt (not redirect)
		await expect(page).toHaveURL('/dashboard/account');
		await expect(page.getByText('Login Required')).toBeVisible();

		// Click the login button in main content
		await page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i }).click();

		// Should navigate to login with returnUrl
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Faccount');

		// Complete sign-in
		await page.click('text=Import Existing');

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		await expect(
			page.locator('text=Welcome to Decent Cloud!'),
		).toBeVisible({ timeout: 10000 });

		await page.click('button:has-text("Go to Dashboard")');

		// Should redirect back to the originally requested page (account)
		await expect(page).toHaveURL(/\/dashboard\/account/, { timeout: 10000 });
	});

	test('should redirect to login page when action=login parameter is present', async ({ page }) => {
		// Navigate with action=login parameter
		await page.goto('/?action=login');

		// Should redirect to /login page
		await expect(page).toHaveURL('/login', { timeout: 5000 });
		await expect(page.locator('text=Import Existing')).toBeVisible();
	});
});
