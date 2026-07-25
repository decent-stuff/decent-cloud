import { test, expect } from './fixtures/test-account';
import {
	seedOffering,
	deleteOfferingsByProvider,
	randomHex,
} from './fixtures/seed-helpers';

/**
 * #5 (net-new UX): client-side ownership guard on the offering-edit route.
 *
 * /dashboard/offerings/[id]/edit previously rendered a full pre-filled edit
 * form + Save button to ANY authenticated user, even one who does not own the
 * offering. Save is server-rejected (the PUT is provider-scoped via
 * check_authorization on :pubkey), so this is NOT a security hole — but the
 * misleading UX let a non-owner fill in the whole form only to be denied, and
 * it bypassed the "Provider Setup Required" gate that the offerings list enforces.
 *
 * This spec seeds an offering under a RANDOM provider pubkey (identity A) and,
 * authenticated as the testAccount (identity B — a different pubkey), opens the
 * edit route. It must see the no-permission state and NOT the editable form or
 * the Save button.
 */
test.describe('offering-edit ownership guard', () => {
	test.describe.configure({ mode: 'serial' });

	test('blocks a non-owner from the editable form (no Save button) @smoke', async ({ page }) => {
		// Identity A: a random provider that is NOT the signed-in testAccount.
		const ownerPubkey = randomHex(32);
		const offeringId = await seedOffering(ownerPubkey, {
			name: 'Owned By Someone Else',
			offeringSource: 'self_provisioned',
		});
		try {
			await page.goto(`/dashboard/offerings/${offeringId}/edit`);

			// The no-permission heading is the deterministic "blocked" signal.
			await expect(
				page.getByRole('heading', { name: "You can't edit this offering" }),
			).toBeVisible({ timeout: 10_000 });

			// The editable form must NOT render for a non-owner.
			await expect(page.locator('#offer-name')).toHaveCount(0);
			// No Save button — the guard is effective, not just cosmetic.
			await expect(page.getByRole('button', { name: 'Save Changes' })).toHaveCount(0);

			// The view-only escape hatch is offered.
			await expect(page.getByRole('link', { name: 'View offering' })).toBeVisible();
		} finally {
			await deleteOfferingsByProvider(ownerPubkey);
		}
	});
});
