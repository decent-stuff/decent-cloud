import { test, expect } from '@playwright/test';

/**
 * E2E coverage for the /verify-email route (GAP route).
 *
 * The page has three states driven by an onMount side effect:
 *   - 'verifying' (initial spinner)
 *   - 'success'   (after a valid token POST)
 *   - 'error'     (missing token, or the API rejects the token)
 *
 * Anonymous tests cover the two error paths. A success-path test would require
 * a real verification token minted via the email-send flow, which is out of
 * scope for a route-rendering smoke test.
 */
test.describe('/verify-email route', () => {
	test('shows a missing-token error when navigated to without a token', async ({ page }) => {
		await page.goto('/verify-email');

		// The page must settle on the error state, not hang on the spinner.
		await expect(page.getByRole('heading', { name: 'Verification Failed' })).toBeVisible({
			timeout: 10000,
		});

		// The specific guidance for the no-token branch must be shown — this is
		// what tells the user WHY verification failed, not just that it did.
		await expect(
			page.getByText('Verification token is missing from the URL'),
		).toBeVisible();

		// The recovery CTA must be present.
		await expect(
			page.getByRole('button', { name: 'Go to Login' }),
		).toBeVisible();
	});

	test('shows an invalid/expired error for an unrecognized token', async ({ page }) => {
		await page.goto('/verify-email?token=invalid');

		// Wait for the API call to settle. The page starts in 'verifying' and
		// flips to 'error' once the API rejects the bogus token. The heading
		// change is the deterministic signal that the state transitioned.
		await expect(page.getByRole('heading', { name: 'Verification Failed' })).toBeVisible({
			timeout: 15000,
		});

		// The page must NOT be stuck on the verifying spinner.
		await expect(page.getByRole('heading', { name: 'Verifying Email' })).toBeHidden();

		// The "expired or already used" explanation is the meaningful guidance
		// for the invalid-token branch (as opposed to the missing-token branch).
		await expect(
			page.getByText('The verification link may have expired or been used already.'),
		).toBeVisible();
	});
});
