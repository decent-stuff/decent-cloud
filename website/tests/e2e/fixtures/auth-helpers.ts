import { type Page, expect } from '@playwright/test';

/**
 * Test helper utilities for authentication flows
 */

export interface AuthCredentials {
	username: string;
	seedPhrase: string;
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

	// Click "Connect Wallet" button
	await page.click('text=Connect Wallet');

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
	await expect(page.locator('text=Available').or(page.locator('text=Continue'))).toBeVisible({ timeout: 5000 });

	// Click Continue
	await page.click('button:has-text("Continue")');

	// Select "Seed Phrase" auth method
	await expect(page.locator('text=Seed Phrase')).toBeVisible();
	await page.click('text=Seed Phrase');
	await page.click('button:has-text("Continue")');

	// Wait for seed phrase to be generated
	await expect(page.locator('.seed-phrase, [class*="seed"]').or(page.locator('text=Copy to Clipboard'))).toBeVisible();

	// Extract seed phrase from the page
	const seedPhraseElement = page.locator('.seed-phrase, [class*="seed"]').first();
	const seedPhrase = (await seedPhraseElement.textContent()) || '';

	// If seed phrase is empty, try to get it from individual word elements
	let finalSeedPhrase = seedPhrase.trim();
	if (!finalSeedPhrase) {
		const words = await page.locator('[class*="word"], .word').allTextContents();
		finalSeedPhrase = words.join(' ').trim();
	}

	expect(finalSeedPhrase.split(' ').length).toBeGreaterThanOrEqual(12);

	// Confirm backup
	await page.check('input[type="checkbox"]');
	await page.click('button:has-text("Continue")');

	// Confirm account creation
	await expect(page.locator('text=Create Account')).toBeVisible();
	await page.click('button:has-text("Create Account")');

	// Wait for success
	await expect(
		page.locator('text=Welcome').or(page.locator('text=Success')),
	).toBeVisible({ timeout: 10000 });

	// Go to dashboard
	await page.click('button:has-text("Go to Dashboard"), a:has-text("Dashboard")');

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

	// Click "Connect Wallet" button
	await page.click('text=Connect Wallet');

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
		page.locator('text=Welcome').or(page.locator('text=Success')),
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
	await expect(page.locator('text=Connect Wallet')).toBeVisible();
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
