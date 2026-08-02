/**
 * NotificationBell unread-count polling behavior.
 *
 * Regression coverage for the profiling finding that
 * GET /api/v1/users/{pubkey}/notifications/unread-count was called ~10.7
 * times per test (376 calls across 35 tests, 10.8s total server time). The
 * root cause was that NotificationBell re-fetched on every activeIdentity
 * store emit, and authStore.initialize() emits several times per page load
 * (redundant layout calls + intermediate account-load state transitions).
 *
 * The contract these tests pin down:
 *   1. The bell badge renders the correct unread count from the DB.
 *   2. A single dashboard page load triggers exactly ONE unread-count fetch
 *      (not one per intermediate identity emit).
 *   3. Client-side navigation between dashboard pages triggers ZERO extra
 *      fetches (the layout stays mounted; the interval owns polling).
 */
import { test, expect } from './fixtures/test-account';
import { sql, pubkeyHexFromSeed } from './fixtures/seed-helpers';

/** Count unread-count GETs issued by the page. Attach BEFORE navigating. */
function countUnreadCountRequests(page: import('@playwright/test').Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'GET' && req.url().includes('/notifications/unread-count')) {
			count++;
		}
	});
	return () => count;
}

test.describe('NotificationBell unread-count', () => {
	test('badge displays the correct unread count from the DB', async ({ page, testAccount }) => {
		// NotificationBell lives in the mobile-only header (md:hidden). Force
		// a mobile viewport so the badge is interactable.
		await page.setViewportSize({ width: 375, height: 667 });

		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Seed three unread notifications directly. created_at is in
			// nanoseconds (Rust SystemTime → i64); Date.now() is ms.
			const nowNs = (Date.now() * 1_000_000).toString();
			await sql(`
				INSERT INTO user_notifications (user_pubkey, type, title, body, created_at)
				VALUES
					(decode('${pubkey}', 'hex'), 'contract_status', 'Unread one', '{"text":"a"}', ${nowNs}),
					(decode('${pubkey}', 'hex'), 'contract_status', 'Unread two', '{"text":"b"}', ${nowNs}),
					(decode('${pubkey}', 'hex'), 'contract_status', 'Unread three', '{"text":"c"}', ${nowNs})
			`);

			await page.goto('/dashboard');
			await page
				.getByRole('button', { name: 'Logout' })
				.waitFor({ state: 'visible', timeout: 15000 });

			const bell = page.getByRole('button', { name: 'Notifications' });
			await expect(bell).toBeVisible();
			// UnreadBadge renders the count as its text content. The expect
			// auto-retries so the async fetch + render has time to land.
			await expect(bell.locator('span span')).toHaveText('3', { timeout: 10000 });
		} finally {
			await sql(`DELETE FROM user_notifications WHERE user_pubkey = decode('${pubkey}', 'hex')`);
		}
	});

	test('a single dashboard load triggers exactly one unread-count fetch', async ({ page }) => {
		// Attach the counter BEFORE triggering a fresh full reload so every
		// unread-count request issued during one initialize() cycle is caught.
		const getCount = countUnreadCountRequests(page);

		// Set up the response listener BEFORE goto (the fetch is triggered
		// during initialize() right after hydration).
		const unreadResponse = page.waitForResponse(
			(resp) => resp.url().includes('/notifications/unread-count'),
			{ timeout: 15000 },
		);
		await page.goto('/dashboard');
		await page
			.getByRole('button', { name: 'Logout' })
			.waitFor({ state: 'visible', timeout: 15000 });
		// Wait for the expected unread-count response to land. Any duplicate
		// fetches from the bug (triggered by intermediate identity emits) are
		// issued synchronously during initialize() before this response arrives.
		await unreadResponse;

		// Exactly one fetch per page load — no per-emit spam. The bug
		// produced 4+ (addIdentity + account-load, called from both the
		// root and dashboard layouts).
		expect(getCount()).toBe(1);
	});

	test('client-side navigation between dashboard pages does not re-fetch', async ({ page }) => {
		// Land on /dashboard first — the page fixture no longer pre-navigates
		// (see fixtures/test-account.ts), and this test reloads the current URL
		// to trigger a fresh initialize() cycle.
		await page.goto('/dashboard');
		await page
			.getByRole('button', { name: 'Logout' })
			.waitFor({ state: 'visible', timeout: 15000 });

		// Reload to trigger a fresh initialize() cycle, then wait for the
		// unread-count response to land. After this, the initial fetch storm
		// is complete and the counter starts clean (replaces a 2s fixed sleep).
		const unreadResponse = page.waitForResponse(
			(resp) => resp.url().includes('/notifications/unread-count'),
			{ timeout: 15000 },
		);
		await page.reload();
		await page
			.getByRole('button', { name: 'Logout' })
			.waitFor({ state: 'visible', timeout: 15000 });
		await unreadResponse;

		const getCount = countUnreadCountRequests(page);

		// Client-side navigation via sidebar links keeps the dashboard layout
		// (and NotificationBell) mounted, so no new fetch should occur.
		// Routes chosen from the sidebar's Browse + My Activity sections.
		await page.locator('a[href="/dashboard/marketplace"]').first().click();
		await expect(page).toHaveURL(/\/dashboard\/marketplace/);

		await page.locator('a[href="/dashboard/rentals"]').first().click();
		await expect(page).toHaveURL(/\/dashboard\/rentals/);

		await page.locator('a[href="/dashboard/reputation"]').first().click();
		await expect(page).toHaveURL(/\/dashboard\/reputation/);

		// After the last navigation settles, assert no unread-count fetches
		// were triggered. Any fetch would have been issued during navigation
		// (before the URL changed), so no fixed sleep is needed.
		expect(getCount()).toBe(0);
	});
});
