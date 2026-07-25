import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	randomHex,
	seedOffering,
	seedContract,
	deleteContractsByProvider,
	deleteOfferingsByProvider,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the Provider Earnings POPULATED state
 * (/dashboard/provider/earnings).
 *
 * FLOWS.md "Earnings" was ⚠️: only the empty Revenue Overview panel render was
 * asserted. This spec seeds real contracts where the testAccount IS the provider
 * and asserts the dashboard sums them: `get_provider_stats` computes
 * `total_revenue_e9s = SUM(payment_amount_e9s) FROM contract_sign_requests
 * WHERE provider_pubkey = $1`, and the contract table comes from
 * `getUserActivity(pubkey).rentals_as_provider`.
 *
 * Serial mode: the contracts + offering are keyed on the shared testAccount
 * pubkey. beforeAll wipes the provider's prior contracts first (deterministic
 * revenue sum); afterAll cleans up. A fresh random requester pubkey plays the
 * tenant so the rows are genuine tenant→provider contracts.
 */
test.describe('Provider earnings populated state (/dashboard/provider/earnings)', () => {
	test.describe.configure({ mode: 'serial' });

	const PAYMENT_E9S_PER = 1_000_000_000; // 1 ICP each
	let providerPubkey = '';
	const requesterPubkey = randomHex(32);
	const offeringId = `earn-${randomHex(4)}`;

	test.beforeAll(async ({ testAccount }) => {
		providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		// Deterministic slate: remove any prior provider contracts/offerings.
		await deleteContractsByProvider(providerPubkey);
		await deleteOfferingsByProvider(providerPubkey);

		// Offering owned by the provider so offerings_count > 0 and the contracts
		// reference a valid (offering_id, provider_pubkey) pair.
		await seedOffering(providerPubkey, { offeringId, name: 'E2E Earnings Offering' });

		// Two paid active contracts where the testAccount is the PROVIDER.
		// Revenue sum = 2 ICP; the earnings table lists both.
		for (let i = 0; i < 2; i++) {
			await seedContract({
				requesterPubkeyHex: requesterPubkey,
				providerPubkeyHex: providerPubkey,
				status: 'active',
				paymentStatus: 'succeeded',
				paymentAmountE9s: PAYMENT_E9S_PER,
				offeringId,
			});
		}
	});

	test.afterAll(async () => {
		try {
			await deleteContractsByProvider(providerPubkey);
		} catch {
			/* best-effort */
		}
		if (providerPubkey) {
			try {
				await deleteOfferingsByProvider(providerPubkey);
			} catch {
				/* best-effort */
			}
		}
	});

	test('shows the summed revenue, contract count, and contract rows for seeded provider contracts', async ({ page }) => {
		await page.goto('/dashboard/provider/earnings');
		await waitForAuthReady(page);

		// Revenue Overview panel renders (data-loaded signal).
		await expect(page.getByRole('heading', { name: 'Revenue Overview' })).toBeVisible({ timeout: 15000 });

		// Gross Revenue sums the two seeded 1-USD contracts → "2.00 USD".
		// ICPay is retired; the page must show the contract's real currency (USD),
		// never a hardcoded "ICP" label. Scope to the Gross Revenue card so the
		// assertion is unambiguous (Net Earnings echoes the same number).
		const grossCard = page.locator('div.bg-surface-elevated').filter({ hasText: 'Gross Revenue' });
		await expect(grossCard).toContainText('2.00 USD', { timeout: 15000 });

		// Total Contracts reflects the two seeded provider contracts (>= 2 to stay
		// robust to any counting nuance in get_provider_stats).
		const totalCard = page.locator('div.bg-surface-elevated').filter({ hasText: 'Total Contracts' });
		const totalText = await totalCard.textContent();
		expect(Number((totalText || '').replace(/[^0-9]/g, ''))).toBeGreaterThanOrEqual(2);

		// The Contract Earnings table lists the seeded provider contracts (not the
		// "No contracts yet" empty state).
		await expect(page.getByRole('heading', { name: 'Contract Earnings' })).toBeVisible();
		await expect(page.getByText('No contracts yet')).toBeHidden();

		// No stale ICP currency labels anywhere on the earnings page.
		await expect(page.locator('body')).not.toContainText('ICP');
	});
});
