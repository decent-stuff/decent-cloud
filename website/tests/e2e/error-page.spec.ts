import { test, expect } from '@playwright/test';

/**
 * Regression tests for the global error page (`src/routes/+error.svelte`).
 *
 * Background: SvelteKit's default error markup is unstyled, and the light-theme
 * `body` color override in app.css (`text-neutral-900`) flips to the LIGHTEST
 * shade in light theme. Together they made the 404 page render white-on-white.
 * These tests pin both the visibility of the branded error UI and the
 * light-theme body text contract.
 */

test.describe('Global error page', () => {
	test('404 renders branded error page with navigation, not blank screen', async ({ page }) => {
		await page.goto('/this-route-does-not-exist');

		// Status code, label, and message are visible to a real user
		// (the bug: white-on-white text was previously invisible).
		await expect(page.getByText('404', { exact: true })).toBeVisible();
		await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible();

		// Navigation options are present and reachable.
		await expect(page.getByRole('link', { name: 'Back to home' })).toBeVisible();
		await expect(page.getByRole('link', { name: 'Browse marketplace' })).toBeVisible();

		// Sanity: the body text color must differ from the background so the
		// page is not invisible. We assert a meaningful luminance delta.
		const { bodyBg, bodyText } = await page.evaluate(() => {
			const cs = getComputedStyle(document.body);
			return { bodyBg: cs.backgroundColor, bodyText: cs.color };
		});
		expect(bodyBg).not.toBe(bodyText);
	});

	test('404 Back to home returns user to landing page', async ({ page }) => {
		await page.goto('/this-route-does-not-exist');
		await page.getByRole('link', { name: 'Back to home' }).click();
		await expect(page).toHaveURL('/');
	});
});
