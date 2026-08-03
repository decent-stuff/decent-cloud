import { test, expect } from './fixtures/test-account';

/**
 * PRODUCT-DIRECTION.md (F9): "Become a Provider must mean real onboarding to the
 * technical path (install the agent, register a pool, list an offering), not
 * just a support-profile wizard." The landing "Become a Provider" CTA used to
 * land on /dashboard/provider/support (a support-profile completeness wizard)
 * and never exposed the real provider-setup path. It now points to
 * /dashboard/provider/start — a technical-onboarding hub that links the dc-agent
 * install docs, the create-offering flow, and the support profile.
 *
 * This spec pins both halves of that contract: the CTA repoint, and the start
 * page surfacing the three technical-onboarding destinations.
 */
test.describe('"Become a Provider" CTA → technical onboarding (F9)', () => {
	test('landing CTA points to the technical-onboarding start page', async ({ browser }) => {
		// Public landing page — no auth needed to assert the hero CTA href.
		const context = await browser.newContext();
		const page = await context.newPage();
		await page.goto('/');

		const cta = page.getByRole('link', { name: 'Become a Provider' });
		await expect(cta).toBeVisible();
		await expect(cta).toHaveAttribute('href', '/dashboard/provider/start');
		await context.close();
	});

	test(
		'start page exposes the three technical-onboarding destinations',
		async ({ page }) => {
			await page.goto('/dashboard/provider/start');

			// Page heading confirms we landed on the onboarding hub.
			await expect(
				page.getByRole('heading', { name: 'List your infrastructure on Decent Cloud' }),
			).toBeVisible();

			// Step 1: dc-agent installation docs (external link to the repo docs).
			const installDocs = page.getByTestId('provider-start-install-docs-link');
			await expect(installDocs).toBeVisible();
			await expect(installDocs).toHaveAttribute('target', '_blank');
			await expect(installDocs).toHaveAttribute('href', /provider-agent-installation\.md$/);

			// Step 2: the create-offering flow (the real technical path).
			const createOffering = page.getByRole('link', { name: 'Create an offering' });
			await expect(createOffering).toBeVisible();
			await expect(createOffering).toHaveAttribute('href', '/dashboard/offerings/create');

			// Step 3: the support profile (still reachable, but no longer the
			// sole/primary destination — it is step 3 of the technical onboarding).
			const supportProfile = page.getByRole('link', { name: 'Open the support profile' });
			await expect(supportProfile).toBeVisible();
			await expect(supportProfile).toHaveAttribute('href', '/dashboard/provider/support');
		},
	);
});
