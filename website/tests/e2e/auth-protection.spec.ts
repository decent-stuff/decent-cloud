import { test, expect } from '@playwright/test';

/**
 * E2E Tests for Auth Protection
 *
 * Tests that protected pages show view-only mode with login prompts
 * for anonymous users, while allowing full access for authenticated users.
 */

test.describe('Auth Protection', () => {
	test.beforeEach(async ({ page }) => {
		// Ensure we start logged out
		await page.goto('/');
		// Clear any existing auth
		await page.evaluate(() => {
			localStorage.clear();
		});
	});

	test('should show login prompt on /dashboard/account for anonymous user', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Should stay on the page (view-only)
		await expect(page).toHaveURL('/dashboard/account');

		// Should show login prompt in main content
		await expect(page.getByText('Login Required')).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
	});

	test('should show login prompt on /dashboard/account/security for anonymous user', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Should stay on the page (view-only)
		await expect(page).toHaveURL('/dashboard/account/security');

		// Should show login prompt in main content
		await expect(page.getByText('Login Required')).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
	});

	test('should show login prompt on /dashboard/account/profile for anonymous user', async ({ page }) => {
		await page.goto('/dashboard/account/profile');

		// Should stay on the page (view-only)
		await expect(page).toHaveURL('/dashboard/account/profile');

		// Should show login prompt in main content
		await expect(page.getByText('Login Required')).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
	});

	test('should show login prompt on /dashboard/rentals for anonymous user', async ({ page }) => {
		await page.goto('/dashboard/rentals');

		// Should stay on the page (view-only)
		await expect(page).toHaveURL('/dashboard/rentals');

		// Should show login prompt in main content
		await expect(page.getByText('Login Required')).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
	});

	test('should show login prompt on /dashboard/provider/requests for anonymous user', async ({ page }) => {
		await page.goto('/dashboard/provider/requests');

		// Should stay on the page (view-only)
		await expect(page).toHaveURL('/dashboard/provider/requests');

		// Should show login prompt in main content
		await expect(page.getByText('Login Required')).toBeVisible();
		await expect(page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i })).toBeVisible();
	});

	test('should redirect to /login with returnUrl when clicking login button', async ({ page }) => {
		await page.goto('/dashboard/rentals');

		// Click the login button in main content
		await page.getByRole('main').getByRole('button', { name: /Login \/ Create Account/i }).click();

		// Should navigate to /login with returnUrl
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Frentals');
	});

	test('should allow access to public pages without login prompt', async ({ page }) => {
		const publicPages = [
			'/dashboard',
			'/dashboard/marketplace',
			'/dashboard/offerings',
			'/dashboard/validators'
		];

		for (const pagePath of publicPages) {
			await page.goto(pagePath);

			// Should NOT show login prompt
			await expect(page).toHaveURL(pagePath);

			// Should NOT see "Login Required" heading
			await expect(page.getByRole('heading', { name: 'Login Required' })).not.toBeVisible();
		}
	});

	test('should show auth prompt banner on public dashboard pages', async ({ page }) => {
		await page.goto('/dashboard');

		// Should show banner prompting to create account
		await expect(page.getByText(/Create an account to rent resources/i)).toBeVisible();
	});
});
