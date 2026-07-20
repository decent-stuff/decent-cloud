import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /checkout (cancel + success leaf routes).
 *
 * Both routes are anonymous-OK destinations that Stripe redirects to
 * after a checkout attempt. They have no auth requirement and render
 * hard-coded UI — perfect targets for structural assertions.
 *
 * `/checkout/cancel` reads an optional `contract_id` query param.
 * `/checkout/success` reads `session_id` and falls back to a clear
 * "Something Went Wrong" error when it's missing.
 */

test.describe('/checkout', () => {
	test.describe('cancel page', () => {
		test('renders the cancelled-payment page without a contract_id', async ({ page }) => {
			await page.goto('/checkout/cancel');

			await expect(page.getByRole('heading', { name: 'Payment Cancelled' })).toBeVisible();
			await expect(page.getByText(/Your payment was cancelled\. No charges/i)).toBeVisible();

			// Without contract_id, the secondary "View My Rentals" button is hidden.
			// Only "Browse Marketplace" is rendered.
			await expect(page.getByRole('button', { name: 'Browse Marketplace' })).toBeVisible();
			await expect(page.getByRole('button', { name: 'View My Rentals' })).toHaveCount(0);
		});

		test('shows the "View My Rentals" link when contract_id is present', async ({ page }) => {
			await page.goto('/checkout/cancel?contract_id=abc123');

			await expect(page.getByRole('heading', { name: 'Payment Cancelled' })).toBeVisible();
			await expect(page.getByRole('button', { name: 'View My Rentals' })).toBeVisible();
		});
	});

	test.describe('success page', () => {
		test('renders "Something Went Wrong" when session_id is missing', async ({ page }) => {
			// No session_id query param — the page short-circuits with an error
			// before attempting verification. This is the easy anonymous state
			// to assert without stubbing the verify-checkout endpoint.
			await page.goto('/checkout/success');

			await expect(page.getByRole('heading', { name: 'Something Went Wrong' })).toBeVisible();
			await expect(page.getByText('No session_id in URL')).toBeVisible();
			await expect(page.getByRole('button', { name: 'View My Rentals' })).toBeVisible();
		});
	});
});
