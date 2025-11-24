import { test, expect } from '@playwright/test';

/**
 * E2E Tests for Anonymous Browsing
 *
 * Tests that anonymous users can browse public pages without authentication
 * and are prompted to authenticate only when attempting protected actions.
 */

test.describe('Anonymous Browsing', () => {
	test('should allow anonymous user to view dashboard home', async ({ page }) => {
		await page.goto('/dashboard');

		// Should show auth prompt banner
		await expect(page.locator('text=Create an account to rent resources')).toBeVisible();

		// Should show single login/create account button in banner
		await expect(page.locator('button:has-text("Login / Create Account")')).toBeVisible();

		// Page should load without redirect
		await expect(page).toHaveURL('/dashboard');
	});

	test('should allow anonymous user to view marketplace', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Should show marketplace content
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		await expect(page.locator('text=Discover and purchase cloud services')).toBeVisible();

		// Should show auth prompt banner
		await expect(page.locator('text=Create an account to rent resources')).toBeVisible();

		// Page should not redirect
		await expect(page).toHaveURL('/dashboard/marketplace');
	});

	test('should allow anonymous user to view offerings', async ({ page }) => {
		await page.goto('/dashboard/offerings');

		// Page should load (even if empty)
		await expect(page).toHaveURL('/dashboard/offerings');

		// Should show auth prompt banner
		await expect(page.locator('button:has-text("Login / Create Account")')).toBeVisible();
	});

	test('should allow anonymous user to view validators', async ({ page }) => {
		await page.goto('/dashboard/validators');

		// Page should load
		await expect(page).toHaveURL('/dashboard/validators');

		// Should show auth prompt banner
		await expect(page.locator('button:has-text("Login / Create Account")')).toBeVisible();
	});

	test('should show auth modal when anonymous user tries to rent resource', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Wait for offerings to load
		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });

		// Check if there are any rent buttons
		const rentButton = page.locator('button:has-text("Rent Resource")').first();

		if (await rentButton.isVisible()) {
			await rentButton.click();

			// Should show auth modal
			await expect(page.locator('text=Authentication Required')).toBeVisible();
			await expect(page.locator('text=You need an account to perform this action')).toBeVisible();

			// Modal should have single login/create button and continue browsing
			await expect(page.locator('button:has-text("Login / Create Account")')).toBeVisible();
			await expect(page.locator('button:has-text("Continue Browsing")')).toBeVisible();
		}
	});

	test('should allow dismissing auth modal', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Wait for page load
		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });

		const rentButton = page.locator('button:has-text("Rent Resource")').first();

		if (await rentButton.isVisible()) {
			await rentButton.click();

			// Should show modal
			await expect(page.locator('text=Authentication Required')).toBeVisible();

			// Click "Continue Browsing"
			await page.click('button:has-text("Continue Browsing")');

			// Modal should close
			await expect(page.locator('text=Authentication Required')).not.toBeVisible();

			// Should still be on marketplace
			await expect(page).toHaveURL('/dashboard/marketplace');
		}
	});

	test('should navigate to /login with returnUrl when clicking button from banner', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Click "Login / Create Account" button in banner
		const bannerButton = page.locator('button:has-text("Login / Create Account")').first();
		await bannerButton.click();

		// Should navigate to /login with returnUrl parameter
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Fmarketplace');
	});

	test('should show sidebar for anonymous users with all navigation items', async ({ page }) => {
		await page.goto('/dashboard');

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Should show sidebar with all navigation items (now visible to anonymous users too)
		await expect(page.locator('a[href="/dashboard"]')).toBeVisible();
		await expect(page.locator('a[href="/dashboard/marketplace"]')).toBeVisible();
		await expect(page.locator('a[href="/dashboard/rentals"]')).toBeVisible();
		await expect(page.locator('a[href="/dashboard/provider/requests"]')).toBeVisible();

		// Should NOT show Account link in sidebar (it's in the bottom section for authenticated users only)
		const accountLinks = page.locator('aside a[href="/dashboard/account"]');
		await expect(accountLinks).not.toBeVisible();

		// Should show Login / Create Account button instead of Logout
		await expect(page.locator('button:has-text("Login / Create Account")')).toBeVisible();
		await expect(page.locator('button:has-text("Logout")')).not.toBeVisible();
	});

	test('should show auth prompt banner on all public dashboard pages', async ({ page }) => {
		const publicPages = [
			'/dashboard',
			'/dashboard/marketplace',
			'/dashboard/offerings',
			'/dashboard/validators'
		];

		for (const url of publicPages) {
			await page.goto(url);
			await expect(page.locator('text=Create an account to rent resources')).toBeVisible();
		}
	});
});
