import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import { seedRentableOffering, deleteOfferingsByProvider } from './fixtures/seed-helpers';
import { createHmac } from 'crypto';

/**
 * E2E Tests for Rental Request Payment Flows
 *
 * Prerequisites:
 * - Warm stack: API at http://localhost:59011, website at http://localhost:59010
 *   (or PLAYWRIGHT_API_URL / PLAYWRIGHT_BASE_URL overrides).
 * - A rentable offering is seeded per-suite via seedRentableOffering() (a
 *   self_provisioned public offering, always-online, non-example).
 *
 * Test Coverage:
 * - ICPay payment method selection and contract creation UI
 * - Stripe payment-method option visibility for supported currencies
 * - Stripe Elements rendering after Credit Card selection
 */

/** API base URL for direct backend calls (webhook sim, contract fetch).
 *  Mirrors playwright.config.ts apiURL default. The previous derivation
 *  (`baseURL.replace('59000','59001')`) was a no-op against the warm stack
 *  (59010 has no '59000' substring) and would have POSTed webhooks to the
 *  web server. */
const API_BASE_URL = process.env.PLAYWRIGHT_API_URL || 'http://localhost:59011';

/**
 * Helper: Get contract details via API
 */
async function getContract(page: import('@playwright/test').Page, contractId: string): Promise<any> {
	const response = await page.request.get(`${API_BASE_URL}/api/v1/contracts/${contractId}`);
	const result = await response.json();
	return result.data;
}

/**
 * Helper: Simulate ICPay webhook event
 * Creates properly signed webhook payload matching real ICPay webhook structure
 *
 * Structure based on: https://docs.icpay.org/webhooks
 * Signature format: "t=<timestamp>,v1=<HMAC-SHA256>"
 */
async function simulateIcpayWebhook(
	page: import('@playwright/test').Page,
	eventType: string,
	paymentId: string,
	contractIdHex: string,
	webhookSecret: string = 'whsec_test_secret'
): Promise<void> {
	const event = {
		id: `evt_icpay_${Date.now()}`,
		type: eventType,
		data: {
			object: {
				id: paymentId,
				status: eventType === 'payment.completed' ? 'completed' : 'failed',
				amount: '1000000000', // 1 ICP in e8s
				metadata: {
					contract_id: contractIdHex
				}
			}
		}
	};

	const payload = JSON.stringify(event);
	const timestamp = Math.floor(Date.now() / 1000);
	const signedPayload = `${timestamp}.${payload}`;

	// Create HMAC signature (same algorithm as Stripe, per ICPay docs)
	const signature = createHmac('sha256', webhookSecret)
		.update(signedPayload)
		.digest('hex');

	await page.request.post(`${API_BASE_URL}/api/v1/webhooks/icpay`, {
		data: payload,
		headers: {
			'x-icpay-signature': `t=${timestamp},v1=${signature}`
		}
	});
}

/**
 * Helper: Simulate Stripe webhook event
 * Creates properly signed webhook payload matching real Stripe webhook structure
 *
 * Structure based on: https://docs.stripe.com/webhooks/stripe-events
 * This matches the actual webhook format Stripe sends in production
 */
async function simulateStripeWebhook(
	page: import('@playwright/test').Page,
	eventType: string,
	paymentIntentId: string,
	webhookSecret: string = 'whsec_test_secret'
): Promise<void> {
	// Create event matching real Stripe webhook structure
	// Based on actual webhook payload from Stripe docs
	const event = {
		id: `evt_test_${Date.now()}`,
		object: 'event',  // Real webhooks have this
		api_version: '2023-10-16',  // Current Stripe API version
		created: Math.floor(Date.now() / 1000),
		type: eventType,
		data: {
			object: {
				id: paymentIntentId,
				object: 'payment_intent',
				amount: 2000,
				amount_capturable: 0,
				amount_received: 2000,
				currency: 'usd',
				status: eventType === 'payment_intent.succeeded' ? 'succeeded' : 'failed',
				livemode: false,
				metadata: {},
				payment_method_types: ['card']
			}
		},
		livemode: false,
		pending_webhooks: 1,
		request: {
			id: null,
			idempotency_key: null
		}
	};

	const payload = JSON.stringify(event);
	const timestamp = Math.floor(Date.now() / 1000);
	const signedPayload = `${timestamp}.${payload}`;

	// Create HMAC signature (same algorithm Stripe uses)
	const signature = createHmac('sha256', webhookSecret)
		.update(signedPayload)
		.digest('hex');

	await page.request.post(`${API_BASE_URL}/api/v1/webhooks/stripe`, {
		data: payload,
		headers: {
			'stripe-signature': `t=${timestamp},v1=${signature}`
		}
	});
}

test.describe('Payment Flows', () => {
	// Seeded once per worker in beforeAll and cleaned in afterAll. The three
	// tests only READ the offering (open its rental dialog), so parallel
	// workers are safe — each seeds+cleans its own unique-pubkey offerings.
	// Two currencies: ICP drives the ICPay wallet-connection guard; USD enables
	// the Stripe Credit Card path (card payment is disabled for ICP offerings).
	let icpOffering: { providerPubkeyHex: string; offeringName: string };
	let usdOffering: { providerPubkeyHex: string; offeringName: string };

	test.beforeAll(async () => {
		icpOffering = await seedRentableOffering({ name: 'E2E ICPay Offering', currency: 'ICP' });
		usdOffering = await seedRentableOffering({ name: 'E2E Stripe Offering', currency: 'USD' });
	});
	test.afterAll(async () => {
		await deleteOfferingsByProvider(icpOffering.providerPubkeyHex);
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

	test('ICPay Payment UI - should show ICPay payment option and wallet connection requirement', async ({
		page,
	}) => {
		/**
		 * NOTE: Full ICPay payment flow testing requires a connected wallet.
		 *
		 * This test verifies:
		 * - ICPay payment UI loads correctly
		 * - ICPay is selected by default
		 * - Wallet connection is required before payment submission
		 * - All form fields are present
		 *
		 * Cannot test (limitations of e2e testing with ICPay):
		 * - Actual wallet connection (requires Internet Identity, Plug, etc.)
		 * - Payment processing (requires real wallet and ICPay testnet)
		 *
		 * For full payment flow testing:
		 * - Manual testing with ICPay testnet and a connected wallet
		 * - Backend webhook tests verify payment confirmation logic
		 */

		await openRentalDialog(page, icpOffering.offeringName);

		// ICPay should be selected by default
		const icpayButton = page.locator('button:has-text("Crypto (ICPay)")');
		await expect(icpayButton).toBeVisible();

		// Verify ICPay payment section appears with wallet connection prompt
		await expect(page.locator('text=Crypto Payment via ICPay')).toBeVisible();
		await expect(page.locator('text=Connect your wallet')).toBeVisible();
		await expect(page.locator('button:has-text("Connect Wallet")')).toBeVisible();

		// Fill in rental details
		await page.fill('textarea[placeholder*="ssh-ed25519"]', 'ssh-ed25519 AAAAB3NzaC1lZDI1NTE5AAAAITest test@example.com');
		await page.fill('input[placeholder*="email:you@example.com"]', 'email:test@example.com');
		await page.fill('textarea[placeholder*="special requirements"]', 'E2E test rental - ICPay payment');

		// Try to submit without wallet connection - should show error
		await page.click('button:has-text("Pay now")');

		// Should show wallet connection error
		await expect(page.locator('text=Please connect your wallet first')).toBeVisible({ timeout: 5000 });

		// Verify all other form fields are still present
		await expect(page.locator('textarea[placeholder*="ssh-ed25519"]')).toBeVisible();
		await expect(page.locator('input[placeholder*="email:you@example.com"]')).toBeVisible();
		await expect(page.locator('textarea[placeholder*="special requirements"]')).toBeVisible();

		// Test passed - ICPay UI loads correctly and requires wallet connection
	});

	test('Stripe payment UI - should show credit card option for supported currencies', async ({
		page,
	}) => {
		await openRentalDialog(page, usdOffering.offeringName);

		// Should show payment method options
		await expect(page.locator('legend:has-text("Payment Method")')).toBeVisible();
		await expect(page.locator('button:has-text("ICPay")')).toBeVisible();

		// Credit Card option should be visible (may be disabled for unsupported currencies)
		await expect(page.locator('button:has-text("Credit Card")')).toBeVisible();
	});

	test('Stripe Payment UI - should show Stripe Checkout redirect UI for fiat currencies', async ({
		page,
	}) => {
		// The Stripe integration is redirect-based (Stripe Checkout), NOT embedded
		// Stripe Elements — there is no in-page card-entry iframe. After selecting
		// Credit Card the dialog shows a redirect explainer (+ test card reference
		// in DEV). Submitting hands off to Stripe's hosted checkout page.
		await openRentalDialog(page, usdOffering.offeringName);

		// For USD, Credit Card (Stripe) is available; ICPay is disabled.
		const creditCard = page.locator('button:has-text("Credit Card")');
		await expect(creditCard).toBeVisible();
		await creditCard.click();

		// Stripe Checkout section renders with the redirect explainer.
		await expect(page.getByRole('heading', { name: 'Credit Card Payment via Stripe' })).toBeVisible({ timeout: 5000 });
		await expect(page.getByText("You will be redirected to Stripe's secure checkout")).toBeVisible();

		// Rental form fields are present below the payment section.
		await expect(page.locator('textarea[placeholder*="ssh-ed25519"]')).toBeVisible();
		await expect(page.locator('input[placeholder*="email:you@example.com"]')).toBeVisible();
		await expect(page.locator('textarea[placeholder*="special requirements"]')).toBeVisible();

		// Submit button shows "Pay now" (payment is required up front).
		await expect(page.locator('button:has-text("Pay now")')).toBeVisible();
	});

	/**
	 * Stripe Payment Success/Failure Flow Tests
	 *
	 * Not in the automated e2e suite: completing a Stripe payment requires the
	 * hosted Stripe Checkout page (an external, cross-origin redirect), which
	 * Playwright cannot drive in-process. The redirect handoff + webhook-driven
	 * contract activation are covered instead via:
	 *
	 * 1. **Manual Testing** (development)
	 *    - Start API and website servers (warm stack: 59010/59011).
	 *    - Marketplace → click "Rent" on a USD offering → Credit Card → "Pay now".
	 *    - Test cards: 4242 4242 4242 4242 (success), 4000 0000 0000 0002 (declined),
	 *      more at https://stripe.com/docs/testing#cards
	 *
	 * 2. **Stripe CLI** (webhook verification)
	 *    stripe listen --forward-to http://localhost:59011/api/v1/webhooks/stripe
	 *    stripe trigger payment_intent.succeeded
	 *
	 * 3. **Backend webhook logic** — simulateIcpayWebhook / simulateStripeWebhook /
	 *    getContract above sign payloads and POST to the real webhook endpoints,
	 *    exercising the backend's signature verification + contract-state transitions
	 *    once wired into a flow that has created a real contract id.
	 */
});
