import { test, expect } from '@playwright/test';
import { clickAndRetry } from './fixtures/auth-helpers';

/**
 * E2E Tests for Auth Protection
 *
 * Tests that protected pages show view-only mode with login prompts
 * for anonymous users, while allowing full access for authenticated users.
 *
 * These pages are SSR'd for anonymous visitors (AuthRequiredCard renders in
 * the initial HTML — no signed `/api/v1/` fetch fires), so all `goto` calls
 * use `waitUntil: 'domcontentloaded'`. The default 'load' waits for ALL
 * resources including external fonts, which under 4-worker CPU contention
 * intermittently fail (404 / ERR_CONNECTION_CLOSED) and hold back 'load' long
 * enough to trip the assertions. 'domcontentloaded' returns as soon as the
 * SSR'd "Login Required" chrome is in the DOM.
 */

// Shared goto options for every navigation in this spec (see header comment).
const GO = { waitUntil: 'domcontentloaded' as const };

test.describe('Auth Protection', () => {
	test.beforeEach(async ({ page }) => {
		// Ensure we start logged out
		await page.goto('/', GO);
		// Clear any existing auth
		await page.evaluate(() => {
			localStorage.clear();
		});
	});

	test('should show login prompt on protected pages for anonymous users', async ({ page }) => {
		const protectedPages = [
			'/dashboard/account',
			'/dashboard/account/security',
			'/dashboard/account/profile',
			'/dashboard/rentals',
			'/dashboard/provider/requests',
			'/dashboard/offerings'
		];

		for (const pagePath of protectedPages) {
			await page.goto(pagePath, GO);

			// Should stay on the page (view-only)
			await expect(page).toHaveURL(pagePath);

			// Should show login prompt in main content
			await expect(page.getByText('Login Required')).toBeVisible();
			await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
		}
	});

	test('should redirect to /login with returnUrl when clicking login button', async ({ page }) => {
		await page.goto('/dashboard/rentals', GO);

		// AuthRequiredCard is SSR'd for anonymous visitors, so the Login button's
		// onclick binds only on hydration — clickAndRetry until the URL changes.
		const loginButton = page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i });
		await expect(loginButton).toBeVisible();
		await clickAndRetry(page, loginButton, async () =>
			page.url().includes('/login'),
		);

		// Should navigate to /login with returnUrl
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Frentals');
	});

	test('should allow access to public pages without login prompt', async ({ page }) => {
		const publicPages = [
			'/dashboard',
			'/dashboard/marketplace'
		];

		for (const pagePath of publicPages) {
			await page.goto(pagePath, GO);

			// Should NOT show login prompt
			await expect(page).toHaveURL(pagePath);

			// Should NOT see "Login Required" heading
			await expect(page.getByRole('heading', { name: 'Login Required' })).not.toBeVisible();
		}
	});

	test('offerings page renders AuthRequiredCard (not red error box) when unauthenticated (#3)', async ({ page }) => {
		// Audit #3: sister pages (rentals, invoices, account) all render
		// <AuthRequiredCard> for anonymous visitors. The offerings page instead
		// set error = 'Please authenticate to view your offerings' and rendered
		// it inside a red "Error loading offerings" panel — a hard error with no
		// recovery CTA. Fix: gate the page on isAuthenticated like rentals and
		// render the AuthRequiredCard.
		await page.goto('/dashboard/offerings', GO);

		// AuthRequiredCard renders its heading + Login button.
		await expect(page.getByRole('heading', { name: 'Login Required' })).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();

		// The red "Error loading offerings" panel must NOT appear.
		await expect(page.getByText('Error loading offerings')).toHaveCount(0);
		await expect(page.getByText('Please authenticate to view your offerings')).toHaveCount(0);
	});

	test('should show auth prompt banner on public dashboard pages', async ({ page }) => {
		await page.goto('/dashboard', GO);

		// Should show banner prompting to create account
		await expect(page.getByText(/Create an account to rent resources/i)).toBeVisible();
	});
});
