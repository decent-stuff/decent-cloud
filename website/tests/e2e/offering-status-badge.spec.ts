import { test, expect } from '@playwright/test';

/**
 * E2E coverage for the OfferingStatusBadge component's keyboard a11y.
 *
 * Audit #15: the "more details" tooltip opened only on mouseenter/mouseleave
 * (+ click toggle), so keyboard-only users navigating with Tab could not
 * reach the tooltip content (Trust score, Subscription, Has setup recipe,
 * Has warnings). The fix adds onfocus/onblur handlers, aria-describedby
 * pointing at the tooltip, and Escape-to-close.
 *
 * The badge is rendered on the marketplace page for every offering that
 * carries trust/subscription/recipe/warnings metadata. We load the
 * marketplace with demo + offline offerings visible (per search-dsl.spec.ts
 * pattern) so there's at least one badge with a tooltip to test against.
 */

const MARKETPLACE_URL = '/dashboard/marketplace?demo=1&offline=1';

test.describe('OfferingStatusBadge keyboard a11y', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(MARKETPLACE_URL);
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		await expect(page.locator('tbody tr[id^="offering-"]').first()).toBeVisible({ timeout: 15000 });
	});

	test('tooltip becomes visible when the badge button receives focus (#15)', async ({ page }) => {
		// Find a badge button that exposes a tooltip (it has aria-label="More details"
		// and is only rendered when there's additional info to show).
		const badgeButton = page.getByRole('button', { name: 'More details' }).first();
		await expect(badgeButton).toBeVisible();

		// Before focus, the tooltip node may or may not be in the DOM (Svelte
		// renders it conditionally on showTooltip). Either way it should not be
		// visible to a screen-reader-by-sight user.
		// Focus the badge button via keyboard-equivalent action (Playwright .focus()
		// dispatches the same focus event a real Tab would).
		await badgeButton.focus();

		// The tooltip must become visible. It carries role="tooltip".
		const tooltip = page.getByRole('tooltip').first();
		await expect(tooltip).toBeVisible({ timeout: 2000 });

		// The button must expose an aria-describedby pointing at the tooltip so
		// screen readers announce the same content keyboard users now see.
		const describedBy = await badgeButton.getAttribute('aria-describedby');
		expect(describedBy, 'badge button must have aria-describedby').toBeTruthy();
		const tooltipId = await tooltip.getAttribute('id');
		expect(describedBy).toContain(tooltipId);
	});

	test('Escape closes the tooltip while the badge retains focus (#15)', async ({ page }) => {
		const badgeButton = page.getByRole('button', { name: 'More details' }).first();
		await badgeButton.focus();
		const tooltip = page.getByRole('tooltip').first();
		await expect(tooltip).toBeVisible({ timeout: 2000 });

		// Press Escape — the tooltip should close.
		await page.keyboard.press('Escape');
		await expect(tooltip).toHaveCount(0);
	});
});
