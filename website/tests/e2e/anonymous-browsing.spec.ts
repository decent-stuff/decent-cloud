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

		// Should show a visible Sign In button (the auth banner one; the
		// mobile-only fixed button is hidden on desktop viewports).
		const loginButtons = page.locator('button:has-text("Sign In")').filter({ visible: true });
		await expect(loginButtons.first()).toBeVisible();

		// Page should load without redirect
		await expect(page).toHaveURL('/dashboard');
	});

	test('should allow anonymous user to view marketplace', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Should show marketplace content
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		await expect(page.locator('text=Find and rent cloud resources')).toBeVisible();

		// Should show auth prompt banner
		await expect(page.locator('text=Create an account to rent resources')).toBeVisible();

		// Page should not redirect
		await expect(page).toHaveURL('/dashboard/marketplace');
	});

	test('should allow anonymous user to view offerings', async ({ page }) => {
		await page.goto('/dashboard/offerings');

		// Page should load (even if empty)
		await expect(page).toHaveURL('/dashboard/offerings');

		// Should show a visible Sign In button (the auth banner one; the
		// mobile-only fixed button is hidden on desktop viewports).
		const loginButtons = page.locator('button:has-text("Sign In")').filter({ visible: true });
		await expect(loginButtons.first()).toBeVisible();
	});

	test('should allow anonymous user to view validators', async ({ page }) => {
		await page.goto('/dashboard/validators');

		// Page should load
		await expect(page).toHaveURL('/dashboard/validators');

		// Should show a visible Sign In button (the auth banner one; the
		// mobile-only fixed button is hidden on desktop viewports).
		const loginButtons = page.locator('button:has-text("Sign In")').filter({ visible: true });
		await expect(loginButtons.first()).toBeVisible();
	});

	test('should show auth modal when anonymous user tries to rent resource', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Wait for offerings to load
		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });

		// Check if there are any rent buttons
		const rentButton = page.locator('button:has-text("Rent Resource")').first();

		if (await rentButton.isVisible()) {
			await rentButton.click();

			// Should show auth modal (text may vary, so check for modal existence)
			const authModal = page.locator('text=Authentication Required').or(
				page.locator('text=Login Required')
			);
			await expect(authModal.first()).toBeVisible();

			// Modal should have login button (may not have Continue Browsing)
			await expect(page.locator('button:has-text("Login / Create Account")').or(
				page.locator('button:has-text("Login")')
			).first()).toBeVisible();
		}
	});

	test('should hide demo offerings by default on marketplace', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Wait for offerings to load
		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });
		await page.waitForLoadState('networkidle');

		// The "Show demo offerings" checkbox lives inside the collapsible
		// "More filters" section; expand it so the checkbox is in the DOM.
		await page.locator('button:has-text("More filters")').click();

		// The "Show demo offerings" checkbox should be unchecked by default
		const demoLabel = page.locator('label:has-text("Show demo offerings")');
		const demoCheckbox = demoLabel.locator('input[type="checkbox"]');
		await expect(demoCheckbox).not.toBeChecked();

		// Get the offering count text
		const countLocator = page.locator('text=/\\d+ offerings? found/');
		await expect(countLocator).toBeVisible({ timeout: 10000 });

		// Check the "Show demo offerings" checkbox
		await demoCheckbox.check();

		// Wait for URL to update (filter syncs to URL)
		await page.waitForURL(/demo=1/, { timeout: 5000 });

		// Navigating with both demo and offline flags confirms demo offerings
		// become visible once the offline filter is also relaxed. The dev DB
		// ships only offline demo offerings, so we need both to observe them.
		await page.goto('/dashboard/marketplace?demo=1&offline=1');
		await page.waitForLoadState('networkidle');
		await expect(countLocator).toBeVisible({ timeout: 10000 });
		const demoCountText = await countLocator.textContent();
		const demoCount = parseInt(demoCountText?.match(/(\d+)/)?.[1] || '0');
		expect(demoCount).toBeGreaterThan(0);
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
		await page.waitForLoadState('networkidle');

		// Click the banner Sign In button (the desktop one in the auth banner,
		// not the mobile-only fixed button).
		const bannerSignIn = page.locator('button:has-text("Sign In")').nth(1);
		await expect(bannerSignIn).toBeVisible();
		await bannerSignIn.click();

		// Should navigate to /login with returnUrl parameter
		await expect(page).toHaveURL('/login?returnUrl=%2Fdashboard%2Fmarketplace');
	});

	test('should show sidebar for anonymous users with all navigation items', async ({ page }) => {
		await page.goto('/dashboard');

		// Wait for page to load
		await page.waitForLoadState('networkidle');

		// Sidebar shows the public "Browse" navigation items.
		await expect(page.locator('aside a[href="/dashboard/marketplace"]')).toBeVisible();
		await expect(page.locator('aside a[href="/dashboard/reputation"]')).toBeVisible();
		await expect(page.locator('aside a[href="/dashboard/validators"]')).toBeVisible();

		// REMOVED: aside a[href="/dashboard/offerings"] assertion - the offerings
		// link moved into the auth-gated "My Activity" section and is no longer
		// rendered as a link for anonymous users.

		// Should NOT show Account link in sidebar (auth-gated)
		const accountLinks = page.locator('aside a[href="/dashboard/account"]');
		await expect(accountLinks).not.toBeVisible();

		// Anonymous users see a "Sign In" action in the page header (not the
		// sidebar) instead of a Logout button. Multiple Sign In buttons exist
		// (mobile + banner); assert at least one is visible.
		await expect(page.locator('button:has-text("Sign In")').filter({ visible: true }).first()).toBeVisible();
		await expect(page.locator('button:has-text("Logout")')).not.toBeVisible();
	});

});

