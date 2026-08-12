import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the rental DETAIL page cancel flow (two-step inline confirm).
 *
 * The detail page cancel previously used a native confirm() dialog — which both
 * blocks headless e2e (Playwright auto-dismisses it, so the cancel never
 * fires) and is a poor mobile UX. It now uses the inline two-step pattern
 * (first Cancel click reveals an inline Confirm button, second click cancels),
 * mirroring the offerings delete pattern (commit 1077dd33).
 *
 * Serial mode: all testAccount users share one pubkey, and this spec mutates
 * (seeds + cancels) contract_sign_requests rows for that pubkey.
 *
 * Auth wait: the detail page renders the Cancel button only after its signed
 * `GET /api/v1/users/<pubkey>/contracts` fetch resolves, so each goto is gated
 * on that response — under 4-worker contention the click would otherwise race
 * the contract-detail render.
 */
test.describe.configure({ mode: 'serial' });

// Gate the detail-page goto on the signed contracts fetch that renders the
// Cancel button (deterministic; no networkidle, no content polling).
const contractsLoaded = (page: import('@playwright/test').Page) =>
	page.waitForResponse(
		(r) =>
			r.url().includes('/api/v1/users/') &&
			r.url().includes('/contracts') &&
			r.request().method() === 'GET',
		{ timeout: 10000 },
	);

test.describe('Rental detail cancel (inline two-step confirm)', () => {
	test('first Cancel click reveals an inline confirm; second click cancels the contract', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'requested',
			paymentStatus: 'pending',
		});
		try {
			await Promise.all([
				contractsLoaded(page),
				page.goto(`/dashboard/rentals/${contractId}`),
			]);

			// First click: no native dialog — it reveals an inline Confirm button.
			await page.getByRole('button', { name: 'Cancel', exact: true }).first().click();
			const confirmBtn = page.getByRole('button', { name: 'Confirm', exact: true });
			await expect(confirmBtn).toBeVisible();
			// An Abort button lets the user back out.
			await expect(page.getByRole('button', { name: 'Abort', exact: true })).toBeVisible();

			// Second click: performs the cancellation. Wait for the signed PUT.
			const putCancel = page.waitForResponse(
				(resp) =>
					resp.request().method() === 'PUT' &&
					resp.url().includes(`/api/v1/contracts/${contractId}/cancel`),
				{ timeout: 15000 },
			);
			await confirmBtn.click();
			await putCancel;

			// Terminal contract: the Renew link appears, the Cancel button is gone.
			await expect(page.getByRole('link', { name: /Renew/i }).first()).toBeVisible({ timeout: 10000 });
			await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toHaveCount(0);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('Abort hides the inline confirm and keeps the contract cancellable', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'requested',
			paymentStatus: 'pending',
		});
		try {
			await Promise.all([
				contractsLoaded(page),
				page.goto(`/dashboard/rentals/${contractId}`),
			]);

			await page.getByRole('button', { name: 'Cancel', exact: true }).first().click();
			await page.getByRole('button', { name: 'Abort', exact: true }).click();

			// Confirm/Abort disappear; the plain Cancel button returns.
			await expect(page.getByRole('button', { name: 'Confirm', exact: true })).toHaveCount(0);
			await expect(page.getByRole('button', { name: 'Abort', exact: true })).toHaveCount(0);
			await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});
});
