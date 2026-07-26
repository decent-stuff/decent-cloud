import { type Page, type Locator, expect } from '@playwright/test';

/**
 * Test helper utilities for authentication flows
 */

export interface AuthCredentials {
	username: string;
	seedPhrase: string;
}

/**
 * Set up console logging for a page
 * Logs all browser console messages (including errors, warnings, etc.) to the test output
 */
export function setupConsoleLogging(page: Page): void {
	page.on('console', (msg) => {
		const type = msg.type();
		const text = msg.text();
		const location = msg.location();

		// Format with color and location info
		const prefix = `[Browser ${type.toUpperCase()}]`;
		const locationStr = location.url ? ` at ${location.url}:${location.lineNumber}` : '';

		// Log all console messages to test output
		console.log(`${prefix}${locationStr}: ${text}`);
	});

	// Also listen for page errors (uncaught exceptions)
	page.on('pageerror', (error) => {
		console.error('[Browser UNCAUGHT ERROR]:', error.message);
		console.error(error.stack);
	});
}

/**
 * Generate a truly unique test username
 * Format: test<timestamp><random>
 * Example: test17320278909823, test17320278906547
 */
export function generateTestUsername(): string {
	const timestamp = Date.now();
	const random = Math.floor(Math.random() * 10000);
	// Combine for uniqueness: even if timestamps collide, random won't
	return `test${timestamp}${random}`;
}

/**
 * Complete the registration flow and return credentials
 */
export async function registerNewAccount(
	page: Page,
): Promise<AuthCredentials> {
	const username = generateTestUsername();

	// Navigate to login page and reveal seed-phrase options. The button is
	// SSR-rendered but its onclick binds on hydration — clickAndRetry (via
	// revealSeedPhraseOptions) handles this deterministically (no networkidle).
	await page.goto('/login');
	await revealSeedPhraseOptions(page);

	// Wait for seed phrase choice to appear and be interactive
	const generateNewButton = page.locator('button:has-text("Generate New")');
	await expect(generateNewButton).toBeVisible({ timeout: 10000 });

	// Click "Generate New" to generate seed phrase
	await generateNewButton.click();

	// Wait for seed phrase to be generated and "Copy to Clipboard" button to appear
	await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });

	// Extract seed phrase from the grid - words are in divs with class containing "font-mono"
	const wordElements = page.locator('.font-mono');
	const words = await wordElements.allTextContents();
	const finalSeedPhrase = words.join(' ').trim();

	expect(finalSeedPhrase.split(' ').length).toBe(12);

	// Check the confirmation checkbox ("I have saved my seed phrase in a secure location")
	await page.check('input[type="checkbox"]');

	// Click Continue button (not "Create Account" - that comes later)
	await page.click('button:has-text("Continue")');

	// Wait for username input to appear (in the "Choose Your Username" step)
	await expect(
		page.locator('input[placeholder="alice"]'),
	).toBeVisible({ timeout: 10000 });

	// Enter username
	await page.fill('input[placeholder="alice"]', username);

	// Wait for validation (username should be available)
	// The UsernameInput component shows "Available" when valid
	await expect(page.getByText('available', { exact: false })).toBeVisible({ timeout: 10000 });

	// Fill email address (required for account creation)
	const testEmail = `${username}@test.example.com`;
	await page.fill('input[placeholder="you@example.com"]', testEmail);

	// Wait for Create Account button to become enabled
	const createButton = page.locator('button:has-text("Create Account")');
	await expect(createButton).toBeEnabled({ timeout: 10000 });

	// Click "Create Account" button
	await createButton.click();

	// Wait for success message
	await expect(
		page.locator('text=Welcome to Decent Cloud'),
	).toBeVisible({ timeout: 15000 });

	// Click the "Go to Dashboard" button
	await page.click('button:has-text("Go to Dashboard")');

	// Verify we're on dashboard
	await expect(page).toHaveURL(/\/dashboard/, { timeout: 10000 });

	return { username, seedPhrase: finalSeedPhrase };
}

/**
 * Click `target` and retry until `success` is satisfied.
 *
 * Replaces waitForLoadState('networkidle') on SSR'd SvelteKit pages: the
 * element is visible in the SSR HTML immediately, but its onclick handler
 * binds only on hydration, so a click before hydration is a silent no-op.
 * Retrying until the click's observable effect appears makes the wait
 * deterministic under Vite HMR, which keeps the network busy and makes
 * networkidle contend across parallel workers.
 *
 * `success` is either a Locator (waited via isVisible) for the common case of
 * an element appearing, or a predicate for navigation-style effects (e.g. a
 * URL change) where no single element marks completion.
 */
export async function clickAndRetry(
	page: Page,
	target: Locator,
	success: Locator | (() => Promise<boolean>),
): Promise<void> {
	const check =
		typeof success === 'function' ? success : () => success.isVisible();
	for (let attempt = 0; attempt < 20; attempt++) {
		await target.click({ timeout: 5000 }).catch(() => {});
		if (await check().catch(() => false)) return;
		await page.waitForTimeout(100);
	}
	if (typeof success === 'function') {
		expect(await check(), 'clickAndRetry: readiness never satisfied').toBe(true);
	} else {
		await expect(success).toBeVisible({ timeout: 10000 });
	}
}

/**
 * On the /login page, click "Sign in with seed phrase instead" and wait for
 * the "Import Existing" option to appear.
 *
 * The button is SSR-rendered (visible immediately) but its onclick handler
 * isn't bound until SvelteKit hydrates. Uses clickAndRetry so a pre-hydration
 * click is retried rather than silently dropped.
 */
export async function revealSeedPhraseOptions(page: Page): Promise<void> {
	await clickAndRetry(
		page,
		page.locator('button:has-text("Sign in with seed phrase instead")'),
		page.locator('button:has-text("Import Existing")'),
	);
}

/**
 * Sign in with existing credentials
 */
export async function signIn(
	page: Page,
	credentials: AuthCredentials,
): Promise<void> {
	// Navigate to login page
	await page.goto('/login');

	// Reveal seed phrase options (click-and-retry instead of networkidle —
	// see revealSeedPhraseOptions for rationale).
	await revealSeedPhraseOptions(page);

	// Click "Import Existing"
	const importButton = page.locator('button:has-text("Import Existing")');
	await importButton.click();

	// Wait for seed phrase textarea
	const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
	await expect(seedInput).toBeVisible({ timeout: 10000 });

	// Enter seed phrase
	await seedInput.fill(credentials.seedPhrase);

	// Click Continue
	await page.click('button:has-text("Continue")');

	// Wait for success message (should auto-login if account exists)
	await expect(
		page.locator('text=Welcome to Decent Cloud'),
	).toBeVisible({ timeout: 10000 });

	// Click "Go to Dashboard"
	await page.click('button:has-text("Go to Dashboard")');

	// Verify we're on dashboard
	await expect(page).toHaveURL(/\/dashboard/);
}

/**
 * Sign out from the application
 */
export async function signOut(page: Page): Promise<void> {
	// Click logout button in sidebar
	await page.click('button:has-text("Logout")');

	// Verify we're back on home page
	await expect(page).toHaveURL('/');
	await expect(page.locator('text=Sign In')).toBeVisible();
}

/**
 * Wait for API request to complete
 */
export async function waitForApiResponse(
	page: Page,
	urlPattern: string | RegExp,
): Promise<void> {
	await page.waitForResponse(
		(response) => {
			const url = response.url();
			const matches =
				typeof urlPattern === 'string'
					? url.includes(urlPattern)
					: urlPattern.test(url);
			return matches && response.status() < 400;
		},
		{ timeout: 10000 },
	);
}

/**
 * Assert that no native browser dialog (alert/confirm/prompt) fires on `page`.
 *
 * Specs that replaced a native `confirm()` with the inline two-step pattern
 * (contact/device/ext-key/social delete, offerings editor replace, …) install
 * this guard so a regression that re-introduces `confirm()` fails loudly
 * instead of being silently auto-dismissed by Playwright. The dialog type is
 * compared to the literal `'never'` so ANY dialog type trips the assertion.
 */
export function assertNoNativeDialog(page: Page): void {
	page.on('dialog', (d) => expect(d.type(), 'native dialog must not fire').toBe('never'));
}

/**
 * Drive the inline two-step confirm pattern used across the dashboard: click
 * the `arm` button → assert the inline Confirm (+ optional `secondary`)
 * button appears → click Confirm → optionally wait for the mutation response.
 *
 * Replaces the per-spec boilerplate of the same shape (inline-confirm-delete's
 * parametrized driver, rentals cancel, …). `row` scopes every button lookup, so
 * pass the card/row locator that uniquely contains the buttons (or `page` when
 * the labels are page-unique). `arm` is the first-click label that reveals the
 * confirm bar; `confirm` defaults to 'Confirm'.
 *
 * `secondary` (e.g. 'Cancel' or 'Abort') is also asserted visible when passed —
 * omit it for surfaces that expose no cancel affordance, or where it would
 * match an unrelated control (page-scoped footer Cancel, etc.). `exact` is
 * forwarded to every `getByRole` lookup; leave it unset for the default
 * case-insensitive substring match. `waitForResponse` is a URL substring; when
 * set, a non-GET response whose URL contains it is awaited — armed BEFORE the
 * Confirm click so the mutation's own response is the one captured.
 */
export async function confirmInlineAction(
	page: Page,
	row: Locator,
	opts: {
		arm: string;
		confirm?: string;
		secondary?: string;
		waitForResponse?: string;
		exact?: boolean;
	},
): Promise<void> {
	const confirmLabel = opts.confirm ?? 'Confirm';
	await row.getByRole('button', { name: opts.arm, exact: opts.exact }).click();
	const confirmBtn = row.getByRole('button', { name: confirmLabel, exact: opts.exact });
	await expect(confirmBtn).toBeVisible();
	if (opts.secondary) {
		await expect(
			row.getByRole('button', { name: opts.secondary, exact: opts.exact }),
		).toBeVisible();
	}
	const responsePromise = opts.waitForResponse
		? page.waitForResponse(
				(r) => r.url().includes(opts.waitForResponse!) && r.request().method() !== 'GET',
				{ timeout: 15000 },
			)
		: undefined;
	await confirmBtn.click();
	if (responsePromise) await responsePromise;
}
