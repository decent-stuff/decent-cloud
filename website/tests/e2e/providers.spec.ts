import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /dashboard/providers/[identifier].
 *
 * Anonymous-OK provider-profile route. When the identifier resolves
 * to no known provider, the page renders a unique "Provider Not Found"
 * card — that's the structural state we assert here so the test is
 * self-contained (no seeding needed).
 */

test.describe('/dashboard/providers/[identifier]', () => {
	test('renders "Provider Not Found" card for an unknown identifier', async ({ page }) => {
		// Random hex-ish identifier unlikely to match any seeded provider
		const unknown = 'no-such-provider-9f8e7d6c5b4a';
		await page.goto(`/dashboard/providers/${unknown}`);

		// Breadcrumb confirms the route mounted
		await expect(page.getByRole('link', { name: 'Marketplace' }).first()).toBeVisible();

		// The unique "Provider Not Found" card heading + body copy
		await expect(page.getByRole('heading', { name: 'Provider Not Found' })).toBeVisible();
		await expect(page.getByText(`Provider not found: ${unknown}`)).toBeVisible();

		// Recovery link back to marketplace
		await expect(page.getByRole('link', { name: 'Back to Marketplace' })).toBeVisible();
	});

	test('renders the breadcrumb to the Marketplace', async ({ page }) => {
		await page.goto('/dashboard/providers/unknown');

		// Scope to <main> so we don't match the sidebar Marketplace nav item.
		const breadcrumb = page.getByRole('main').locator('nav').getByRole('link', { name: 'Marketplace' });
		await expect(breadcrumb).toBeVisible();
		await expect(breadcrumb).toHaveAttribute('href', '/dashboard/marketplace');
	});
});
