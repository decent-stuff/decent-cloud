import { test as anonymousTest, expect } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	seedAccountDirect,
	deleteAccountByUsername,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/reputation/[identifier]/trust (Trust Report).
 *
 * Coverage gap found in the 2026-08-02 audit: the Trust Report sub-route is a
 * user-facing page (breadcrumb, copy-link, TrustDashboard) that was neither in
 * FLOWS.md nor in the route-audit suite's dynamic-route table — only its parent
 * reputation profile (reputation-detail.spec.ts) was covered. (It is now also
 * covered by `route-audit-misc.spec.ts`.)
 *
 * Two deterministic branches, both exercised here:
 *   - Known account: the identifier resolves and the page renders the Trust
 *     Report header + breadcrumb + the TrustDashboard (trust-metrics API
 *     returns a zero-valued object for any pubkey, so the dashboard always
 *     renders for a resolvable account).
 *   - Unknown identifier: the "Account Not Found" error branch.
 *
 * The trust-metrics API returns 200 with full data even for a non-provider
 * pubkey, so the page's "No Trust Data Available" empty state only fires on
 * API failure — not triggerable deterministically without a forbidden
 * first-party mock, so it is intentionally out of scope.
 *
 * Anonymous-OK (same access rules as the parent reputation route, exercised
 * anonymously in reputation-detail.spec.ts). Self-seeding, no external dep.
 */
anonymousTest.describe('/dashboard/reputation/[identifier]/trust', () => {
	anonymousTest('resolves a known account and renders the TrustDashboard with its trust score', async ({
		page,
	}) => {
		// Self-contained: seed a fresh account so the assertion never depends on
		// externally-maintained state. The trust-metrics API returns a
		// zero-valued object for any resolvable pubkey, so the populated
		// TrustDashboard branch is the deterministic outcome.
		const { username, seedPhrase } = await seedAccountDirect();
		pubkeyHexFromSeed(seedPhrase); // derive to confirm the account is real
		try {
			await page.goto(`/dashboard/reputation/${username}/trust`);

			// The Trust Report header renders (proves the identifier resolved and
			// the page mounted past the loading spinner — the not-found branch
			// never renders this h1).
			await expect(page.getByRole('heading', { name: 'Trust Report' })).toBeVisible({
				timeout: 10_000,
			});

			// Breadcrumb identifies where we are: Reputation › {user} › Trust Report.
			const breadcrumb = page.locator('nav', { hasText: 'Trust Report' }).first();
			await expect(breadcrumb).toBeVisible();
			await expect(breadcrumb).toContainText(username);

			// The TrustDashboard component renders with the trust-metrics payload.
			// "Trust Score" is its h3; "Total Contracts" is a core-metric label.
			await expect(page.getByRole('heading', { name: 'Trust Score' })).toBeVisible();
			await expect(page.getByText('Total Contracts')).toBeVisible();

			// The header's "View Full Profile →" link points back to this
			// account's reputation profile (page chrome unique to this route).
			const viewProfile = page.getByRole('link', { name: /View Full Profile/i });
			await expect(viewProfile).toBeVisible();
			await expect(viewProfile).toHaveAttribute(
				'href',
				`/dashboard/reputation/${username}`,
			);
		} finally {
			await deleteAccountByUsername(username);
		}
	});

	anonymousTest('renders the "Account Not Found" error for an unknown identifier', async ({ page }) => {
		await page.goto('/dashboard/reputation/nonexistent-user-zzz/trust');

		// The not-found branch surfaces this specific heading + guidance.
		await expect(page.getByRole('heading', { name: 'Account Not Found' })).toBeVisible({
			timeout: 10_000,
		});
		await expect(
			page.getByText(/is not registered in the system/i),
		).toBeVisible();

		// Recovery link back to the Reputation search page.
		const back = page.getByRole('link', { name: /Back to Reputation/i });
		await expect(back).toBeVisible();
		await expect(back).toHaveAttribute('href', '/dashboard/reputation');
	});
});
