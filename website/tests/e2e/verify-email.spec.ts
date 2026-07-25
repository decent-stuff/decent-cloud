import { test, expect } from '@playwright/test';
import {
	seedAccountDirect,
	deleteAccountByUsername,
	accountIdHex,
	seedEmailVerificationToken,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the /verify-email route (GAP route).
 *
 * The page has three states driven by an onMount side effect:
 *   - 'verifying' (initial spinner)
 *   - 'success'   (after a valid token POST)
 *   - 'error'     (missing token, or the API rejects the token)
 *
 * The two anonymous tests cover the error branches. The success-path test
 * seeds a real `email_verification_tokens` row DB-side (the row the email-send
 * flow would normally write) and drives the REAL app to flip the account's
 * `email_verified` flag — no external email service required.
 */

/** `email_verified` flag ('t'/'f') for an account id (hex). */
async function emailVerified(accountHex: string): Promise<string> {
	return (
		await sql(`SELECT email_verified FROM accounts WHERE id = decode('${accountHex}', 'hex')`)
	).trim();
}

test.describe('/verify-email route', () => {
	test('@smoke shows a missing-token error when navigated to without a token', async ({ page }) => {
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

	test('success: a valid DB-seeded token verifies the email and shows the success state', async ({ page }) => {
		// Seed a standalone account + token so the test is fully self-contained
		// (no dependency on the worker-scoped testAccount fixture, which would
		// leave email_verified=true behind for sibling tests on the same worker).
		const { username } = await seedAccountDirect();
		try {
			const accountHex = await accountIdHex(username);
			const email = `${username}@test.example.com`;

			// Pre-condition: account starts unverified (seedAccountDirect does
			// not set email_verified, so it defaults to false).
			expect(await emailVerified(accountHex)).toBe('f');

			const token = await seedEmailVerificationToken(accountHex, email);

			await page.goto(`/verify-email?token=${token}`);

			// The page must flip to the success state (not hang on the spinner
			// and not fall through to the error branch).
			await expect(page.getByRole('heading', { name: 'Email Verified!' })).toBeVisible({
				timeout: 15000,
			});
			await expect(page.getByText('Thank you for verifying your email!')).toBeVisible();
			// Success-state CTAs are present.
			await expect(page.getByRole('button', { name: 'Go to Dashboard' })).toBeVisible();

			// Post-condition: the real verify handler flipped email_verified
			// server-side AND marked the token used (so a replay can't re-verify).
			expect(await emailVerified(accountHex)).toBe('t');
			const tokenUsed = (
				await sql(`SELECT used_at FROM email_verification_tokens WHERE token = decode('${token}', 'hex')`)
			).trim();
			expect(tokenUsed).not.toBe('');
		} finally {
			await deleteAccountByUsername(username);
		}
	});
});
