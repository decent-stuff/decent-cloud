import { testLoggedOut as test, expect } from './fixtures/test-account';
import {
	signIn,
	revealSeedPhraseOptions,
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

	test('@smoke should sign in successfully with valid credentials', async ({
		page,
		testAccountLoggedOut,
	}) => {
		// Navigate to login and reveal seed phrase options. revealSeedPhraseOptions
		// uses click-and-retry instead of networkidle (which tanks parallel runs
		// under Vite HMR — see playwright.config.ts:28-33).
		await page.goto('/login');
		await revealSeedPhraseOptions(page);

		// Click "Import Existing"
		const importButton = page.locator('button:has-text("Import Existing")');
		await importButton.click();

		// Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await expect(seedInput).toBeVisible();
		await seedInput.fill(testAccountLoggedOut.seedPhrase);

		// Continue button should be enabled
		const continueBtn = page.locator('button:has-text("Continue")');
		await expect(continueBtn).toBeEnabled();
		await continueBtn.click();

		// Should auto-detect account and show success
		await expect(
			page.locator('text=Welcome to Decent Cloud'),
		).toBeVisible({ timeout: 10000 });

		// Should show username
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();

		// Go to dashboard
		await page.click('button:has-text("Go to Dashboard")');

		// Verify dashboard access
		await expect(page).toHaveURL(/\/dashboard/);

		// Authenticated state is confirmed by the presence of the Logout button
		// (the dashboard no longer surfaces @username in its chrome)
		await expect(
			page.locator('button:has-text("Logout")'),
		).toBeVisible();
	});

	test('should reject invalid seed phrase', async ({ page }) => {
		await page.goto('/login');
		await revealSeedPhraseOptions(page);
		await page.locator('button:has-text("Import Existing")').click();

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await expect(seedInput).toBeVisible();

		// Enter invalid seed phrase. Validation runs synchronously inside
		// `validateSeedPhrase()` on the Continue click below, so there is no
		// debounce or async state to wait for here.
		await seedInput.fill('invalid seed phrase that is not valid at all');

		// Click Continue to trigger validation
		await page.click('button:has-text("Continue")');

		// Should show validation error
		await expect(
			page.locator('text=Invalid seed phrase'),
		).toBeVisible({ timeout: 2000 });
	});

	test('should maintain session after page refresh', async ({ page, testAccountLoggedOut }) => {
		// Sign in (uses the shared helper: fewer clicks + no networkidle)
		await signIn(page, testAccountLoggedOut);

		// Refresh page
		await page.reload();

		// Should still be signed in
		await expect(page).toHaveURL(/\/dashboard/);
		await expect(page.locator('button:has-text("Logout")')).toBeVisible();
	});

	test('@smoke should sign out successfully', async ({ page, testAccountLoggedOut }) => {
		// Sign in first (shared helper)
		await signIn(page, testAccountLoggedOut);

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
		// Navigate to login and reveal seed phrase options
		await page.goto('/login');
		await revealSeedPhraseOptions(page);
		await page.locator('button:has-text("Import Existing")').click();

		// Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Should show "Detecting Account" briefly then auto-sign in.
		// The account detection step may be very fast, so we wait for success.
		await expect(
			page.locator('text=Welcome to Decent Cloud'),
		).toBeVisible({ timeout: 15000 });

		// Should show the auto-detected username
		await expect(
			page.locator(`text=@${testAccountLoggedOut.username}`),
		).toBeVisible();

		// Go to dashboard
		await page.click('button:has-text("Go to Dashboard")');
		await expect(page).toHaveURL(/\/dashboard/);
	});

	test('should redirect to returnUrl after successful sign-in', async ({ page, testAccountLoggedOut }) => {
		// Navigate to login with returnUrl parameter
		await page.goto('/login?returnUrl=%2Fdashboard%2Frentals');
		await revealSeedPhraseOptions(page);
		await page.locator('button:has-text("Import Existing")').click();

		// Enter seed phrase
		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		// Should show success screen
		await expect(
			page.locator('text=Welcome to Decent Cloud'),
		).toBeVisible({ timeout: 10000 });

		// Click "Go to Dashboard"
		await page.click('button:has-text("Go to Dashboard")');

		// Should redirect to the returnUrl (rentals)
		await expect(page).toHaveURL(/\/dashboard\/rentals/, { timeout: 10000 });
	});

	test('should redirect to returnUrl when accessing protected page directly', async ({ page, testAccountLoggedOut }) => {
		// Try to access protected page directly while logged out
		await page.goto('/dashboard/account');

		// Should stay on page with login prompt (not redirect).
		// The "Login Required" text is SSR-rendered, so waiting for it is
		// deterministic (no networkidle needed).
		await expect(page).toHaveURL('/dashboard/account');
		await expect(page.getByText('Login Required')).toBeVisible();

		// Click the login button in main content. Like the seed-phrase CTA,
		// it's SSR-rendered but its onclick isn't bound until SvelteKit
		// hydrates — click-and-retry instead of networkidle. Check URL first
		// so we never click during an in-progress navigation.
		const loginButton = page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i });
		await expect(loginButton).toBeVisible();
		for (let attempt = 0; attempt < 10; attempt++) {
			if (page.url().includes('/login')) break;
			await loginButton.click({ timeout: 5000 }).catch(() => {});
			await page.waitForURL(/\/login/, { timeout: 1000 }).catch(() => {});
		}

		// Should navigate to login with returnUrl
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Faccount');

		// Complete sign-in
		await revealSeedPhraseOptions(page);
		await page.locator('button:has-text("Import Existing")').click();

		const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
		await expect(seedInput).toBeVisible({ timeout: 10000 });
		await seedInput.fill(testAccountLoggedOut.seedPhrase);
		await page.click('button:has-text("Continue")');

		await expect(
			page.locator('text=Welcome to Decent Cloud'),
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
		// Verify login page rendered (the seed-phrase button is the primary CTA after Google sign-in)
		await expect(
			page.locator('button:has-text("Sign in with seed phrase instead")'),
		).toBeVisible();
	});
});
