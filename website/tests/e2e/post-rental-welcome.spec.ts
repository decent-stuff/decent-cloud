import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedContract,
	deleteContractsForRequester,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the post-rental welcome banner on the contract detail page.
 *
 * After a successful rental, the marketplace redirects to
 * `/dashboard/rentals/{contractId}?welcome=true`, which shows a celebratory
 * banner with next-step guidance (rentals/[contract_id]/+page.svelte:889).
 * The banner is dismissable (localStorage key per contract id) and only
 * renders when the `welcome` query param is present AND a contract loaded.
 *
 * We seed a contract directly for the test user — a full rental requires real
 * payment (ICPay/Stripe) and is out of scope for the e2e harness — then
 * navigate to its detail page to exercise the banner logic against the real UI.
 *
 * The separate "checkout success redirect" path is NOT covered here: it would
 * require mocking the first-party /contracts/verify-checkout endpoint (a
 * mock-policy violation), and the redirect-URL construction is already covered
 * by the unit test at src/routes/checkout/success/page.test.ts.
 */

test.describe('Post-rental welcome banner', () => {
	// Serial mode: all tests share the testAccount pubkey and clean up via
	// deleteContractsForRequester(pubkey). Parallel execution would let one
	// worker's afterAll nuke another's seeded contracts.
	test.describe.configure({ mode: 'serial' });

	let pubkey: string;

	test.beforeAll(async ({ testAccount }) => {
		pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
	});

	test('shows the welcome banner when arriving with ?welcome=true', async ({ page }) => {
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'active',
			paymentStatus: 'succeeded',
		});

		await page.goto(`/dashboard/rentals/${contractId}?welcome=true`);

		const banner = page.getByTestId('welcome-banner');
		await expect(banner).toBeVisible();
		await expect(banner.getByText('Rental request submitted!')).toBeVisible();
		await expect(banner.getByText("Here's what to expect next:")).toBeVisible();
		await expect(
			banner.getByText('The provider will review your request'),
		).toBeVisible();
		await expect(banner.getByRole('link', { name: 'View All Rentals' })).toBeVisible();
	});

	test('banner is dismissable and clears the welcome param', async ({ page }) => {
		// Fresh contract so the per-contractId localStorage dismissal state from
		// the previous test does not leak.
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'active',
			paymentStatus: 'succeeded',
		});

		await page.goto(`/dashboard/rentals/${contractId}?welcome=true`);

		const banner = page.getByTestId('welcome-banner');
		await expect(banner).toBeVisible();

		await banner.getByRole('button', { name: 'Dismiss' }).click();

		// Banner hidden and the welcome param stripped from the URL.
		await expect(banner).not.toBeVisible();
		await expect(page).toHaveURL(new RegExp(`/dashboard/rentals/${contractId}$`));
	});

	test('no banner on a regular visit without ?welcome', async ({ page }) => {
		const contractId = await seedContract({
			requesterPubkeyHex: pubkey,
			status: 'active',
			paymentStatus: 'succeeded',
		});

		await page.goto(`/dashboard/rentals/${contractId}`);

		await expect(page.getByTestId('welcome-banner')).not.toBeVisible();
	});

	test.afterAll(async () => {
		await deleteContractsForRequester(pubkey);
	});
});
