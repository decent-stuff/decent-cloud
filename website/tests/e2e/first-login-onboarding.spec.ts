import { test, expect } from './fixtures/test-account';

test.describe('First login onboarding', () => {
	test('@smoke guides a new user through all onboarding steps once', async ({ page }) => {
		await page.route('**/api/v1/accounts/*/external-keys', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({ success: true, data: [] }),
			});
		});

		await page.goto('/dashboard');

		await expect(page.getByRole('heading', { name: 'Complete your profile' })).toBeVisible();

		const modal = page.locator('.fixed.inset-0.z-50');
		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Add your SSH key' })).toBeVisible();
		await expect(page.getByText('No SSH key found yet. Add one in Security settings.')).toBeVisible();

		await modal.getByRole('button', { name: 'Continue' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).toBeVisible();

		await modal.getByRole('button', { name: 'Stay on dashboard' }).click();
		await expect(page.getByRole('heading', { name: 'Choose your next action' })).not.toBeVisible();

		await page.reload();
		await expect(page.getByRole('heading', { name: 'Complete your profile' })).not.toBeVisible();
	});
});
