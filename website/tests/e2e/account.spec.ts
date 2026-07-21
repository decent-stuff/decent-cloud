import { test, expect } from '@playwright/test';
import { registerNewAccount, setupConsoleLogging } from './fixtures/auth-helpers';

/**
 * E2E coverage for the /dashboard/account page's error-recovery path.
 *
 * Account-page-specific edge cases that the existing account-page.spec.ts
 * does not cover (it focuses on happy-path rendering and tab navigation).
 *
 * The fast-auth fixture pre-seeds localStorage.seed_phrases before
 * navigation, so the only way to reach the "account fetch failed" branch
 * is to force the /accounts?publicKey=... call to fail.
 *
 * MOCK STRATEGY: the app registers a service worker (static/sw.js) that
 * intercepts all fetches via a fetch-event handler. That SW claims the
 * page immediately on first load (skipWaiting + clients.claim) and answers
 * from its cache before requests reach the network — which makes
 * Playwright's `page.route` / `context.route` mocks ineffective against
 * the app's own fetches. We instead patch `window.fetch` via
 * addInitScript, which runs at document_start before any app code (and
 * before the SW can claim the page). The patched fetch returns 500 only
 * for the account-lookup URL; everything else passes through unchanged.
 */

test.describe('/dashboard/account error recovery', () => {
	test('shows error card with Retry and Logout when account fetch fails (#6)', async ({ browser }) => {
		// Audit #6: if the /accounts?publicKey=... fetch silently failed (network
		// blip, 500, soft-deleted row), the page stayed on "Loading..." forever
		// with no recovery. The fix adds an explicit error card with Retry +
		// Logout so the user isn't stuck looking at a half-rendered page.

		// Register a real account first so the seed phrase is valid.
		const setupContext = await browser.newContext();
		const setupPage = await setupContext.newPage();
		setupConsoleLogging(setupPage);
		const credentials = await registerNewAccount(setupPage);
		await setupContext.close();

		// Fresh context. Pre-seed the seed phrase + dismiss the WelcomeModal
		// exactly like the fast-auth fixture does.
		const context = await browser.newContext();
		await context.addInitScript(() => {
			// Patch window.fetch BEFORE the app (and the service worker) load.
			// addInitScript runs at document_start, so this is in place before
			// any module script or app.html inline script runs.
			const orig = window.fetch.bind(window);
			window.fetch = (input: RequestInfo | URL, init?: RequestInit) => {
				const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
				if (url && url.includes('/api/v1/accounts?publicKey=')) {
					return Promise.resolve(new Response('forced failure for E2E', { status: 500 }));
				}
				return orig(input as RequestInfo, init);
			};
		});
		await context.addInitScript((s: string) => {
			localStorage.setItem('seed_phrases', JSON.stringify([s]));
			sessionStorage.setItem('first_login_onboarding_completed', 'true');
		}, credentials.seedPhrase);

		const page = await context.newPage();
		setupConsoleLogging(page);
		await page.goto('/dashboard/account');

		// The error card must render — not the perpetual "Loading..." placeholder.
		await expect(page.getByText(/couldn't load your account|failed to load your account/i)).toBeVisible({ timeout: 15000 });

		// Retry button must be present so the user can recover without leaving the page.
		await expect(page.getByRole('button', { name: /^retry$/i })).toBeVisible();

		// Logout button must be present so the user can escape the broken session.
		await expect(page.getByRole('button', { name: /^logout$/i })).toBeVisible();

		// The perpetual "Loading..." text must NOT be the final state.
		await expect(page.getByText('Loading...', { exact: true })).toHaveCount(0);

		await context.close();
	});
});




