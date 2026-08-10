import { test, expect } from '@playwright/test';
import {
	seedMarketplaceOffering,
	deleteOfferingById,
} from './fixtures/seed-helpers';

/**
 * E2E tests for marketplace sort controls (#439, #marketplace-buy-flow).
 *
 * Regression: the desktop sort pills were wrapped in `hidden md:flex`, so mobile
 * users (viewport < 768px) had no way to change sort order. The fix adds a
 * `<select>` visible on mobile that shares the same `sortField` / `sortDir`
 * state and `syncFiltersToUrl` path as the desktop pills.
 *
 * Desktop contract (buy-flow fix): the pills are the ONLY sort affordance on
 * desktop — the `<select>` is `md:hidden` there so the two controls never
 * duplicate on the same screen and keyboard users get no phantom focus on a
 * hidden element. The pills are real `<button>`s, so desktop a11y is fully
 * served by them. The `<select>` is the sole affordance on mobile.
 *
 * The drop-demos pivot (migration 053) left the marketplace empty, so this spec
 * self-seeds two is_example (demo) offerings under the example provider pubkey.
 * They're self_provisioned (online) so they pass the marketplace query's
 * pool/self-provisioned filter, and is_example so they stay out of the DEFAULT
 * view (hidden by showDemoOfferings=false) — which keeps the
 * marketplace-empty-state spec on another worker unaffected. ?demo=1 reveals
 * them (the offline flag is now a harmless no-op for these online rows).
 */

const MARKETPLACE_URL = '/dashboard/marketplace?demo=1&offline=1';

// Distinct mobile and desktop viewports — the bug only manifests below the
// Tailwind `md` breakpoint (768px).
const MOBILE = { width: 375, height: 812 } as const;
const DESKTOP = { width: 1280, height: 800 } as const;

test.describe('Marketplace sort', () => {
	const seededIds: string[] = [];
	test.beforeAll(async () => {
		// Two demo offerings with distinct prices so a sort-control change has
		// rows to operate on. is_example + self_provisioned → hidden by the
		// default demo filter but surfaced by ?demo=1.
		for (const price of [10, 50]) {
			const handle = await seedMarketplaceOffering({
				isExample: true,
				online: true,
				name: `E2E Sort Offering ${price}`,
				monthlyPrice: price,
			});
			seededIds.push(handle.offeringNumericId);
		}
	});
	test.afterAll(async () => {
		await Promise.all(seededIds.map((id) => deleteOfferingById(id)));
	});

	test.beforeEach(async ({ page }) => {
		await page.goto(MARKETPLACE_URL);
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		// Wait for at least one offering to render before asserting on controls.
		await expect(page.locator('tbody tr[id^="offering-"]').first()).toBeVisible({ timeout: 15000 });
	});

	test('mobile viewport exposes a sort <select> that syncs to URL (#439)', async ({ page }) => {
		// Reproduce the original bug: at 375px the sort pills are wrapped in
		// `hidden md:flex` and are therefore invisible. A `<select>` must be
		// visible instead and must drive the same sortField/sortDir state.
		await page.setViewportSize(MOBILE);

		const sortSelect = page.getByRole('combobox', { name: /sort/i });
		await expect(sortSelect).toBeVisible();

		// Desktop pills must remain hidden on mobile so the two controls don't
		// compete for the same screen region.
		await expect(page.getByRole('button', { name: /^Price ↑$/ })).toBeHidden();

		// Changing the select must update sortField/sortDir and propagate to the
		// URL via the existing syncFiltersToUrl() machinery. Price is the default
		// sortField, so syncFiltersToUrl omits `sort=` for it and only writes
		// `dir=desc` when descending.
		await sortSelect.selectOption('Price ↓');
		await expect(page).not.toHaveURL(/\bsort=/);
		await expect(page).toHaveURL(/\bdir=desc\b/);

		// Non-default sortField must write `sort=` explicitly.
		await sortSelect.selectOption('Reliability ↓');
		await expect(page).toHaveURL(/\bsort=trust\b/);
	});

	test('desktop uses ONLY the pill UI; the <select> is hidden (buy-flow fix)', async ({ page }) => {
		// Desktop must keep the pill UI as the SOLE sort affordance. The <select>
		// is `md:hidden` on desktop so the two never duplicate (regression: both
		// used to render, creating two competing sort controls). The pills are
		// real <button>s, so keyboard/AT users are served by them.
		await page.setViewportSize(DESKTOP);

		await expect(page.getByRole('button', { name: /^Price ↑$/ })).toBeVisible();
		await expect(page.getByRole('button', { name: /^Price ↓$/ })).toBeVisible();
		await expect(page.getByRole('button', { name: /^Reliability ↓$/ })).toBeVisible();

		// The <select> must NOT render on desktop (no duplicate control).
		const sortSelect = page.getByRole('combobox', { name: /sort/i });
		await expect(sortSelect).toBeHidden();

		// Driving a pill must update the shared sort state and the URL.
		await page.getByRole('button', { name: /^Reliability ↓$/ }).click();
		await expect(page.getByRole('button', { name: /^Reliability ↓$/ })).toHaveClass(/bg-primary-500/);
		await expect(page).toHaveURL(/\bsort=trust\b/);
	});

	test('select and pills stay in sync across viewport switches (single source of truth)', async ({ page }) => {
		// The pills (desktop) and the <select> (mobile) are two views of the same
		// sortField/sortDir state. Driving one must be reflected by the other when
		// the viewport changes — proving they never disagree.
		// 1. Drive via a pill on desktop.
		await page.setViewportSize(DESKTOP);
		await page.getByRole('button', { name: /^Price ↓$/ }).click();
		await expect(page).toHaveURL(/\bdir=desc\b/);

		// 2. Shrink to mobile: the <select> is now the visible affordance and must
		//    reflect the descending sort the pill just applied.
		await page.setViewportSize(MOBILE);
		const sortSelect = page.getByRole('combobox', { name: /sort/i });
		await expect(sortSelect).toBeVisible();
		await expect(sortSelect).toHaveValue('Price ↓');

		// 3. Drive via the select on mobile, then grow back to desktop: the pills
		//    must reflect the select's change.
		await sortSelect.selectOption('Reliability ↓');
		await expect(page).toHaveURL(/\bsort=trust\b/);
		await page.setViewportSize(DESKTOP);
		await expect(page.getByRole('button', { name: /^Reliability ↓$/ })).toHaveClass(/bg-primary-500/);
	});
});
