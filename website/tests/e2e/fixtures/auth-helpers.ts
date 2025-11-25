import { type Page, expect } from '@playwright/test';

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

	// Navigate to home page
	await page.goto('/');

	// Click "Sign In" button
	await page.click('text=Sign In');

	// Wait for auth dialog to appear
	await expect(page.locator('text=Create Account')).toBeVisible();

	// Click "Create Account"
	await page.click('text=Create Account');

	// Wait for username input (placeholder is "alice")
	await expect(
		page.locator('input[placeholder="alice"]'),
	).toBeVisible();

	// Enter username
	await page.fill('input[placeholder="alice"]', username);

	// Wait for validation (username should be available)
	await expect(page.locator('text=Available').or(page.locator('text=Continue')).first()).toBeVisible({ timeout: 5000 });

	// Click Continue
	await page.click('button:has-text("Continue")');

	// Select "Seed Phrase" auth method
	await expect(page.locator('text=Seed Phrase')).toBeVisible();
	await page.click('text=Seed Phrase');
	await page.click('button:has-text("Continue")');

	// Wait for seed phrase to be generated
	await expect(page.locator('text=Copy to Clipboard')).toBeVisible();

	// Extract seed phrase from the page - words are in span.font-mono elements
	const wordElements = page.locator('span.font-mono');
	const words = await wordElements.allTextContents();
	const finalSeedPhrase = words.join(' ').trim();

	expect(finalSeedPhrase.split(' ').length).toBeGreaterThanOrEqual(12);

	// Confirm backup
	await page.check('input[type="checkbox"]');
	await page.click('button:has-text("Continue")');

	// Confirm account creation - click the button at the bottom of the form
	await expect(page.locator('button:has-text("Create Account")').last()).toBeVisible();
	await page.locator('button:has-text("Create Account")').last().click();

	// Wait for success message
	await expect(
		page.locator('text=Welcome to Decent Cloud!'),
	).toBeVisible({ timeout: 10000 });

	// Click the "Go to Dashboard" button in the success modal
	await page.locator('button:has-text("Go to Dashboard")').click();

	// Verify we're on dashboard
	await expect(page).toHaveURL(/\/dashboard/);

	return { username, seedPhrase: finalSeedPhrase };
}

/**
 * Sign in with existing credentials
 */
export async function signIn(
	page: Page,
	credentials: AuthCredentials,
): Promise<void> {
	// Navigate to home page
	await page.goto('/');

	// Click "Sign In" button
	await page.click('text=Sign In');

	// Wait for auth dialog
	await expect(page.locator('text=Sign In')).toBeVisible();

	// Click "Sign In"
	await page.click('text=Sign In');

	// Select "Seed Phrase" method
	await expect(page.locator('text=Seed Phrase')).toBeVisible();
	await page.click('text=Seed Phrase');
	await page.click('button:has-text("Continue")');

	// Enter seed phrase
	const seedInput = page.locator(
		'textarea[placeholder*="seed" i], input[placeholder*="seed" i]',
	);
	await expect(seedInput).toBeVisible();
	await seedInput.fill(credentials.seedPhrase);
	await page.click('button:has-text("Continue")');

	// Enter username
	await expect(
		page.locator('input[placeholder="alice"]'),
	).toBeVisible();
	await page.fill('input[placeholder="alice"]', credentials.username);
	await page.click('button:has-text("Continue"), button:has-text("Sign In")');

	// Wait for success
	await expect(
		page.locator('text=Welcome').or(page.locator('text=Success')).first(),
	).toBeVisible({ timeout: 10000 });

	// Go to dashboard
	await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

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
