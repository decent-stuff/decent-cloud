import { test, expect } from './fixtures/test-account';
import { test as anonymousTest } from '@playwright/test';

/**
 * E2E Tests for Admin Dashboard
 *
 * Tests admin dashboard access control and basic functionality.
 * Note: Full admin functionality tests require an admin account.
 */

anonymousTest.describe('Admin Dashboard - Anonymous Access', () => {
	anonymousTest.beforeEach(async ({ page }) => {
		// Ensure we start logged out
		await page.goto('/');
		await page.evaluate(() => {
			localStorage.clear();
		});
	});

	anonymousTest('should show access denied for anonymous users', async ({ page }) => {
		await page.goto('/dashboard/admin');
		await page.waitForLoadState('networkidle');

		// Should show Access Denied
		await expect(page.locator('h2:has-text("Access Denied")')).toBeVisible();
		await expect(
			page.locator('text=You do not have admin privileges to access this page')
		).toBeVisible();
	});
});

test.describe('Admin Dashboard - Authenticated Non-Admin', () => {
	test('should show access denied for non-admin users', async ({ page }) => {
		await page.goto('/dashboard/admin');
		await page.waitForLoadState('networkidle');

		// Should show Access Denied (regular test users are not admins)
		await expect(page.locator('h2:has-text("Access Denied")')).toBeVisible();
		await expect(
			page.locator('text=You do not have admin privileges to access this page')
		).toBeVisible();
	});

	test('should display admin dashboard title', async ({ page }) => {
		await page.goto('/dashboard/admin');

		// Page title should still be visible
		await expect(page.locator('h1:has-text("Admin Dashboard")')).toBeVisible();
	});

	test('should not show admin features when access denied', async ({ page }) => {
		await page.goto('/dashboard/admin');
		await page.waitForLoadState('networkidle');

		// Should NOT show admin-only sections
		await expect(page.locator('h2:has-text("Email Queue Statistics")')).not.toBeVisible();
		await expect(page.locator('h2:has-text("Send Test Email")')).not.toBeVisible();
		await expect(page.locator('h2:has-text("Account Lookup")')).not.toBeVisible();
		await expect(page.locator('h2:has-text("Failed Emails")')).not.toBeVisible();
	});
});
