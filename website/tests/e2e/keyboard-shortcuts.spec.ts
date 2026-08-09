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

	test('email verification action stays dismissed across navigation (per-session)', async ({ page }) => {
		// After A8/UX-009 the email reminder lives inside the consolidated
		// ActionRequiredBanner (data-testid="action-required-banner"). The
		// test-account identity has an unverified email, so the indicator
		// surfaces a 'verify your email' action. Dismissing it persists for the
		// browser session (sessionStorage), so it must NOT reappear on a
		// subsequent dashboard navigation. (dashboard-banners.spec.ts covers the
		// collapsed/expanded presentation; this guards the per-session part.)
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		const indicator = page.getByTestId('action-required-banner');
		await expect(indicator).toBeVisible({ timeout: 10000 });
		await expect(indicator).toContainText('verify your email');

		// Expand to reveal the per-action CTAs and dismiss only the email action.
		await page.getByRole('button', { name: 'Review' }).click();
		await page
			.getByRole('button', { name: 'Dismiss verify your email', exact: true })
			.click();

		// Email action dismissed; the indicator must no longer mention it.
		await expect(indicator).not.toContainText('verify your email');

		// Navigate to another banner surface — the per-session dismissal persists.
		await page.goto('/dashboard/account');
		await expect(indicator).toBeVisible({ timeout: 10000 });
		await expect(indicator).not.toContainText('verify your email');
	});
});
