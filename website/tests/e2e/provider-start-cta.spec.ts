import { test, expect } from './fixtures/test-account';

/**
 * PRODUCT-DIRECTION.md: decent-cloud is "OpenRouter, but for cloud resources" —
 * a proxy/reselling platform unifying many providers behind one common API.
 * "Become a Provider must mean real onboarding to the technical path … not just
 * a support-profile wizard." The landing "Become a Provider" CTA points to
 * /dashboard/provider/start, a technical-onboarding hub.
 *
 * There are TWO honest provider paths and this spec pins both:
 *   A) Resell a managed cloud (Hetzner/Vultr) — add a cloud account, no dc-agent.
 *      (Hetzner/Vultr are central-API CloudBackends; cloud VMs get public IPs.)
 *   B) List your own infrastructure — install dc-agent (Proxmox/Docker/DO).
 * Both converge on step 2 (create an offering). The spec also pins the CTA
 * repoint from the landing page.
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
		'start page presents BOTH provider paths plus the shared onboarding steps',
		async ({ page }) => {
			await page.goto('/dashboard/provider/start');

			// Page heading confirms we landed on the onboarding hub.
			await expect(
				page.getByRole('heading', { name: 'Provide capacity on Decent Cloud' }),
			).toBeVisible();

			// Path A: resell a managed cloud → the cloud-accounts flow (no dc-agent).
			const cloudAccount = page.getByTestId('provider-start-cloud-accounts-link');
			await expect(cloudAccount).toBeVisible();
			await expect(cloudAccount).toHaveAttribute('href', '/dashboard/cloud/accounts');
			// Internal link — must NOT open in a new tab.
			await expect(cloudAccount).not.toHaveAttribute('target', '_blank');

			// Path B: list your own infrastructure → dc-agent install docs (external).
			const installDocs = page.getByTestId('provider-start-install-docs-link');
			await expect(installDocs).toBeVisible();
			await expect(installDocs).toHaveAttribute('target', '_blank');
			await expect(installDocs).toHaveAttribute('href', /provider-agent-installation\.md$/);

			// Shared step 2: the create-offering flow (both paths converge here).
			const createOffering = page.getByRole('link', { name: 'Create an offering' });
			await expect(createOffering).toBeVisible();
			await expect(createOffering).toHaveAttribute('href', '/dashboard/offerings/create');

			// Shared step 3: the support profile.
			const supportProfile = page.getByRole('link', { name: 'Open the support profile' });
			await expect(supportProfile).toBeVisible();
			await expect(supportProfile).toHaveAttribute('href', '/dashboard/provider/support');
		},
	);
});
