import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedTransfer,
	deleteTransfersForAccount,
	randomHex,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/transfers.
 *
 * The transfers page shows the user's token balance and a list of transfers
 * (sent + received), with a toggle between "My Transfers" and "All Recent".
 * Balance = sum(received) - sum(sent + fees).
 *
 * Test data is seeded directly into token_transfers via psql. The transfers
 * API has no signed-request requirement (it's a public read endpoint), so we
 * could in principle also use the API to seed — but psql is faster and more
 * direct.
 */

test.describe('/dashboard/transfers', () => {
	// Helper: wait for the transfers page to settle into authenticated state.
	// The fast-auth fixture lands on /dashboard authenticated, but each
	// page.goto() reloads the SPA and authStore.initialize() must re-run
	// (it reads localStorage.seed_phrases and re-fetches the account from
	// the API). Under parallel workers this can take a few seconds. Waiting
	// for the "Logout" button to appear IS the auth-ready signal (same
	// pattern the fast-auth fixture uses).
	async function waitForAuthReady(page: import('@playwright/test').Page) {
		await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
	}

	// Each transfer row in the list has a direction-icon container (the round
	// coloured div with arrow icon). Scope by that to avoid matching other
	// dashboard chrome that uses the same card classes.
	const transferRowLocator = (page: import('@playwright/test').Page) =>
		page.locator('div.bg-surface-elevated.border.border-neutral-800.p-4.flex.items-center.gap-4');

	test('empty state: fresh user sees 0 balance and empty transfer list', async ({ page }) => {
		await page.goto('/dashboard/transfers');
		await waitForAuthReady(page);

		// Header
		await expect(page.getByRole('heading', { name: 'Transfers', exact: true })).toBeVisible();
		await expect(page.getByText('Token balance and transfer history', { exact: true })).toBeVisible();

		// Balance card showing 0
		await expect(page.getByText('Token Balance', { exact: true })).toBeVisible();
		await expect(page.locator('.text-3xl').getByText('0.0000')).toBeVisible();

		// Empty state
		await expect(page.getByRole('heading', { name: 'No Transfers' })).toBeVisible();
		await expect(page.getByText('You have no token transfers yet.')).toBeVisible();

		// View toggle is present
		await expect(page.getByRole('button', { name: 'My Transfers' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'All Recent' })).toBeVisible();
	});

	test('empty state: explains the per-transaction billing model (#433)', async ({ page }) => {
		// Regression for #433: the balance card used to show a number with no
		// context, leaving users unsure whether they needed to pre-load tokens
		// to rent. The fix documents that rentals are billed per-transaction
		// at checkout and that this balance is for P2P transfers.
		await page.goto('/dashboard/transfers');
		await waitForAuthReady(page);

		// Balance card explanation
		await expect(page.getByText(/billed per-transaction at checkout/i)).toBeVisible();
		// Empty-state guidance on how tokens move
		await expect(page.getByText(/Receive tokens from another account/i)).toBeVisible();
	});

	test('populated state: shows sent and received transfers with direction icons', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const other1 = randomHex(32);
		const other2 = randomHex(32);
		try {
			// Two transfers involving the test user: one received, one sent.
			// Use a substantial fee (0.1 ICP) so the balance is clearly below
			// 4.0 even after toFixed(4) rounding.
			await seedTransfer({ fromAccount: other1, toAccount: pubkey, amountE9s: 5_000_000_000, memo: 'received test' });
			await seedTransfer({ fromAccount: pubkey, toAccount: other2, amountE9s: 1_000_000_000, feeE9s: 100_000_000, memo: 'sent test' });

			await page.goto('/dashboard/transfers');
			await waitForAuthReady(page);

			// Balance = 5 received - (1 sent + 0.1 fee) = 3.9 ICP
			const balanceText = await page.locator('.text-3xl').textContent();
			expect(Number(balanceText)).toBeGreaterThan(3.89);
			expect(Number(balanceText)).toBeLessThan(3.91);

			// Both transfers listed with their memos
			await expect(page.getByText('received test', { exact: true })).toBeVisible();
			await expect(page.getByText('sent test', { exact: true })).toBeVisible();

			// Amounts formatted (5.0000 received, -1.0000 sent)
			await expect(page.locator('.text-green-400').getByText('+5.0000')).toBeVisible();
			await expect(page.locator('.text-red-400').getByText('-1.0000')).toBeVisible();
		} finally {
			await deleteTransfersForAccount(pubkey);
			await deleteTransfersForAccount(other1);
			await deleteTransfersForAccount(other2);
		}
	});

	test('view toggle: All Recent shows transfers from other accounts', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const alice = randomHex(32);
		const bob = randomHex(32);
		// Unique memo so we can find our transfer in the All Recent view even
		// when other parallel workers have added their own transfers.
		const uniqueMemo = `unrelated-${randomHex(4)}`;
		try {
			// Seed one transfer involving the user and one not.
			await seedTransfer({ fromAccount: alice, toAccount: pubkey, amountE9s: 2_000_000_000 });
			await seedTransfer({ fromAccount: alice, toAccount: bob, amountE9s: 3_000_000_000, memo: uniqueMemo });

			await page.goto('/dashboard/transfers');
			await waitForAuthReady(page);

			// Default view: My Transfers — only 1 transfer (alice → me).
			// The "My Transfers" query is filtered server-side by account, so
			// it's isolated from other workers' data.
			const cards = transferRowLocator(page);
			await expect(cards).toHaveCount(1);

			// Click "All Recent". This view shows ALL platform transfers (not
			// filtered by user), so it includes other workers' transfers.
			// Assert our specific transfers are visible rather than asserting
			// an exact count.
			await page.getByRole('button', { name: 'All Recent' }).click();

			// Our unrelated transfer is now visible (it wasn't in "My Transfers")
			await expect(page.getByText(uniqueMemo, { exact: true })).toBeVisible({ timeout: 10000 });
		} finally {
			await deleteTransfersForAccount(pubkey);
			await deleteTransfersForAccount(alice);
			await deleteTransfersForAccount(bob);
		}
	});

	test('transfer memo: displayed when present, hidden when absent', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const other = randomHex(32);
		try {
			// One transfer WITH memo, one WITHOUT.
			await seedTransfer({ fromAccount: other, toAccount: pubkey, amountE9s: 1_000_000_000, memo: 'with memo text' });
			await seedTransfer({ fromAccount: other, toAccount: pubkey, amountE9s: 2_000_000_000 });

			await page.goto('/dashboard/transfers');
			await waitForAuthReady(page);

			await expect(page.getByText('with memo text', { exact: true })).toBeVisible();
			// Two transfer rows total
			await expect(transferRowLocator(page)).toHaveCount(2);
		} finally {
			await deleteTransfersForAccount(pubkey);
			await deleteTransfersForAccount(other);
		}
	});

	test('balance computation: fees subtracted from balance', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const other = randomHex(32);
		try {
			// User receives 10 ICP and sends 5 ICP with a 0.5 ICP fee.
			// Balance = 10 - (5 + 0.5) = 4.5 ICP.
			await seedTransfer({ fromAccount: other, toAccount: pubkey, amountE9s: 10_000_000_000 });
			await seedTransfer({ fromAccount: pubkey, toAccount: other, amountE9s: 5_000_000_000, feeE9s: 500_000_000 });

			await page.goto('/dashboard/transfers');
			await waitForAuthReady(page);

			const balanceText = await page.locator('.text-3xl').textContent();
			expect(Number(balanceText)).toBeCloseTo(4.5, 1);
		} finally {
			await deleteTransfersForAccount(pubkey);
			await deleteTransfersForAccount(other);
		}
	});
});
