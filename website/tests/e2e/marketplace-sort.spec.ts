import { test, expect } from '@playwright/test';

/**
 * E2E tests for marketplace sort controls (#439).
 *
 * Regression: the desktop sort pills were wrapped in `hidden md:flex`, so mobile
 * users (viewport < 768px) had no way to change sort order. The fix adds a
 * `<select>` visible on mobile (and as an a11y alternative on desktop) that
 * shares the same `sortField` / `sortDir` state and `syncFiltersToUrl` path as
 * the desktop pills.
 *
 * The dev DB ships only offline demo offerings, so we load the marketplace with
 * `?demo=1&offline=1` (same pattern as search-dsl.spec.ts).
 */

const MARKETPLACE_URL = '/dashboard/marketplace?demo=1&offline=1';

// Distinct mobile and desktop viewports — the bug only manifests below the
// Tailwind `md` breakpoint (768px).
const MOBILE = { width: 375, height: 812 } as const;
const DESKTOP = { width: 1280, height: 800 } as const;

test.describe('Marketplace sort', () => {
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

	test('desktop keeps the pill UI and exposes the <select> as an a11y alternative (#439)', async ({ page }) => {
		// Desktop must keep the original pill UI (regression guard) AND expose
		// the new <select> as a keyboard/screen-reader-friendly alternative.
		await page.setViewportSize(DESKTOP);

		await expect(page.getByRole('button', { name: /^Price ↑$/ })).toBeVisible();
		await expect(page.getByRole('button', { name: /^Price ↓$/ })).toBeVisible();
		await expect(page.getByRole('button', { name: /^Reliability ↓$/ })).toBeVisible();

		// The <select> must remain reachable on desktop as well.
		const sortSelect = page.getByRole('combobox', { name: /sort/i });
		await expect(sortSelect).toBeVisible();

		// Driving the select must keep the pills in sync (single source of truth).
		await sortSelect.selectOption('Reliability ↓');
		await expect(page.getByRole('button', { name: /^Reliability ↓$/ })).toHaveClass(/bg-primary-500/);
		await expect(page).toHaveURL(/\bsort=trust\b/);
	});

	test('select and pills stay in sync when either changes (#439)', async ({ page }) => {
		// Single source of truth: changing one control must update the other.
		await page.setViewportSize(DESKTOP);

		// Drive via pill first.
		await page.getByRole('button', { name: /^Price ↓$/ }).click();
		const sortSelect = page.getByRole('combobox', { name: /sort/i });
		await expect(sortSelect).toHaveValue('Price ↓');

		// Then drive via select.
		await sortSelect.selectOption('Price ↑');
		await expect(page.getByRole('button', { name: /^Price ↑$/ })).toHaveClass(/bg-primary-500/);
	});
});
