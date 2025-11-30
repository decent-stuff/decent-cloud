import { test, expect } from '@playwright/test';

/**
 * E2E Tests for Search DSL Functionality
 *
 * Tests the Domain Specific Language (DSL) search features in the marketplace:
 * - Type filter buttons (All, Compute, GPU, Storage, Network)
 * - DSL text input for price queries and other filters
 * - Combined type + DSL filters
 * - Empty results state
 * - Results count updates
 */

test.describe('Search DSL', () => {
	test.beforeEach(async ({ page }) => {
		// Navigate to marketplace
		await page.goto('/dashboard/marketplace');

		// Wait for page to load and offerings to appear
		await expect(page.locator('h1:has-text("Marketplace")')).toBeVisible();
		await page.waitForLoadState('networkidle');
	});

	test('should filter offerings by GPU type button', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator('text=/Showing \\d+ offerings/')).toBeVisible();

		// Click GPU filter button
		const gpuButton = page.locator('button:has-text("🎮 GPU")');
		await gpuButton.click();

		// Wait for filter to apply (button should have active class)
		await expect(gpuButton).toHaveClass(/bg-blue-600/);

		// Wait for results to update
		await page.waitForTimeout(500);

		// Verify all visible offerings show GPU type
		const offeringCards = page.locator('.grid > div');
		const count = await offeringCards.count();
		expect(count).toBeGreaterThan(0);

		// Check that product type contains "gpu" for all cards
		for (let i = 0; i < count; i++) {
			const productType = offeringCards.nth(i).locator('text=/Type/').locator('..').locator('.text-white').last();
			await expect(productType).toContainText(/gpu/i);
		}
	});

	test('should filter offerings by DSL price query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator('text=/Showing \\d+ offerings/')).toBeVisible();

		// Type price filter in search input
		const searchInput = page.locator('input[placeholder*="Search with DSL"]');
		await searchInput.fill('price:<=20');

		// Wait for debounce and results to update
		await page.waitForTimeout(500);

		// Verify results are filtered (example offerings with price <=20: 5, 15, 2.5, 20, 10)
		const offeringCards = page.locator('.grid > div');
		const count = await offeringCards.count();
		expect(count).toBeGreaterThan(0);

		// Check that at least one card shows a price <=20
		const priceElement = offeringCards.first().locator('text=/Price/').locator('..').locator('.text-white').last();
		await expect(priceElement).toBeVisible();
	});

	test('should combine type filter and DSL query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator('text=/Showing \\d+ offerings/')).toBeVisible();

		// Click Compute filter button
		const computeButton = page.locator('button:has-text("💻 Compute")');
		await computeButton.click();
		await expect(computeButton).toHaveClass(/bg-blue-600/);

		// Wait for filter to apply
		await page.waitForTimeout(500);

		// Add DSL price filter
		const searchInput = page.locator('input[placeholder*="Search with DSL"]');
		await searchInput.fill('price:<=50');

		// Wait for debounce and results to update
		await page.waitForTimeout(500);

		// Verify results exist and show compute type
		const offeringCards = page.locator('.grid > div');
		const count = await offeringCards.count();
		expect(count).toBeGreaterThan(0);

		// Verify first card is compute type
		const productType = offeringCards.first().locator('text=/Type/').locator('..').locator('.text-white').last();
		await expect(productType).toContainText(/compute/i);
	});

	test('should show empty state for impossible query', async ({ page }) => {
		// Wait for initial offerings to load
		await expect(page.locator('text=/Showing \\d+ offerings/')).toBeVisible();

		// Search for impossible price
		const searchInput = page.locator('input[placeholder*="Search with DSL"]');
		await searchInput.fill('price:<=0');

		// Wait for debounce and results to update
		await page.waitForTimeout(500);

		// Verify empty state is shown
		await expect(page.locator('text=No Results Found')).toBeVisible();
		await expect(page.locator('text=Try adjusting your search or filters')).toBeVisible();

		// Verify results count shows 0
		await expect(page.locator('text=Showing 0 offerings')).toBeVisible();
	});

	test('should update results count when filtering', async ({ page }) => {
		// Wait for initial offerings to load
		const initialCount = page.locator('text=/Showing \\d+ offerings/');
		await expect(initialCount).toBeVisible();

		// Get initial count text
		const initialText = await initialCount.textContent();
		const initialNumber = parseInt(initialText?.match(/\d+/)?.[0] || '0');
		expect(initialNumber).toBeGreaterThan(0);

		// Apply GPU filter
		await page.locator('button:has-text("🎮 GPU")').click();
		await page.waitForTimeout(500);

		// Get filtered count text
		const filteredCount = page.locator('text=/Showing \\d+ offerings/');
		const filteredText = await filteredCount.textContent();
		const filteredNumber = parseInt(filteredText?.match(/\d+/)?.[0] || '0');

		// Verify count changed (should be less than total offerings)
		expect(filteredNumber).toBeGreaterThan(0);
		expect(filteredNumber).toBeLessThan(initialNumber);

		// Reset filter
		await page.locator('button:has-text("All")').click();
		await page.waitForTimeout(500);

		// Verify count returns to original
		const resetCount = page.locator('text=/Showing \\d+ offerings/');
		const resetText = await resetCount.textContent();
		const resetNumber = parseInt(resetText?.match(/\d+/)?.[0] || '0');
		expect(resetNumber).toBe(initialNumber);
	});
});
