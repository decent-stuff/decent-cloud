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
 * The fixture (seedRentableWithResource) explicitly pins the provider's
 * `auto_accept_rentals=false` so the contract stays at `requested` on purpose
 * — this exercises the MANUAL-review path. That pin is load-bearing for the
 * wallet math: once a contract auto-accepts and self-provisioned activation
 * flips it to `active`, `provisioning_completed_at_ns` is set and cancel
 * refunds become PRORATED (not full). The auto-accept path is covered instead
 * by rent-wallet-auto-accept.spec.ts; this spec owns the pre-service
 * full-refund invariant.
 *
 * Wallet-debit is asserted with REAL money math, not just ledger-row presence:
 * each rent proves the balance dropped by exactly the contract's
 * payment_amount_e9s and a matching rental_debit row exists; each pre-service
 * cancel proves the balance was restored to its pre-rent value and the
 * rental_refund row carries the full principal (provisioning_completed_at_ns is
 * NULL at `requested`, so the prorated refund equals the whole payment).
 *
 * Shared-pubkey hazard: all testAccount users derive the same requester pubkey,
 * and this spec CREATES real contracts + wallet rows for it. Serial mode +
 * beforeAll/afterAll cleanup (deleteContractsForRequester + wallet teardown)
 * prevents parallel workers from nuking each other's data.
 */

const SSH_KEY = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eRentFlowTestKeyData1234 e2e@rentflow';
const CONTACT = 'email:e2e-rentflow@test.example.com';
const MEMO = 'E2E rent-flow happy path';

/**
 * Read the current prepaid wallet balance (e9s) for a pubkey; 0n if the user
 * has never topped up. Used to PROVE the rent debits and the cancel refunds
 * the balance by the exact expected amount — not just that a ledger row exists.
 */
async function readWalletBalance(pubkey: string): Promise<bigint> {
	const out = await sql(
		`SELECT balance_e9s FROM wallet_balances WHERE pubkey = '${pubkey}'`,
	);
	return BigInt(out || '0');
}

/**
 * Read the payment_amount_e9s charged for a contract — the principal the
 * wallet debit + (pre-service) refund must match exactly. Asserting against
 * the row's own value (not a re-derived number) keeps the math test robust to
 * changes in the rent dialog's default duration / offering price.
 */
async function readContractPaymentAmount(contractId: string): Promise<bigint> {
	const out = await sql(
		`SELECT payment_amount_e9s FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
	);
	if (!out) throw new Error(`no payment_amount_e9s for contract ${contractId}`);
	return BigInt(out);
}

/** Outcome of one rent-via-dialog cycle: the contract + the wallet facts the
 * matching cancel test must reconcile against. */
interface RentOutcome {
	contractId: string;
	/** Principal debited at creation (and, pre-service, refunded at cancel). */
	paymentAmountE9s: bigint;
	/** Wallet balance captured immediately before the POST /contracts. */
	balanceBeforeE9s: bigint;
}

test.describe('Rent → pay → view → cancel (primary tenant flow)', () => {
	test.describe.configure({ mode: 'serial' });

	let seed: RentableWithResourceSeed;
	let requesterPubkey: string;
	// Shared across the serial tests: the contract the first rental creates.
	let firstContractId: string;
	// Wallet facts for the first rent, captured so the detail-page cancel test
	// can prove the refund restores the balance to its pre-rent value.
	let firstRent: RentOutcome | undefined;

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
	 * the contract_id + wallet facts for the contract that the real signed POST
	 * creates.
	 *
	 * After submit, the contract is committed at `requested` with
	 * payment_status `succeeded` — the wallet debit is applied atomically in the
	 * same transaction as the contract insert. We assert the balance ACTUALLY
	 * dropped by exactly `payment_amount_e9s` (the row's own principal) so a
	 * silent $0 debit or a missing balance update can never pass the suite.
	 */
	async function rentViaDialog(
		page: import('@playwright/test').Page,
	): Promise<RentOutcome> {
		// Capture the balance RIGHT before the POST so the debit assertion sees
		// only this rental's effect (no race with a sibling serial test).
		const balanceBeforeE9s = await readWalletBalance(requesterPubkey);

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
		// Contact + Notes live in the collapsed "Advanced (optional)" disclosure.
		await page.getByText('Advanced (optional)').click();
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

		// Extract the contract_id from the API response (data.contractId — the
		// API serializes RentalRequestResponse with serde rename_all = camelCase).
		const json = await resp.json();
		const contractId: string | undefined = json?.data?.contractId;
		if (!contractId || !/^[0-9a-f]+$/.test(contractId)) {
			throw new Error(`POST /contracts response had no data.contractId: ${JSON.stringify(json)}`);
		}

		// Wallet-debit proof: the balance must have dropped by EXACTLY the
		// contract's payment_amount_e9s, and a rental_debit ledger row for this
		// contract must exist with the matching negative amount.
		const paymentAmountE9s = await readContractPaymentAmount(contractId);
		const balanceAfterRentE9s = await readWalletBalance(requesterPubkey);
		expect(
			balanceAfterRentE9s,
			`wallet must decrease by payment_amount_e9s after rent: before=${balanceBeforeE9s}, after=${balanceAfterRentE9s}, expected debit=${paymentAmountE9s}`,
		).toBe(balanceBeforeE9s - paymentAmountE9s);

		const debitRaw = await sql(
			`SELECT amount_e9s FROM wallet_ledger WHERE reference = '${contractId}' AND entry_type = 'rental_debit'`,
		);
		expect(BigInt(debitRaw)).toBe(-paymentAmountE9s);

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
		const contractIdFromHref = href.split('/').pop()!;
		expect(contractIdFromHref).toBe(contractId);

		return { contractId, paymentAmountE9s, balanceBeforeE9s };
	}

	test('rent an offering → contract appears on the rentals list with a Cancel button', async ({
		page,
	}) => {
		const outcome = await rentViaDialog(page);
		expect(outcome.contractId).toMatch(/^[0-9a-f]+$/);
		firstContractId = outcome.contractId;
		firstRent = outcome;
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

		// Wallet money integrity (strengthened): the cancel refunded the debited
		// principal back to the wallet. Because the contract was cancelled at
		// `requested` (service never started → provisioning_completed_at_ns IS
		// NULL), the prorated refund equals the FULL payment_amount_e9s, so the
		// balance must be restored to its pre-rent value AND both ledger rows
		// for this contract must carry the exact matching amounts.
		const balanceAfterCancelE9s = await readWalletBalance(requesterPubkey);
		expect(
			balanceAfterCancelE9s,
			`wallet must be restored to pre-rent balance after a pre-service cancel: before=${firstRent!.balanceBeforeE9s}, after=${balanceAfterCancelE9s}`,
		).toBe(firstRent!.balanceBeforeE9s);

		// Ledger: exactly a rental_debit (-principal) and a rental_refund
		// (+principal) for this contract, with matching magnitudes.
		const ledgerRaw = await sql(`
			SELECT entry_type || ':' || amount_e9s FROM wallet_ledger
			WHERE reference = '${contractId}'
			ORDER BY id
		`);
		const byType = new Map<string, bigint>();
		for (const line of ledgerRaw.split('\n').map((l) => l.trim()).filter(Boolean)) {
			const [type, amount] = line.split(':');
			byType.set(type, BigInt(amount));
		}
		expect(byType.has('rental_debit'), 'rental_debit ledger row must exist').toBe(true);
		expect(byType.has('rental_refund'), 'rental_refund ledger row must exist').toBe(true);
		expect(byType.get('rental_debit')).toBe(-firstRent!.paymentAmountE9s);
		expect(byType.get('rental_refund')).toBe(firstRent!.paymentAmountE9s);
	});

	test('cancel a rental directly from the rentals list', async ({ page }) => {
		// A fresh rental (uses the next reservable cloud_resource).
		const outcome = await rentViaDialog(page);
		const contractId = outcome.contractId;

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

		// Wallet integrity (strengthened): same pre-service full-refund invariant
		// as the detail-page cancel — balance restored to its pre-rent value.
		const balanceAfterCancelE9s = await readWalletBalance(requesterPubkey);
		expect(
			balanceAfterCancelE9s,
			`wallet must be restored to pre-rent balance after list-level cancel: before=${outcome.balanceBeforeE9s}, after=${balanceAfterCancelE9s}`,
		).toBe(outcome.balanceBeforeE9s);
	});
});
