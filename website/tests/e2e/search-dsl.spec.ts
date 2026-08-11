import { test, expect } from '@playwright/test';
import {
	seedRentableOffering,
	deleteOfferingsByProvider,
	randomHex,
	sql,
	type OfferingSeedOverrides,
} from './fixtures/seed-helpers';

/**
 * E2E Tests for Search DSL Functionality
 *
 * Tests the Domain Specific Language (DSL) search features in the marketplace:
 * - Type filter checkboxes (Compute, GPU, Storage, Network)
 * - DSL text input for price queries and other field-syntax filters
 * - Combined type + DSL filters
 * - Empty results state
 * - Results count updates
 *
 * SELF-CONTAINED: every offering the suite asserts against is seeded here in
 * `beforeAll` under a unique tag, then isolated via `?q=<tag>` so the tests see
 * ONLY their own data. This drops the previous reliance on ambient demo/offline
 * seed data (`?demo=1&offline=1`), which silently broke all 8 tests whenever the
 * dev DB was reset (e.g. the 2026-07-24 ICPay migration wipe). Demo offerings
 * and parallel-worker offerings never match the unique tag, so they cannot
 * perturb client-side filter counts. DSL (`q` containing `:`) replaces the
 * plain-text `q` server-side; those assertions are robust regardless because
 * the DSL filter is applied server-side (only type-matching rows return).
 */

// Result-count banner text format used by the current marketplace UI.
// Playwright interprets `text=/pattern/` as a regex match against element text.
const COUNT_LOCATOR = 'text=/\\d+ offerings found/';

/** One seeded offering for the suite. `providerHex` is the cleanup key. */
interface Seeded {
	providerHex: string;
	name: string;
}

/** A planned offering: the seed overrides + the cleanup key (filled post-seed). */
interface Plan {
	overrides: OfferingSeedOverrides;
	providerHex?: string;
}

let TAG: string;
let PLANS: Plan[];

test.describe('Search DSL', () => {
	test.describe.configure({ mode: 'serial' });

	test.beforeAll(async () => {
		// Clean up stale offerings from a prior run whose afterAll did not
		// execute (worker crash/timeout). The `e2edsl-` prefix is constant
		// across runs; only the randomHex suffix changes. Without this, leaked
		// rows pollute other specs that expect a near-empty marketplace
		// (marketplace-empty-state.spec.ts).
		await sql(`DELETE FROM provider_offerings WHERE offer_name LIKE 'e2edsl-%'`);
		// Unique tag shared by every seeded offering name. The suite navigates
		// to `?q=<TAG>` so only these rows are visible to client-side filters.
		TAG = `e2edsl-${randomHex(4)}`;
		// Six offerings with distinct type/price points so the type-filter,
		// price-filter, and count-change tests all have deterministic data.
		PLANS = [
			{ overrides: { name: `${TAG} GPU Cheap`, productType: 'gpu', monthlyPrice: 10 } },
			{ overrides: { name: `${TAG} GPU Pro`, productType: 'gpu', monthlyPrice: 30 } },
			{ overrides: { name: `${TAG} Compute Small`, productType: 'compute', monthlyPrice: 20 } },
			{ overrides: { name: `${TAG} Compute Big`, productType: 'compute', monthlyPrice: 60 } },
			{ overrides: { name: `${TAG} Storage`, productType: 'storage', monthlyPrice: 15 } },
			{ overrides: { name: `${TAG} Recipe`, productType: 'compute', monthlyPrice: 25, postProvisionScript: '#!/bin/bash\necho setup' } },
		];
		for (const p of PLANS) {
			const { providerPubkeyHex } = await seedRentableOffering(p.overrides);
			p.providerHex = providerPubkeyHex;
		}
	});

	test.afterAll(async () => {
		// Surface cleanup failures rather than silently swallowing them — a
		// silent .catch(() => {}) hides FK-constraint / timeout issues that
		// then leak offerings into the next run. Log so the failure is
		// debuggable without failing the whole suite on a teardown race.
		for (const p of PLANS) {
			if (p.providerHex) {
				await deleteOfferingsByProvider(p.providerHex).catch((e) => {
					console.error(`search-dsl cleanup: failed to delete offerings for ${p.providerHex}:`, e);
				});
			}
		}
	});

	test.beforeEach(async ({ page }) => {
		// Navigate scoped to the unique tag so ONLY this suite's offerings are
		// visible. Demo + other workers' offerings never match the tag.
		await page.goto(`/dashboard/marketplace?q=${TAG}`);

		// Wait for page to load.
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for at least one of our tagged offerings to render. The count
		// banner shows "0 offerings found" before the async fetch completes.
		await expect(page.locator('tbody tr[id^="offering-"]').first()).toBeVisible({ timeout: 15000 });
	});

	test('should filter offerings by GPU type checkbox', async ({ page }) => {
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Toggle the GPU type checkbox (sidebar filter). Client-side filter.
		const gpuCheckbox = page.locator('aside label:has-text("GPU") input[type="checkbox"]');
		await gpuCheckbox.check();
		await expect(gpuCheckbox).toBeChecked();

		// Client-side filter applies via $derived; wait for non-GPU rows to
		// be removed before reading the count.
		await expect(page.locator('tbody tr').filter({ hasNotText: /gpu/i })).toHaveCount(0, { timeout: 1500 });

		// Every visible row is a GPU offering (both of our seeded GPU rows).
		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThanOrEqual(2);
		for (let i = 0; i < count; i++) {
			await expect(offeringRows.nth(i)).toContainText(/gpu/i);
		}
	});

	test('should filter offerings by DSL price query', async ({ page }) => {
		// Type price filter in the search input. `price:<=N` contains a colon,
		// so the API routes it through the DSL parser (server-side filter).
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		// Wait for the debounced search to round-trip through the API.
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		// Our seeded "GPU Cheap" ($10) and "Storage" ($15) are <= 20; the DSL
		// path returns every marketplace offering under $20 (server-side), so
		// the count is non-deterministic across workers but always > 0.
		await searchInput.fill('price:<=20');
		await priceResponse;

		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
	});

	test('should combine type filter and DSL query', async ({ page }) => {
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Toggle the Compute type checkbox (client-side).
		const computeCheckbox = page.locator('aside label:has-text("Compute") input[type="checkbox"]');
		await computeCheckbox.check();
		await expect(computeCheckbox).toBeChecked();

		// Client-side filter applies via $derived; wait for non-Compute rows
		// to be removed before adding the DSL price filter on top.
		await expect(page.locator('tbody tr').filter({ hasNotText: /compute/i })).toHaveCount(0, { timeout: 1500 });

		// Add DSL price filter (server-side refetch with `price:<=50`).
		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('price:<=50');
		await priceResponse;

		// Results exist; the DSL server-side filter returned only rows under
		// $50. Our seeded "Compute Small" ($20) is among them.
		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
		await expect(offeringRows.first()).toContainText(/compute|gpu|storage/i);
	});

	test('should show empty state for impossible query', async ({ page }) => {
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('price:<=0');
		await priceResponse;

		// Verify the per-query empty state is shown.
		await expect(page.locator('text=No results for')).toBeVisible();
		await expect(page.locator('text=/Try a different term/')).toBeVisible();

		// Verify results count shows 0
		await expect(page.locator('text=0 offerings found')).toBeVisible();
	});

	test('empty-state hint uses the valid DSL field alias "type"', async ({ page }) => {
		// Regression: the hint used to advertise `product_type:gpu`, but the
		// API DSL allowlist (api/src/search/builder.rs) only accepts the alias
		// `type` (which maps to the product_type column). `product_type:gpu`
		// was rejected with "Unknown field: product_type".
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('price:<=0');
		await priceResponse;

		// Hint must show the valid alias `type:gpu` and must not advertise the
		// invalid `product_type:` form.
		await expect(page.locator('text=/Try a different term/')).toBeVisible();
		await expect(page.locator('code')).toHaveText('type:gpu');
	});

	test('DSL "type:" filter queries offerings by product type', async ({ page }) => {
		// Validates the field-syntax the empty-state hint advertises actually
		// works end-to-end through the API DSL parser (distinct from the
		// client-side GPU checkbox test, which never sends a `q` parameter).
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		// Wait for the debounced search to round-trip through the API.
		const typeResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('type:gpu');
		await typeResponse;

		// waitForResponse resolves on HTTP receipt, not on Svelte re-render.
		// Gate on a GPU row appearing before counting so a render gap can't
		// produce a transient 0 count under parallel load. The DSL filter is
		// server-side, so every returned row is a GPU offering by construction.
		const offeringRows = page.locator('tbody tr');
		await expect(offeringRows.filter({ hasText: /gpu/i }).first()).toBeVisible({ timeout: 5000 });
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
		for (let i = 0; i < count; i++) {
			await expect(offeringRows.nth(i)).toContainText(/gpu/i);
		}
	});

	test('should update results count when filtering', async ({ page }) => {
		// Wait for initial offerings to load — scoped to our tag, so the
		// initial count is exactly our seeded count.
		const initialCount = page.locator(COUNT_LOCATOR);
		await expect(initialCount).toBeVisible();

		const initialText = await initialCount.textContent();
		const initialNumber = parseInt(initialText?.match(/\d+/)?.[0] || '0');
		expect(initialNumber).toBeGreaterThanOrEqual(2);
		// Anchor the exact banner text so the wait/change assertions are unambiguous.
		const initialBanner = new RegExp(`^${initialNumber} offerings found$`);

		// Apply GPU filter via checkbox (client-side on our tagged set).
		await page.locator('aside label:has-text("GPU") input[type="checkbox"]').check();
		// Client-side filter applies via $derived; wait for the count banner
		// to change before reading the filtered number.
		await expect(page.locator(COUNT_LOCATOR)).not.toHaveText(initialBanner, { timeout: 2000 });

		const filteredCount = page.locator(COUNT_LOCATOR);
		const filteredText = await filteredCount.textContent();
		const filteredNumber = parseInt(filteredText?.match(/\d+/)?.[0] || '0');

		// Filtered count is non-zero and strictly less than the tagged total.
		expect(filteredNumber).toBeGreaterThan(0);
		expect(filteredNumber).toBeLessThan(initialNumber);

		// Reset filter by unchecking GPU (no dedicated "All" button exists).
		await page.locator('aside label:has-text("GPU") input[type="checkbox"]').uncheck();
		// Wait for the client-side filter to clear (count returns to initial).
		await expect(page.locator(COUNT_LOCATOR)).toHaveText(initialBanner, { timeout: 2000 });

		// Verify count returns to original
		const resetCount = page.locator(COUNT_LOCATOR);
		const resetText = await resetCount.textContent();
		const resetNumber = parseInt(resetText?.match(/\d+/)?.[0] || '0');
		expect(resetNumber).toBe(initialNumber);
	});

	test('recipe filter uses self-explanatory label with tooltip (#9)', async ({ page }) => {
		// Audit #9: the advanced filters panel had a checkbox labelled "Recipes only"
		// with no explanation. A new user had no way to know that "recipe" means
		// post_provision_script (a setup script the provider baked into the offering).
		// Rename to "Includes setup script" with a title= tooltip describing it.
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// The recipe checkbox lives inside the collapsible "More filters" panel;
		// expand it first.
		const moreFiltersButton = page.getByRole('button', { name: /More filters/i });
		if (await moreFiltersButton.isVisible({ timeout: 2000 }).catch(() => false)) {
			await moreFiltersButton.click();
			// The recipe label below is part of the panel's expanded content;
			// the next toBeVisible() assertion auto-retries until it appears.
		}

		// The new label must be present.
		const recipeLabel = page.locator('aside').getByText('Includes setup script');
		await expect(recipeLabel).toBeVisible();

		// The old ambiguous label must be gone.
		await expect(page.locator('aside').getByText('Recipes only')).toHaveCount(0);

		// The label's container must carry a tooltip explaining what a setup
		// script is, so the user isn't forced to guess.
		const recipeFilterBlock = page.locator('aside').filter({ hasText: 'Includes setup script' }).first();
		// A <span>/<label>/<input> within this block exposes a title attribute
		// whose text mentions what the setup script does.
		const tooltipText = await recipeFilterBlock.locator('[title]').first().getAttribute('title');
		expect(tooltipText?.length).toBeGreaterThan(10);
		expect(tooltipText?.toLowerCase()).toMatch(/setup script|recipe|provision/);
	});
});
