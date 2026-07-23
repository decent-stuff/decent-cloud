import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	seedOffering,
	deleteOfferingsByProvider,
	sql,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering EDIT flow (/dashboard/offerings/[id]/edit).
 *
 * Previously zero coverage — this is a primary provider action. The page loads
 * an existing offering via getOffering, pre-fills an editable form, renders a
 * live "Changes Since Last Save" diff (buildOfferingDraftDiff), and on submit
 * signs a PUT to /providers/:pubkey/offerings/:id then redirects to the list.
 *
 * Because editing requires the offering's owner pubkey to match the signer,
 * each test seeds its offering directly under the testAccount's pubkey (derived
 * from the fixed seed phrase). Serial mode + per-test finally cleanup prevents
 * parallel workers' deleteOfferingsByProvider(pubkey) from nuking each other's
 * rows (same convention as invoices.spec.ts).
 */

const SEEDED_NAME = 'E2E Edit Target';

test.describe('Offering edit flow (/dashboard/offerings/[id]/edit)', () => {
	test.describe.configure({ mode: 'serial' });

	// Helper: navigate to the edit page and wait until the form is populated.
	// The #offer-name input only renders once loading=false and `existing` is set,
	// so its visibility + value IS the "offering loaded" signal — no networkidle.
	async function openEditAndWait(page: import('@playwright/test').Page, offeringId: string) {
		await page.goto(`/dashboard/offerings/${offeringId}/edit`);
		await waitForAuthReady(page);
		const offerNameInput = page.locator('#offer-name');
		await expect(offerNameInput).toBeVisible({ timeout: 10_000 });
		return offerNameInput;
	}

	test('loads existing offering and pre-fills form fields', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringId = await seedOffering(pubkey, {
			name: SEEDED_NAME,
			offeringSource: 'self_provisioned',
		});
		try {
			const offerNameInput = await openEditAndWait(page, offeringId);

			await expect(page.getByRole('heading', { name: 'Edit Offering' })).toBeVisible();
			await expect(offerNameInput).toHaveValue(SEEDED_NAME);
			// Seeded monthly_price is 25.0 -> rendered as "25" by a number input.
			await expect(page.locator('#monthly-price')).toHaveValue('25');
			// The disabled Offering ID input shows the seeded offering_id.
			await expect(page.locator('#offering-id')).not.toBeEmpty();
			// On a clean load the diff panel is empty.
			await expect(page.getByText('No unsaved changes yet.')).toBeVisible();
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});

	test('diff panel updates as fields are edited', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringId = await seedOffering(pubkey, {
			name: SEEDED_NAME,
			offeringSource: 'self_provisioned',
		});
		try {
			await openEditAndWait(page, offeringId);

			// Seeded offering has no description; typing one flips exactly one diff row.
			const newDescription = 'Added by the e2e edit spec';
			await page.locator('#description').fill(newDescription);

			// Empty-state text is gone, replaced by the "N fields changed" summary.
			await expect(page.getByText('No unsaved changes yet.')).toHaveCount(0);
			await expect(page.getByText('1 field changed.')).toBeVisible();

			// The diff row renders the new value in its "After" block.
			const diffCard = page
				.locator('div.card')
				.filter({ has: page.getByRole('heading', { name: 'Changes Since Last Save' }) });
			await expect(diffCard.getByText(newDescription)).toBeVisible();
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});

	test('submit persists the change and redirects to the offerings list', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringId = await seedOffering(pubkey, {
			name: SEEDED_NAME,
			offeringSource: 'self_provisioned',
		});
		const renamed = 'E2E Edited Name';
		try {
			const offerNameInput = await openEditAndWait(page, offeringId);
			await expect(offerNameInput).toHaveValue(SEEDED_NAME);

			// Rename + submit.
			await offerNameInput.fill(renamed);
			await page.getByRole('button', { name: 'Save Changes' }).click();

			// On success the page redirects to the offerings list.
			await expect(page).toHaveURL(/\/dashboard\/offerings\/?$/);

			// Verify the PUT actually persisted the rename in the DB.
			const row = await sql(
				`SELECT offer_name FROM provider_offerings WHERE id = ${offeringId}`,
			);
			expect(row).toContain(renamed);
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});

	test('validation disables Save Changes when required fields are empty', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const offeringId = await seedOffering(pubkey, {
			name: SEEDED_NAME,
			offeringSource: 'self_provisioned',
		});
		try {
			await openEditAndWait(page, offeringId);
			const saveButton = page.getByRole('button', { name: 'Save Changes' });
			await expect(saveButton).toBeEnabled();

			// Clearing the required Offer Name disables the button.
			await page.locator('#offer-name').fill('');
			await expect(saveButton).toBeDisabled();

			// Restoring the name re-enables it, then zeroing the price disables it again.
			await page.locator('#offer-name').fill(SEEDED_NAME);
			await expect(saveButton).toBeEnabled();
			await page.locator('#monthly-price').fill('0');
			await expect(saveButton).toBeDisabled();
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});
});
