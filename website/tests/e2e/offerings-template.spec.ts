import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';

/**
 * E2E Tests for Offerings Template Download
 *
 * Prerequisites:
 * - API server running at http://localhost:8080 (or configured API_URL)
 * - Dev server running at http://localhost:5173
 */

test.describe('Offerings Template Download', () => {
	test.beforeEach(async ({ page }) => {
		setupConsoleLogging(page);
	});

	test('should show product type selector when clicking Download Template button', async ({
		page,
	}) => {
		await page.goto('/dashboard/offerings');
		await page.waitForLoadState('networkidle');
		await expect(page.locator('h1:has-text("My Offerings")')).toBeVisible();

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();

		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });
		await expect(
			page.locator('text=Choose a product type to download an example template'),
		).toBeVisible();
	});

	test('should display product type options in template dialog', async ({ page }) => {
		await page.goto('/dashboard/offerings');
		await page.waitForLoadState('networkidle');

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		const productTypeButtons = page.locator('.grid button:has-text("Download template")');
		const count = await productTypeButtons.count();
		expect(count).toBeGreaterThan(0);
	});

	test('should close template dialog when clicking Cancel', async ({ page }) => {
		await page.goto('/dashboard/offerings');
		await page.waitForLoadState('networkidle');

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		await page.click('button:has-text("Cancel")');
		await expect(page.locator('h2:has-text("Select Product Type")')).not.toBeVisible();
	});

	test('should download CSV template when selecting a product type', async ({
		page,
	}) => {
		await page.goto('/dashboard/offerings');
		await page.waitForLoadState('networkidle');

		const downloadPromise = page.waitForEvent('download');

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		const firstProductType = page
			.locator('.grid button:has-text("Download template")')
			.first();
		await firstProductType.click();

		const download = await downloadPromise;
		expect(download.suggestedFilename()).toMatch(/^offerings-template-\w+\.csv$/);
	});

	test('should show Edit options when offerings list is empty', async ({ page }) => {
		await page.goto('/dashboard/offerings');
		await page.waitForLoadState('networkidle');

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		const hasOfferings = (await page.locator('.grid > div').count()) > 0;

		if (!hasOfferings) {
			await expect(
				page.locator('text=Or start editing with a template:'),
			).toBeVisible();

			const editButtons = page.locator('button:has-text("Edit")');
			const count = await editButtons.count();
			expect(count).toBeGreaterThan(0);
		}
	});
});
