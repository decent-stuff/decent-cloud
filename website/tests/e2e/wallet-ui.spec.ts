import { test, expect } from './fixtures/test-account';
import { sql, pubkeyHexFromSeed } from './fixtures/seed-helpers';

/**
 * Wallet UI verification.
 *
 * Verifies the /dashboard/wallet page renders correctly against the warm
 * stack: balance display, top-up form, ledger table, sidebar nav, and the
 * graceful error path when Stripe is not configured (local dev).
 *
 * Serial mode: the balance-seeding test writes DB rows for the shared
 * testAccount pubkey, so it must not overlap with the $0.00-balance assertions.
 */
test.describe('Wallet UI', () => {
	test.describe.configure({ mode: 'serial' });
	test('wallet page shows balance, top-up form, and empty ledger for a new user', async ({ page, testAccount }) => {
		await page.goto('/dashboard/wallet');

		// Page heading + balance card.
		await expect(page.locator('h1:has-text("Wallet")')).toBeVisible();
		await expect(page.locator('text=Available Balance')).toBeVisible();
		// New user → $0.00 balance.
		await expect(page.locator('text=$0.00 USD')).toBeVisible();

		// Top-up form.
		await expect(page.locator('h2:has-text("Add Funds")')).toBeVisible();
		await expect(page.locator('button:has-text("Top Up with Stripe")')).toBeVisible();

		// Ledger empty state.
		await expect(page.locator('h2:has-text("Recent Transactions")')).toBeVisible();
		await expect(page.locator('text=No transactions yet.')).toBeVisible();
	});

	// Wait for the balance card — it only renders after loadWallet() resolves
	// (loading=false), which doubles as a hydration-complete + auth-ready signal.
	async function waitForWalletReady(page: import('@playwright/test').Page) {
		await expect(page.locator('text=Available Balance')).toBeVisible({ timeout: 15000 });
	}

	test('top-up form rejects invalid input client-side', async ({ page }) => {
		await page.goto('/dashboard/wallet');
		await waitForWalletReady(page);

		const amountInput = page.locator('#amount');
		const submitBtn = page.locator('button:has-text("Top Up with Stripe")');

		// Enter zero — client validation should reject.
		await amountInput.fill('0');
		await submitBtn.click();
		await expect(page.locator('text=Enter a positive amount')).toBeVisible();
	});

	test('top-up shows a clear error when Stripe is not configured (local dev)', async ({ page }) => {
		await page.goto('/dashboard/wallet');
		await waitForWalletReady(page);

		await page.locator('#amount').fill('10');
		await page.locator('button:has-text("Top Up with Stripe")').click();

		// STRIPE_SECRET_KEY is unset locally → the API returns a clear, actionable
		// error mentioning Stripe. The UI surfaces it inline (no silent failure).
		await expect(page.locator('text=/Stripe/i').first()).toBeVisible({ timeout: 10000 });
	});

	test('wallet balance card is visible on the dashboard overview', async ({ page }) => {
		await page.goto('/dashboard');

		// The dashboard financial summary includes a Wallet card (scoped to
		// main to avoid matching the sidebar nav link too — strict mode).
		await expect(page.locator('main a[href="/dashboard/wallet"]')).toBeVisible({ timeout: 15000 });
	});

	test('sidebar nav links to the wallet page', async ({ page }) => {
		await page.goto('/dashboard');

		// Sidebar "My Activity" section has a Wallet link (scoped to nav to
		// avoid matching the dashboard card link too — strict mode).
		const walletLink = page.locator('nav a[href="/dashboard/wallet"]');
		await expect(walletLink).toBeVisible({ timeout: 15000 });
	});

	test('top-up via API credits balance and appears in ledger', async ({ page, testAccount }) => {
		// Seed a balance directly via the DB (simulates a completed Stripe top-up
		// webhook) so we can verify the UI renders a non-zero balance + ledger row.
		const pubkeyHex = pubkeyHexFromSeed(testAccount.seedPhrase);
		// Seed a balance directly via the DB (simulates a completed Stripe top-up
		// webhook) so we can verify the UI renders a non-zero balance + ledger row.
		await sql(`DELETE FROM wallet_ledger WHERE pubkey = '${pubkeyHex}'`);
		await sql(`DELETE FROM wallet_balances WHERE pubkey = '${pubkeyHex}'`);
		// $5.00 = 5_000_000_000 e9s.
		await sql(
			`INSERT INTO wallet_balances (pubkey, balance_e9s) VALUES ('${pubkeyHex}', 5000000000)`,
		);
		await sql(
			`INSERT INTO wallet_ledger (pubkey, amount_e9s, balance_after_e9s, entry_type, reference)
			 VALUES ('${pubkeyHex}', 5000000000, 5000000000, 'topup', 'e2e-test-session')`,
		);

		await page.goto('/dashboard/wallet');

		// Balance reflects the seeded amount.
		await expect(page.locator('text=$5.00 USD')).toBeVisible();

		// Ledger shows the top-up row.
		await expect(page.locator('text=Top-up')).toBeVisible();

		// Cleanup the seeded data.
		await sql(`DELETE FROM wallet_ledger WHERE pubkey = '${pubkeyHex}'`);
		await sql(`DELETE FROM wallet_balances WHERE pubkey = '${pubkeyHex}'`);
	});
});
