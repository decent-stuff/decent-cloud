import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	randomHex,
	seedContract,
	deleteContractsForRequester,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the provider batch Accept All / Reject All flow (two-step
 * inline confirm).
 *
 * The batch action previously used a native window.confirm() dialog — which
 * both blocks headless e2e (Playwright auto-dismisses it, so the batch never
 * fires) and is a poor mobile UX. It now uses the inline two-step pattern
 * (first Accept/Reject All click reveals an inline Confirm button, second
 * click runs the batch), mirroring the offerings delete pattern (commit
 * 1077dd33).
 *
 * This spec covers the BATCH two-step confirm only — single accept/reject is
 * covered by provider-accept-reject.spec.ts (no overlap).
 *
 * Seeding model: the testAccount user is the PROVIDER; two random pubkeys play
 * the tenants. Contracts are inserted directly with status='requested' and
 * payment_status='succeeded' so they land in the pending list.
 *
 * Serial mode: all testAccount users share one provider pubkey; these tests
 * mutate rows keyed on it.
 */
test.describe.configure({ mode: 'serial' });

test.describe('Provider batch accept/reject (inline two-step confirm)', () => {
	test('first Reject All click reveals an inline confirm; Cancel aborts and keeps the requests pending', async ({ page, testAccount }) => {
		const providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const r1 = randomHex(32);
		const r2 = randomHex(32);
		try {
			await seedContract({ requesterPubkeyHex: r1, providerPubkeyHex: providerPubkey, status: 'requested', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: r2, providerPubkeyHex: providerPubkey, status: 'requested', paymentStatus: 'succeeded' });

			await page.goto('/dashboard/provider/requests');

			// Batch buttons render once >1 pending request exists.
			await expect(page.getByRole('button', { name: 'Reject All' })).toBeVisible({ timeout: 10000 });

			// First click: no native dialog — it reveals an inline Confirm + Cancel.
			await page.getByRole('button', { name: 'Reject All' }).click();
			await expect(page.getByText(/Reject all 2\?/)).toBeVisible();
			await expect(page.getByRole('button', { name: 'Confirm', exact: true })).toBeVisible();
			await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();

			// Cancel aborts: the Confirm/Cancel pair disappears, Reject All returns.
			await page.getByRole('button', { name: 'Cancel', exact: true }).click();
			await expect(page.getByRole('button', { name: 'Reject All' })).toBeVisible();
		} finally {
			await deleteContractsForRequester(r1);
			await deleteContractsForRequester(r2);
		}
	});

	test('second Accept All click runs the batch and clears the pending list', async ({ page, testAccount }) => {
		const providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const r1 = randomHex(32);
		const r2 = randomHex(32);
		try {
			await seedContract({ requesterPubkeyHex: r1, providerPubkeyHex: providerPubkey, status: 'requested', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: r2, providerPubkeyHex: providerPubkey, status: 'requested', paymentStatus: 'succeeded' });

			await page.goto('/dashboard/provider/requests');
			await expect(page.getByRole('button', { name: 'Accept All' })).toBeVisible({ timeout: 10000 });

			// Arm, then confirm the batch. The two signed POSTs fire after Confirm.
			await page.getByRole('button', { name: 'Accept All' }).click();
			await page.getByRole('button', { name: 'Confirm', exact: true }).click();

			// Batch completes: success message surfaces and the pending list empties
			// (accepted contracts leave the pending view → batch buttons disappear).
			await expect(page.getByText(/Accepted all 2 requests/)).toBeVisible({ timeout: 15000 });
			await expect(page.getByRole('button', { name: 'Accept All' })).toHaveCount(0);
		} finally {
			await deleteContractsForRequester(r1);
			await deleteContractsForRequester(r2);
		}
	});
});
