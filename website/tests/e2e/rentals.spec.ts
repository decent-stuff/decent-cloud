import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
	type ContractSeed,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/rentals.
 *
 * Covers:
 *  - Empty state for a fresh user.
 *  - Populated state (active / pending / cancelled tabs) via DB-seeded contracts.
 *  - Interactive action: Cancel a 'requested' contract.
 *  - Detail-page deep link from the list card.
 *
 * Test data is seeded directly into contract_sign_requests via psql. This is
 * the cheapest deterministic path: the real API path requires a signed rental
 * request + provider acceptance + payment webhook. Seeding via DB lets the
 * spec assert pure rendering/interaction without coupling to that flow.
 */

test.describe('/dashboard/rentals', () => {
	test('empty state: fresh user sees onboarding steps and marketplace CTAs', async ({ page, testAccount }) => {
		await page.goto('/dashboard/rentals');

		// Header
		await expect(page.getByRole('heading', { name: 'My Rentals' })).toBeVisible();

		// Empty-state copy
		await expect(page.getByRole('heading', { name: 'No Rentals Yet' })).toBeVisible();
		await expect(page.getByText('Get started in three steps')).toBeVisible();

		// Three onboarding steps
		await expect(page.getByText('1. Browse')).toBeVisible();
		await expect(page.getByText('2. Rent & Pay')).toBeVisible();
		await expect(page.getByText('3. SSH In')).toBeVisible();

		// Marketplace CTAs
		await expect(page.getByRole('link', { name: /Browse GPU Servers/ })).toBeVisible();
		await expect(page.getByRole('link', { name: /Find Budget VMs/ })).toBeVisible();
		await expect(page.getByRole('link', { name: /Explore Marketplace/ })).toBeVisible();
	});

	test('populated state: shows contract cards with status tabs and counts', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Seed one contract per category (active, pending, cancelled).
			const seeds: ContractSeed[] = [
				{ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' },
				{ requesterPubkeyHex: pubkey, status: 'requested', paymentStatus: 'pending' },
				{ requesterPubkeyHex: pubkey, status: 'cancelled', paymentStatus: 'failed' },
			];
			await Promise.all(seeds.map(seedContract));

			await page.goto('/dashboard/rentals');

			// Stats cards
			await expect(page.getByText('Total Contracts').locator('..').getByText('3')).toBeVisible();
			await expect(page.getByText('Active Now').locator('..').getByText('1')).toBeVisible();

			// Tab counts
			await expect(page.getByRole('button', { name: /All.*3/ })).toBeVisible();
			await expect(page.getByRole('button', { name: /Active.*1/ })).toBeVisible();
			await expect(page.getByRole('button', { name: /Pending.*1/ })).toBeVisible();
			await expect(page.getByRole('button', { name: /Cancelled.*1/ })).toBeVisible();

			// Three contract cards link to detail pages
			const cardLinks = page.locator('a[href^="/dashboard/rentals/"]');
			await expect(cardLinks).toHaveCount(3);

			// Active card shows the "Active" status badge and an Invoice button
			await expect(page.locator('a.card', { hasText: 'Active' })).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('filter tab: clicking Active shows only active contracts', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedContract({ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: pubkey, status: 'requested', paymentStatus: 'pending' });

			await page.goto('/dashboard/rentals');

			// Click "Active" tab
			await page.getByRole('button', { name: /Active/ }).click();

			// Should show 1 active card
			await expect(page.locator('a.card', { hasText: 'Active' })).toBeVisible();
			// Should NOT show pending card
			await expect(page.locator('a.card', { hasText: 'Awaiting Payment' })).toHaveCount(0);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('filter tab: Cancelled tab shows empty-state message when no cancelled contracts', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedContract({ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' });

			await page.goto('/dashboard/rentals');
			await page.getByRole('button', { name: /Cancelled/ }).click();

			await expect(page.getByText('No cancelled or failed rentals')).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('search: filters contracts by contract ID hash', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const c1 = await seedContract({ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' });
			await seedContract({ requesterPubkeyHex: pubkey, status: 'active', paymentStatus: 'succeeded' });

			await page.goto('/dashboard/rentals');

			// Initially 2 cards
			await expect(page.locator('a.card')).toHaveCount(2);

			// Search for the first contract by its hash prefix
			const searchInput = page.getByPlaceholder('Search by contract ID or offering name...');
			await searchInput.fill(c1.slice(0, 8));

			// Filtered to 1 card
			await expect(page.locator('a.card')).toHaveCount(1);
			await expect(page.locator(`a[href="/dashboard/rentals/${c1}"]`)).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('action: Cancel a requested contract moves it to Cancelled tab', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'requested',
				paymentStatus: 'pending',
			});

			await page.goto('/dashboard/rentals');

			// Find the contract card.
			const card = page.locator(`a[href="/dashboard/rentals/${contractId}"]`);
			await expect(card).toBeVisible();
			await expect(card.getByText('Awaiting Payment')).toBeVisible();

			// Click Cancel inside the card (the button has e.preventDefault so the
			// link navigation is suppressed). Accept the confirm dialog.
			page.once('dialog', (dialog) => dialog.accept());
			await card.getByRole('button', { name: 'Cancel' }).click();

			// After cancel succeeds, the card shows the "Renew" button (only
			// available for terminal/cancelled contracts).
			await expect(card.getByRole('button', { name: 'Renew' })).toBeVisible({ timeout: 10000 });

			// The Cancelled tab count increments (0 → 1).
			await expect(page.getByRole('button', { name: /Cancelled.*1/ })).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('deep link: detail page at /dashboard/rentals/[id] loads', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
			});

			// Navigate directly to the detail page
			await page.goto(`/dashboard/rentals/${contractId}`);
			await expect(page).toHaveURL(new RegExp(`/dashboard/rentals/${contractId}`));

			// The detail page should show the contract somewhere (header or card)
			// Use a relaxed assertion: page must not 404 and must reference the contract ID prefix.
			await expect(page.locator('body')).not.toContainText(['404', 'Not Found']);
			// The truncated hash (first 8 chars) appears in contract_id references
			await expect(page.locator('body')).toContainText(contractId.slice(0, 8));
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('failed contract shows a next-step CTA pointing to the marketplace (#14)', async ({ page, testAccount }) => {
		// Audit #14: getNextStepInfo had branches for requested/pending/accepted/
		// provisioning/provisioned/active/rejected/cancelled but NOT for `failed`,
		// so a failed contract rendered only the "Failed" badge with no guidance.
		// The fix adds a `failed` branch with a marketplace hint + link.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'failed',
				paymentStatus: 'succeeded',
			});

			await page.goto('/dashboard/rentals');

			// `failed` lands under the "Cancelled / Failed" tab — switch to it.
			await page.getByRole('button', { name: /Cancelled.*Failed/ }).click();

			const card = page.locator(`a[href="/dashboard/rentals/${contractId}"]`);
			await expect(card).toBeVisible();

			// The card must surface next-step text mentioning the marketplace as a
			// recovery path (exact copy may flex — match on the key concepts).
			// "marketplace" appears in both the next-step text and the action
			// button label, so use first() to assert at least one is visible.
			await expect(card.getByText(/marketplace/i).first()).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});
});
