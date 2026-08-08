import { test as baseTest, expect } from '@playwright/test';
import { test as authTest, waitForAuthReady } from './fixtures/test-account';

/**
 * UX regression guards — fast e2e tests that pin recently-shipped UX fixes
 * (commits 25945664..2dd8e373). Each test FAILS if its fix is reverted, so a
 * future regression trips CI instead of silently reaching users.
 *
 * Two `test` instances coexist in this file on purpose:
 *   - `baseTest` (@playwright/test) drives the ANONYMOUS / SSR assertions
 *     (UX-001/002/005/008/013). It must NOT carry the testAccount fixture's
 *     localStorage seed injection, or those pages would load authenticated
 *     and the anonymous-side assertions would be vacuous.
 *   - `authTest` (fixtures/test-account) is used only by UX-004, the single
 *     test that needs an authenticated dashboard session. It pre-seeds the
 *     seed phrase via addInitScript so the page lands signed-in.
 *
 * Conventions (website/AGENTS.md + FLOWS.md): no `networkidle`, explicit
 * `{ timeout }` on every wait, fast (<5s) + low-seed so each qualifies for
 * `@smoke`.
 */

baseTest.describe('UX regression guards (anonymous)', () => {
	baseTest(
		'@smoke UX-001 homepage hero shows the "Anatomy of a Trust Score" graphic, not fabricated provider data',
		async ({ page }) => {
			const res = await page.goto('/');
			expect(res?.status()).toBe(200);

			// The educational graphic replaced the fake "provider_alpha" card.
			await expect(page.getByText('Anatomy of a Trust Score')).toBeVisible();

			// Fabricated provider data must be gone. These three signatures
			// only ever appeared on the removed fake card (a made-up handle +
			// invented numbers), so their absence is a deterministic signal.
			const body = page.locator('body');
			await expect(body).not.toContainText('provider_alpha');
			await expect(body).not.toContainText('87 Trust Score');
			await expect(body).not.toContainText('1,247 Contracts');

			// A substring check for "Verified Provider" is intentionally NOT
			// used: the hero's rotating typing animation legitimately spells
			// "Verified Provider Track Records", so a substring assertion
			// would false-positive the instant the loop typed that phrase.
			// The fabricated handle + numbers above fully cover the fake-card
			// regression.
		},
	);

	baseTest(
		'@smoke UX-002 /dashboard/validators is retired (404) and no sidebar link remains',
		async ({ page }) => {
			// ICP validators were retired; the route must not resolve.
			const res = await page.goto('/dashboard/validators');
			expect(res?.status()).toBe(404);

			// On a real dashboard route, the sidebar must not advertise the
			// retired page at all — no link, no label.
			await page.goto('/dashboard');
			await expect(page.locator('aside')).toBeVisible();
			await expect(page.locator('aside a[href="/dashboard/validators"]')).toHaveCount(0);
			await expect(page.locator('aside')).not.toContainText(/validators/i);
		},
	);

	baseTest(
		'@smoke UX-005 homepage stats grid omits dead ICP metrics (Validators / Transfers)',
		async ({ page }) => {
			await page.goto('/');
			// Pin the stats section so the assertion targets the real grid.
			await expect(
				page.getByRole('heading', { name: 'Marketplace Statistics' }),
			).toBeVisible();

			// Removed dead ICP metrics: there is no validator network and no
			// ICP token-transfer rail anymore, so neither may be advertised.
			const body = page.locator('body');
			await expect(body).not.toContainText('Active Validators');
			await expect(body).not.toContainText('Total Transfers');
		},
	);

	baseTest(
		'@smoke UX-008 unauthenticated sidebar hides "My Activity" (single Sign In CTA)',
		async ({ page }) => {
			await page.goto('/dashboard');
			const aside = page.locator('aside');
			await expect(aside).toBeVisible();

			// "My Activity" is an auth-gated section; anonymous users see only
			// the public Browse items + a single Sign In CTA in the header.
			await expect(aside).not.toContainText('My Activity');
			await expect(aside.getByText('My Activity')).toHaveCount(0);

			// Sanity: a Sign In entry point is still reachable.
			await expect(
				page.locator('button:has-text("Sign In")').filter({ visible: true }).first(),
			).toBeVisible();
		},
	);

	baseTest(
		'@smoke UX-013 login page heading reads "Sign In or Create Account"',
		async ({ page }) => {
			await page.goto('/login');
			// The AuthFlow card heading was broadened from "Sign In" to also
			// advertise account creation.
			const heading = page.getByRole('heading', { name: /Create Account/i });
			await expect(heading).toBeVisible();
			await expect(heading).toHaveText(/Sign In or Create Account/i);
		},
	);
});

authTest.describe('UX regression guards (authenticated)', () => {
	authTest(
		'@smoke UX-004 dashboard welcome card shows @username, not a raw principal',
		async ({ page, testAccount }) => {
			await page.goto('/dashboard');
			await waitForAuthReady(page);

			// The welcome card (.card-accent) is the identity surface that
			// previously rendered the raw ICP principal.
			const welcomeCard = page.locator('.card-accent').first();
			await expect(welcomeCard).toBeVisible();

			// Identity must surface as the @username handle...
			await expect(welcomeCard).toContainText(`@${testAccount.username}`);

			// ...and NOT as a raw textual principal (dashed form like
			// xxxxx-xxxxx-xxxxx-...). The seeded username is `test<digits>`
			// (no dashes), so this pattern can only trip on a leaked principal.
			await expect(welcomeCard).not.toContainText(
				/[a-z0-9]{3,}-[a-z0-9]{3,}-[a-z0-9]{3,}/,
			);
		},
	);
});
