import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	deleteSavedOfferingsForUser,
	seedOffering,
	deleteOfferingsByProvider,
	randomHex,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering-detail save (bookmark) flow.
 *
 * Other specs cover the saved-offerings page (read/unsave/bulk). This spec
 * pins the *write* path: a user lands on an offering's detail page and saves
 * it with a single click on the visible bookmark toggle (previously the only
 * path was 'More options' → 'Save', a 2-click flow).
 *
 * Self-contained: the offering is seeded under a fresh provider pubkey via
 * seedOffering (not the demo seed_data.sql row) so the test never depends on
 * externally-maintained seed rows drifting.
 */

const OFFERING_NAME = 'E2E Bookmark Offering';
const PROVIDER_PUBKEY = randomHex(32);
let offeringId: string;

test.beforeAll(async () => {
	offeringId = await seedOffering(PROVIDER_PUBKEY, { name: OFFERING_NAME, offeringSource: 'self_provisioned' });
});

test.afterAll(async () => {
	await deleteOfferingsByProvider(PROVIDER_PUBKEY);
});

test.describe('Offering detail save flow', () => {
	test('bookmark toggle on offering detail page saves in a single click', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Detail route loads the offering directly regardless of marketplace filter state.
			await page.goto(`/dashboard/marketplace/${offeringId}`);

			// The bookmark toggle is visible alongside the Rent button.
			const bookmark = page.getByRole('button', { name: new RegExp(`Save ${OFFERING_NAME}`, 'i') });
			await expect(bookmark).toBeVisible();
			await expect(bookmark).toHaveAttribute('aria-pressed', 'false');

			// Single click saves. The aria-label flips to 'Remove … from saved'
			// and the button reflects the pressed state.
			await bookmark.click();
			const savedToggle = page.getByRole('button', { name: new RegExp(`Remove ${OFFERING_NAME} from saved`, 'i') });
			await expect(savedToggle).toBeVisible();
			await expect(savedToggle).toHaveAttribute('aria-pressed', 'true');

			// Saved listing page reflects the new save.
			await page.goto('/dashboard/saved');
			await expect(page.getByRole('link', { name: OFFERING_NAME })).toBeVisible();
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});

	test('@smoke breadcrumb root crumb matches its destination', async ({ page }) => {
		// The breadcrumb root on the offering detail page links to
		// /dashboard/rentals but was labeled "Dashboard" — a mismatch. The
		// label must read "My Rentals" so it matches where it goes.
		await page.goto(`/dashboard/marketplace/${offeringId}`);

		const breadcrumb = page.getByRole('main').locator('nav[aria-label="Breadcrumb"]');
		const rootCrumb = breadcrumb.getByRole('link', { name: 'My Rentals' });
		await expect(rootCrumb).toBeVisible({ timeout: 10000 });
		await expect(rootCrumb).toHaveAttribute('href', '/dashboard/rentals');
	});
});
