import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import {
	seedRentableOffering,
	deleteOfferingsByProvider,
	verifyAccountEmail,
	pubkeyHexFromSeed,
} from './fixtures/seed-helpers';

/**
 * F3: email verification is a hard backend prerequisite for creating a rental,
 * but the rent entry points used to give no warning — the user only discovered
 * the wall after choosing an offering and submitting. Now the offering-detail
 * "Rent this offering" button relabels to "Verify email to rent" for an
 * unverified user, and the rental dialog gates Submit. This spec pins both.
 *
 * Serial mode: the three tests share the worker-scoped testAccount pubkey and
 * the middle test flips its email_verified flag DB-side, so they must run in
 * order (same shared-pubkey hazard pattern as rent-flow.spec.ts).
 */
test.describe.configure({ mode: 'serial' });
test.describe('Email verification gate on the rent flow (F3)', () => {
	let seeded: { providerPubkeyHex: string; offeringNumericId: string; offeringName: string };

	test.beforeAll(async () => {
		seeded = await seedRentableOffering({ name: 'F3 Gate Rentable' });
	});
	test.afterAll(async () => {
		await deleteOfferingsByProvider(seeded.providerPubkeyHex);
	});

	test('offering detail shows "Verify email to rent" for an unverified user', async ({ page }) => {
		setupConsoleLogging(page);
		// testAccount starts unverified (seedAccountDirect leaves email_verified=false).
		await page.goto(`/dashboard/marketplace/${seeded.offeringNumericId}`);
		await page.waitForSelector('h1', { timeout: 10000 });

		// Both Rent CTAs (header + main) relabel to surface the prerequisite.
		const gated = page.getByRole('button', { name: 'Verify email to rent' });
		await expect(gated.first()).toBeVisible({ timeout: 5000 });
		// The normal label must NOT appear while unverified.
		await expect(page.getByRole('button', { name: 'Rent this offering' })).toHaveCount(0);
	});

	test('rental dialog gates Submit and explains why for an unverified user', async ({ page }) => {
		setupConsoleLogging(page);
		// Still unverified (the verify flip happens in the next test). Open the
		// rent dialog from the marketplace card — the shared choke point.
		await page.goto('/dashboard/marketplace');
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		const enabledRent = page.getByRole('button', { name: 'Rent', exact: true }).first();
		await enabledRent.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({ timeout: 5000 });

		// The dialog surfaces the prerequisite inline and locks Submit ("Pay now"
		// for a priced offering, "Submit Request" otherwise — accept either).
		await expect(page.getByRole('heading', { name: 'Email verification required' })).toBeVisible();
		const submit = page.getByRole('button', { name: /^(Pay now|Submit Request)$/ });
		await expect(submit).toBeDisabled();
	});

	test('offering detail shows "Rent this offering" once the user is verified', async ({ page, testAccount }) => {
		setupConsoleLogging(page);
		// Flip email_verified DB-side (mirrors rent-flow.spec.ts setup), then load
		// the detail page so the auth store re-fetches the account.
		await verifyAccountEmail(pubkeyHexFromSeed(testAccount.seedPhrase));

		await page.goto(`/dashboard/marketplace/${seeded.offeringNumericId}`);
		await page.waitForSelector('h1', { timeout: 10000 });

		await expect(page.getByRole('button', { name: 'Rent this offering' }).first()).toBeVisible({ timeout: 5000 });
		await expect(page.getByRole('button', { name: 'Verify email to rent' })).toHaveCount(0);
	});
});
