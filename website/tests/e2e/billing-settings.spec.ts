import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Billing Settings Page
 *
 * Prerequisites:
 * - API server running at http://localhost:8080
 * - Dev server running at http://localhost:5173
 * - Clean test database
 */

test.describe('Billing Settings Page', () => {
	test('should display billing settings page correctly', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Verify page title
		await expect(page.locator('h1:has-text("Billing Settings")')).toBeVisible();

		// Verify description
		await expect(page.locator('text=Manage your billing address and VAT information')).toBeVisible();

		// Verify form elements
		await expect(page.locator('text=Invoice Information')).toBeVisible();
		await expect(page.locator('label:has-text("Billing Address")')).toBeVisible();
		await expect(page.locator('label:has-text("Country (for VAT)")')).toBeVisible();
		await expect(page.locator('label:has-text("VAT ID")')).toBeVisible();
	});

	test('should navigate to billing from account page', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Should see billing tab
		await expect(page.locator('a:has-text("Billing")')).toBeVisible();

		// Click billing tab
		await page.click('a:has-text("Billing")');

		// Should navigate to billing page
		await expect(page).toHaveURL('/dashboard/account/billing');
		await expect(page.locator('h1:has-text("Billing Settings")')).toBeVisible();
	});

	test('should save billing address', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Fill in billing address
		const testAddress = 'Test Company\n123 Test Street\nTest City, 12345\nGermany';
		await page.locator('#billingAddress').fill(testAddress);

		// Save
		await page.click('button:has-text("Save Billing Settings")');

		// Should show success message
		await expect(page.locator('text=Billing settings saved')).toBeVisible({ timeout: 5000 });
	});

	test('should save VAT settings', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Select country
		await page.locator('#billingCountryCode').selectOption('DE');

		// Enter VAT ID (use dummy VAT for test - real validation happens on server)
		await page.locator('#billingVatId').fill('123456789');

		// Save
		await page.click('button:has-text("Save Billing Settings")');

		// Should show success message
		await expect(page.locator('text=Billing settings saved')).toBeVisible({ timeout: 5000 });
	});

	test('should persist billing settings on reload', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Fill in billing info
		const testAddress = 'Persistent Test Company\n456 Test Ave';
		await page.locator('#billingAddress').fill(testAddress);
		await page.locator('#billingCountryCode').selectOption('NL');
		await page.locator('#billingVatId').fill('987654321');

		// Save
		await page.click('button:has-text("Save Billing Settings")');
		await expect(page.locator('text=Billing settings saved')).toBeVisible({ timeout: 5000 });

		// Reload page
		await page.reload();
		await page.waitForLoadState('networkidle');

		// Verify data persisted
		await expect(page.locator('#billingAddress')).toHaveValue(testAddress);
		await expect(page.locator('#billingCountryCode')).toHaveValue('NL');
		await expect(page.locator('#billingVatId')).toHaveValue('987654321');
	});

	test('should show verify button for VAT ID', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Verify button should be present
		await expect(page.locator('button:has-text("Verify")')).toBeVisible();

		// Button should be disabled without country and VAT ID
		await expect(page.locator('button:has-text("Verify")')).toBeDisabled();

		// Fill in country and VAT ID
		await page.locator('#billingCountryCode').selectOption('DE');
		await page.locator('#billingVatId').fill('123456789');

		// Button should now be enabled
		await expect(page.locator('button:has-text("Verify")')).toBeEnabled();
	});

	test('should show EU country options in dropdown', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Open dropdown and verify some EU countries are present
		const select = page.locator('#billingCountryCode');

		// Check for specific EU countries
		await expect(select.locator('option[value="DE"]')).toHaveText('Germany (DE)');
		await expect(select.locator('option[value="FR"]')).toHaveText('France (FR)');
		await expect(select.locator('option[value="NL"]')).toHaveText('Netherlands (NL)');
		await expect(select.locator('option[value="OTHER"]')).toHaveText('Other (non-EU)');
	});

	test('should display VAT info section', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Should show VAT info section
		await expect(page.locator('text=About VAT and Invoices')).toBeVisible();
		await expect(page.locator('text=EU businesses with valid VAT IDs may qualify for reverse charge')).toBeVisible();
	});
});
