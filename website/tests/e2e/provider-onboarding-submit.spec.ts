import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	deleteProviderProfileByPubkey,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the Become-Provider ONBOARDING SUBMIT flow
 * (/dashboard/provider/support → step 3 "Help Center Profile").
 *
 * FLOWS.md "Become provider / setup wizard" was ⚠️: the smoke test stops at the
 * step-1/step-2 render; the full onboarding submit was never asserted. This spec
 * drives the Help Center form to its submit and asserts the data PERSISTS.
 *
 * Verified path: `saveOnboarding` (`support/+page.svelte:372`) signs a PUT to
 * /api/v1/providers/:pubkey/onboarding — a pure DB upsert into provider_profiles
 * (support_hours, support_channels, regions, payment_methods…). It then fires a
 * helpcenter/sync (Chatwoot) which may throw when Chatwoot is unconfigured; that
 * does NOT roll back the PUT. So the assertion is on PERSISTENCE (reload
 * re-populates the saved values), which holds whether or not the sync succeeds.
 *
 * Serial mode: the PUT upserts a provider_profiles row keyed on the shared
 * testAccount pubkey; afterAll deletes it (plus provider_onboarding) via
 * deleteProviderProfileByPubkey so sibling specs are not polluted.
 */
test.describe('Become-provider onboarding submit (/dashboard/provider/support)', () => {
	test.describe.configure({ mode: 'serial' });

	const SUPPORT_HOURS = '24/7';
	const CHANNEL = 'Email';
	const REGION = 'Europe';
	const PAYMENT = 'PayPal';

	test('submitting the Help Center form persists onboarding data across reload', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await page.goto('/dashboard/provider/support');
			await waitForAuthReady(page);

			// Wait for the wizard chrome (authenticated + loaded), then advance to
			// step 3 (Help Center Profile). The wizard step is persisted to
			// localStorage, so a later reload returns straight to step 3.
			await page.getByRole('button', { name: 'Save & Continue' }).first().waitFor({ state: 'visible', timeout: 15000 });
			await page.getByRole('button', { name: 'Save & Continue' }).first().click();
			await page.getByRole('button', { name: 'Save & Continue' }).first().click();
			await expect(page.getByRole('heading', { name: 'Help Center Profile' })).toBeVisible({ timeout: 15000 });

			// Fill the four required fields (saveOnboarding rejects if any is empty).
			const form = page.locator('#helpcenter');
			await form.locator('#support-hours').selectOption(SUPPORT_HOURS);
			await form.locator('label').filter({ hasText: CHANNEL }).locator('input[type="checkbox"]').check();
			await form.locator('label').filter({ hasText: REGION }).locator('input[type="checkbox"]').check();
			await form.locator('label').filter({ hasText: PAYMENT }).locator('input[type="checkbox"]').check();

			// Submit and wait specifically for the onboarding PUT (the persistence
			// step). The subsequent helpcenter/sync may fail when Chatwoot is
			// unconfigured, but the PUT has already committed by then.
			const putDone = page.waitForResponse(
				(resp) =>
					resp.url().includes('/api/v1/providers/') &&
					resp.url().includes('/onboarding') &&
					resp.request().method() === 'PUT',
				{ timeout: 20000 },
			);
			await page.getByRole('button', { name: 'Save & Publish' }).click();
			const putRes = await putDone;
			expect(putRes.ok()).toBeTruthy();

			// Reload — the page re-fetches the onboarding data and re-populates the
			// form (loadOnboarding). Because the wizard step persisted, it returns
			// to step 3 directly.
			await page.reload();
			await waitForAuthReady(page);
			await expect(page.getByRole('heading', { name: 'Help Center Profile' })).toBeVisible({ timeout: 15000 });

			// The saved support-hours value round-tripped through the DB and is now
			// the selected option again — proof the submit persisted.
			await expect(page.locator('#support-hours')).toHaveValue(SUPPORT_HOURS, { timeout: 15000 });
			// And the saved channel is still checked.
			await expect(
				page.locator('#helpcenter label').filter({ hasText: CHANNEL }).locator('input[type="checkbox"]'),
			).toBeChecked();
		} finally {
			await deleteProviderProfileByPubkey(pubkey);
		}
	});
});
