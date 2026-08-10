import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	deleteContractsForRequester,
	seedRentableWithResource,
	cleanupRentableWithResource,
	verifyAccountEmail,
	sql,
	type RentableWithResourceSeed,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the PRIMARY tenant journey: rent → pay → view → cancel.
 *
 * Before this spec, cancel was only ever exercised on DB-seeded contracts
 * (rentals.spec.ts). This spec drives the REAL flow end-to-end against the warm
 * stack: the marketplace Rent dialog → a real signed POST /api/v1/contracts that
 * creates a genuine contract (paid instantly via wallet debit) → the rentals
 * list → the rental detail page → a real signed PUT /api/v1/contracts/:id/cancel
 * (which refunds the wallet). No first-party API is mocked.
 *
 * The prepaid wallet is the sole paid rail. The beforeAll seeds a generous
 * wallet balance for the requester so the wallet debit at contract creation
 * succeeds; cancel credits it back (rental_refund). The contract lands at
 * `requested` + payment_status `succeeded` (paid, awaiting provider review).
 *
 * Shared-pubkey hazard: all testAccount users derive the same requester pubkey,
 * and this spec CREATES real contracts + wallet rows for it. Serial mode +
 * beforeAll/afterAll cleanup (deleteContractsForRequester + wallet teardown)
 * prevents parallel workers from nuking each other's data.
 */

const SSH_KEY = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eRentFlowTestKeyData1234 e2e@rentflow';
const CONTACT = 'email:e2e-rentflow@test.example.com';
const MEMO = 'E2E rent-flow happy path';

test.describe('Rent → pay → view → cancel (primary tenant flow)', () => {
	test.describe.configure({ mode: 'serial' });

	let seed: RentableWithResourceSeed;
	let requesterPubkey: string;
	// Shared across the serial tests: the contract the first rental creates.
	let firstContractId: string;

	test.beforeAll(async ({ testAccount }) => {
		requesterPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		// Fresh slate so the assertions see only this spec's contracts.
		await deleteContractsForRequester(requesterPubkey);
		// Rentals are rejected unless the requester's email is verified
		// (API guard in contracts.rs). seedAccountDirect creates the testAccount
		// with email_verified=false, so verify it DB-side as test-account setup.
		await verifyAccountEmail(requesterPubkey);
		// Seed a generous prepaid wallet balance so the wallet debit at contract
		// creation succeeds. $1000 (1e12 e9s) covers multiple rentals + the
		// cancel refund credits it back. ON CONFLICT handles re-runs.
		await sql(`
			INSERT INTO wallet_balances (pubkey, balance_e9s)
			VALUES ('${requesterPubkey}', 1000000000000)
			ON CONFLICT (pubkey) DO UPDATE SET balance_e9s = 1000000000000
		`);
		seed = await seedRentableWithResource({ name: 'E2E Rent Flow Offering' });
	});

	test.afterAll(async () => {
		// Guard: beforeAll may have thrown before assigning `seed`.
		if (seed) await cleanupRentableWithResource(seed);
		await deleteContractsForRequester(requesterPubkey);
		// Teardown the wallet rows so sibling specs see a clean balance.
		// Delete ledger first (FK from wallet_ledger → wallet_balances).
		await sql(`DELETE FROM wallet_ledger WHERE pubkey = '${requesterPubkey}'`);
		await sql(`DELETE FROM wallet_balances WHERE pubkey = '${requesterPubkey}'`);
	});

	/**
	 * Drive the real marketplace Rent dialog for the seeded offering and return
	 * the contract_id of the contract that the real signed POST creates.
	 *
	 * After submit, the contract is committed at `requested`. The Stripe checkout
	 * session cannot finish in-harness, so we do not assert dialog success — we
	 * navigate to the rentals list and confirm the contract landed there.
	 */
	async function rentViaDialog(
		page: import('@playwright/test').Page,
	): Promise<string> {
		await page.goto(`/dashboard/marketplace/${seed.offeringNumericId}`);
		// The detail page renders the Rent button twice (responsive variants).
		const rentBtn = page.getByRole('button', { name: 'Rent this offering' }).first();
		await expect(rentBtn).toBeVisible({ timeout: 15000 });
		await rentBtn.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({
			timeout: 5000,
		});

		// Wallet is the default (and only) payment method for paid rentals.
		await page.locator('textarea[placeholder*="ssh-ed25519"]').fill(SSH_KEY);
		await page.locator('input[placeholder*="email:you@example.com"]').fill(CONTACT);
		const memo = page.locator('textarea[placeholder*="special requirements"]');
		if (await memo.isVisible().catch(() => false)) await memo.fill(MEMO);

		// Submit fires the real signed POST /api/v1/contracts (creates the contract
		// + debits the wallet atomically).
		const postContracts = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'POST' &&
				resp.url().includes('/api/v1/contracts'),
			{ timeout: 20000 },
		);
		await page.getByRole('button', { name: 'Pay now', exact: true }).click();
		const resp = await postContracts;
		// Surface the API error clearly instead of silently proceeding to a
		// confusing "card not found" failure on the rentals page.
		if (!resp.ok()) {
			const body = await resp.text().catch(() => '<no body>');
			throw new Error(`POST /contracts failed (${resp.status()}): ${body}`);
		}

		// The contract now exists at `requested` + payment_status `succeeded`
		// (wallet-debited instantly). Navigate to the rentals list.
		await page.goto('/dashboard/rentals');

		// The contract card for our offering must appear, with a Cancel button
		// (isCancellable includes 'requested'/'pending').
		const card = page.locator(`a.card:has-text("${seed.offeringName}")`).first();
		await expect(card).toBeVisible({ timeout: 15000 });
		// Paid + awaiting provider → "Pending Provider" badge (not "Awaiting Payment").
		await expect(card.getByText(/Pending Provider/i)).toBeVisible();
		await expect(card.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();

		const href = await card.getAttribute('href');
		if (!href) throw new Error('rental card had no href');
		return href.split('/').pop()!;
	}

	test('rent an offering → contract appears on the rentals list with a Cancel button', async ({
		page,
	}) => {
		const contractId = await rentViaDialog(page);
		expect(contractId).toMatch(/^[0-9a-f]+$/);
		firstContractId = contractId;
	});

	test('view the rental detail page', async ({ page }) => {
		const contractId = firstContractId;
		expect(contractId, 'rent test must have produced a contract id').toBeTruthy();

		await page.goto(`/dashboard/rentals/${contractId}`);
		await expect(page).toHaveURL(new RegExp(`/dashboard/rentals/${contractId}`));

		// Detail page must not 404 and must reference the contract + its offering.
		await expect(page.locator('body')).not.toContainText(['404', 'Not Found']);
		await expect(page.locator('body')).toContainText(contractId.slice(0, 8));
		// The detail header shows the offering_id (e.g. rentflow-<tag>).
		await expect(page.locator('body')).toContainText(seed.offeringId);
		// A 'requested' + paid contract shows "Pending Provider" status + a Cancel.
		await expect(page.getByText(/Pending Provider/i).first()).toBeVisible({ timeout: 10000 });
		await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible({
			timeout: 10000,
		});
	});

	test('cancel the rental from the detail page → status moves to cancelled', async ({
		page,
	}) => {
		const contractId = firstContractId;

		await page.goto(`/dashboard/rentals/${contractId}`);

		const cancelBtn = page.getByRole('button', { name: 'Cancel', exact: true }).first();
		await expect(cancelBtn).toBeVisible({ timeout: 10000 });

		const putCancel = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'PUT' &&
				resp.url().includes(`/api/v1/contracts/${contractId}/cancel`),
			{ timeout: 15000 },
		);
		// Two-step inline confirm: first Cancel arms, then Confirm fires the PUT.
		await cancelBtn.click();
		await page.getByRole('button', { name: 'Confirm', exact: true }).click();
		await putCancel;

		// After refresh, the contract is terminal: Cancel disappears, Renew appears.
		await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toHaveCount(0);
		await expect(page.getByRole('link', { name: /Renew/i }).first()).toBeVisible({
			timeout: 10000,
		});

		// Verify the DB reflects the cancellation.
		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(status).toBe('cancelled');

		// Wallet money integrity: the cancel refunded the debited amount back
		// to the wallet. The ledger must show both the rental_debit (at creation)
		// and the rental_refund (at cancel) for this contract.
		const ledger = await sql(`
			SELECT entry_type FROM wallet_ledger
			WHERE reference = '${contractId}'
			ORDER BY id
		`);
		const entries = ledger.trim().split('\n').map((e) => e.trim());
		expect(entries).toContain('rental_debit');
		expect(entries).toContain('rental_refund');
	});

	test('cancel a rental directly from the rentals list', async ({ page }) => {
		// A fresh rental (uses the next reservable cloud_resource).
		const contractId = await rentViaDialog(page);

		// Re-open the list and cancel via the card-level Cancel button.
		await page.goto('/dashboard/rentals');
		const card = page.locator(`a[href="/dashboard/rentals/${contractId}"]`);
		await expect(card).toBeVisible({ timeout: 15000 });

		const cancelBtn = card.getByRole('button', { name: 'Cancel', exact: true });
		await expect(cancelBtn).toBeVisible();

		const putCancel = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'PUT' &&
				resp.url().includes(`/api/v1/contracts/${contractId}/cancel`),
			{ timeout: 15000 },
		);
		// Two-step inline confirm: first Cancel arms, then Confirm fires the PUT.
		await cancelBtn.click();
		await card.getByRole('button', { name: 'Confirm', exact: true }).click();
		await putCancel;

		// Terminal contract: Renew appears, Cancel gone (on this card). The Renew
		// button label is "↺ Renew", so match by substring (non-exact).
		await expect(card.getByRole('button', { name: 'Renew' })).toBeVisible({
			timeout: 10000,
		});
		await expect(card.getByRole('button', { name: 'Cancel', exact: true })).toHaveCount(0);

		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(status).toBe('cancelled');
	});
});
