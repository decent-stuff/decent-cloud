import { test, expect } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	verifyAccountEmail,
	seedRentableWithResource,
	cleanupRentableWithResource,
	type RentableWithResourceSeed,
} from './fixtures/seed-helpers';

/**
 * Keyboard + a11y behavior for the marketplace Rent dialog
 * (RentalRequestDialog.svelte). Mirrors the KeyboardHelpOverlay pattern:
 * role="dialog"/aria-modal, focus moves in on open, Escape closes from
 * anywhere, Tab is trapped inside, and Enter in a text field submits the form.
 *
 * No contracts are created here (the happy-path submit is covered by
 * rent-flow.spec.ts). These tests only exercise dialog interaction, so they
 * do not touch wallet/contract rows for the shared requester pubkey and are
 * safe to run in parallel with rent-flow.spec.ts.
 */
test.describe('Rent dialog keyboard + a11y', () => {
	test.describe.configure({ mode: 'serial' });

	let seed: RentableWithResourceSeed;

	test.beforeAll(async ({ testAccount }) => {
		const requesterPubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		// Rentals are rejected unless the requester's email is verified
		// (API guard). Idempotent + safe to run alongside rent-flow.spec.
		await verifyAccountEmail(requesterPubkey);
		seed = await seedRentableWithResource({ name: 'E2E Rent Dialog Keyboard Offering' });
	});

	test.afterAll(async () => {
		if (seed) await cleanupRentableWithResource(seed);
	});

	async function openRentDialog(page: import('@playwright/test').Page) {
		await page.goto(`/dashboard/marketplace/${seed.offeringNumericId}`);
		const rentBtn = page.getByRole('button', { name: 'Rent this offering' }).first();
		await expect(rentBtn).toBeVisible({ timeout: 15000 });
		await rentBtn.click();
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toBeVisible({
			timeout: 5000,
		});
	}

	test('dialog is a real modal: role=dialog, aria-modal, focus moves in on open', async ({
		page,
	}) => {
		await openRentDialog(page);
		const dialog = page.getByTestId('rent-dialog');
		await expect(dialog).toBeVisible();
		await expect(dialog).toHaveAttribute('role', 'dialog');
		await expect(dialog).toHaveAttribute('aria-modal', 'true');
		// Focus moves into the dialog on open (autofocus lands on the Duration
		// field — the first input — without scrolling the cost out of view).
		await expect.poll(async () => {
			return await page.evaluate(() => document.activeElement?.id);
		}, { message: 'a field inside the dialog should be autofocused on open' }).toBe('duration');
	});

	test('Tab stays trapped inside the dialog', async ({ page }) => {
		await openRentDialog(page);
		// Deterministic starting point.
		await page.locator('#ssh-key').focus();
		// Tab several times across a full cycle; focus must never leave the dialog.
		for (let i = 0; i < 8; i++) {
			await page.keyboard.press('Tab');
			const inside = await page.evaluate(() => {
				const d = document.querySelector('[data-testid="rent-dialog"]');
				return d ? d.contains(document.activeElement) : false;
			});
			expect(inside, `focus escaped the dialog after Tab #${i + 1}`).toBe(true);
		}
	});

	test('Escape closes the dialog even when an inner field is focused', async ({ page }) => {
		await openRentDialog(page);
		// Focus an inner field (NOT the backdrop) to prove Escape works from inside.
		await page.locator('#ssh-key').focus();
		await page.keyboard.press('Escape');
		await expect(page.getByRole('heading', { name: 'Rent Resource' })).toHaveCount(0);
	});

	test('Enter in a text field submits the form (keyboard submit)', async ({ page }) => {
		await openRentDialog(page);
		// Non-empty but invalid SSH key passes native `required` so the form
		// actually submits, then handleSubmit's validateSshKey rejects the format.
		await page.locator('#ssh-key').fill('not-a-valid-ssh-key');
		// Open the Advanced disclosure so a single-line text input is reachable.
		await page.getByText('Advanced (optional)').click();
		const contact = page.locator('#contact');
		await contact.waitFor({ state: 'visible' });
		await contact.focus();
		await page.keyboard.press('Enter');
		// The error BANNER (data-testid) is set only inside handleSubmit, so
		// its appearance proves Enter actually submitted the form (the inline
		// field hint shows reactively on input and would not prove submission).
		const errorBanner = page.getByTestId('rent-dialog-error');
		await expect(errorBanner).toBeVisible({ timeout: 5000 });
		await expect(errorBanner).toContainText('Invalid SSH key format');
	});
});
