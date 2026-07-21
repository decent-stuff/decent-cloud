import { test, expect } from './fixtures/test-account';

test.describe('First login onboarding', () => {
	test('@smoke guides a new user through all onboarding steps once', async ({ page }) => {
		// The fast-auth fixture dismisses the WelcomeModal by default. This test
		// exercises the modal explicitly, so add a page-level init script (runs
		// AFTER the context-level one) to re-arm the modal on every navigation.
		await page.addInitScript(() => sessionStorage.removeItem('first_login_onboarding_completed'));

		await page.goto('/dashboard');

		await expect(page.getByRole('heading', { name: 'Complete your profile' })).toBeVisible();

		const modal = page.locator('.fixed.inset-0.z-50');
		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Add your SSH key' })).toBeVisible();
		await expect(page.getByText('No SSH key found yet. Add one in Security settings.')).toBeVisible();

		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).toBeVisible();

		await modal.getByRole('button', { name: 'Stay on dashboard' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).not.toBeVisible();

		await page.reload();
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).not.toBeVisible();
	});

	test('backdrop click closes the modal without completing onboarding (audit #12)', async ({ page }) => {
		// Audit #12: a single accidental backdrop click used to call
		// finishOnboarding(), which sets sessionStorage.first_login_onboarding_completed
		// = 'true' and permanently dismisses onboarding across reloads. Backdrop
		// should only close the modal — explicit CTAs are the only path to
		// completion.
		//
		// We assert sessionStorage directly because the fast-auth fixture's
		// context-level init script writes 'true' to first_login_onboarding_completed
		// on every navigation, which would mask the reload-visible behavior.
		// The completion flag is the single source of truth for WelcomeModal
		// visibility (see WelcomeModal.svelte:18), so checking it is equivalent.
		await page.addInitScript(() => sessionStorage.removeItem('first_login_onboarding_completed'));

		await page.goto('/dashboard');
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).toBeVisible();

		// The backdrop is the absolutely-positioned div with role="presentation".
		// Clicking at its centre would land inside the modal box, so click at
		// the top-left corner instead.
		const backdrop = page.locator('.fixed.inset-0.z-50 > .absolute.inset-0');
		await backdrop.click({ position: { x: 0, y: 0 } });

		// Modal closes (proves the click registered).
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).not.toBeVisible();

		// CRITICAL: the completion flag must NOT be set. The bug set it to 'true',
		// which would permanently suppress the modal across reloads.
		const flag = await page.evaluate(() => sessionStorage.getItem('first_login_onboarding_completed'));
		expect(flag).toBeNull();
	});
});
