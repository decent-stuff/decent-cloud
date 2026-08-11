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
 * E2E proof of the marketplace buy-flow bug fix: a wallet-paid rental whose
 * provider has `auto_accept_rentals=true` advances PAST `requested`.
 *
 * THE BUG (api/src/openapi/contracts.rs::create_rental_request, fixed): the
 * payment branching used to call `try_auto_accept_contract` only on the
 * self-rental/test branch. The real wallet branch debited the wallet and
 * returned `None` WITHOUT ever auto-accepting, so every wallet-paid contract
 * got stuck at `requested` forever — never accepted, never provisioned. That
 * broke the entire marketplace buy flow for real buyers. The fix moved the
 * auto-accept + fulfillment block OUTSIDE the payment if/else so it runs
 * uniformly for every paid contract. This spec proves that fix end-to-end by
 * driving the REAL marketplace Rent dialog (the path a real buyer takes) and
 * asserting the contract is no longer stuck.
 *
 * Sibling spec rent-flow.spec.ts owns the pre-service FULL-refund wallet math;
 * it pins `auto_accept_rentals=false` so the contract stays at `requested`
 * (provisioning_completed_at_ns IS NULL → full principal refunded on cancel).
 * This spec is the COMPLEMENT: it pins `auto_accept_rentals=true`, lets the
 * contract auto-advance to `active` (provisioning_completed_at_ns gets SET by
 * self-provisioned activation), and therefore does NOT assert full-refund math
 * — a cancel at `active` is prorated, not full. The two specs together pin
 * both sides of the auto-accept decision.
 *
 * Shared-pubkey hazard: same as rent-flow — all testAccount users share one
 * requester pubkey, so this spec is serial and tears down its contracts +
 * wallet rows in afterAll. Run alongside rent-flow under `--workers 1`.
 */

const SSH_KEY = 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIE2eWalletAutoAcceptKey9982 e2e@wallet-auto-accept';
const CONTACT = 'email:e2e-wallet-auto-accept@test.example.com';

/**
 * Read the current prepaid wallet balance (e9s) for a pubkey; 0n if the user
 * has never topped up. Used to PROVE the rent debited the wallet (the bug was
 * specifically about the wallet-paid branch).
 */
async function readWalletBalance(pubkey: string): Promise<bigint> {
	const out = await sql(
		`SELECT balance_e9s FROM wallet_balances WHERE pubkey = '${pubkey}'`,
	);
	return BigInt(out || '0');
}

/** payment_amount_e9s charged for a contract — the principal the wallet debit
 * must match exactly. */
async function readContractPaymentAmount(contractId: string): Promise<bigint> {
	const out = await sql(
		`SELECT payment_amount_e9s FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
	);
	if (!out) throw new Error(`no payment_amount_e9s for contract ${contractId}`);
	return BigInt(out);
}

test.describe('Wallet-paid rental auto-accepts (marketplace buy-flow bug fix)', () => {
	test.describe.configure({ mode: 'serial' });

	let seed: RentableWithResourceSeed;
	let requesterPubkey: string;
	let contractId: string;
	let balanceBeforeE9s: bigint;

	test.beforeAll(async ({ testAccount }) => {
		requesterPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		// Fresh slate so the assertions see only this spec's contract.
		await deleteContractsForRequester(requesterPubkey);
		// Rentals are rejected unless the requester's email is verified.
		await verifyAccountEmail(requesterPubkey);
		// Seed a generous prepaid wallet balance so the wallet debit succeeds.
		// $1000 (1e12 e9s) covers the rental; cancel credits most of it back.
		await sql(`
			INSERT INTO wallet_balances (pubkey, balance_e9s)
			VALUES ('${requesterPubkey}', 1000000000000)
			ON CONFLICT (pubkey) DO UPDATE SET balance_e9s = 1000000000000
		`);
		// autoAcceptRentals: true is the WHOLE POINT — this is the provider
		// config the bug used to strand at `requested`.
		seed = await seedRentableWithResource({
			name: 'E2E Wallet Auto-Accept Offering',
			autoAcceptRentals: true,
		});
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

	test('wallet-paid rental with auto_accept=true advances past requested (the bug fix)', async ({
		page,
	}) => {
		// Capture the balance RIGHT before the POST so the debit assertion is exact.
		balanceBeforeE9s = await readWalletBalance(requesterPubkey);

		// Drive the REAL marketplace Rent dialog — the exact path a real buyer
		// takes and the path the bug broke.
		await page.goto(`/dashboard/marketplace/${seed.offeringNumericId}`);
		const rentBtn = page.getByRole('button', { name: 'Rent this offering' }).first();
		await expect(rentBtn).toBeVisible({ timeout: 15000 });
		await rentBtn.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({
			timeout: 5000,
		});

		await page.locator('textarea[placeholder*="ssh-ed25519"]').fill(SSH_KEY);
		await page.getByText('Advanced (optional)').click();
		await page.locator('input[placeholder*="email:you@example.com"]').fill(CONTACT);

		// Submit fires the real signed POST /api/v1/contracts (creates the
		// contract + debits the wallet + now, with the fix, auto-accepts).
		const postContracts = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'POST' &&
				resp.url().includes('/api/v1/contracts'),
			{ timeout: 20000 },
		);
		await page.getByRole('button', { name: 'Pay now', exact: true }).click();
		const resp = await postContracts;
		if (!resp.ok()) {
			const body = await resp.text().catch(() => '<no body>');
			throw new Error(`POST /contracts failed (${resp.status()}): ${body}`);
		}
		const json = await resp.json();
		const id: string | undefined = json?.data?.contractId;
		if (!id || !/^[0-9a-f]+$/.test(id)) {
			throw new Error(`POST /contracts response had no data.contractId: ${JSON.stringify(json)}`);
		}
		contractId = id;

		// 1. Wallet-debit proof: balance dropped by EXACTLY payment_amount_e9s
		//    (the bug was in the wallet-paid branch, so this must still hold).
		const paymentAmountE9s = await readContractPaymentAmount(contractId);
		const balanceAfterE9s = await readWalletBalance(requesterPubkey);
		expect(
			balanceAfterE9s,
			`wallet must decrease by payment_amount_e9s: before=${balanceBeforeE9s}, after=${balanceAfterE9s}, expected debit=${paymentAmountE9s}`,
		).toBe(balanceBeforeE9s - paymentAmountE9s);

		// 2. THE BUG-FIX PROOF: the contract must NOT be stuck at `requested`.
		//    For a self_provisioned offering with auto_accept=true, the handler
		//    runs try_auto_accept_contract (requested→accepted) then
		//    try_activate_self_provisioned_contract (accepted→active, which
		//    sets provisioning_completed_at_ns) synchronously before returning
		//    the 200 — so by now the status is `active`. The load-bearing
		//    assertion is the FIRST one: status !== 'requested'. Under the bug,
		//    this was always 'requested'; the fix makes it advance.
		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(
			status.trim(),
			`wallet-paid contract must NOT be stuck at 'requested' (the marketplace buy-flow bug); got '${status.trim()}'`,
		).not.toBe('requested');
		expect(
			['accepted', 'provisioning', 'provisioned', 'active'].includes(status.trim()),
			`status should have advanced past 'requested' via auto-accept; got '${status.trim()}'`,
		).toBe(true);

		// 3. payment_status must be 'succeeded' (the auto-accept eligibility
		//    check requires it; the bug-fix comment calls this out explicitly).
		const payStatus = await sql(
			`SELECT payment_status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(payStatus.trim()).toBe('succeeded');

		// 4. provisioning_completed_at_ns must be set — proves self-provisioned
		//    activation ran (accepted→active). This is exactly why rent-flow
		//    pins auto_accept=false: once this column is set, cancel refunds
		//    become PRORATED, so the full-refund math only holds at `requested`.
		const completedNs = await sql(
			`SELECT provisioning_completed_at_ns FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(
			completedNs.trim(),
			`an auto-accepted+activated contract must have provisioning_completed_at_ns set`,
		).toBeTruthy();
	});

	test('cancel the auto-accepted contract moves it to cancelled (cleanup)', async ({ page }) => {
		expect(contractId, 'previous test must have produced a contract id').toBeTruthy();

		await page.goto(`/dashboard/rentals/${contractId}`);
		const cancelBtn = page.getByRole('button', { name: 'Cancel', exact: true }).first();
		// An active contract is still cancellable; the button may take a moment
		// to mount as the detail page hydrates.
		await expect(cancelBtn).toBeVisible({ timeout: 15000 });

		const putCancel = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'PUT' &&
				resp.url().includes(`/api/v1/contracts/${contractId}/cancel`),
			{ timeout: 20000 },
		);
		// Two-step inline confirm: first Cancel arms, then Confirm fires the PUT.
		await cancelBtn.click();
		await page.getByRole('button', { name: 'Confirm', exact: true }).click();
		await putCancel;

		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
		);
		expect(status.trim()).toBe('cancelled');
		// NOTE: no full-refund assertion here — the contract was cancelled at
		// `active` (provisioning_completed_at_ns IS set), so the refund is
		// PRORATED, not the full principal. rent-flow.spec.ts owns the
		// full-refund invariant (cancel at `requested`).
	});
});
