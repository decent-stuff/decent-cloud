import { test, expect } from './fixtures/test-account';

test.describe('Public Profile page', () => {
	test('profile editor renders for authenticated user (not Login Required)', async ({
		page,
	}) => {
		await page.goto('/dashboard/account/profile');

		// The page must NOT show the auth-required card for a logged-in user.
		await expect(
			page.locator('text=Login Required'),
		).not.toBeVisible({ timeout: 10000 });

		// The profile editor must render with its fields.
		await expect(page.locator('h2:has-text("Basic Information")')).toBeVisible();
		await expect(page.locator('#display-name')).toBeVisible();
		await expect(page.locator('#bio')).toBeVisible();
		await expect(
			page.locator('button:has-text("Save Profile")'),
		).toBeVisible();
	});

	test('profile edit persists after save and reload', async ({ page }) => {
		await page.goto('/dashboard/account/profile');
		await expect(page.locator('#display-name')).toBeVisible({
			timeout: 10000,
		});

		const name = `TestUser-${Date.now()}`;
		const bio = `Bio at ${Date.now()}`;

		await page.locator('#display-name').fill(name);
		await page.locator('#bio').fill(bio);
		await page.locator('button:has-text("Save Profile")').click();

		await expect(
			page.locator('text=Profile updated successfully'),
		).toBeVisible({ timeout: 10000 });

		// Reload and verify persistence.
		await page.reload();
		await expect(page.locator('#display-name')).toHaveValue(name);
		await expect(page.locator('#bio')).toHaveValue(bio);
	});
});
