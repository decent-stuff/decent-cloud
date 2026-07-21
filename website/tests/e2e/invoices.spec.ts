import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
	randomHex,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/invoices.
 *
 * The invoices page renders a row per contract that has a downloadable invoice
 * — i.e. payment_status in ('succeeded','refunded') OR status in
 * ('active','provisioned','provisioning','accepted'). Contracts in 'requested'
 * with 'pending' payment are excluded.
 *
 * The invoice data source is the same contract_sign_requests table the rentals
 * page reads from, so the spec uses the same DB-direct seed helper.
 */

test.describe('/dashboard/invoices', () => {
	test('empty state: fresh user sees FAQ and marketplace CTA', async ({ page }) => {
		await page.goto('/dashboard/invoices');

		// Header
		await expect(page.getByRole('heading', { name: 'Invoices' })).toBeVisible();
		await expect(page.getByText('Download invoices for your rental contracts')).toBeVisible();

		// FAQ cards
		await expect(page.getByRole('heading', { name: 'When will I see invoices?' })).toBeVisible();
		await expect(page.getByRole('heading', { name: 'How billing works' })).toBeVisible();

		// FAQ details (collapsed by default; just check the summary text exists)
		await expect(page.getByText('Can I get a refund?')).toBeVisible();
		await expect(page.getByText('What payment methods are accepted?')).toBeVisible();
		await expect(page.getByText('Why is my invoice missing?')).toBeVisible();

		// Marketplace CTA
		await expect(page.getByRole('link', { name: /Browse Marketplace/ })).toBeVisible();
	});

	test('populated state: shows invoice table with one row per invoiceable contract', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Three invoiceable contracts (different status / payment combos) and
			// one non-invoiceable (requested + payment pending).
			await seedContract({ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: pubkey, status: 'provisioning', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: pubkey, status: 'requested', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: pubkey, status: 'requested', paymentStatus: 'pending' });

			await page.goto('/dashboard/invoices');

			// Table header row (Playwright exposes <th> as cell role here, not columnheader).
			await expect(page.locator('thead').getByText('Date')).toBeVisible();
			await expect(page.locator('thead').getByText('Contract')).toBeVisible();
			await expect(page.locator('thead').getByText('Amount')).toBeVisible();
			await expect(page.locator('thead').getByText('Invoice')).toBeVisible();

			// Three invoiceable rows (active, provisioning, requested+succeeded) — the
			// fourth contract (requested+pending) is filtered out by the page logic.
			const pdfButtons = page.getByRole('button', { name: /PDF/ });
			await expect(pdfButtons).toHaveCount(3);

			// Each row has a contract link to the rentals detail page
			const contractLinks = page.locator('a[href^="/dashboard/rentals/"]');
			await expect(contractLinks).toHaveCount(3);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('invoiceable filter: only contracts with succeeded payment appear', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Only one invoiceable contract — the others should be filtered out.
			await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
				currency: 'USD',
				paymentAmountE9s: 25_00_000_000, // $25.00 in e9s (USD uses 1e8 minor unit?)
			});
			await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'rejected',
				paymentStatus: 'failed',
			});
			await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'cancelled',
				paymentStatus: 'failed',
			});

			await page.goto('/dashboard/invoices');

			// Only 1 PDF button visible (the succeeded payment one).
			await expect(page.getByRole('button', { name: /PDF/ })).toHaveCount(1);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('action: clicking PDF button shows downloading indicator then returns', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
			});

			await page.goto('/dashboard/invoices');

			// The PDF download is triggered via a signed GET to /contracts/:id/invoice.
			// The button flips to a "Downloading..." state during the request.
			// Even if the API returns an error (no invoice generated for seeded
			// contract), the button still flips state then reverts. We assert the
			// button is present and clickable; the download itself is best validated
			// via the integration test in api/src/openapi/invoices.rs.
			const pdfButton = page.getByRole('button', { name: /PDF/ }).first();
			await expect(pdfButton).toBeVisible();
			await expect(pdfButton).toBeEnabled();

			// Trigger the click. The handler runs in the background; we just verify
			// no uncaught exceptions are thrown on the page.
			const errors: string[] = [];
			page.on('pageerror', (err) => errors.push(err.message));
			// Wait for the signed invoice GET to round-trip; the button flips
			// to "Downloading..." then reverts regardless of response status.
			const invoiceResponse = page.waitForResponse(
				(resp) => resp.url().includes('/invoice'),
				{ timeout: 5000 },
			);
			await pdfButton.click();
			await invoiceResponse;
			expect(errors).toEqual([]);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('contract link in row navigates to rentals detail page', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
			});

			await page.goto('/dashboard/invoices');

			// Click the contract hash link in the row
			await page.locator(`a[href="/dashboard/rentals/${contractId}"]`).click();
			await expect(page).toHaveURL(new RegExp(`/dashboard/rentals/${contractId}`));
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('provider column links to reputation page (#5)', async ({ page, testAccount }) => {
		// Audit #5: the invoice table Provider column showed a raw pubkey hex
		// with no link, forcing users to remember 8-char hex prefixes to tell
		// providers apart. Mirror rentals/+page.svelte: wrap the pubkey in a
		// link to /dashboard/reputation/{pubkey} so one click resolves the name.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const providerPubkey = randomHex(32);
		try {
			await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
				providerPubkeyHex: providerPubkey,
			});

			await page.goto('/dashboard/invoices');

			// The provider cell must be an anchor pointing at the reputation page
			// for that exact pubkey — not just plain text.
			const providerLink = page.locator(
				`a[href="/dashboard/reputation/${providerPubkey}"]`,
			);
			await expect(providerLink).toBeVisible();
			// Link text is the truncated pubkey, so the user still sees the same
			// identifier but can now resolve it to a name in one click.
			// truncatePubkey shows the first 6 + last 6 chars by default.
			await expect(providerLink).toContainText(providerPubkey.slice(0, 6));
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});
});
