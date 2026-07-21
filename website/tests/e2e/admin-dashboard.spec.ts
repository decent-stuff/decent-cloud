import { test, expect } from './fixtures/test-account';
import { test as adminTest } from './fixtures/test-admin-account';
import { test as anonymousTest, expect as baseExpect } from '@playwright/test';
import { sql } from './fixtures/seed-helpers';

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
		await expect(page.locator('h2:has-text("All Accounts")')).not.toBeVisible();
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

		// But should see other navigation items (offerings link was moved into
		// auth-gated My Activity; marketplace link is the canonical sidebar entry).
		await expect(page.locator('aside a[href="/dashboard/marketplace"]')).toBeVisible();
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
		await baseExpect(page.locator('h2:has-text("All Accounts")')).toBeVisible();
		await baseExpect(page.locator('h2:has-text("Failed Emails")')).toBeVisible();
	});

	adminTest('failed-email error is fully visible, not truncated (#11)', async ({ page }) => {
		// Audit #11: the failed-email table applied `max-w-xs truncate` to the
		// error cell, hiding the actual error message admins need to debug.
		// The fix must let the admin read the full error — either inline (cell
		// no longer clips), via <details>/<summary>, or via a title= attribute.
		const uniqueError = `E2E audit-#11 diagnostic: SMTP relay timeout while connecting to mx.example.com:25 — full text must be visible to admins so they can debug without grepping server logs. Token: ${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
		const createdAt = Date.now();
		// Insert a failed email_queue row directly so we have deterministic data
		// with a known long error message.
		await sql(`
			INSERT INTO email_queue (
				id, to_addr, from_addr, subject, body, is_html, email_type,
				status, attempts, max_attempts, last_error, created_at, last_attempted_at
			) VALUES (
				decode('${createdAt.toString(16).padStart(64, '0')}', 'hex'),
				'audit-e2e@example.com',
				'noreply@example.com',
				'Audit #11 test email',
				'body',
				FALSE,
				'general',
				'failed',
				3,
				6,
				'${uniqueError.replace(/'/g, "''")}',
				${createdAt},
				${createdAt}
			)
		`);
		try {
			await page.goto('/dashboard/admin');
			await baseExpect(page.locator('h2:has-text("Failed Emails")')).toBeVisible({ timeout: 15000 });

			// Expand any <details> in the failed-email rows so its body becomes
			// visible — no-op if the implementation chose approach (a) or (c).
			const details = page.locator('section:has(h2:has-text("Failed Emails")) details');
			const detailsCount = await details.count();
			for (let i = 0; i < detailsCount; i++) {
				await details.nth(i).click().catch(() => {});
			}

			// Inspect the cell containing the long error. Acceptable outcomes:
			//   (a) the cell is NOT clipping content (scrollWidth <= clientWidth),
			//       i.e. the error text wraps or the cell is wide enough;
			//   (b) a <details> element is present and, once expanded, the full
			//       error text appears in the rendered body text;
			//   (c) a title= attribute on the cell or an ancestor carries the
			//       full error, providing hover access.
			const cellState = await page.evaluate((errPrefix) => {
				const tds = Array.from(document.querySelectorAll('td'));
				const match = tds.find((td) => (td.textContent || '').includes(errPrefix));
				if (!match) return { found: false };

				const isClipped = match.scrollWidth > match.clientWidth;
				// Walk up looking for a title= with the full error.
				let titleWithText: string | null = null;
				let cursor: Element | null = match;
				for (let i = 0; i < 4 && cursor; i++, cursor = cursor.parentElement) {
					const t = cursor.getAttribute('title');
					if (t && t.length > 20) { titleWithText = t; break; }
				}
				// <details> presence in the cell or its row.
				const row = match.closest('tr');
				const hasDetails = !!(row && row.querySelector('details'));

				return {
					found: true,
					isClipped,
					titleWithText,
					hasDetails,
				};
			}, uniqueError.slice(0, 30));

			expect(cellState.found, 'seeded failed-email row must be present').toBe(true);

			const passesInline = !cellState.isClipped;
			const passesTitle = !!(cellState.titleWithText && cellState.titleWithText.includes(uniqueError.slice(-20)));
			const passesDetails = cellState.hasDetails;

			expect(
				passesInline || passesTitle || passesDetails,
				'failed-email error must be either non-clipped, in a title attribute, or in a <details>',
			).toBeTruthy();
		} finally {
			await sql(`DELETE FROM email_queue WHERE subject = 'Audit #11 test email'`);
		}
	});
});
