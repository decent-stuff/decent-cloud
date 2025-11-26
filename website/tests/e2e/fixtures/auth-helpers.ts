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

	// Navigate to login page
	await page.goto('/login');

	// Wait for seed phrase choice to appear
	await expect(page.locator('text=Generate New')).toBeVisible({ timeout: 10000 });

	// Click "Generate New" to generate seed phrase
	await page.click('text=Generate New');

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
	await expect(page.locator('text=Available').or(page.locator('button:has-text("Create Account")')).first()).toBeVisible({ timeout: 10000 });

	// Click "Create Account" button
	await page.click('button:has-text("Create Account")');

	// Wait for success message
	await expect(
		page.locator('text=Welcome to Decent Cloud!'),
	).toBeVisible({ timeout: 15000 });

	// Click the "Go to Dashboard" button
	await page.click('button:has-text("Go to Dashboard")');

	// Verify we're on dashboard
	await expect(page).toHaveURL(/\/dashboard/, { timeout: 10000 });

	return { username, seedPhrase: finalSeedPhrase };
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

	// Wait for seed phrase choice to appear
	await expect(page.locator('text=Import Existing')).toBeVisible();

	// Click "Import Existing"
	await page.click('text=Import Existing');

	// Wait for seed phrase textarea
	const seedInput = page.locator('textarea[placeholder*="word1 word2 word3"]');
	await expect(seedInput).toBeVisible();

	// Enter seed phrase
	await seedInput.fill(credentials.seedPhrase);

	// Click Continue
	await page.click('button:has-text("Continue")');

	// Wait for success message (should auto-login if account exists)
	await expect(
		page.locator('text=Welcome to Decent Cloud!'),
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
