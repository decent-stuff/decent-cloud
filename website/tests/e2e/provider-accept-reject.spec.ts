import { test, expect, waitForAuthReady } from './fixtures/test-account';
import type { Page } from '@playwright/test';
import {
	pubkeyHexFromSeed,
	randomHex,
	nowNs,
	seedOffering,
	seedContract,
	deleteContractsForRequester,
	deleteOfferingsByProvider,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the AUTHENTICATED provider accept/reject contract flow.
 *
 * Before this spec, /dashboard/provider/requests was only exercised for
 * ANONYMOUS users — provider-batch-actions.spec.ts asserts the accept/reject
 * buttons are absent when logged out. The real authenticated decision flow
 * (provider sees a tenant's request, then accepts or rejects it via a signed
 * POST .../respond) had NO E2E coverage (FLOWS.md gap #1, row "Accept / reject
 * a request").
 *
 * Seeding model: the testAccount user is the PROVIDER (its fixed seed → fixed
 * pubkey = provider_pubkey on the seeded contracts); a random pubkey plays the
 * tenant/requester. Contracts are inserted directly into
 * contract_sign_requests with status='requested' and payment_status='succeeded'
 * so they (a) land in the pending list (status IN requested/pending) and (b)
 * satisfy the accept payment gate. No first-party API is mocked.
 *
 * Serial mode is mandatory: every testAccount user shares the same provider
 * pubkey, and these tests seed/delete rows keyed on it — parallel workers would
 * race on the same provider's pending list and nuke each other's contracts
 * (same hazard as invoices.spec.ts / rent-flow.spec.ts).
 */

test.describe('Provider accept/reject contract requests (authenticated)', () => {
	test.describe.configure({ mode: 'serial' });

	let providerPubkey = '';
	const offeringId = `provacc-${randomHex(4)}`;
	// Two distinct tenants so each test targets exactly one contract by DB key.
	// Seeded in an order that makes the accept-target appear first (see below).
	const requesterReject = randomHex(32);
	const requesterAccept = randomHex(32);
	const requesters = [requesterAccept, requesterReject];

	test.beforeAll(async ({ testAccount }) => {
		providerPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);

		// Fresh slate for this worker's contracts + offering.
		for (const r of requesters) await deleteContractsForRequester(r);
		await deleteOfferingsByProvider(providerPubkey);

		// Offering owned by the provider so each card renders an offering name +
		// the provider's offering list (used by the filter dropdown).
		await seedOffering(providerPubkey, {
			offeringId,
			name: 'E2E Provider Accept/Reject',
		});

		// The auto-accept toggle PUTs an UPDATE on provider_profiles, which
		// 0-rows-affects ("Provider profile not found") for a fresh account. Seed
		// a minimal row with auto_accept_rentals=false so the toggle starts in the
		// "Enable" state the toggle test clicks. (Column defaults to TRUE, so the
		// explicit false is load-bearing.)
		await sql(`
			INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals)
			VALUES (decode('${providerPubkey}', 'hex'), 'E2E Provider', 'v1', '1', ${nowNs()}, false)
			ON CONFLICT (pubkey) DO UPDATE SET auto_accept_rentals = false
		`);

		// Seed the REJECT-target first, then (after a tick so created_at_ns
		// strictly increases) the ACCEPT-target. The pending query orders by
		// created_at_ns DESC, so the accept-target card renders FIRST — letting
		// the accept test use .first() deterministically.
		await seedContract({
			requesterPubkeyHex: requesterReject,
			providerPubkeyHex: providerPubkey,
			status: 'requested',
			paymentStatus: 'succeeded',
			offeringId,
		});
		await new Promise((r) => setTimeout(r, 10));
		await seedContract({
			requesterPubkeyHex: requesterAccept,
			providerPubkeyHex: providerPubkey,
			status: 'requested',
			paymentStatus: 'succeeded',
			offeringId,
		});
	});

	test.afterAll(async () => {
		for (const r of requesters) {
			try {
				await deleteContractsForRequester(r);
			} catch {
				/* best-effort cleanup */
			}
		}
		if (providerPubkey) {
			try {
				await deleteOfferingsByProvider(providerPubkey);
			} catch {
				/* best-effort */
			}
			try {
				// provider_profiles_contacts cascades on provider_profiles delete.
				await sql(
					`DELETE FROM provider_profiles WHERE pubkey = decode('${providerPubkey}', 'hex')`,
				);
			} catch {
				/* best-effort */
			}
		}
	});

	/** Navigate to the provider requests page and wait for auth to settle. */
	async function openRequests(page: Page): Promise<void> {
		await page.goto('/dashboard/provider/requests');
		await waitForAuthReady(page);
	}

	/** Match the signed POST .../rental-requests/:id/respond the Accept/Reject
	 * buttons trigger. */
	function waitForRespond(page: Page) {
		return page.waitForResponse(
			(resp) =>
				resp.request().method() === 'POST' &&
				resp.url().includes('/api/v1/provider/rental-requests/') &&
				resp.url().includes('/respond'),
			{ timeout: 20000 },
		);
	}

	test('provider sees pending requests from tenants', async ({ page }) => {
		await openRequests(page);

		await expect(
			page.getByRole('heading', { name: 'Pending Requests' }),
		).toBeVisible({ timeout: 15000 });

		// Two seeded requests → two cards. Match the per-card buttons with
		// exact:true so the batch "Accept All" / "Reject All" controls (which
		// appear when >1 request is pending) are not counted.
		await expect(
			page.getByRole('button', { name: 'Accept', exact: true }),
		).toHaveCount(2, { timeout: 15000 });
		await expect(
			page.getByRole('button', { name: 'Reject', exact: true }),
		).toHaveCount(2);

		// The seeded offering_id renders as each card's heading.
		await expect(
			page.getByRole('heading', { name: offeringId }),
		).toHaveCount(2);
	});

	test('accept a contract request removes it from pending', async ({ page }) => {
		await openRequests(page);
		await expect(
			page.getByRole('button', { name: 'Accept', exact: true }),
		).toHaveCount(2, { timeout: 15000 });

		const respond = waitForRespond(page);
		// The accept-target was seeded last → highest created_at_ns → first card.
		await page.getByRole('button', { name: 'Accept', exact: true }).first().click();
		const res = await respond;
		expect(res.ok()).toBeTruthy();

		// Success banner + one card removed (the accepted contract moved to
		// 'accepted', which is not in the pending set).
		await expect(page.getByText('Request accepted', { exact: true })).toBeVisible({
			timeout: 10000,
		});
		await expect(
			page.getByRole('button', { name: 'Accept', exact: true }),
		).toHaveCount(1);

		// DB reflects the transition for the targeted requester.
		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterAccept}', 'hex')`,
		);
		expect(status).toBe('accepted');
	});

	test('reject a contract request removes it from pending', async ({ page }) => {
		await openRequests(page);
		// Only the reject-target remains pending.
		await expect(
			page.getByRole('button', { name: 'Reject', exact: true }),
		).toHaveCount(1, { timeout: 15000 });

		const respond = waitForRespond(page);
		await page.getByRole('button', { name: 'Reject', exact: true }).first().click();
		const res = await respond;
		expect(res.ok()).toBeTruthy();

		await expect(page.getByText('Request rejected', { exact: true })).toBeVisible({
			timeout: 10000,
		});
		// No pending cards remain at all.
		await expect(
			page.getByRole('button', { name: 'Accept', exact: true }),
		).toHaveCount(0);
		await expect(
			page.getByRole('button', { name: 'Reject', exact: true }),
		).toHaveCount(0);

		const status = await sql(
			`SELECT status FROM contract_sign_requests WHERE requester_pubkey = decode('${requesterReject}', 'hex')`,
		);
		expect(status).toBe('rejected');
	});

	test('auto-accept toggle can be enabled', async ({ page }) => {
		await openRequests(page);

		// Fresh account → auto-accept is off; the toggle offers to enable it.
		const enableToggle = page.getByRole('button', {
			name: 'Enable auto-accept rentals',
		});
		await expect(enableToggle).toBeVisible({ timeout: 15000 });

		// The toggle PUTs /provider/settings/auto-accept (page-load reads are
		// GETs, so matching PUT disambiguates the toggle action).
		const update = page.waitForResponse(
			(resp) =>
				resp.request().method() === 'PUT' &&
				resp.url().includes('/api/v1/provider/settings/auto-accept'),
			{ timeout: 20000 },
		);
		await enableToggle.click();
		await update;

		// Toggle flipped on: its aria-label inverts and the success banner shows.
		await expect(
			page.getByRole('button', { name: 'Disable auto-accept rentals' }),
		).toBeVisible({ timeout: 10000 });
		await expect(
			page.getByText('Auto-accept enabled', { exact: false }),
		).toBeVisible();
	});
});
