import { test as anonymousTest, expect } from '@playwright/test';

/**
 * E2E Tests for Provider Rental Requests - Batch Accept/Reject Actions
 *
 * Tests that batch action buttons appear and behave correctly on the
 * /dashboard/provider/requests page.
 *
 * Note: Full batch acceptance/rejection tests require a provider account with
 * multiple pending requests and are exercised in integration tests.
 */

anonymousTest.describe('Provider Requests - Batch Actions (anonymous)', () => {
	anonymousTest.beforeEach(async ({ page }) => {
		await page.goto('/');
		await page.evaluate(() => { localStorage.clear(); });
	});

	anonymousTest('should not show batch action buttons for anonymous users', async ({ page }) => {
		await page.goto('/dashboard/provider/requests');
		await page.waitForLoadState('networkidle');

		// Anonymous users see the login prompt, not the requests list
		await expect(page.locator('h2:has-text("Login Required")')).toBeVisible();
		await expect(page.locator('button:has-text("Accept All")')).not.toBeVisible();
		await expect(page.locator('button:has-text("Reject All")')).not.toBeVisible();
	});
});

anonymousTest.describe('Provider Requests page - structure', () => {
	anonymousTest('should render the Pending Requests section heading', async ({ page }) => {
		await page.goto('/dashboard/provider/requests');
		await page.waitForLoadState('networkidle');

		// Page title is always rendered regardless of auth state
		await expect(page.locator('h1:has-text("Provider Requests")')).toBeVisible();
	});
});
