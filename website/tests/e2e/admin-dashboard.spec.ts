import { test, expect } from './fixtures/test-account';
import { test as adminTest } from './fixtures/test-admin-account';
import { test as anonymousTest, expect as baseExpect } from '@playwright/test';

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

	test('should not show Admin link in sidebar for non-admin users', async ({ page }) => {
		// Go to any dashboard page to load the sidebar
		await page.goto('/dashboard/marketplace');
		await page.waitForLoadState('networkidle');

		// Non-admin user should NOT see the Admin link in sidebar
		// This tests that isAdmin is properly returned from the API
		const adminLink = page.locator('aside a[href="/dashboard/admin"]');
		await expect(adminLink).not.toBeVisible();

		// But should see other navigation items
		await expect(page.locator('aside a[href="/dashboard/marketplace"]')).toBeVisible();
		await expect(page.locator('aside a[href="/dashboard/offerings"]')).toBeVisible();
	});
});

adminTest.describe('Admin Dashboard - Authenticated Admin', () => {
	adminTest('should show Admin link in sidebar for admin users', async ({ page }) => {
		// Go to any dashboard page to load the sidebar
		await page.goto('/dashboard/marketplace');
		await page.waitForLoadState('networkidle');

		// Admin user SHOULD see the Admin link in sidebar
		const adminLink = page.locator('aside a[href="/dashboard/admin"]');
		await baseExpect(adminLink).toBeVisible();
	});

	adminTest('should show admin features when user is admin', async ({ page }) => {
		await page.goto('/dashboard/admin');
		await page.waitForLoadState('networkidle');

		// Should NOT show Access Denied
		await baseExpect(page.locator('h2:has-text("Access Denied")')).not.toBeVisible();

		// Should show admin-only sections
		await baseExpect(page.locator('h2:has-text("Email Queue Statistics")')).toBeVisible();
		await baseExpect(page.locator('h2:has-text("Send Test Email")')).toBeVisible();
		await baseExpect(page.locator('h2:has-text("Account Lookup")')).toBeVisible();
		await baseExpect(page.locator('h2:has-text("Failed Emails")')).toBeVisible();
	});
});
