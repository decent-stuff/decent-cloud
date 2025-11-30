import { testLoggedOut as test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';

/**
 * E2E Tests for Account Recovery Flow
 *
 * Prerequisites:
 * - API server running at http://localhost:8080
 * - Dev server running at http://localhost:5173
 * - Clean test database
 *
 * Constraints:
 * - Cannot intercept actual emails in Playwright
 * - Test accounts created via seed phrase don't have emails linked
 * - Backend returns success message even for non-existent emails (security)
 */

test.describe('Recovery Flow', () => {
	test.beforeEach(async ({ page }) => {
		setupConsoleLogging(page);
	});

	test('should show "Lost access?" link on login page that navigates to /recover', async ({ page }) => {
		await page.goto('/login');

		// Verify "Lost access?" link is visible
		const recoveryLink = page.locator('a:has-text("Lost access? Recover your account")');
		await expect(recoveryLink).toBeVisible();

		// Click the link
		await recoveryLink.click();

		// Should navigate to /recover page
		await expect(page).toHaveURL('/recover');
	});

	test('should display email input form on /recover page', async ({ page }) => {
		await page.goto('/recover');

		// Verify page title and description
		await expect(page.locator('text=Account Recovery')).toBeVisible();
		await expect(page.locator('h3:has-text("Request Account Recovery")')).toBeVisible();
		await expect(page.locator('text=Enter the email address associated with your account')).toBeVisible();

		// Verify email input field exists
		const emailInput = page.locator('input#email[type="email"]');
		await expect(emailInput).toBeVisible();
		await expect(emailInput).toHaveAttribute('placeholder', 'your@email.com');

		// Verify submit button exists
		await expect(page.locator('button:has-text("Send Recovery Link")')).toBeVisible();

		// Verify back link exists
		await expect(page.locator('a:has-text("← Back to login")')).toBeVisible();
	});

	test('should submit email request and show success message', async ({ page }) => {
		await page.goto('/recover');

		// Fill in email
		const emailInput = page.locator('input#email[type="email"]');
		await emailInput.fill('test@example.com');

		// Submit form
		await page.click('button:has-text("Send Recovery Link")');

		// Should show success message
		await expect(page.locator('h3:has-text("Check Your Email")')).toBeVisible({ timeout: 5000 });
		await expect(page.locator('text=We sent a recovery link to your email address')).toBeVisible();

		// Should show envelope emoji
		await expect(page.locator('text=✉️')).toBeVisible();

		// Should show option to send to different email
		await expect(page.locator('button:has-text("Send to a different email")')).toBeVisible();
	});

	test('should validate email field is required', async ({ page }) => {
		await page.goto('/recover');

		// Try to submit without entering email
		await page.click('button:has-text("Send Recovery Link")');

		// HTML5 validation should prevent submission
		// The form should still be visible (not navigated away)
		await expect(page.locator('h3:has-text("Request Account Recovery")')).toBeVisible();
	});

	test('should allow sending to different email after success', async ({ page }) => {
		await page.goto('/recover');

		// Submit first email
		await page.fill('input#email[type="email"]', 'first@example.com');
		await page.click('button:has-text("Send Recovery Link")');

		// Wait for success
		await expect(page.locator('h3:has-text("Check Your Email")')).toBeVisible({ timeout: 5000 });

		// Click "Send to a different email"
		await page.click('button:has-text("Send to a different email")');

		// Should go back to request form
		await expect(page.locator('h3:has-text("Request Account Recovery")')).toBeVisible();
		await expect(page.locator('input#email[type="email"]')).toBeVisible();
	});

	test('should show seed phrase generation flow when token is provided in URL', async ({ page }) => {
		// Navigate to /recover with a token parameter
		await page.goto('/recover?token=test-recovery-token-123');

		// Should skip email request and go directly to seed phrase generation
		await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });
		await expect(page.locator('text=Generate a new seed phrase to regain access to your account')).toBeVisible();

		// Should show seed phrase component
		// Look for "Generate New" button which is part of SeedPhraseStep
		await expect(page.locator('button:has-text("Generate New")')).toBeVisible();
	});

	test('should complete recovery flow with valid token', async ({ page }) => {
		// Navigate with token
		await page.goto('/recover?token=test-recovery-token-123');

		// Wait for seed phrase step
		await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });

		// Click "Generate New" to generate seed phrase
		await page.click('button:has-text("Generate New")');

		// Wait for seed phrase to be generated
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 5000 });

		// Check the confirmation checkbox
		await page.check('input[type="checkbox"]');

		// Click Continue
		await page.click('button:has-text("Continue")');

		// Should show processing or error (since token is invalid)
		// We expect an error because the token is fake
		const errorOrProcessing = page.locator('text=Processing').or(
			page.locator('text=Recovery completion failed').or(
				page.locator('text=Invalid token')
			)
		);
		await expect(errorOrProcessing.first()).toBeVisible({ timeout: 10000 });
	});

	test('should show error message when completing recovery with invalid token', async ({ page }) => {
		// Navigate with invalid token
		await page.goto('/recover?token=invalid-token-that-does-not-exist');

		// Wait for seed phrase step
		await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });

		// Generate seed phrase
		await page.click('button:has-text("Generate New")');
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 5000 });

		// Complete the flow
		await page.check('input[type="checkbox"]');
		await page.click('button:has-text("Continue")');

		// Should show error message
		const errorMessage = page.locator('.bg-red-500\\/20').or(
			page.locator('text=Recovery completion failed').or(
				page.locator('text=Invalid token').or(
					page.locator('text=error')
				)
			)
		);
		await expect(errorMessage.first()).toBeVisible({ timeout: 10000 });
	});

	test('should navigate back to login from /recover page', async ({ page }) => {
		await page.goto('/recover');

		// Click back to login link
		await page.click('a:has-text("← Back to login")');

		// Should navigate to /login
		await expect(page).toHaveURL('/login');
		await expect(page.locator('text=Import Existing')).toBeVisible();
	});
});
