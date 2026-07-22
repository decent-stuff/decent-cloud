import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the authenticated (non-provider) state of
 * /dashboard/provider/requests — the gap left by provider-batch-actions.spec.ts.
 *
 * provider-batch-actions.spec.ts pins the ANONYMOUS state (Login Required).
 * This spec pins the authenticated non-provider state: the fixture account is
 * not a provider, so the ProviderSetupBanner must render ("Provider Setup
 * Required") with a link into the provider-support onboarding page. That banner
 * is the meaningful gate between "anonymous" and "fully-onboarded provider".
 */
test.describe('/dashboard/provider/requests (authenticated, non-provider)', () => {
	test('shows the provider-setup-required banner for a non-provider account', async ({ page }) => {
		await page.goto('/dashboard/provider/requests');
		const main = page.locator('main');

		// Page heading always renders (covered for anonymous too); re-assert
		// here to pin that the authenticated page, not a redirect, loaded.
		await expect(
			main.getByRole('heading', { name: 'Provider Requests', exact: true }),
		).toBeVisible({ timeout: 10000 });

		// Must NOT show the auth-required card — the fixture is authenticated.
		await expect(page.getByText('Login Required')).toBeHidden();

		// The ProviderSetupBanner renders only when onboardingCompleted === false
		// (the non-provider state). Its headline is the meaningful gate message.
		await expect(main.getByText('Provider Setup Required')).toBeVisible({ timeout: 10000 });

		// The banner must link into the provider-support onboarding flow — this
		// is the actionable next step for a non-provider.
		const setupLink = main.getByRole('link', { name: 'Provider Setup' });
		await expect(setupLink).toBeVisible();
		await expect(setupLink).toHaveAttribute('href', '/dashboard/provider/support');

		// Batch action buttons are provider-only; a non-provider must not see
		// them even though they're authenticated.
		await expect(main.getByRole('button', { name: 'Accept All' })).toBeHidden();
		await expect(main.getByRole('button', { name: 'Reject All' })).toBeHidden();
	});
});
