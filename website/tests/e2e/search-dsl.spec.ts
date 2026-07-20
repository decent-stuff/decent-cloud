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

		// Wait for results to update (filter is applied client-side).
		await page.waitForTimeout(500);

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
		await searchInput.fill('price:<=20');

		// Wait for debounce (300ms) and results to update.
		await page.waitForTimeout(800);

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

		// Wait for filter to apply
		await page.waitForTimeout(500);

		// Add DSL price filter
		const searchInput = page.locator('input[aria-label="Search offerings by name, description, or type"]');
		await searchInput.fill('price:<=50');

		// Wait for debounce and results to update
		await page.waitForTimeout(800);

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
		await searchInput.fill('price:<=0');

		// Wait for debounce and results to update
		await page.waitForTimeout(800);

		// Verify the per-query empty state is shown.
		await expect(page.locator('text=No results for')).toBeVisible();
		await expect(page.locator('text=/Try a different term/')).toBeVisible();

		// Verify results count shows 0
		await expect(page.locator('text=0 offerings found')).toBeVisible();
	});

	test('should update results count when filtering', async ({ page }) => {
		// Wait for initial offerings to load
		const initialCount = page.locator(COUNT_LOCATOR);
		await expect(initialCount).toBeVisible();

		// Get initial count text
		const initialText = await initialCount.textContent();
		const initialNumber = parseInt(initialText?.match(/\d+/)?.[0] || '0');
		expect(initialNumber).toBeGreaterThan(0);

		// Apply GPU filter via checkbox
		await page.locator('aside label:has-text("GPU") input[type="checkbox"]').check();
		await page.waitForTimeout(500);

		// Get filtered count text
		const filteredCount = page.locator(COUNT_LOCATOR);
		const filteredText = await filteredCount.textContent();
		const filteredNumber = parseInt(filteredText?.match(/\d+/)?.[0] || '0');

		// Verify count changed (should be less than total offerings)
		expect(filteredNumber).toBeGreaterThan(0);
		expect(filteredNumber).toBeLessThan(initialNumber);

		// Reset filter by unchecking GPU (no dedicated "All" button exists).
		await page.locator('aside label:has-text("GPU") input[type="checkbox"]').uncheck();
		await page.waitForTimeout(500);

		// Verify count returns to original
		const resetCount = page.locator(COUNT_LOCATOR);
		const resetText = await resetCount.textContent();
		const resetNumber = parseInt(resetText?.match(/\d+/)?.[0] || '0');
		expect(resetNumber).toBe(initialNumber);
	});
});
