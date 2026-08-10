import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import {
	seedRentableOffering,
	deleteOfferingsByProvider,
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
	sql,
} from './fixtures/seed-helpers';
import { API_BASE_URL } from './fixtures/api-base';
import { createHmac } from 'crypto';

/**
 * E2E Tests for Rental Request Payment Flows
 *
 * Prerequisites:
 * - Warm stack: API at http://localhost:59011, website at http://localhost:59010
 *   (or PLAYWRIGHT_API_URL / PLAYWRIGHT_BASE_URL overrides).
 * - A rentable offering is seeded per-suite via seedRentableOffering() (a
 *   self_provisioned public offering, always-online, non-example).
 * - STRIPE_WEBHOOK_SECRET=whsec_test_secret is set on the api-server
 *   (injected by `scripts/dev-server.sh --e2e`) so the webhook handler can
 *   verify the test-signed payload.
 *
 * Test Coverage:
 * - Wallet payment-method UI visibility for supported currencies
 * - Submit button rendering for the wallet-debit flow
 * - Stripe `checkout.session.completed` webhook → contract payment_status flip
 *   (legacy per-contract path; still exercised DB-side. The wallet top-up
 *   webhook branch is covered in wallet-api.spec.ts). No real Stripe round-trip.
 */

/**
 * Simulate a real Stripe `checkout.session.completed` webhook: build the event
 * payload matching the backend's `StripeCheckoutSession` shape (webhooks.rs:24),
 * sign it the SAME way Stripe does (`t=<ts>,v1=HMAC-SHA256(secret, "<ts>.<body>")`
 * over the RAW body string), and POST it to the webhook endpoint.
 *
 * The backend reads the raw body bytes for signature verification
 * (webhooks.rs:168), so `body` MUST be sent verbatim — Playwright's
 * `request.post({ data: <string> })` sends the string as-is, and we sign that
 * exact string. The signature scheme is identical to the existing helper; the
 * fix is emitting the event type the backend ACTS on (`checkout.session.completed`
 * with `metadata.contract_id`) instead of the ignored `payment_intent.succeeded`.
 */
async function simulateCheckoutSessionCompletedWebhook(
	page: import('@playwright/test').Page,
	opts: {
		contractId: string;
		sessionId?: string;
		paymentIntentId?: string;
		webhookSecret?: string;
	},
): Promise<import('@playwright/test').APIResponse> {
	const webhookSecret = opts.webhookSecret ?? 'whsec_test_secret';
	const sessionId = opts.sessionId ?? `cs_test_${Date.now()}`;
	const event = {
		id: `evt_test_${Date.now()}`,
		object: 'event',
		api_version: '2023-10-16',
		created: Math.floor(Date.now() / 1000),
		type: 'checkout.session.completed',
		data: {
			object: {
				id: sessionId,
				object: 'checkout.session',
				payment_intent: opts.paymentIntentId ?? `pi_test_${Date.now()}`,
				metadata: { contract_id: opts.contractId },
				total_details: { amount_tax: null },
				customer_details: { tax_ids: null },
			},
		},
		livemode: false,
		pending_webhooks: 1,
	};
	const payload = JSON.stringify(event);
	const timestamp = Math.floor(Date.now() / 1000);
	const signedPayload = `${timestamp}.${payload}`;
	const signature = createHmac('sha256', webhookSecret).update(signedPayload).digest('hex');

	return page.request.post(`${API_BASE_URL}/api/v1/webhooks/stripe`, {
		data: payload,
		headers: { 'stripe-signature': `t=${timestamp},v1=${signature}` },
	});
}

test.describe('Payment Flows', () => {
	// Seeded once per worker in beforeAll and cleaned in afterAll. The tests
	// only READ the offering (open its rental dialog), so parallel workers are
	// safe — each seeds+cleans its own unique-pubkey offering. USD enables the
	// Stripe Credit Card path (the sole paid rail; self-rental stays free).
	let usdOffering: { providerPubkeyHex: string; offeringName: string };

	test.beforeAll(async () => {
		usdOffering = await seedRentableOffering({ name: 'E2E Stripe Offering', currency: 'USD' });
	});
	test.afterAll(async () => {
		await deleteOfferingsByProvider(usdOffering.providerPubkeyHex);
	});

	test.beforeEach(async ({ page }) => {
		setupConsoleLogging(page);
	});

	/**
	 * Open the rental dialog from the marketplace for the seeded rentable offering.
	 *
	 * The marketplace action button reads "Rent" (enabled) for online non-example
	 * offerings; our self_provisioned seed is always-online and non-example, so its
	 * Rent button is enabled. Demo offerings render a disabled "Demo only" button
	 * and are skipped by getByRole({ name: 'Rent', exact: true }). The dialog that
	 * opens is titled "Rent Resource" (RentalRequestDialog.svelte).
	 */
	async function openRentalDialog(page: import('@playwright/test').Page, offeringName: string) {
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		// The marketplace renders offerings as table rows; scope the Rent button
		// to THIS offering's row so multiple rentable offerings don't collide
		// (a global .first() would click whichever row renders first).
		const row = page.locator('tr').filter({ hasText: offeringName }).first();
		await expect(row).toBeVisible({ timeout: 10000 });
		const rentButton = row.getByRole('button', { name: 'Rent', exact: true });
		await expect(rentButton).toBeVisible({ timeout: 5000 });
		await rentButton.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({ timeout: 5000 });
	}

	test('Wallet payment UI - renders the prepaid wallet section for paid rentals', async ({
		page,
	}) => {
		// The prepaid wallet is the sole paid rail (self-rental stays free). The
		// rental dialog renders the "Wallet Payment" section with a debit explainer
		// and a link to top up, instead of the old Stripe Checkout redirect.
		await openRentalDialog(page, usdOffering.offeringName);

		await expect(page.getByRole('heading', { name: 'Wallet Payment' })).toBeVisible({ timeout: 5000 });
		await expect(page.getByText(/debited from your prepaid wallet/i)).toBeVisible();
		await expect(page.getByRole('link', { name: '/dashboard/wallet' })).toBeVisible();

		// Rental form fields are present below the payment section.
		await expect(page.locator('textarea[placeholder*="ssh-ed25519"]')).toBeVisible();
		await expect(page.locator('input[placeholder*="email:you@example.com"]')).toBeVisible();
		await expect(page.locator('textarea[placeholder*="special requirements"]')).toBeVisible();
	});

	test('Wallet payment UI - shows Pay now for fiat-currency rentals', async ({ page }) => {
		// Submit button shows "Pay now" (payment is required up front via wallet
		// debit at contract creation — there is no redirect to an external checkout).
		await openRentalDialog(page, usdOffering.offeringName);

		await expect(page.locator('button:has-text("Pay now")')).toBeVisible();
	});

	// Serial mode: the webhook test seeds + deletes a contract for the shared
	// testAccount pubkey; it must not race a sibling doing the same.
	test.describe.configure({ mode: 'serial' });

	test('checkout.session.completed webhook flips payment_status to succeeded (the money path)', async ({
		page,
		testAccount,
	}) => {
		// This closes the Payment-flows ⚠️ in FLOWS.md: the BACKEND half of the
		// payment path (webhook signature verification → update_checkout_session_payment
		// → payment_status flip) needs no Stripe Checkout round-trip. We seed a
		// contract at the post-rental pre-payment state (requested + payment
		// pending), POST a real signed checkout.session.completed webhook whose
		// metadata links to that contract, and assert the backend activates it.
		const requesterPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const contractId = await seedContract({
			requesterPubkeyHex: requesterPubkey,
			status: 'requested',
			paymentStatus: 'pending',
		});
		try {
			// Pre-condition: the seeded contract is pending (not yet paid).
			const before = await sql(
				`SELECT payment_status FROM contract_sign_requests WHERE contract_id = decode('${contractId}', 'hex')`,
			);
			expect(before.trim()).toBe('pending');

			// POST the signed webhook. The backend verifies the HMAC-SHA256
			// signature over "<ts>.<raw body>", extracts metadata.contract_id,
			// hex-decodes it, and calls update_checkout_session_payment.
			const sessionId = `cs_test_${Date.now()}`;
			const paymentIntentId = `pi_test_${Date.now()}`;
			const resp = await simulateCheckoutSessionCompletedWebhook(page, {
				contractId,
				sessionId,
				paymentIntentId,
			});

			// The webhook MUST return 200 (Stripe uses 2xx to stop retrying). A
			// 401 would mean the signature failed; a 400 would mean a malformed
			// payload; a 500 would mean the DB update broke.
			expect(resp.status(), `webhook HTTP status: ${resp.status()}`).toBe(200);

			// Post-condition: the real handler flipped payment_status AND recorded
			// the Stripe session + payment-intent ids on the contract row — these
			// are the fields downstream refund/dispute lookups key on.
			// psql --no-align emits columns pipe-separated on one line.
			const row = await sql(`
				SELECT payment_status,
				       stripe_checkout_session_id,
				       stripe_payment_intent_id
				FROM contract_sign_requests
				WHERE contract_id = decode('${contractId}', 'hex')
			`);
			const [payStatus, csId, piId] = row.split('|').map((l) => l.trim());
			expect(payStatus).toBe('succeeded');
			expect(csId).toBe(sessionId);
			expect(piId).toBe(paymentIntentId);
		} finally {
			await deleteContractsForRequester(requesterPubkey);
		}
	});
});
