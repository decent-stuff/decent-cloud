import { test, expect } from '@playwright/test';

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
 * The dev DB ships only offline demo offerings, so each test loads the
 * marketplace with ?demo=1&offline=1 to ensure there is data to filter.
 */

const MARKETPLACE_URL = '/dashboard/marketplace?demo=1&offline=1';

// Result-count banner text format used by the current marketplace UI.
// Playwright interprets `text=/pattern/` as a regex match against element text.
const COUNT_LOCATOR = 'text=/\\d+ offerings found/';

test.describe('Search DSL', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to marketplace with demo + offline offerings visible so
		// the filters have something to act on.
		await page.goto(MARKETPLACE_URL);

		// Wait for page to load
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();

		// Wait for actual offerings to render. The count banner shows
		// "0 offerings found" before the async fetch completes, so wait for
		// at least one offering row instead of the count text alone.
		await expect(page.locator('tbody tr[id^="offering-"]').first()).toBeVisible({ timeout: 15000 });
	});

	test('should filter offerings by GPU type checkbox', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Toggle the GPU type checkbox (sidebar filter).
		const gpuCheckbox = page.locator('aside label:has-text("GPU") input[type="checkbox"]');
		await gpuCheckbox.check();
		await expect(gpuCheckbox).toBeChecked();

		// Client-side filter applies via $derived; wait for non-GPU rows to
		// be removed before reading the count.
		await expect(page.locator('tbody tr').filter({ hasNotText: /gpu/i })).toHaveCount(0, { timeout: 1500 });

		// Verify all visible offering rows show a gpu product type. The
		// offering table renders each product_type inside a span in a row.
		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
		for (let i = 0; i < count; i++) {
			await expect(offeringRows.nth(i)).toContainText(/gpu/i);
		}
	});

	test('should filter offerings by DSL price query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Type price filter in the search input (field syntax is sent to
		// the API as the `q` parameter).
		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		// Wait for the debounced search to round-trip through the API.
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('price:<=20');
		await priceResponse;

		// Verify results are filtered (demo offerings with price <=20 exist).
		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
	});

	test('should combine type filter and DSL query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Toggle the Compute type checkbox.
		const computeCheckbox = page.locator('aside label:has-text("Compute") input[type="checkbox"]');
		await computeCheckbox.check();
		await expect(computeCheckbox).toBeChecked();

		// Client-side filter applies via $derived; wait for non-Compute rows
		// to be removed before adding the DSL price filter on top.
		await expect(page.locator('tbody tr').filter({ hasNotText: /compute/i })).toHaveCount(0, { timeout: 1500 });

		// Add DSL price filter
		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		// Wait for the debounced search to round-trip through the API.
		const priceResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/offerings'),
			{ timeout: 3000 },
		);
		await searchInput.fill('price:<=50');
		await priceResponse;

		// Verify results exist and show compute type.
		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
		await expect(offeringRows.first()).toContainText(/compute/i);
	});

	test('should show empty state for impossible query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator(COUNT_LOCATOR)).toBeVisible();

		// Search for impossible price
		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		// Wait for the debounced search to round-trip through the API.
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
		// Wait for the debounced search to round-trip through the API.
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

		const offeringRows = page.locator('tbody tr');
		const count = await offeringRows.count();
		expect(count).toBeGreaterThan(0);
		for (let i = 0; i < count; i++) {
			await expect(offeringRows.nth(i)).toContainText(/gpu/i);
		}
	});

	test('should update results count when filtering', async ({ page }) => {
		// Wait for initial offerings to load
		const initialCount = page.locator(COUNT_LOCATOR);
		await expect(initialCount).toBeVisible();

		// Get initial count text
		const initialText = await initialCount.textContent();
		const initialNumber = parseInt(initialText?.match(/\d+/)?.[0] || '0');
		expect(initialNumber).toBeGreaterThan(0);
		// Anchor the exact banner text so the wait/change assertions are unambiguous.
		const initialBanner = new RegExp(`^${initialNumber} offerings found$`);

		// Apply GPU filter via checkbox
		await page.locator('aside label:has-text("GPU") input[type="checkbox"]').check();
		// Client-side filter applies via $derived; wait for the count banner
		// to change before reading the filtered number.
		await expect(page.locator(COUNT_LOCATOR)).not.toHaveText(initialBanner, { timeout: 2000 });

		// Get filtered count text
		const filteredCount = page.locator(COUNT_LOCATOR);
		const filteredText = await filteredCount.textContent();
		const filteredNumber = parseInt(filteredText?.match(/\d+/)?.[0] || '0');

		// Verify count changed (should be less than total offerings)
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
