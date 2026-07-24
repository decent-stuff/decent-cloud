import { test, expect } from './fixtures/test-account';

test.describe('/ keyboard shortcut + email banner dismiss', () => {
	test('@smoke / focuses marketplace search input', async ({ page }) => {
		// The '/' handler binds via <svelte:window onkeydown> at hydration, and
		// the page fetches /api/v1/offerings in onMount — so that response is a
		// deterministic hydration signal (registered before goto to avoid a race).
		const offeringsReady = page.waitForResponse((r) =>
			r.url().includes('/api/v1/offerings'),
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
		const offeringsReady = page.waitForResponse((r) =>
			r.url().includes('/api/v1/offerings'),
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
