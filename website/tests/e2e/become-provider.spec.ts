import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the /dashboard/offerings/create wizard (KEY GAP route).
 *
 * The wizard has 3 steps (Basics → Infrastructure → Pricing & Recipe). This
 * spec pins the step-1→step-2 transition and the no-Hetzner-account branch of
 * step 2, which is the exact path a brand-new provider (the fixture account)
 * takes. It deliberately does NOT submit step 3, which would create a real
 * provider_offerings row.
 */
test.describe('/dashboard/offerings/create wizard', () => {
	test('@smoke renders step 1, advances to step 2, and links Hetzner onboarding', async ({ page }) => {
		await page.goto('/dashboard/offerings/create');

		// Step 1 must render with its required form fields. Waiting on the
		// Offer Name input IS the "wizard hydrated and auth loaded" signal —
		// before auth resolves the page shows a loading spinner instead.
		const offerNameInput = page.locator('#offer-name');
		await expect(offerNameInput).toBeVisible({ timeout: 15000 });
		await expect(page.locator('#offering-id')).toBeVisible();
		await expect(page.locator('#description')).toBeVisible();

		// Step 1 must render with its card heading. (The step indicator also
		// contains the word "Basics", so target the heading role to be unique.)
		await expect(page.getByRole('heading', { name: 'Basics', exact: true })).toBeVisible();

		// Fill step 1 with valid data and advance. Offering ID auto-derives
		// from the name on blur, so we only need to fill the name + blur.
		await offerNameInput.fill('E2E Wizard Smoke Offering');
		await offerNameInput.blur();

		// Click "Next: Infrastructure". Use a role/name selector scoped to the
		// step-1 footer so we don't accidentally match a future step's button.
		await page.getByRole('button', { name: 'Next: Infrastructure' }).click();

		// Step 2 must render. The Infrastructure heading is unique to step 2.
		await expect(page.getByRole('heading', { name: 'Infrastructure', exact: true })).toBeVisible();

		// The fixture account has zero Hetzner cloud accounts, so step 2 must
		// show the "no accounts" guidance branch (not the account picker).
		await expect(page.getByText('No Hetzner cloud accounts found')).toBeVisible();

		// The guidance must link to the cloud-accounts onboarding page.
		const hetznerLink = page.getByRole('link', { name: 'Connect a Hetzner account' });
		await expect(hetznerLink).toBeVisible();
		await expect(hetznerLink).toHaveAttribute('href', '/dashboard/cloud/accounts');

		// The "Next: Pricing & Recipe" button must be present even without a
		// Hetzner account — the page explicitly allows proceeding without one.
		await expect(
			page.getByRole('button', { name: 'Next: Pricing & Recipe' }),
		).toBeVisible();

		// Deliberately stop here: clicking Next again would reach step 3 and
		// its submit button, which would create a real offering row on click.
	});
});
