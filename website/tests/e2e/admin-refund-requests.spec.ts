import { test, expect } from './fixtures/test-admin-account';
import {
	identityFromSeedPhrase,
	signedApiCall,
	seedAccountDirect,
	seedContract,
	seedRefundRequest,
	deleteAccountByUsername,
	deleteContractsForRequester,
	pubkeyHexFromSeed,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the refund approval gate admin panel (FLOWS.md).
 *
 * Verifies the admin panel renders refund requests and the approve/decline
 * flow works end-to-end through the REAL api + admin UI. Refund requests are
 * seeded directly into the `refund_requests` table (the same state
 * `process_gated_refund` produces), because the warm stack has a real
 * STRIPE_SECRET_KEY configured — driving a cancel through the API would hit
 * Stripe with a fake payment_intent and fail before the gate runs.
 *
 * The cancel→gate→refund paths (auto-issue, cap-exceeded hold) are covered by
 * the Rust integration tests in `api/src/database/contracts/tests.rs` with
 * `stripe_client=None`. This spec adds the UI + admin API layer on top.
 *
 * The admin CALLER is the `adminAccount` fixture (DB-granted is_admin). Each
 * test seeds its own throwaway TARGET user whose pubkey owns the seeded
 * refund_request rows.
 */
test.describe('Admin refund requests panel (/dashboard/admin)', () => {
	test.describe.configure({ mode: 'serial' });

	/**
	 * Test 1: The admin API returns seeded refund requests. This verifies the
	 * GET /admin/refund-requests endpoint and its filtering/pagination,
	 * independent of the UI layer.
	 */
	test('admin API lists pending refund requests with correct fields', async ({ adminAccount }) => {
		const admin = identityFromSeedPhrase(adminAccount.seedPhrase);
		const target = await seedAccountDirect();
		const targetPubkeyHex = pubkeyHexFromSeed(target.seedPhrase);

		try {
			// Need a contract row for the FK-adjacent reference (refund_requests
			// has no FK but the admin UI shows the contract hex).
			const contractId = await seedContract({
				requesterPubkeyHex: targetPubkeyHex,
				status: 'cancelled',
				paymentMethod: 'stripe',
				paymentStatus: 'succeeded',
				paymentAmountE9s: 2_000_000_000,
				stripePaymentIntentId: 'pi_test_list',
			});

			const refundId = await seedRefundRequest({
				contractIdHex: contractId,
				requesterPubkeyHex: targetPubkeyHex,
				refundAmountE9s: 2_000_000_000,
				reason: 'cancel',
				status: 'pending',
				userLatestPaymentE9s: 10_000_000,
				capExceeded: true,
				paymentIntentId: 'pi_test_list',
			});

			// List pending requests.
			const listRes = await signedApiCall(
				admin,
				'GET',
				'/api/v1/admin/refund-requests?status=pending&limit=50',
			);
			expect(listRes.status).toBe(200);
			const body = await listRes.json();
			expect(body.success).toBe(true);
			expect(body.data.total).toBeGreaterThanOrEqual(1);

			const found = body.data.requests.find(
				(r: { contractId: string }) => r.contractId === contractId,
			);
			expect(found).toBeTruthy();
			expect(found.id).toBe(Number(refundId));
			expect(found.status).toBe('pending');
			expect(found.reason).toBe('cancel');
			expect(found.capExceeded).toBe(true);
			expect(found.refundAmountE9s).toBe(2_000_000_000);
			expect(found.userLatestPaymentE9s).toBe(10_000_000);
			expect(found.currency).toBe('USD');
		} finally {
			await deleteContractsForRequester(targetPubkeyHex);
			await deleteAccountByUsername(target.username);
		}
	});

	/**
	 * Test 2: The admin UI shows pending refund requests, and the admin can
	 * decline one through the panel (decline does NOT call Stripe, so it works
	 * fully end-to-end on the warm stack without a mock).
	 *
	 * This is the flow the user asked to be "verified e2e that it shows up and
	 * works": pending request appears in the table → admin clicks Decline →
	 * confirms → request leaves the pending list.
	 */
	test('admin UI shows pending request and decline works end-to-end', async ({ adminAccount, page }) => {
		const target = await seedAccountDirect();
		const targetPubkeyHex = pubkeyHexFromSeed(target.seedPhrase);
		const contractId = await seedContract({
			requesterPubkeyHex: targetPubkeyHex,
			status: 'cancelled',
			paymentMethod: 'stripe',
			paymentStatus: 'succeeded',
			paymentAmountE9s: 5_000_000_000,
			stripePaymentIntentId: 'pi_test_decline',
		});

		try {
			await seedRefundRequest({
				contractIdHex: contractId,
				requesterPubkeyHex: targetPubkeyHex,
				refundAmountE9s: 5_000_000_000,
				reason: 'cancel',
				status: 'pending',
				userLatestPaymentE9s: 10_000_000,
				capExceeded: true,
				paymentIntentId: 'pi_test_decline',
			});

			await page.goto('/dashboard/admin');

			// The "Refund Requests" heading must be visible (section rendered).
			await expect(page.getByRole('heading', { name: 'Refund Requests' })).toBeVisible({
				timeout: 15000,
			});

			// Ensure the status filter is on "pending".
			await page.locator('#refund-status-filter').selectOption('pending');

			// The pending request for our contract must appear in the table.
			// Contract IDs are truncated in the UI (first 10 + last 6 hex chars).
			const contractPrefix = contractId.slice(0, 10);
			const row = page.locator('tr', { hasText: contractPrefix });
			await expect(row).toBeVisible({ timeout: 10000 });
			await expect(row).toContainText('Pending');
			await expect(row).toContainText('Exceeded');
			await expect(row).toContainText('cancel');

			// Click "Decline" on that row, then confirm.
			await row.getByRole('button', { name: 'Decline' }).click();

			// The inline review panel must appear — target it by its unique
			// warning text (scoped to avoid matching ancestor containers).
			const reviewText = page.getByText('Decline refund for');
			await expect(reviewText).toBeVisible({ timeout: 5000 });

			// Enter a note and confirm.
			await page.locator('textarea').fill('E2E test decline');
			await page.getByRole('button', { name: /Confirm decline/ }).click();

			// After declining the pending list refreshes; our row must be gone.
			await expect(row).not.toBeVisible({ timeout: 10000 });

			// Verify via API that it moved to 'declined'.
			const admin = identityFromSeedPhrase(adminAccount.seedPhrase);
			const declinedRes = await signedApiCall(
				admin,
				'GET',
				'/api/v1/admin/refund-requests?status=declined&limit=50',
			);
			expect(declinedRes.status).toBe(200);
			const declinedBody = await declinedRes.json();
			const declined = declinedBody.data.requests.find(
				(r: { contractId: string }) => r.contractId === contractId,
			);
			expect(declined).toBeTruthy();
			expect(declined.status).toBe('declined');
			expect(declined.reviewNote).toBe('E2E test decline');
		} finally {
			await deleteContractsForRequester(targetPubkeyHex);
			await deleteAccountByUsername(target.username);
		}
	});

	/**
	 * Test 3: The admin UI status filter works — switching to "auto_issued"
	 * shows requests that the gate auto-issued (cap passed), and they have no
	 * Approve/Decline buttons (only pending requests have action buttons).
	 */
	test('admin UI status filter shows auto_issued without action buttons', async ({
		adminAccount,
		page,
	}) => {
		const target = await seedAccountDirect();
		const targetPubkeyHex = pubkeyHexFromSeed(target.seedPhrase);
		const contractId = await seedContract({
			requesterPubkeyHex: targetPubkeyHex,
			status: 'cancelled',
			paymentMethod: 'stripe',
			paymentStatus: 'succeeded',
			paymentAmountE9s: 1_000_000_000,
			stripePaymentIntentId: 'pi_test_auto',
		});

		try {
			await seedRefundRequest({
				contractIdHex: contractId,
				requesterPubkeyHex: targetPubkeyHex,
				refundAmountE9s: 1_000_000_000,
				reason: 'cancel',
				status: 'auto_issued',
				userLatestPaymentE9s: 1_000_000_000,
				capExceeded: false,
				paymentIntentId: 'pi_test_auto',
			});

			await page.goto('/dashboard/admin');
			await expect(page.getByRole('heading', { name: 'Refund Requests' })).toBeVisible({
				timeout: 15000,
			});

			// Switch filter to auto_issued.
			await page.locator('#refund-status-filter').selectOption('auto_issued');

			const contractPrefix = contractId.slice(0, 10);
			const row = page.locator('tr', { hasText: contractPrefix });
			await expect(row).toBeVisible({ timeout: 10000 });
			await expect(row).toContainText('Auto');
			await expect(row).toContainText('OK'); // cap NOT exceeded

			// Auto-issued requests must NOT have Approve/Decline buttons.
			await expect(row.getByRole('button', { name: 'Approve' })).toHaveCount(0);
			await expect(row.getByRole('button', { name: 'Decline' })).toHaveCount(0);
		} finally {
			await deleteContractsForRequester(targetPubkeyHex);
			await deleteAccountByUsername(target.username);
		}
	});
});
