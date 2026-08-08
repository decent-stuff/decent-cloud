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

	test('shows the Top Providers leaderboard as the default view (no longer a dead-end)', async ({
		page,
	}) => {
		// UX-006: the landing view is now a browseable reputation leaderboard,
		// not the bare "Search Reputation" idle prompt. The section renders by
		// default (the fetch fires on mount) even with zero seeded providers.
		await expect(page.getByRole('heading', { name: 'Top Providers' })).toBeVisible({
			timeout: 10_000,
		});
		await expect(page.getByText(/Ranked by trust score/i)).toBeVisible();
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
