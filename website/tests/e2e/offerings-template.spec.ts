import { test, expect } from './fixtures/test-account';
import { setupConsoleLogging } from './fixtures/auth-helpers';
import {
	seedMarketplaceOffering,
	deleteOfferingById,
} from './fixtures/seed-helpers';

/**
 * E2E Tests for Offerings Template Download
 *
 * Consolidated: dialog snapshot assertions (product types + edit options) are
 * in one test; dialog interactions (Cancel + CSV download) are in a second.
 * Two navigations total instead of five.
 *
 * The template dialog's product-type options come from
 * `GET /api/v1/offerings/product-types`, which derives its types from
 * example-provider offerings. Migration 053 dropped those, so this spec
 * self-seeds two example offerings (compute + gpu) to populate the dialog
 * deterministically instead of relying on ambient demo data.
 *
 * Prerequisites:
 * - Warm stack: api at http://localhost:59011, web at http://localhost:59010
 */

test.describe('Offerings Template Download', () => {
	const seededIds: string[] = [];
	test.beforeAll(async () => {
		// Seed two distinct product types under the example provider pubkey so
		// /offerings/product-types returns a non-empty list and the dialog has
		// real "Download template" buttons to interact with.
		for (const productType of ['compute', 'gpu']) {
			const handle = await seedMarketplaceOffering({
				isExample: true,
				productType,
				name: `E2E Template ${productType}`,
			});
			seededIds.push(handle.offeringNumericId);
		}
	});
	test.afterAll(async () => {
		await Promise.all(seededIds.map((id) => deleteOfferingById(id)));
	});

	test.beforeEach(async ({ page }) => {
		setupConsoleLogging(page);
	});

	test('template dialog shows heading, product type options, and edit section', async ({
		page,
	}) => {
		// Single navigation covers what was previously three snapshot tests:
		// "show product type selector", "display product type options", and
		// "show Edit options when offerings list is empty".
		await page.goto('/dashboard/offerings');
		await expect(page.locator('h1:has-text("My Offerings")')).toBeVisible();

		// Open the template dialog
		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();

		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });
		await expect(
			page.locator('text=Choose a product type to download an example template'),
		).toBeVisible();

		// Product type options exist
		const productTypeButtons = page.locator('.grid button:has-text("Download template")');
		const count = await productTypeButtons.count();
		expect(count).toBeGreaterThan(0);

		// Edit options (conditional: only shown when offerings list is empty)
		const hasOfferings = (await page.locator('.grid > div').count()) > 0;
		if (!hasOfferings) {
			await expect(
				page.locator('text=Or start editing with a template:'),
			).toBeVisible();

			const editButtons = page.locator('button:has-text("Edit")');
			const editCount = await editButtons.count();
			expect(editCount).toBeGreaterThan(0);
		}
	});

	test('dialog interactions: close with Cancel and download CSV template', async ({ page }) => {
		// Single navigation covers what was previously two interaction tests:
		// "close template dialog when clicking Cancel" and
		// "download CSV template when selecting a product type".
		await page.goto('/dashboard/offerings');

		const downloadBtn = page.locator('button:has-text("Download Template")');
		await expect(downloadBtn).toBeVisible();
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		// Close with Cancel
		await page.click('button:has-text("Cancel")');
		await expect(page.locator('h2:has-text("Select Product Type")')).not.toBeVisible();

		// Reopen and download a CSV template
		await downloadBtn.click();
		await expect(page.locator('h2:has-text("Select Product Type")')).toBeVisible({ timeout: 10000 });

		const downloadPromise = page.waitForEvent('download');
		const firstProductType = page
			.locator('.grid button:has-text("Download template")')
			.first();
		await firstProductType.click();

		const download = await downloadPromise;
		expect(download.suggestedFilename()).toMatch(/^offerings-template-\w+\.csv$/);
	});
});
