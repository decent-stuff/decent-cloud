import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /dashboard/validators.
 *
 * Anonymous-OK public page showing active network validators, a stats
 * summary, and a "Become a Validator" CTA. The validator list comes
 * from a public read endpoint (no auth), so it works the same way for
 * anonymous and authenticated visitors; we assert the unique page
 * scaffolding rather than row counts.
 */

test.describe('/dashboard/validators', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/dashboard/validators');
		// Wait for the loading spinner to clear before asserting content.
		await expect(page.getByRole('heading', { name: 'Validators', exact: true })).toBeVisible();
	});

	test('renders the three stat cards with their unique labels', async ({ page }) => {
		// The metric-label spans are unique to this page's stats grid
		await expect(page.getByText('Active (24h)', { exact: true })).toBeVisible();
		await expect(page.getByText('Active (7d)', { exact: true })).toBeVisible();
		await expect(page.getByText('Total (30d)', { exact: true })).toBeVisible();

		// Subtext labels confirm the cards rendered with their captions
		await expect(page.getByText('Validators checked in today')).toBeVisible();
		await expect(page.getByText('Validators active this week')).toBeVisible();
		await expect(page.getByText('Validators active this month')).toBeVisible();
	});

	test('renders the validators table with the expected column headers', async ({ page }) => {
		// The <th> cells expose role="cell" in Playwright's a11y tree (no
		// scope="col"), so query them directly as table header cells.
		const headerCells = page.locator('thead th');
		await expect(headerCells).toHaveCount(4);
		await expect(headerCells.nth(0)).toHaveText('Validator');
		await expect(headerCells.nth(1)).toHaveText('Check-ins');
		await expect(headerCells.nth(2)).toHaveText('Last Seen');
		await expect(headerCells.nth(3)).toHaveText('Status');
	});

	test('renders the "Become a Validator" CTA', async ({ page }) => {
		// Unique CTA at the bottom of the route
		await expect(page.getByRole('heading', { name: 'Become a Validator' })).toBeVisible();
		await expect(
			page.getByRole('link', { name: /Learn More/i }).first(),
		).toHaveAttribute('href', 'https://decent-stuff.github.io/decent-cloud/mining-and-validation.html');
	});
});
