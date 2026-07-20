import { test, expect } from '@playwright/test';

/**
 * E2E coverage for /dashboard/user/[identifier].
 *
 * Anonymous-OK user-activity route. When the identifier resolves to
 * no known user, the page renders a unique "User Not Found" card —
 * a stable structural state we assert without seeding.
 */

test.describe('/dashboard/user/[identifier]', () => {
	test('renders the User Info header and "User Not Found" card for an unknown name', async ({ page }) => {
		// Random username unlikely to collide with seeded accounts
		const unknown = 'no-such-user-9f8e7d6c5b4a';
		await page.goto(`/dashboard/user/${unknown}`);

		// Page heading is always rendered, even on the error branch
		await expect(page.getByRole('heading', { name: 'User Info', exact: true })).toBeVisible();

		// The unique error card with the echoed identifier
		await expect(page.getByRole('heading', { name: 'User Not Found' })).toBeVisible();
		await expect(page.getByText(/was not found in the system/i)).toBeVisible();

		// Recovery link back to marketplace (unique to this branch)
		await expect(page.getByRole('link', { name: /Back to Marketplace/i })).toBeVisible();
	});

	test('explains the possible reasons for not finding a user', async ({ page }) => {
		// The Not Found card lists three explanations; assert the unique copy.
		await page.goto('/dashboard/user/unknown-identifier');

		await expect(page.getByText(/hasn't created any offerings or contracts/i)).toBeVisible();
		await expect(page.getByText('The username or pubkey is incorrect')).toBeVisible();
		await expect(page.getByText('The user is new to the platform')).toBeVisible();
	});
});
