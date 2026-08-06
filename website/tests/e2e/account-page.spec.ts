import { test, expect } from './fixtures/test-account';

/**
 * E2E Tests for Account Settings Page
 *
 * Consolidated: same-page snapshot assertions are grouped into single tests
 * to reduce redundant page.goto() calls. Behavioral flows (clicks that change
 * state) remain separate so each flow stays a clear, documented behavior.
 *
 * Prerequisites:
 * - Warm stack: api at http://localhost:59011, web at http://localhost:59010
 * - Clean test database (test-account fixture seeds + tears down per worker)
 */

test.describe('Account Settings Page', () => {
	// No beforeEach needed - the test fixture handles authentication automatically

	test('account page: overview renders correctly via direct URL', async ({
		page,
		testAccount,
	}) => {
		// Single navigation covers what was previously three snapshot tests:
		// "display account overview", "format created date as human-readable",
		// and "accessible via direct URL" (whose h1 + @username assertions were
		// a strict subset of the overview assertions).
		await page.goto('/dashboard/account');

		// Page title and overview section
		await expect(page.locator('h1:has-text("Account Settings")')).toBeVisible();
		await expect(page.locator('text=Account overview')).toBeVisible();

		// Username (first() — also appears in sidebar)
		await expect(
			page.locator(`text=@${testAccount.username}`).first(),
		).toBeVisible();

		// Account ID is no longer shown on the overview page (moved to /security);
		// identity coverage here is preserved by Username + Created assertions below.

		// Created date is present AND human-readable (month name appears)
		await expect(page.locator('text=Created')).toBeVisible();
		const createdSection = page.locator('text=Created').locator('..');
		const dateText = await createdSection.textContent();
		const hasMonthName =
			/(January|February|March|April|May|June|July|August|September|October|November|December)/.test(
				dateText || '',
			);
		expect(hasMonthName).toBeTruthy();

		// Active keys count
		await expect(page.locator('text=Active Keys')).toBeVisible();
		await expect(page.locator('text=1 key')).toBeVisible(); // New account has 1 key
	});

	test('account page: sidebar link navigates to account page', async ({ page }) => {
		// Land on /dashboard so the sidebar (with the Account link) is present.
		// (The page fixture no longer pre-navigates — see fixtures/test-account.ts.)
		await page.goto('/dashboard');
		await expect(page.getByRole('button', { name: 'Logout' })).toBeVisible({ timeout: 15000 });

		// Verify the "Account" link exists in the sidebar (exact match — the
		// dashboard cards use longer text like "Profile Account settings").
		const accountLink = page.getByRole('link', { name: 'Account', exact: true });
		await expect(accountLink).toBeVisible({ timeout: 10000 });

		// Click it and verify navigation
		await accountLink.click();
		await expect(page).toHaveURL('/dashboard/account');
	});

	test('account page: navigation between sections', async ({ page }) => {
		// Start at account page
		await page.goto('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Navigate to security section
		await page.click('a:has-text("Security")');
		await expect(page).toHaveURL('/dashboard/account/security');
		await expect(page.locator('h1:has-text("Security")')).toBeVisible();

		// Navigate back to account overview. Exact-name match on role=link: the
		// substring `has-text("Account")` is ambiguous here because the sidebar
		// also has a "Support Account" link (/dashboard/provider/support), so a
		// substring click lands on the wrong one. No SettingsTabs tab is named
		// "Account", so exact-name scopes uniquely to the sidebar Account link.
		await page.getByRole('link', { name: 'Account', exact: true }).click();
		await expect(page).toHaveURL('/dashboard/account');
		await expect(
			page.locator('h1:has-text("Account Settings")'),
		).toBeVisible();

		// Navigate to profile section
		await page.click('a:has-text("Public Profile")');
		await expect(page).toHaveURL('/dashboard/account/profile');
		await expect(
			page.locator('h1:has-text("Public Profile")'),
		).toBeVisible();
	});

	test('account page: security overview renders device key info for single-key account', async ({
		page,
	}) => {
		// Single navigation covers what was previously two snapshot tests:
		// "not show Remove button for single key account" and
		// "display device key info correctly".
		await page.goto('/dashboard/account/security');

		// Devices section present, single key
		await expect(page.locator('text=Devices')).toBeVisible();
		await expect(page.locator('text=1 key')).toBeVisible();

		// Remove button should NOT be visible (can't remove last key)
		await expect(page.locator('button:has-text("Remove")')).not.toBeVisible();

		// Device shows Active status, key icon, and truncated public key (hex)
		await expect(page.locator('text=Active').first()).toBeVisible({ timeout: 10000 });
		await expect(page.locator('text=🔑')).toBeVisible();
		const keyDisplay = page.locator('.font-mono').filter({ hasText: /[0-9a-f]+\.\.\.[0-9a-f]+/i });
		await expect(keyDisplay.first()).toBeVisible();
	});

	test('account page: edit device name', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Find the Devices section
		await expect(page.locator('text=Devices')).toBeVisible();

		// Click on device name to start editing (default is "Unnamed Device")
		const deviceNameBtn = page.locator('button:has-text("Unnamed Device")').first();
		await deviceNameBtn.click();

		// Should show edit input
		const editInput = page.locator('input[placeholder="Device name"]');
		await expect(editInput).toBeVisible();

		// Enter new device name
		const newName = 'My Test Device';
		await editInput.fill(newName);

		// Click Save
		await page.click('button:has-text("Save")');

		// Wait for save to complete - button should disappear
		await expect(editInput).not.toBeVisible({ timeout: 5000 });

		// New device name should be displayed
		await expect(page.locator(`button:has-text("${newName}")`)).toBeVisible();
	});

	test('account page: open Add Device modal', async ({ page }) => {
		await page.goto('/dashboard/account/security');

		// Click Add Device button
		await page.click('button:has-text("+ Add Device")');

		// Modal should appear with seed phrase options (use heading for specificity)
		await expect(page.locator('h3:has-text("Seed Phrase")')).toBeVisible();
		await expect(page.locator('text=Generate a new seed phrase or import an existing one').first()).toBeVisible();

		// Should show options to generate or import
		await expect(page.locator('button:has-text("Generate New")')).toBeVisible();
		await expect(page.locator('button:has-text("Import Existing")')).toBeVisible();
	});
});
