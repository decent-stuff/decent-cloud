import { test, expect } from './fixtures/test-account';

test.describe('First login onboarding', () => {
	test('@smoke guides a new user through all onboarding steps once', async ({ page }) => {
		// The fast-auth fixture dismisses the WelcomeModal by default. This test
		// exercises the modal explicitly, so add a page-level init script (runs
		// AFTER the context-level one) to re-arm the modal on every navigation.
		await page.addInitScript(() => {
			sessionStorage.removeItem('first_login_onboarding_completed');
			localStorage.removeItem('first_login_onboarding_completed');
		});

		await page.goto('/dashboard');

		// Test account has a complete profile (seedAccountDirect sets username +
		// email), so step 1 acknowledges readiness rather than asking to complete.
		await expect(page.getByRole('heading', { name: 'Your profile is ready' })).toBeVisible();

		const modal = page.locator('.fixed.inset-0.z-50');
		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Add your SSH key' })).toBeVisible();
		await expect(page.getByText('No SSH key found yet. Add one in Security settings.')).toBeVisible();

		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).toBeVisible();

		await modal.getByRole('button', { name: 'Stay on dashboard' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).not.toBeVisible();

		await page.reload();
		await expect(page.getByRole('heading', { name: 'Your profile is ready' })).not.toBeVisible();
	});

	test('backdrop click closes the modal without completing onboarding (audit #12)', async ({ page }) => {
		// Audit #12: a single accidental backdrop click used to call
		// finishOnboarding(), which sets first_login_onboarding_completed
		// = 'true' and permanently dismisses onboarding across reloads. Backdrop
		// should only close the modal — explicit CTAs are the only path to
		// completion.
		//
		// We assert localStorage directly because the fast-auth fixture's
		// context-level init script writes 'true' to first_login_onboarding_completed
		// on every navigation, which would mask the reload-visible behavior.
		// The completion flag is the single source of truth for WelcomeModal
		// visibility (see WelcomeModal.svelte), so checking it is equivalent.
		await page.addInitScript(() => {
			sessionStorage.removeItem('first_login_onboarding_completed');
			localStorage.removeItem('first_login_onboarding_completed');
		});

		await page.goto('/dashboard');
		await expect(page.getByRole('heading', { name: 'Your profile is ready' })).toBeVisible();

		// The backdrop is the absolutely-positioned div with role="presentation".
		// Clicking at its centre would land inside the modal box, so click at
		// the top-left corner instead.
		const backdrop = page.locator('.fixed.inset-0.z-50 > .absolute.inset-0');
		await backdrop.click({ position: { x: 0, y: 0 } });

		// Modal closes (proves the click registered).
		await expect(page.getByRole('heading', { name: 'Your profile is ready' })).not.toBeVisible();

		// CRITICAL: the completion flag must NOT be set. The bug set it to 'true',
		// which would permanently suppress the modal across reloads.
		const flag = await page.evaluate(() => localStorage.getItem('first_login_onboarding_completed'));
		expect(flag).toBeNull();
	});

	test('completing onboarding persists in localStorage across browser sessions (F2)', async ({ page }) => {
		// F2: the completion flag was stored in sessionStorage, which clears on
		// every new browser session — so returning users saw the modal again.
		// The flag must persist in localStorage so completion survives a browser
		// restart. sessionStorage is per-tab-session and is the wrong store.
		await page.addInitScript(() => {
			sessionStorage.removeItem('first_login_onboarding_completed');
			localStorage.removeItem('first_login_onboarding_completed');
		});

		await page.goto('/dashboard');
		await expect(page.locator('.fixed.inset-0.z-50')).toBeVisible();

		const modal = page.locator('.fixed.inset-0.z-50');
		await modal.getByRole('button', { name: 'Continue' }).click();
		await modal.getByRole('button', { name: 'Continue' }).click();
		await modal.getByRole('button', { name: 'Stay on dashboard' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).not.toBeVisible();

		// Flag persisted in localStorage (survives browser restart).
		const lsFlag = await page.evaluate(() => localStorage.getItem('first_login_onboarding_completed'));
		expect(lsFlag).toBe('true');

		// Flag NOT in sessionStorage (that was the buggy store).
		const ssFlag = await page.evaluate(() => sessionStorage.getItem('first_login_onboarding_completed'));
		expect(ssFlag).toBeNull();
	});

	test('step 1 acknowledges a complete profile instead of stale "Complete your profile" copy (F2)', async ({ page }) => {
		// F2: step 1 always said "Complete your profile" even when username +
		// email were both set (shown as green in the modal). When the profile is
		// already complete, the heading must reflect that rather than implying
		// action is needed.
		await page.addInitScript(() => {
			sessionStorage.removeItem('first_login_onboarding_completed');
			localStorage.removeItem('first_login_onboarding_completed');
		});

		await page.goto('/dashboard');

		// The test account has a username + email (seedAccountDirect), so the
		// profile is complete. The stale "Complete your profile" heading must NOT appear.
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).toHaveCount(0);
		await expect(page.getByText('Your profile is ready')).toBeVisible();
	});
});
