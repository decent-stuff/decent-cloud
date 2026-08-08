import { test, expect, waitForAuthReady } from './fixtures/test-account';

test.describe('/ keyboard shortcut + email banner dismiss', () => {
	test('@smoke / focuses marketplace search input', async ({ page }) => {
		// The '/' handler binds via <svelte:window onkeydown> at hydration, and
		// the page fetches /api/v1/offerings in onMount — so that response is a
		// deterministic hydration signal (registered before goto to avoid a race).
		// Explicit timeout: a bare waitForResponse waits forever, which turns a
		// missed response into a 30s test timeout instead of a fast failure.
		const offeringsReady = page.waitForResponse(
			(r) => r.url().includes('/api/v1/offerings'),
			{ timeout: 15000 },
		);
		await page.goto('/dashboard/marketplace');
		await offeringsReady;

		// Type / — should focus the search input, not insert text
		await page.keyboard.press('/');

		const searchInput = page.locator('#marketplace-search');
		await expect(searchInput).toBeFocused();

		// Now typing should go into the search field
		await page.keyboard.type('gpu');
		await expect(searchInput).toHaveValue('gpu');
	});

	test('/ does not hijack input when already typing in a field', async ({ page }) => {
		const offeringsReady = page.waitForResponse(
			(r) => r.url().includes('/api/v1/offerings'),
			{ timeout: 15000 },
		);
		await page.goto('/dashboard/marketplace');
		await offeringsReady;

		const searchInput = page.locator('#marketplace-search');
		await searchInput.click();
		await searchInput.fill('already here');

		// Move cursor to end, type / — should be inserted as text, not trigger shortcut
		await searchInput.press('End');
		await page.keyboard.type('/');
		await expect(searchInput).toHaveValue('already here/');
	});

	test.describe('? keyboard help overlay', () => {
		// The '?' handler binds via <svelte:window onkeydown> on the dashboard
		// layout, which only hydrates after auth settles. Land on /dashboard
		// and gate on the Logout button (the auth/hydration signal) before the
		// keypress. (The page fixture no longer pre-navigates — see
		// fixtures/test-account.ts.)
		test.beforeEach(async ({ page }) => {
			await page.goto('/dashboard');
			await waitForAuthReady(page);
		});

		test('@smoke ? opens help overlay listing all shortcuts', async ({ page }) => {
			await page.keyboard.press('?');

			const overlay = page.getByTestId('keyboard-help');
			await expect(overlay).toBeVisible();

			// Every documented shortcut must be listed.
			await expect(overlay.getByText('Focus marketplace search')).toBeVisible();
			await expect(overlay.getByText('Open command palette')).toBeVisible();
			await expect(overlay.getByText('Show this help')).toBeVisible();
			await expect(overlay.getByText('Close dialogs/overlays')).toBeVisible();
		});

		test('Esc closes the help overlay', async ({ page }) => {
			await page.keyboard.press('?');
			const overlay = page.getByTestId('keyboard-help');
			await expect(overlay).toBeVisible();

			await page.keyboard.press('Escape');
			await expect(overlay).not.toBeVisible();
		});

		test('? does not trigger while typing in an input', async ({ page }) => {
			// The '/' handler + marketplace search live on the marketplace
			// route; wait for its hydration signal (offerings fetch) before
			// interacting. The help handler guards on activeElement tag.
			const offeringsReady = page.waitForResponse(
				(r) => r.url().includes('/api/v1/offerings'),
				{ timeout: 15000 },
			);
			await page.goto('/dashboard/marketplace');
			await offeringsReady;

			const searchInput = page.locator('#marketplace-search');
			await searchInput.click();

			// Typing '?' into the field must insert the character, not open help.
			await page.keyboard.type('?');
			await expect(searchInput).toHaveValue('?');
			await expect(page.getByTestId('keyboard-help')).toHaveCount(0);
		});
	});

	test('email verification banner can be dismissed per-session', async ({ page }) => {
		// The banner is client-rendered (reads authStore), so toBeVisible already
		// gates hydration — no networkidle needed. The Dismiss button lives in the
		// same component, so it is hydrated once the banner is visible.
		await page.goto('/dashboard');

		// Banner should be visible (test account has unverified email)
		const banner = page.getByText('Verify Your Email Address');
		await expect(banner).toBeVisible({ timeout: 10000 });

		// Dismiss it
		await page.getByRole('button', { name: 'Dismiss reminder' }).click();

		// Banner should disappear
		await expect(banner).not.toBeVisible();

		// Navigate to another page — banner stays dismissed
		await page.goto('/dashboard/account');
		await expect(banner).not.toBeVisible();
	});
});
