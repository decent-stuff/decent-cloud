import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /dashboard/user/[identifier].
 *
 * This route is a legacy subset of the richer reputation page and had ZERO
 * inbound links anywhere in the app. It now redirects (307) to
 * /dashboard/reputation/[identifier], which resolves both username and
 * pubkey identifiers itself — so the same identifier is forwarded.
 */

test.describe('/dashboard/user/[identifier] redirect', () => {
	test('@smoke redirects to the reputation page preserving the identifier', async ({ page }) => {
		// Unknown identifier — previously rendered a "User Not Found" card.
		// Now it must redirect to the reputation route and land there.
		const identifier = 'no-such-user-9f8e7d6c5b4a';
		await page.goto(`/dashboard/user/${identifier}`);

		await expect(page).toHaveURL(`/dashboard/reputation/${identifier}`);
		// The reputation page renders its own not-found heading for the
		// unknown identifier, proving the redirect reached a real page.
		await expect(page.getByRole('heading', { name: 'No Account Data' })).toBeVisible({
			timeout: 10000,
		});
	});

	test('redirects a pubkey-shaped identifier to the reputation page', async ({ page }) => {
		// A 64-char hex pubkey must be forwarded unchanged.
		const pubkey = '0'.repeat(64);
		await page.goto(`/dashboard/user/${pubkey}`);

		await expect(page).toHaveURL(`/dashboard/reputation/${pubkey}`);
	});
});
