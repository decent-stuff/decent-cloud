import { test, expect, type Page, type Browser } from '@playwright/test';
import { signIn, type AuthCredentials } from './fixtures/auth-helpers';
import { seedAccountDirect, deleteAccountByUsername } from './fixtures/seed-helpers';

/**
 * E2E: interactive step-through of the Become-Provider technical-onboarding
 * HUB (/dashboard/provider/start) with a FRESH provider identity.
 *
 * PREMISE NOTE (read me — important): /dashboard/provider/start is a STATIC
 * NAVIGATION HUB, NOT a multi-step form wizard. It renders link cards to the
 * real downstream flows and has NO "Next" buttons, NO form fields, and NO
 * wizard state on this route (see src/routes/dashboard/provider/start/+page.svelte:
 * a single page that {#each}es two "path" cards + two "step" cards, all plain
 * <a> links). The 3-step interactive wizard that actually creates a
 * provider_profiles row lives at /dashboard/provider/support and is ALREADY
 * covered end-to-end (steps 1→2→3 + signed PUT submit + persistence reload)
 * by provider-onboarding-submit.spec.ts.
 *
 * So "stepping through" this hub means CLICKING each downstream link and
 * asserting the destination page mounts for a fresh (non-provider) identity.
 * That is genuinely new coverage: provider-start-cta.spec.ts only asserts the
 * link href ATTRIBUTES statically (it never clicks), so a broken navigation
 * guard or a destination that errors for a brand-new non-provider account
 * would slip past it. This spec closes that gap.
 *
 * This spec does NOT fake a wizard here (there is none on this route) and
 * does NOT re-assert the support-wizard submit (owned by provider-onboarding-
 * submit.spec.ts — re-asserting would duplicate coverage, violating the
 * repo's non-overlap test rule).
 *
 * Auth: a fresh account is created DB-side via seedAccountDirect and signed in
 * via the UI signIn() helper (per the onboarding-test pattern requested), then
 * deleted in afterAll. The first-login WelcomeModal is suppressed via a
 * context-level addInitScript (mirrors the test-account fixture) so it can't
 * intercept the hub-link clicks.
 */
test.describe('Provider onboarding hub (/dashboard/provider/start) interactive step-through', () => {
	let creds: AuthCredentials;

	test.beforeAll(async () => {
		const created = await seedAccountDirect();
		creds = { username: created.username, seedPhrase: created.seedPhrase };
	});

	test.afterAll(async () => {
		// Guard: if beforeAll threw, creds is still undefined here.
		if (creds) await deleteAccountByUsername(creds.username);
	});

	/**
	 * Build a fresh browser context, suppress the first-login WelcomeModal, and
	 * sign the fresh identity in via the UI. Returns the authenticated page;
	 * the caller MUST close the context (it is not fixture-managed).
	 */
	async function freshAuthedPage(browser: Browser): Promise<Page> {
		const context = await browser.newContext();
		await context.addInitScript(() => {
			localStorage.setItem('first_login_onboarding_completed', 'true');
		});
		const page = await context.newPage();
		await signIn(page, creds);
		return page;
	}

	test('fresh provider steps through the onboarding hub: each link navigates to a live destination', async ({ browser }) => {
		const page = await freshAuthedPage(browser);
		const context = page.context();
		try {
			// Land on the onboarding hub. The H1 is the "page mounted + authed"
			// signal (the route is auth-gated under /dashboard/*).
			await page.goto('/dashboard/provider/start');
			await expect(
				page.getByRole('heading', { name: 'Provide capacity on Decent Cloud' }),
			).toBeVisible({ timeout: 15_000 });

			// --- Step 1, Path A: "Add a cloud account" → /dashboard/cloud/accounts ---
			await page.getByTestId('provider-start-cloud-accounts-link').click();
			await expect(page).toHaveURL(/\/dashboard\/cloud\/accounts/, { timeout: 15_000 });
			await expect(
				page.locator('main').getByRole('heading', { name: 'Cloud Accounts', exact: true }),
			).toBeVisible({ timeout: 15_000 });

			// --- Step 2: "Create an offering" → the create wizard step 1 (Basics) ---
			await page.goto('/dashboard/provider/start');
			await page.getByRole('link', { name: 'Create an offering' }).click();
			await expect(page).toHaveURL(/\/dashboard\/offerings\/create/, { timeout: 15_000 });
			await expect(
				page.getByRole('heading', { name: 'Basics', exact: true }),
			).toBeVisible({ timeout: 15_000 });

			// --- Step 3: "Open the support profile" → support wizard step 1 ---
			await page.goto('/dashboard/provider/start');
			await page.getByRole('link', { name: 'Open the support profile' }).click();
			await expect(page).toHaveURL(/\/dashboard\/provider\/support/, { timeout: 15_000 });
			// The step-1 "Support Portal" heading proves the support wizard (where
			// the real provider profile is eventually created) mounted via the hub.
			await expect(
				page.getByRole('heading', { name: 'Support Portal', exact: true }),
			).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});
