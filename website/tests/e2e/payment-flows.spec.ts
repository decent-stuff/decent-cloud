import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging, waitForApiResponse } from './fixtures/auth-helpers';

/**
 * E2E Tests for Rental Request Flows
 *
 * Prerequisites:
 * - API server running at http://localhost:59001 (or configured base URL)
 * - Website running at http://localhost:59000
 * - At least one offering in the marketplace
 *
 * Test Coverage:
 * - DCT payment method selection and contract creation
 * - Stripe payment method UI (if supported for currency)
 */

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
		await expect(page.locator('h2:has-text("Rent Resource")')).toBeVisible();

		// DCT should be selected by default (button style)
		await expect(page.locator('button:has-text("DCT Tokens")').filter({ hasText: /.*/ })).toBeVisible();

		// Fill in rental details
		await page.fill('textarea[placeholder*="ssh-ed25519"]', 'ssh-ed25519 AAAAB3NzaC1lZDI1NTE5AAAAITest test@example.com');
		await page.fill('input[placeholder*="email:you@example.com"]', 'email:test@example.com');
		await page.fill('textarea[placeholder*="special requirements"]', 'E2E test rental - DCT payment');

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

	test('Stripe payment UI - should show credit card option for supported currencies', async ({
		page,
	}) => {
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for offerings to load
		await page.waitForTimeout(1000);

		// Check if there are any offerings
		const rentButtons = page.locator('button:has-text("Rent Resource")');
		const count = await rentButtons.count();

		if (count === 0) {
			// Skip test if no offerings available
			test.skip();
			return;
		}

		// Click on first offering's "Rent Resource" button
		const firstOffering = rentButtons.first();
		await expect(firstOffering).toBeVisible({ timeout: 10000 });
		await firstOffering.click();

		// Wait for rental dialog to appear
		await expect(page.locator('h2:has-text("Rent Resource")')).toBeVisible();

		// Should show payment method options
		await expect(page.locator('legend:has-text("Payment Method")')).toBeVisible();
		await expect(page.locator('button:has-text("DCT Tokens")')).toBeVisible();

		// Credit Card option should be visible (may be disabled for unsupported currencies)
		await expect(page.locator('button:has-text("Credit Card")')).toBeVisible();
	});
});
