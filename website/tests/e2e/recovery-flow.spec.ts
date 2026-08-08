import { testLoggedOut as test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import {
	seedAccountDirect,
	deleteAccountByUsername,
	accountIdHex,
	seedRecoveryToken,
} from './fixtures/seed-helpers';

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

	test('should show "Lost your seed phrase?" link on login page that navigates to /recover', async ({ page }) => {
		await page.goto('/login');

		// Verify the seed-phrase-specific recovery link is visible
		const recoveryLink = page.locator('a:has-text("Lost your seed phrase? Recover account")');
		await expect(recoveryLink).toBeVisible();

		// Click the link
		await recoveryLink.click();

		// Should navigate to /recover page
		await expect(page).toHaveURL('/recover');
	});

	test('should display email input form on /recover page', async ({ page }) => {
		await page.goto('/recover');

		// Verify page title and description
		await expect(page.getByText('Account Recovery', { exact: true })).toBeVisible();
		await expect(page.locator('h3:has-text("Request Account Recovery")')).toBeVisible();
		await expect(page.getByText('Enter the email address associated with your account', { exact: false })).toBeVisible();

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

		// The form's bind:value + onclick handlers need SvelteKit hydration.
		// fill() before hydration doesn't register in Svelte's state, so the
		// submit silently fails validation. Retry the fill+submit until
		// hydration completes (replaces networkidle, which tanks parallel
		// runs under Vite HMR — see playwright.config.ts:28-33).
		const emailInput = page.locator('input#email[type="email"]');
		const submitButton = page.locator('button:has-text("Send Recovery Link")');
		const successHeading = page.locator('h3:has-text("Check Your Email")');
		for (let attempt = 0; attempt < 20; attempt++) {
			if (await successHeading.isVisible().catch(() => false)) break;
			await emailInput.fill('test@example.com');
			await submitButton.click({ timeout: 5000 }).catch(() => {});
			// Wait for the recovery API response (proves hydration completed and
			// the click registered). Times out → loop retries the fill+submit.
			// Replaces a fixed waitForTimeout sleep (see playwright.config.ts).
			await page
				.waitForResponse((r) => r.url().includes('/api/v1/accounts/recovery/request'), { timeout: 1000 })
				.catch(() => {});
		}
		await expect(successHeading).toBeVisible({ timeout: 5000 });

		// Should show success message
		await expect(page.locator('h3:has-text("Check Your Email")')).toBeVisible({ timeout: 5000 });
		await expect(page.locator('text=If an account exists with this email, a recovery link has been sent')).toBeVisible();

		// Should show the mail success icon (Icon name="mail" inside .icon-box-accent;
		// previously a literal ✉️ emoji, now an SVG Icon component)
		await expect(page.locator('.icon-box-accent')).toBeVisible();

		// Should show option to send to different email
		await expect(page.locator('button:has-text("Send to a different email")')).toBeVisible();
	});

	test('should validate email field is required', async ({ page }) => {
		await page.goto('/recover');

		// Try to submit without entering email
		const submitButton = page.locator('button:has-text("Send Recovery Link")');
		await expect(submitButton).toBeVisible();
		await submitButton.click();

		// HTML5 validation should prevent submission
		// The form should still be visible (not navigated away)
		await expect(page.locator('h3:has-text("Request Account Recovery")')).toBeVisible();
	});

	test('should allow sending to different email after success', async ({ page }) => {
		await page.goto('/recover');

		// Submit first email (retry fill+submit until hydration completes —
		// see the submit test above for rationale).
		const emailInput = page.locator('input#email[type="email"]');
		const submitButton = page.locator('button:has-text("Send Recovery Link")');
		const successHeading = page.locator('h3:has-text("Check Your Email")');
		for (let attempt = 0; attempt < 20; attempt++) {
			if (await successHeading.isVisible().catch(() => false)) break;
			await emailInput.fill('first@example.com');
			await submitButton.click({ timeout: 5000 }).catch(() => {});
			// Wait for the recovery API response (proves hydration completed and
			// the click registered). Times out → loop retries the fill+submit.
			// Replaces a fixed waitForTimeout sleep (see playwright.config.ts).
			await page
				.waitForResponse((r) => r.url().includes('/api/v1/accounts/recovery/request'), { timeout: 1000 })
				.catch(() => {});
		}
		await expect(successHeading).toBeVisible({ timeout: 5000 });

		// Wait for success
		await expect(page.locator('h3:has-text("Check Your Email")')).toBeVisible({ timeout: 5000 });

		// Click "Send to a different email"
		const differentEmailButton = page.locator('button:has-text("Send to a different email")');
		await expect(differentEmailButton).toBeVisible();
		await differentEmailButton.click();

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

		// Should show auto-generated seed phrase (no mode choice when token provided)
		// The SeedPhraseStep is initialized with initialMode="generate" and showModeChoice=false
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });
	});

	test('should complete recovery flow with token and surface API error for a fake token', async ({ page }) => {
		// Navigate with token. The token is intentionally fake — the test
		// verifies the Continue → onComplete → handleSeedComplete → completeRecovery
		// wiring fires and the backend's rejection is surfaced to the user
		// (proving the flow does NOT dead-end at the SeedPhraseStep Continue).
		await page.goto('/recover?token=test-recovery-token-123');

		// Wait for seed phrase step - auto-generates when token is provided
		await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });

		// Seed phrase is auto-generated (no mode choice when token provided)
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });

		// Check the confirmation checkbox. The Continue button is disabled
		// (via `disabled={!seedBackedUp}`) until the bind:checked propagates;
		// Playwright's click() actionability wait handles hydration timing.
		await page.check('input[type="checkbox"]');

		// Click Continue and wait for the recovery completion API call. The
		// waitForResponse proves the Continue click reached the backend — i.e.
		// onComplete fired. The fake token ("test-recovery-token-123") is not
		// valid hex, so the API returns success:false with a hex-decode error.
		await Promise.all([
			page.waitForResponse(
				(r) => r.url().includes('/api/v1/accounts/recovery/complete'),
				{ timeout: 10000 },
			),
			page.click('button:has-text("Continue")'),
		]);

		// handleSeedComplete catches the API error and surfaces it in the
		// recover page's error div (class `bg-danger/10` — distinct from the
		// SeedPhraseStep's inline `bg-red-500/20`, which is for component-local
		// validation only). Asserting the recover page's error div proves the
		// flow transitioned generate-seed → processing → generate-seed with
		// error, refuting the #446 hypothesis that handleSeedComplete was
		// never invoked.
		await expect(page.locator('.bg-danger\\/10')).toBeVisible({ timeout: 10000 });
	});

	test('should show error message when completing recovery with invalid token', async ({ page }) => {
		// Navigate with invalid token
		await page.goto('/recover?token=invalid-token-that-does-not-exist');

		// Wait for seed phrase step - auto-generates when token is provided
		await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });

		// Seed phrase is auto-generated (no mode choice when token provided)
		await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });

		// Check the confirmation checkbox and click Continue. The Continue
		// button is disabled until the checkbox binding propagates; Playwright's
		// actionability wait handles hydration timing.
		await page.check('input[type="checkbox"]');

		// Wait for the API call (proves the click reached the backend) and
		// assert the backend's hex-decode error surfaces visibly. The recover
		// page uses class `bg-danger/10` for its error div, NOT the
		// SeedPhraseStep's `bg-red-500/20` (component-local validation only).
		await Promise.all([
			page.waitForResponse(
				(r) => r.url().includes('/api/v1/accounts/recovery/complete'),
				{ timeout: 10000 },
			),
			page.click('button:has-text("Continue")'),
		]);

		await expect(page.locator('.bg-danger\\/10')).toBeVisible({ timeout: 10000 });
		// Confirm the error specifically references the invalid token (the API
		// returns "Invalid recovery token hex: ..." — proves the rejection came
		// from the backend, not a frontend validation guard).
		await expect(page.getByText(/Invalid recovery token/i)).toBeVisible();
	});

	test('should navigate back to login from /recover page', async ({ page }) => {
		await page.goto('/recover');

		// The "← Back to login" is a native <a> link — works without
		// SvelteKit hydration. Wait for it to be visible, then click.
		const backLink = page.locator('a:has-text("← Back to login")');
		await expect(backLink).toBeVisible();
		await backLink.click();

		// Should navigate to /login
		await expect(page).toHaveURL('/login');
		// Verify the login page rendered. "Import Existing" is hidden behind the
		// "Sign in with seed phrase instead" CTA, so assert against that button.
		await expect(
			page.locator('button:has-text("Sign in with seed phrase instead")'),
		).toBeVisible();
	});

	test('success: a valid DB-seeded token completes recovery, auto-logs-in, and shows the auto-redirect countdown (#445)', async ({ page }) => {
		// Seed a standalone account + recovery token so the test exercises the
		// REAL recovery success path end-to-end (token in URL → continue with
		// the auto-generated seed → completeRecovery API → authStore login →
		// success state with auto-redirect countdown).
		const { username } = await seedAccountDirect();
		try {
			const accountHex = await accountIdHex(username);
			const token = await seedRecoveryToken(accountHex);

			await page.goto(`/recover?token=${token}`);

			// Wait for seed phrase step - auto-generates when token is provided
			await expect(page.locator('h3:has-text("Complete Recovery")')).toBeVisible({ timeout: 5000 });
			await expect(page.locator('button:has-text("Copy to Clipboard")')).toBeVisible({ timeout: 10000 });

			// Check the confirmation checkbox and click Continue. The complete
			// call must succeed (token is valid hex + DB-seeded + unused + not
			// expired), then loginWithSeedPhrase must succeed (completeRecovery
			// added the auto-generated pubkey to account_public_keys).
			await page.check('input[type="checkbox"]');
			await Promise.all([
				page.waitForResponse(
					(r) => r.url().includes('/api/v1/accounts/recovery/complete'),
					{ timeout: 10000 },
				),
				page.click('button:has-text("Continue")'),
			]);

			// Success state heading.
			await expect(page.getByRole('heading', { name: 'Recovery Complete!' })).toBeVisible({
				timeout: 15000,
			});

			// The AutoRedirect countdown copy must render on the success screen.
			// We match the prefix (not the exact number) since it decrements.
			await expect(page.getByText(/Redirecting to dashboard in \d+s/)).toBeVisible();

			// The manual "Go now" link provides the accessibility escape hatch
			// (the auto-redirect timer must never be the only path). Clicking it
			// lands on /dashboard without waiting for the countdown.
			const goNowLink = page.getByRole('link', { name: 'Go now' });
			await expect(goNowLink).toBeVisible();
			await goNowLink.click();
			await expect(page).toHaveURL(/\/dashboard/, { timeout: 10000 });
		} finally {
			await deleteAccountByUsername(username);
		}
	});
});
