import { test, expect } from './fixtures/test-account';
import { confirmInlineAction } from './fixtures/auth-helpers';
import {
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
	sql,
	randomHex,
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
	// Serial mode: tests seed/delete contracts for a shared pubkey (all testAccount
	// users derive the same key). Parallel cleanup would nuke sibling tests' data.
	test.describe.configure({ mode: 'serial' });

	test('@smoke empty state: fresh user sees onboarding steps and marketplace CTAs', async ({ page, testAccount }) => {
		// The empty state renders only after onMount's signed GET to
		// /users/{pubkey}/contracts resolves (loadContracts flips `loading`).
		// Under parallel workers that signed fetch can exceed the default 5s
		// toBeVisible timeout, so wait for it deterministically BEFORE asserting.
		// Listener is armed before goto so the response is never missed.
		const contractsLoaded = page.waitForResponse(
			(r) => /\/api\/v1\/users\/[^/]+\/contracts$/.test(r.url()) && r.status() < 400,
			{ timeout: 15000 },
		);
		await page.goto('/dashboard/rentals');
		await contractsLoaded;

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

		// ICPay is fully retired — Stripe is the sole payment rail, so the empty
		// state must NOT advertise "ICP" as a payment option. Scoped to the
		// onboarding steps region to avoid false positives from unrelated text.
		const onboarding = page.locator('.bg-surface-elevated').filter({ hasText: 'Rent' });
		await expect(onboarding).not.toContainText('ICP');
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

			// Two-step inline confirm: first Cancel arms (the button has
			// e.preventDefault so the link navigation is suppressed), Confirm +
			// Abort appear, then Confirm performs the cancellation.
			await confirmInlineAction(page, card, { arm: 'Cancel', secondary: 'Abort' });

			// After cancel succeeds, the card shows the "Renew" button (only
			// available for terminal/cancelled contracts).
			await expect(card.getByRole('button', { name: 'Renew' })).toBeVisible({ timeout: 10000 });

			// The Cancelled tab count increments (0 → 1).
			await expect(page.getByRole('button', { name: /Cancelled.*1/ })).toBeVisible();
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('action: Abort hides the inline cancel confirm and keeps the contract', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'requested',
				paymentStatus: 'pending',
			});

			await page.goto('/dashboard/rentals');
			const card = page.locator(`a[href="/dashboard/rentals/${contractId}"]`);
			await expect(card).toBeVisible();

			// Arm the inline confirm, then abort it.
			await card.getByRole('button', { name: 'Cancel' }).click();
			await card.getByRole('button', { name: 'Abort' }).click();

			// Confirm/Abort disappear; the plain Cancel button returns and the
			// contract is unchanged (still cancellable).
			await expect(card.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(card.getByRole('button', { name: 'Cancel' })).toBeVisible();
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

	test('pending-gateway state shows ETA copy and inline Refresh button (audit #10)', async ({ page, testAccount }) => {
		// Audit #10: when a contract has provisioning_instance_details but no
		// gateway_subdomain/gateway_ssh_port yet, the card showed only
		// "Gateway routing is being configured. Connection details will appear
		// shortly." with no ETA and no way to refresh without a full page reload.
		// Fix adds a "typically 1–3 minutes" hint and an inline Refresh button.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
			});
			// Set provisioning_instance_details and explicitly NULL the gateway
			// fields so the "pending gateway" branch is the only one that fires
			// inside the Connection Details card.
			await sql(
				`UPDATE contract_sign_requests SET provisioning_instance_details = '{"ip_address":"10.0.0.1"}', gateway_subdomain = NULL, gateway_ssh_port = NULL WHERE contract_id = decode('${contractId}', 'hex')`
			);

			await page.goto('/dashboard/rentals');

			const card = page.locator(`a[href="/dashboard/rentals/${contractId}"]`);
			await expect(card).toBeVisible();

		// The pending-gateway hint must surface an ETA hint.
		await expect(card.getByText(/gateway routing is being configured/i)).toBeVisible();
		await expect(card.getByText(/1.{0,3}3 minutes|typically/i)).toBeVisible();
		// And an inline Refresh button must exist on the card.
		await expect(card.getByRole('button', { name: /refresh/i })).toBeVisible();
	} finally {
		await deleteContractsForRequester(pubkey);
	}
});

	test('detail: provider name links to provider profile page (matches marketplace, not reputation)', async ({ page, testAccount }) => {
		// The provider name on the rental detail page must jump to the SAME
		// provider profile destination the marketplace uses
		// (/dashboard/providers/{username || pubkey}), not the reputation page.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const providerPubkey = randomHex(32);
		try {
			const contractId = await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
				providerPubkeyHex: providerPubkey,
			});

			await page.goto(`/dashboard/rentals/${contractId}`);

			// Seeded provider has no account → provider_username is undefined,
			// so the href resolves to the raw pubkey (the marketplace fallback
			// branch of `owner_username || pubkey`).
			await expect(
				page.locator(`a[href="/dashboard/providers/${providerPubkey}"]`),
			).toBeVisible();
			// And it must NOT point at the legacy reputation route.
			await expect(
				page.locator(`a[href="/dashboard/reputation/${providerPubkey}"]`),
			).toHaveCount(0);
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});

	test('list: provider name on a card navigates to provider profile page', async ({ page, testAccount }) => {
		// The provider name on a rentals list card must navigate to the provider
		// profile page (/dashboard/providers/...), matching the marketplace link
		// destination — not the reputation page.
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const providerPubkey = randomHex(32);
		try {
			await seedContract({
				requesterPubkeyHex: pubkey,
				status: 'active',
				paymentStatus: 'succeeded',
				providerPubkeyHex: providerPubkey,
			});

			await page.goto('/dashboard/rentals');
			const card = page.locator('a[href^="/dashboard/rentals/"]').first();
			await expect(card).toBeVisible();

			// The provider name renders as the truncated pubkey (no username on
			// the seeded provider): first6...last6.
			const truncated = `${providerPubkey.slice(0, 6)}...${providerPubkey.slice(-6)}`;
			// Clicking the provider name navigates to the provider profile page.
			const nav = page.waitForURL(`**/dashboard/providers/${providerPubkey}`, { timeout: 10_000 });
			await card.getByRole('button', { name: truncated }).click();
			await nav;
		} finally {
			await deleteContractsForRequester(pubkey);
		}
	});
});
