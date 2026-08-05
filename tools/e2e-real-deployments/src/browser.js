// Browser layer: launches its OWN headless Chromium (NOT the shared operator
// CDP) and drives the decent-cloud website sign-up flow (seed phrase).
//
// The website is the ONLY account-creation path (there is no public
// "register by API key" endpoint), so sign-up must be driven through the UI.
// The captured seed phrase is then used to sign provider-onboarding API calls
// directly (see src/crypto.js), avoiding fragile UI selectors for the provider
// wizard.

import { loadModule } from './deps.js';

let _chromium;
async function chromium() {
	if (!_chromium) {
		const pw = await loadModule('playwright');
		_chromium = pw.chromium;
	}
	return _chromium;
}

/** Launch an isolated headless Chromium browser. */
export async function launchBrowser() {
	const c = await chromium();
	return c.launch({ headless: true });
}

/**
 * Drive the website sign-up flow headlessly and return the new account's seed
 * phrase + identifiers. Mirrors website/tests/e2e/registration-flow.spec.ts.
 *
 * @param {import('playwright').Browser} browser
 * @param {{webUrl: string, email: string, username: string}} p
 * @returns {Promise<{seedPhrase: string, username: string, email: string}>}
 */
export async function signUpViaUi(browser, { webUrl, email, username }) {
	const context = await browser.newContext({
		baseURL: webUrl,
		permissions: ['clipboard-read', 'clipboard-write'],
	});
	// Surface uncaught page errors loudly (no silent swallowing).
	const pageErrors = [];
	context.on('weberror', (e) => pageErrors.push(e.error?.message ?? String(e)));
	const page = await context.newPage();

	try {
		await page.goto('/login', { waitUntil: 'domcontentloaded' });

		// "Sign in with seed phrase instead" is SSR'd; its onclick binds on
		// hydration. Retry the click until the "Import Existing" option appears.
		await clickAndRetry(
			page,
			page.locator('button:has-text("Sign in with seed phrase instead")'),
			page.locator('button:has-text("Import Existing")'),
		);

		// Generate a fresh seed phrase.
		await page.locator('button:has-text("Generate New")').click();
		await page.locator('button:has-text("Copy to Clipboard")').waitFor({ state: 'visible', timeout: 15_000 });

		const words = await page
			.locator('.font-mono')
			.filter({ hasText: /^[a-z]+$/ })
			.allTextContents();
		const seedPhrase = words.join(' ').trim();
		if (seedPhrase.split(/\s+/).length !== 12) {
			throw new Error(`sign-up: expected a 12-word seed phrase, got ${seedPhrase.split(/\s+/).length} words`);
		}

		// Confirm backup, then enter username + email.
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');
		await page.locator('input[placeholder="alice"]').waitFor({ state: 'visible', timeout: 15_000 });
		await page.fill('input[placeholder="alice"]', username);
		// Wait for client-side availability check before submitting.
		await page.getByText('available', { exact: false }).waitFor({ state: 'visible', timeout: 15_000 });
		await page.fill('input[placeholder="you@example.com"]', email);

		const createResponse = page.waitForResponse(
			(r) => /\/api\/v1\/accounts$/.test(r.url()) && r.request().method() === 'POST',
			{ timeout: 30_000 },
		);
		await page.locator('button:has-text("Create Account")').click();
		const resp = await createResponse;
		if (!resp.ok()) {
			const body = await resp.text().catch(() => '');
			throw new Error(`sign-up: account creation failed (HTTP ${resp.status()}): ${body.slice(0, 200)}`);
		}

		// Assert the authenticated (logged-in) state — dashboard reachable.
		await page.locator('text=Welcome to Decent Cloud').waitFor({ state: 'visible', timeout: 20_000 });
		await page.click('button:has-text("Go to Dashboard")');
		await page.waitForURL(/\/dashboard/, { timeout: 20_000 });

		return { seedPhrase, username, email };
	} finally {
		if (pageErrors.length) {
			// Report but do not fail sign-up (some sites throw non-fatal page errors).
			process.stderr.write(`[sign-up page errors] ${pageErrors.join('; ')}\n`);
		}
		await context.close().catch(() => {});
	}
}

/**
 * Click `target` and retry until `success` is satisfied — for SSR'd buttons
 * whose onclick binds only after SvelteKit hydration (no networkidle waits).
 * Mirrors fixtures/auth-helpers.ts clickAndRetry.
 */
async function clickAndRetry(page, target, success) {
	const check = () => success.isVisible().catch(() => false);
	for (let attempt = 0; attempt < 25; attempt++) {
		await target.click({ timeout: 5_000 }).catch(() => {});
		if ((await check()) === true) return;
		await page.waitForTimeout(120);
	}
	await success.waitFor({ state: 'visible', timeout: 10_000 });
}
