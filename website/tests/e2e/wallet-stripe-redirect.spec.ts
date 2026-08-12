import { test, expect } from './fixtures/test-account';
import type { Page } from '@playwright/test';

/**
 * Wallet Stripe Checkout redirect + return-banner coverage (E2E-4).
 *
 * `wallet-ui.spec.ts` only proved the top-up ERROR path (Stripe not configured
 * locally — STRIPE_SECRET_KEY unset → API returns a clear error). The actual
 * redirect wiring (`topupWallet()` → `window.location.href = checkoutUrl`) and
 * the `?topup=success|cancel` return banners were uncovered.
 *
 * Mock policy (website/AGENTS.md "Mock policy"): only the Stripe SDK and
 * outbound external HTTP may be mocked; never first-party API code. Here we
 * mock at the smallest external boundary only:
 *   1. `POST /wallet/topup` response — its payload is a thin wrapper around an
 *      external Stripe Checkout URL (`{checkoutUrl}`), and the real API can't
 *      produce one without STRIPE_SECRET_KEY. Intercepting it lets us verify
 *      the frontend WIRING (sets `window.location.href` to the returned URL)
 *      without a real Stripe round-trip. The GET /wallet call still hits the
 *      real API so the page renders normally; only the topup response is
 *      intercepted.
 *   2. The `https://checkout.stripe.com/**` navigation — purely external; we
 *      fulfill it with a stub so the browser navigation completes.
 *
 * No first-party API code is mocked; no DB rows are written (so this spec is
 * parallel-safe and does not need serial mode).
 */

// Wait for the balance card — it only renders after the real GET /wallet
// resolves (loading=false), which doubles as the hydration + auth-ready gate
// (same pattern as wallet-ui.spec.ts).
async function waitForWalletReady(page: Page): Promise<void> {
	await expect(page.locator('text=Available Balance')).toBeVisible({ timeout: 15000 });
}

test.describe('Wallet Stripe Checkout redirect + return banners', () => {
	test('top-up redirects the browser to the Stripe Checkout URL returned by the API', async ({ page }) => {
		const FAKE_CHECKOUT_URL = 'https://checkout.stripe.com/test-fake-session-id';

		// Intercept the topup API response. The page still calls signRequest()
		// client-side (no network), then fetches the URL — fetch is what we
		// fulfill here with a fake checkoutUrl. GET /wallet stays unmocked so
		// the page renders its real balance + ledger.
		await page.route('**/api/v1/users/*/wallet/topup', (route) =>
			route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					success: true,
					data: { checkoutUrl: FAKE_CHECKOUT_URL },
					error: null,
				}),
			}),
		);

		// The page sets `window.location.href = checkoutUrl`, triggering a full
		// document navigation to checkout.stripe.com. Fulfill that external
		// host so the navigation completes inside the harness (no real network).
		await page.route('https://checkout.stripe.com/**', (route) =>
			route.fulfill({
				status: 200,
				contentType: 'text/html',
				body: '<html><body><h1>Stubbed Stripe Checkout</h1></body></html>',
			}),
		);

		await page.goto('/dashboard/wallet');
		await waitForWalletReady(page);

		await page.locator('#amount').fill('25');
		// Arm the URL waiter BEFORE the click so the navigation race is captured
		// deterministically.
		const redirect = page.waitForURL('https://checkout.stripe.com/**', { timeout: 15000 });
		await page.locator('button:has-text("Top Up with Stripe")').click();
		await redirect;

		// Browser landed on the mocked Stripe Checkout URL — proves the
		// frontend honored the API's checkoutUrl via window.location.href.
		expect(page.url()).toBe(FAKE_CHECKOUT_URL);
	});

	test('success banner appears on /dashboard/wallet?topup=success', async ({ page }) => {
		await page.goto('/dashboard/wallet?topup=success');
		await waitForWalletReady(page);

		// Banner text matches +page.svelte's success branch verbatim.
		await expect(
			page.locator('text=Top-up processed. Your balance has been updated.'),
		).toBeVisible();
	});

	test('cancel banner appears on /dashboard/wallet?topup=cancel', async ({ page }) => {
		await page.goto('/dashboard/wallet?topup=cancel');
		await waitForWalletReady(page);

		// Banner text matches +page.svelte's cancel branch verbatim.
		await expect(
			page.locator('text=Top-up was cancelled. You were not charged.'),
		).toBeVisible();
	});
});
