import { test as accountTest, expect } from './fixtures/test-account';
import { stripeMockScript } from './fixtures/stripe-mock';
import {
	pubkeyHexFromSeed,
	deleteContractsForRequester,
	seedRentableWithResource,
	cleanupRentableWithResource,
	sql,
	type RentableWithResourceSeed,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the PRIMARY tenant journey: rent → pay → view → cancel.
 *
 * Before this spec, cancel was only ever exercised on DB-seeded contracts
 * (rentals.spec.ts). This spec drives the REAL flow end-to-end against the warm
 * stack: the marketplace Rent dialog → a real signed POST /api/v1/contracts that
 * creates a genuine contract → the rentals list → the rental detail page → a
 * real signed PUT /api/v1/contracts/:id/cancel. No first-party API is mocked;
 * only the Stripe SDK script load (an external boundary) is stubbed.
 *
 * Why the contract lands at `requested` (a cancellable state): the offering is
 * seeded self_provisioned (so the marketplace shows an enabled Rent button) and
 * the test rents it as a normal tenant via the Stripe/Credit-Card path. The API
 * commits the contract at `requested` (payment_status `pending`) during
 * create_rental_request, BEFORE attempting the Stripe checkout session. The
 * Stripe checkout session itself cannot complete in the e2e harness (no
 * STRIPE_SECRET_KEY), so the dialog surfaces a payment error — but the contract
 * legitimately exists and `isCancellable()` includes `requested`, so the rentals
 * UI renders a Cancel button for it. A redirect to Stripe Checkout is also
 * route-intercepted so the spec is robust if Stripe is ever configured.
 *
 * Shared-pubkey hazard: all testAccount users derive the same requester pubkey,
 * and this spec CREATES real contracts for it. Serial mode + beforeAll/afterAll
 * cleanup (deleteContractsForRequester) prevents parallel workers from nuking
 * each other's data — same pattern as rentals.spec.ts / invoices.spec.ts.
 */

// Extend the fast-auth fixture: install the Stripe SDK mock (the one allowed
// external-boundary mock) so RentalRequestDialog's loadStripe() does not fetch
// real js.stripe.com. addInitScript applies to every navigation after the base
// page fixture lands on /dashboard, so it is active when the dialog mounts.
const test = accountTest.extend({
	page: async ({ page }, use) => {
		await page.context().addInitScript(stripeMockScript);
		await use(page);
	},
});

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
		await sql(`
			UPDATE accounts SET email_verified = true
			WHERE id = (
				SELECT account_id FROM account_public_keys
				WHERE public_key = decode('${requesterPubkey}', 'hex')
			)
		`);
		seed = await seedRentableWithResource({ name: 'E2E Rent Flow Offering' });
	});

	test.afterAll(async () => {
		// Guard: beforeAll may have thrown before assigning `seed`.
		if (seed) await cleanupRentableWithResource(seed);
		await deleteContractsForRequester(requesterPubkey);
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
		// Robustness: if Stripe were configured, the dialog would redirect to
		// checkout.stripe.com. Intercept that external boundary so the app stays
		// drivable. (Currently Stripe is unconfigured, so this never fires.)
		await page.route('https://checkout.stripe.com/**', (route) =>
			route.fulfill({
				status: 200,
				contentType: 'text/html',
				body: '<!-- stripe checkout intercepted (e2e boundary mock) -->',
			}),
		);

		await page.goto(`/dashboard/marketplace/${seed.offeringNumericId}`);
		// The detail page renders the Rent button twice (responsive variants).
		const rentBtn = page.getByRole('button', { name: 'Rent this offering' }).first();
		await expect(rentBtn).toBeVisible({ timeout: 15000 });
		await rentBtn.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({
			timeout: 5000,
		});

		// USD offering → Stripe (Credit Card) is the default payment method.
		await page.locator('textarea[placeholder*="ssh-ed25519"]').fill(SSH_KEY);
		await page.locator('input[placeholder*="email:you@example.com"]').fill(CONTACT);
		const memo = page.locator('textarea[placeholder*="special requirements"]');
		if (await memo.isVisible().catch(() => false)) await memo.fill(MEMO);

		// Submit fires the real signed POST /api/v1/contracts (creates the contract).
		const postContracts = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'POST' &&
				resp.url().includes('/api/v1/contracts'),
			{ timeout: 20000 },
		);
		await page.getByRole('button', { name: 'Pay now', exact: true }).click();
		await postContracts;

		// The contract now exists at `requested`. Navigate to the rentals list.
		await page.goto('/dashboard/rentals');

		// The contract card for our offering must appear, with a Cancel button
		// (isCancellable includes 'requested'/'pending').
		const card = page.locator(`a.card:has-text("${seed.offeringName}")`).first();
		await expect(card).toBeVisible({ timeout: 15000 });
		await expect(card.getByText(/Awaiting Payment/i)).toBeVisible();
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
		// A 'requested' contract shows the pre-payment status + a Cancel action.
		await expect(page.getByText(/Awaiting Payment/i).first()).toBeVisible({ timeout: 10000 });
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
