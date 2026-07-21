import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Billing Settings Page
 *
 * Consolidated: same-page snapshot assertions are grouped into one test.
 * Behavioral save/persist/verify flows remain separate.
 *
 * Prerequisites:
 * - Warm stack: api at http://localhost:59011, web at http://localhost:59010
 * - Clean test database (test-account fixture seeds + tears down per worker)
 */

test.describe('Billing Settings Page', () => {
	test('billing settings: page renders with form fields, country options, and VAT info', async ({
		page,
	}) => {
		// Single navigation covers what was previously three snapshot tests:
		// "display billing settings page correctly", "show EU country options in
		// dropdown", and "display VAT info section".
		await page.goto('/dashboard/account/billing');

		// Page title and description
		await expect(page.locator('h1:has-text("Billing Settings")')).toBeVisible();
		await expect(page.locator('text=Manage your billing address and VAT information')).toBeVisible();

		// Form labels
		await expect(page.locator('text=Invoice Information')).toBeVisible();
		await expect(page.locator('label:has-text("Billing Address")')).toBeVisible();
		await expect(page.locator('label:has-text("Country (for VAT)")')).toBeVisible();
		await expect(page.locator('label:has-text("VAT ID")')).toBeVisible();

		// EU country options (current format: "CC - Country Name")
		const select = page.locator('#billingCountryCode');
		await expect(select.locator('option[value="DE"]')).toHaveText('DE - Germany');
		await expect(select.locator('option[value="FR"]')).toHaveText('FR - France');
		await expect(select.locator('option[value="NL"]')).toHaveText('NL - Netherlands');
		await expect(select.locator('option[value="OTHER"]')).toHaveText('Other (non-EU)');

		// VAT info section
		await expect(page.locator('text=About VAT and Invoices')).toBeVisible();
		await expect(page.locator('text=EU businesses with valid VAT IDs may qualify for reverse charge')).toBeVisible();
	});

	test('billing settings: navigation from account page', async ({ page }) => {
		await page.goto('/dashboard/account');

		// Should see billing tab
		await expect(page.locator('a:has-text("Billing")')).toBeVisible();

		// Click billing tab
		await page.click('a:has-text("Billing")');

		// Should navigate to billing page
		await expect(page).toHaveURL('/dashboard/account/billing');
		await expect(page.locator('h1:has-text("Billing Settings")')).toBeVisible();
	});

	test('billing settings: save billing address', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Fill in billing address
		const testAddress = 'Test Company\n123 Test Street\nTest City, 12345\nGermany';
		await page.locator('#billingAddress').fill(testAddress);

		// Save
		await page.click('button:has-text("Save Billing Settings")');

		// Should show success message
		await expect(page.locator('text=Billing settings saved')).toBeVisible({ timeout: 5000 });
	});

	test('billing settings: save VAT settings', async ({ page }) => {
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

	test('billing settings: settings persist on reload', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Fill in billing info
		const testAddress = 'Persistent Test Company\n456 Test Ave';
		await page.locator('#billingAddress').fill(testAddress);
		await page.locator('#billingCountryCode').selectOption('NL');
		await page.locator('#billingVatId').fill('987654321');

		// Save
		await page.click('button:has-text("Save Billing Settings")');
		await expect(page.locator('text=Billing settings saved')).toBeVisible({ timeout: 5000 });

		// Reload. The next op (toHaveValue) is auto-retrying, so the prior
		// networkidle wait was redundant and has been removed — toHaveValue
		// itself waits for the form to re-populate from the server.
		await page.reload();

		// Verify data persisted
		await expect(page.locator('#billingAddress')).toHaveValue(testAddress);
		await expect(page.locator('#billingCountryCode')).toHaveValue('NL');
		await expect(page.locator('#billingVatId')).toHaveValue('987654321');
	});

	test('billing settings: verify button reflects VAT field state', async ({ page }) => {
		await page.goto('/dashboard/account/billing');

		// Verify button should be present
		await expect(page.locator('button:has-text("Verify")')).toBeVisible();

		// Reset any persisted state from earlier tests so we can assert the
		// empty-form behaviour. The shared worker account keeps billing data
		// between tests, so explicitly clear it here.
		await page.locator('#billingCountryCode').selectOption('');
		await page.locator('#billingVatId').fill('');

		// Button should be disabled without country and VAT ID
		await expect(page.locator('button:has-text("Verify")')).toBeDisabled();

		// Fill in country and VAT ID
		await page.locator('#billingCountryCode').selectOption('DE');
		await page.locator('#billingVatId').fill('123456789');

		// Button should now be enabled
		await expect(page.locator('button:has-text("Verify")')).toBeEnabled();
	});
});
