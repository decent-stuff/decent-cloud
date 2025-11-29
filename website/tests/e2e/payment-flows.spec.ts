import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging, waitForApiResponse } from './fixtures/auth-helpers';
import crypto from 'crypto';

/**
 * E2E Tests for Payment Flows
 *
 * Prerequisites:
 * - API server running at http://localhost:59001 (or configured base URL)
 * - Website running at http://localhost:59000
 * - Stripe test mode enabled
 *
 * Test Coverage:
 * 1. DCT Payment Flow - verify existing functionality still works
 * 2. Stripe Payment Success - test card payment → webhook → auto-acceptance
 * 3. Stripe Payment Failure - test declined card → error handling
 */

/**
 * Helper: Create a test offering via API
 */
async function createTestOffering(page: any, authPubkey: string): Promise<string> {
	const offeringParams = {
		offer_name: `Test GPU ${Date.now()}`,
		product_type: 'gpu',
		description: 'Test GPU for E2E payment testing',
		monthly_price: 100.0,
		currency: 'USD',
		processor_cores: 8,
		memory_amount: '16GB',
		gpu_name: 'NVIDIA RTX 4090',
		gpu_count: 1,
		gpu_memory_gb: 24,
		total_ssd_capacity: '500GB',
		datacenter_city: 'San Francisco',
		datacenter_country: 'USA',
		in_stock: true,
	};

	// Get auth headers from localStorage (set by test fixture)
	const authHeaders = await page.evaluate(() => {
		const stored = localStorage.getItem('auth_state');
		if (!stored) return null;
		const parsed = JSON.parse(stored);
		return {
			'X-Auth-Pubkey': parsed.pubkey,
			'X-Auth-Timestamp': Date.now().toString(),
		};
	});

	const response = await page.request.post(
		`${page.context()._options.baseURL?.replace('59000', '59001')}/api/v1/providers/${authPubkey}/offerings`,
		{
			data: offeringParams,
			headers: authHeaders,
		}
	);

	const result = await response.json();
	if (!result.success || !result.data) {
		throw new Error(`Failed to create test offering: ${result.error}`);
	}

	return result.data; // offering ID
}

/**
 * Helper: Get contract details via API
 */
async function getContract(page: any, contractId: string): Promise<any> {
	const apiBaseUrl = page.context()._options.baseURL?.replace('59000', '59001') || 'http://localhost:59001';
	const response = await page.request.get(
		`${apiBaseUrl}/api/v1/contracts/${contractId}`
	);

	const result = await response.json();
	return result.data;
}

/**
 * Helper: Simulate Stripe webhook event
 */
async function simulateStripeWebhook(
	page: any,
	eventType: 'payment_intent.succeeded' | 'payment_intent.payment_failed',
	paymentIntentId: string
): Promise<void> {
	const apiBaseUrl = page.context()._options.baseURL?.replace('59000', '59001') || 'http://localhost:59001';

	// Construct webhook event payload (simplified version)
	const event = {
		id: `evt_${Date.now()}`,
		type: eventType,
		data: {
			object: {
				id: paymentIntentId,
				object: 'payment_intent',
				amount: 10000,
				currency: 'usd',
				status: eventType === 'payment_intent.succeeded' ? 'succeeded' : 'failed',
			},
		},
	};

	// For E2E tests, we'll need to compute the signature
	// In a real scenario, this would come from Stripe
	const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET || 'whsec_test_secret';
	const timestamp = Math.floor(Date.now() / 1000);
	const payload = JSON.stringify(event);
	const signedPayload = `${timestamp}.${payload}`;

	// Create HMAC signature
	const signature = crypto
		.createHmac('sha256', webhookSecret)
		.update(signedPayload)
		.digest('hex');

	const stripeSignature = `t=${timestamp},v1=${signature}`;

	const response = await page.request.post(
		`${apiBaseUrl}/api/v1/webhooks/stripe`,
		{
			data: payload,
			headers: {
				'Content-Type': 'application/json',
				'Stripe-Signature': stripeSignature,
			},
		}
	);

	if (!response.ok()) {
		const text = await response.text();
		throw new Error(`Webhook simulation failed: ${response.status()} - ${text}`);
	}
}

test.describe('Payment Flows', () => {
	test.beforeEach(async ({ page }) => {
		setupConsoleLogging(page);
	});

	test('DCT Payment Flow - should create contract with DCT payment method', async ({
		page,
	}) => {
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for offerings to load
		await page.waitForTimeout(1000);

		// Click on first offering's "Rent Resource" button
		const firstOffering = page.locator('button:has-text("Rent Resource")').first();
		await expect(firstOffering).toBeVisible({ timeout: 10000 });
		await firstOffering.click();

		// Wait for rental dialog to appear
		await expect(page.locator('text=Rental Request')).toBeVisible();

		// DCT should be selected by default
		await expect(page.locator('input[value="dct"]:checked')).toBeVisible();

		// Fill in rental details
		await page.fill('input[placeholder*="ssh-rsa"]', 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC test@example.com');
		await page.fill('input[placeholder*="email"]', 'test@example.com');
		await page.fill('textarea[placeholder*="Additional notes"]', 'E2E test rental - DCT payment');

		// Wait for contract creation API call
		const apiResponsePromise = waitForApiResponse(page, /\/api\/v1\/contracts$/);

		// Submit request
		await page.click('button:has-text("Submit Request")');

		// Wait for API response
		await apiResponsePromise;

		// Wait for success message
		await expect(page.locator('text=Rental request created successfully')).toBeVisible({ timeout: 10000 });

		// Extract contract ID from success message
		const successText = await page.locator('text=Contract ID:').textContent();
		const contractId = successText?.match(/Contract ID: ([a-f0-9]+)/)?.[1];
		expect(contractId).toBeTruthy();

		// Verify contract via API
		const contract = await getContract(page, contractId!);
		expect(contract).toBeTruthy();
		expect(contract.payment_method).toBe('dct');
		expect(contract.payment_status).toBe('succeeded'); // DCT payments succeed immediately
		expect(contract.status).toBe('requested'); // Should NOT be auto-accepted
	});

	test('Stripe Payment Success Flow - should create contract, process payment, and auto-accept', async ({
		page,
	}) => {
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for offerings to load
		await page.waitForTimeout(1000);

		// Click on first offering's "Rent Resource" button
		const firstOffering = page.locator('button:has-text("Rent Resource")').first();
		await expect(firstOffering).toBeVisible({ timeout: 10000 });
		await firstOffering.click();

		// Wait for rental dialog to appear
		await expect(page.locator('text=Rental Request')).toBeVisible();

		// Select Stripe payment method (click the "Credit Card" button)
		await page.click('button:has-text("Credit Card")');
		await page.waitForTimeout(500); // Wait for Stripe Elements to initialize
		// Wait for Stripe Elements to load and verify card input appears
		await expect(page.locator('legend:has-text("Card Information")')).toBeVisible({ timeout: 5000 });

		// Wait for Stripe Elements to load and card input to appear
		await page.waitForTimeout(2000); // Increased timeout for Stripe to fully load

		// Fill in rental details
		await page.fill('input[placeholder*="ssh-rsa"]', 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC test@example.com');
		await page.fill('input[placeholder*="email"]', 'test@example.com');
		await page.fill('textarea[placeholder*="Additional notes"]', 'E2E test rental - Stripe payment success');

		// Fill in Stripe test card that always succeeds: 4242 4242 4242 4242
		const cardFrame = page.frameLocator('iframe[name*="__privateStripeFrame"]').first();
		await cardFrame.locator('input[name="cardnumber"]').fill('4242424242424242');
		await cardFrame.locator('input[name="exp-date"]').fill('12/34');
		await cardFrame.locator('input[name="cvc"]').fill('123');
		await cardFrame.locator('input[name="postal"]').fill('12345');

		// Wait for contract creation API call
		const apiResponsePromise = waitForApiResponse(page, /\/api\/v1\/contracts$/);

		// Submit request
		await page.click('button:has-text("Submit Request")');

		// Wait for API response
		await apiResponsePromise;

		// Wait for payment processing
		await expect(page.locator('text=Processing payment')).toBeVisible({ timeout: 5000 });

		// Wait for success message (payment confirmation completes)
		await expect(page.locator('text=Rental request created successfully')).toBeVisible({ timeout: 15000 });

		// Extract contract ID from success message
		const successText = await page.locator('text=Contract ID:').textContent();
		const contractId = successText?.match(/Contract ID: ([a-f0-9]+)/)?.[1];
		expect(contractId).toBeTruthy();

		// Verify contract via API - should have payment_status="pending" initially
		let contract = await getContract(page, contractId!);
		expect(contract).toBeTruthy();
		expect(contract.payment_method).toBe('stripe');
		expect(contract.payment_status).toBe('pending'); // Pending until webhook
		expect(contract.status).toBe('requested'); // Not yet accepted
		expect(contract.stripe_payment_intent_id).toBeTruthy();

		// Simulate webhook: payment_intent.succeeded
		await simulateStripeWebhook(
			page,
			'payment_intent.succeeded',
			contract.stripe_payment_intent_id
		);

		// Wait a bit for webhook processing
		await page.waitForTimeout(500);

		// Verify contract status updated after webhook
		contract = await getContract(page, contractId!);
		expect(contract.payment_status).toBe('succeeded');
		expect(contract.status).toBe('accepted'); // Should be auto-accepted on payment success
	});

	test('Stripe Payment Failure Flow - should handle declined card gracefully', async ({
		page,
	}) => {
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for offerings to load
		await page.waitForTimeout(1000);

		// Click on first offering's "Rent Resource" button
		const firstOffering = page.locator('button:has-text("Rent Resource")').first();
		await expect(firstOffering).toBeVisible({ timeout: 10000 });
		await firstOffering.click();

		// Wait for rental dialog to appear
		await expect(page.locator('text=Rental Request')).toBeVisible();

		// Select Stripe payment method (click the "Credit Card" button)
		await page.click('button:has-text("Credit Card")');
		await page.waitForTimeout(500); // Wait for Stripe Elements to initialize

		// Wait for Stripe Elements to load and card input to appear
		await page.waitForTimeout(2000); // Increased timeout for Stripe to fully load

		// Fill in rental details
		await page.fill('input[placeholder*="ssh-rsa"]', 'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC test@example.com');
		await page.fill('input[placeholder*="email"]', 'test@example.com');
		await page.fill('textarea[placeholder*="Additional notes"]', 'E2E test rental - Stripe payment failure');

		// Fill in Stripe test card that always declines: 4000 0000 0000 0002
		const cardFrame = page.frameLocator('iframe[name*="__privateStripeFrame"]').first();
		await cardFrame.locator('input[name="cardnumber"]').fill('4000000000000002');
		await cardFrame.locator('input[name="exp-date"]').fill('12/34');
		await cardFrame.locator('input[name="cvc"]').fill('123');
		await cardFrame.locator('input[name="postal"]').fill('12345');

		// Wait for contract creation API call
		const apiResponsePromise = waitForApiResponse(page, /\/api\/v1\/contracts$/);

		// Submit request
		await page.click('button:has-text("Submit Request")');

		// Wait for API response
		await apiResponsePromise;

		// Wait for payment processing
		await expect(page.locator('text=Processing payment')).toBeVisible({ timeout: 5000 });

		// Should show error message for declined card
		const errorMessage = page.locator('text=Your card was declined').or(
			page.locator('text=card was declined')
		);
		await expect(errorMessage.first()).toBeVisible({ timeout: 15000 });

		// Verify no success message
		await expect(page.locator('text=Rental request created successfully')).not.toBeVisible();

		// Dialog should still be open (user can try again)
		await expect(page.locator('text=Rental Request')).toBeVisible();
	});
});
