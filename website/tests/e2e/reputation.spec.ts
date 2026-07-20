import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /dashboard/reputation.
 *
 * Anonymous-OK (renders a public account search). Asserts the unique
 * Reputation-page scaffolding — heading, search input, and the empty
 * state — rather than dynamic result counts.
 */

test.describe('/dashboard/reputation', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/dashboard/reputation');
	});

	test('renders the Reputation heading and search box', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'Reputation', exact: true })).toBeVisible();
		await expect(page.getByText(/Search for users and providers by username/i)).toBeVisible();

		// The labelled search input is unique to this page
		const searchInput = page.locator('input#search');
		await expect(searchInput).toBeVisible();
		await expect(searchInput).toHaveAttribute(
			'placeholder',
			'Enter username, display name, or public key...',
		);
	});

	test('shows the idle "Search Reputation" prompt before any query', async ({ page }) => {
		// Empty-state copy unique to this route (only shown when no search yet)
		await expect(page.getByRole('heading', { name: 'Search Reputation' })).toBeVisible();
		await expect(
			page.getByText(/Enter a username, display name, or public key to find accounts/i),
		).toBeVisible();
		await expect(
			page.getByText(/All reputation data is public by design/i),
		).toBeVisible();
	});

	test('shows "No Results Found" for an unknown query', async ({ page }) => {
		// Use a random-looking string unlikely to match any seeded account.
		const uniqueQuery = 'zxqw-no-such-user-9f8e7d';
		await page.locator('input#search').fill(uniqueQuery);

		// Debounce is 300ms; give the search room to resolve.
		await expect(page.getByRole('heading', { name: 'No Results Found' })).toBeVisible({
			timeout: 5000,
		});
		await expect(page.getByText(/No accounts match your search query/i)).toBeVisible();
		// The query is echoed back in the empty state
		await expect(page.getByText(uniqueQuery)).toBeVisible();
	});
});
